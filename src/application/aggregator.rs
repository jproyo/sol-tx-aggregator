use super::Aggregator;
use crate::infrastructure::bc_client::BcClient;
use crate::{
    domain::{
        errors::AggregatorError,
        models::{Account, Notifier, Transaction},
    },
    infrastructure::shutdown::Shutdown,
};
use solana_sdk::{
    instruction::CompiledInstruction, pubkey::Pubkey, system_instruction::SystemInstruction,
    transaction::VersionedTransaction,
};
use solana_transaction_status::UiTransactionStatusMeta;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::task::JoinSet;
use typed_builder::TypedBuilder;

#[derive(Clone, TypedBuilder)]
pub struct SolanaAggregator<C, N, S> {
    bc_client: C,
    notifier: N,
    shutdown: S,
}

#[async_trait::async_trait]
impl<C, N, S> Aggregator for SolanaAggregator<C, N, S>
where
    C: BcClient + Send + Sync + 'static + Clone,
    N: Notifier + Send + Sync + 'static + Clone,
    S: Shutdown + Send + Sync + 'static,
{
    async fn run(self) -> Result<(), AggregatorError> {
        let mut last_processed_slot = self.bc_client.get_current_slot().await?;
        let mut shutdown = self.shutdown.subscribe();

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    tracing::info!("Received shutdown signal, stopping aggregator");
                    break;
                }
                _ = async {
                    let current_slot = self.bc_client.get_current_slot().await?;
                    let blocks = self
                        .bc_client
                        .get_blocks(last_processed_slot + 1, current_slot)
                        .await?;

                    let mut join_set = JoinSet::new();

                    for slot in blocks {
                        let client = self.bc_client.clone();
                        let notifier = self.notifier.clone();
                        join_set.spawn(async move {
                            match Self::process_block(client, notifier, slot).await {
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

                    Ok::<(), AggregatorError>(())
                } => {}
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }
}

struct TransactionDetails<'a> {
    tx_id: &'a str,
    instruction: &'a CompiledInstruction,
    account_keys: &'a [Pubkey],
    lamports: u64,
    meta: &'a UiTransactionStatusMeta,
    block_time: Option<i64>,
    slot: u64,
}

impl<'a> TransactionDetails<'a> {
    fn into_transaction(self) -> Transaction {
        let instruction = self.instruction;
        let account_keys = self.account_keys;
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
        Transaction {
            id: self.tx_id.to_string(),
            sender,
            receiver,
            amount: self.lamports,
            fee: self.meta.fee,
            slot: self.slot,
            timestamp: self.block_time.unwrap_or_else(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs() as i64
            }),
        }
    }
}

impl<C, N, S> SolanaAggregator<C, N, S>
where
    C: BcClient,
    N: Notifier + Clone,
{
    async fn process_block(bc_client: C, notifier: N, slot: u64) -> Result<(), AggregatorError> {
        let block = bc_client.get_block(slot).await?;

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
                                        let tx_details = TransactionDetails {
                                            tx_id: &tx_id,
                                            instruction,
                                            account_keys: &account_keys,
                                            lamports,
                                            meta,
                                            block_time: block.block_time,
                                            slot,
                                        };
                                        let transaction = tx_details.into_transaction();
                                        notifier.notify_transaction(transaction).await?;
                                    }
                                    SystemInstruction::CreateAccount { lamports, .. } => {
                                        Self::process_create_account(
                                            notifier.clone(),
                                            instruction,
                                            &account_keys,
                                            lamports,
                                            slot,
                                        )
                                        .await?;
                                    }
                                    _ => continue,
                                }
                            }
                        }
                        Self::update_account_balances(notifier.clone(), &account_keys, meta, slot)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_create_account(
        notifier: N,
        instruction: &CompiledInstruction,
        account_keys: &[Pubkey],
        lamports: u64,
        slot: u64,
    ) -> Result<(), AggregatorError> {
        let new_account = account_keys[instruction.accounts[1] as usize];
        let account = Account {
            address: new_account,
            balance: lamports,
            last_slot_updated: slot,
        };

        notifier.notify_account(account).await?;

        Ok(())
    }

    async fn update_account_balances(
        notifier: N,
        account_keys: &[Pubkey],
        meta: &UiTransactionStatusMeta,
        slot: u64,
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
                    last_slot_updated: slot,
                };

                notifier.notify_account(account).await?;
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
