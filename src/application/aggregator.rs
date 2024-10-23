use super::Aggregator;
use crate::domain::{
    errors::AggregatorError,
    models::{Account, Notifier, Transaction},
};
use crate::infrastructure::bc_client::BcClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::CompiledInstruction, pubkey::Pubkey,
    system_instruction::SystemInstruction, transaction::VersionedTransaction,
};
use solana_transaction_status::UiTransactionStatusMeta;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::task::JoinSet;
use typed_builder::TypedBuilder;

#[derive(Clone, TypedBuilder)]
pub struct SolanaAggregator<C, N> {
    bc_client: C,
    notifier: N,
}

#[async_trait::async_trait]
impl<C, N> Aggregator for SolanaAggregator<C, N>
where
    C: BcClient + Send + Sync + 'static + Clone,
    N: Notifier + Send + Sync + 'static + Clone,
{
    async fn run(&self) -> Result<(), AggregatorError> {
        let mut last_processed_slot = self.bc_client.get_current_slot().await?;

        loop {
            let current_slot = self.bc_client.get_current_slot().await?;
            let blocks = self
                .bc_client
                .get_blocks(last_processed_slot + 1, current_slot)
                .await?;

            let mut join_set = JoinSet::new();

            for slot in blocks {
                let aggregator = self.clone();
                join_set.spawn(async move {
                    match aggregator.process_block(slot).await {
                        Ok(_) => {
                            tracing::info!("Successfully processed block {}", slot);
                            Ok(slot)
                        }
                        Err(e) => {
                            tracing::error!("Error processing block {}: {:?}", slot, e);
                            Err(slot)
                        }
                    }
                });
            }

            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(Ok(slot)) => {
                        last_processed_slot = last_processed_slot.max(slot);
                    }
                    Ok(Err(slot)) => {
                        tracing::error!("Failed to process block {}. Retrying on next loop", slot);
                    }
                    Err(e) => {
                        tracing::error!("JoinSet error: {:?}", e);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

impl<C: BcClient, N: Notifier> SolanaAggregator<C, N> {
    //let (tx_sender, mut tx_receiver) = mpsc::channel::<Transaction>(1000);
    //let (account_sender, mut account_receiver) = mpsc::channel::<Account>(1000);

    // Spawn a task to handle transactions
    //tokio::spawn(async move {
    //    while let Some(transaction) = tx_receiver.recv().await {
    //        // Process the transaction
    //        println!("Received transaction: {}", transaction.id);
    //    }
    //});

    // Spawn a task to handle account updates
    //tokio::spawn(async move {
    //    while let Some(account) = account_receiver.recv().await {
    //        // Process the account update
    //        println!("Received account update: {}", account.address);
    //    }
    //});

    async fn process_block(&self, slot: u64) -> Result<(), AggregatorError> {
        let block = self.bc_client.get_block(slot).await?;

        if let Some(txs) = block.transactions {
            for tx in txs {
                if let Some(decode_tx) = tx.transaction.decode() {
                    if let Some(meta) = &tx.meta {
                        let tx_id = format!("{}", decode_tx.signatures[0]);
                        let (instructions, account_keys) = Self::get_transfer_details(&decode_tx);
                        for instruction in instructions.iter() {
                            if let Ok(system_instruction) =
                                bincode::deserialize::<SystemInstruction>(&instruction.data)
                            {
                                match system_instruction {
                                    SystemInstruction::Transfer { lamports } => {
                                        self.process_transfer(
                                            &tx_id,
                                            &instruction,
                                            &account_keys,
                                            lamports,
                                            meta,
                                            block.block_time,
                                            slot,
                                        )
                                        .await?;
                                    }
                                    SystemInstruction::CreateAccount { lamports, .. } => {
                                        self.process_create_account(
                                            &instruction,
                                            &account_keys,
                                            lamports,
                                        )
                                        .await?;
                                    }
                                    _ => continue,
                                }
                            }
                        }
                        self.update_account_balances(&account_keys, meta).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_transfer(
        &self,
        tx_id: &str,
        instruction: &CompiledInstruction,
        account_keys: &[Pubkey],
        lamports: u64,
        meta: &UiTransactionStatusMeta,
        block_time: Option<i64>,
        slot: u64,
    ) -> Result<(), AggregatorError> {
        let executer = instruction.program_id_index as usize;
        let sender: Pubkey;
        let receiver: Pubkey;
        if executer == 0 {
            sender = account_keys[instruction.accounts[executer] as usize];
            receiver = account_keys[instruction.accounts[executer + 1] as usize];
        } else {
            sender = account_keys[instruction.accounts[0] as usize];
            receiver = account_keys[executer];
        }
        let transaction = Transaction {
            id: tx_id.to_string(),
            sender,
            receiver,
            amount: lamports,
            fee: meta.fee,
            slot,
            timestamp: block_time.unwrap_or_else(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs() as i64
            }),
        };

        self.notifier.notify_transaction(transaction).await?;

        Ok(())
    }

    async fn process_create_account(
        &self,
        instruction: &CompiledInstruction,
        account_keys: &[Pubkey],
        lamports: u64,
    ) -> Result<(), AggregatorError> {
        let new_account = account_keys[instruction.accounts[1] as usize];
        let account = Account {
            address: new_account,
            balance: lamports,
        };

        self.notifier.notify_account(account).await?;

        Ok(())
    }

    async fn update_account_balances(
        &self,
        account_keys: &[Pubkey],
        meta: &UiTransactionStatusMeta,
    ) -> Result<(), AggregatorError> {
        for (idx, (pre_balance, post_balance)) in meta
            .pre_balances
            .iter()
            .zip(meta.post_balances.iter())
            .enumerate()
        {
            if pre_balance != post_balance {
                let account_key = account_keys[idx];
                let account = Account {
                    address: account_key,
                    balance: *post_balance,
                };

                self.notifier.notify_account(account).await?;
            }
        }
        Ok(())
    }

    fn get_transfer_details(
        transaction: &VersionedTransaction,
    ) -> (Vec<CompiledInstruction>, Vec<Pubkey>) {
        let message = transaction.message.clone();

        match message {
            solana_sdk::message::VersionedMessage::Legacy(msg) => {
                (msg.instructions, msg.account_keys)
            }
            solana_sdk::message::VersionedMessage::V0(msg) => (msg.instructions, msg.account_keys),
        }
    }
}
