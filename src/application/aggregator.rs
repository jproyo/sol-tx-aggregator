/// Module for the Solana blockchain aggregator implementation.
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

/// Struct representing the Solana blockchain aggregator.
#[derive(Clone, TypedBuilder)]
pub struct SolanaAggregator<C, N, S> {
    /// The blockchain client used to interact with the Solana network.
    bc_client: C,
    /// The notifier used to send updates about transactions and accounts.
    notifier: N,
    /// The shutdown mechanism for gracefully stopping the aggregator.
    shutdown: S,
}

#[async_trait::async_trait]
impl<C, N, S> Aggregator for SolanaAggregator<C, N, S>
where
    C: BcClient + Send + Sync + 'static + Clone,
    N: Notifier + Send + Sync + 'static + Clone,
    S: Shutdown + Send + Sync + 'static,
{
    /// Runs the Solana aggregator, processing blocks and notifying about transactions and account updates.
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

/// Struct containing details of a transaction to be processed.
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
    /// Converts the transaction details into a Transaction struct.
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
    /// Processes a single block, extracting and notifying about transactions and account updates.
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

    /// Processes a create account instruction, notifying about the new account.
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

    /// Updates account balances based on the transaction metadata.
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

    /// Extracts transfer details from a versioned transaction.
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use solana_sdk::bs58;
    use solana_sdk::hash::Hash;
    use solana_sdk::instruction::AccountMeta;
    use solana_sdk::instruction::Instruction;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;
    use solana_transaction_status::option_serializer::OptionSerializer;
    use solana_transaction_status::Encodable;
    use solana_transaction_status::EncodedTransactionWithStatusMeta;
    use solana_transaction_status::UiCompiledInstruction;
    use solana_transaction_status::UiInnerInstructions;
    use solana_transaction_status::UiInstruction;
    use solana_transaction_status::UiTransactionEncoding;
    use tokio::sync::broadcast;

    mock! {
        BcClientClone {}
        impl Clone for BcClientClone {
            fn clone(&self) -> Self {
                self.clone()
            }
        }

        unsafe impl Send for BcClientClone {}
        unsafe impl Sync for BcClientClone {}

        #[async_trait]
        impl BcClient for BcClientClone {
            async fn get_current_slot(&self) -> Result<u64, crate::domain::errors::BcClientError>;
            async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, crate::domain::errors::BcClientError>;
            async fn get_block(&self, slot: u64) -> Result<solana_transaction_status::UiConfirmedBlock, crate::domain::errors::BcClientError>;
        }
    }

    mock! {
        NotifierClone {}
        impl Clone for NotifierClone {
            fn clone(&self) -> Self {
                self.clone()
            }
        }

        unsafe impl Send for NotifierClone {}
        unsafe impl Sync for NotifierClone {}

        #[async_trait]
        impl Notifier for NotifierClone {
            async fn notify_transaction(&self, transaction: Transaction) -> Result<(), crate::domain::errors::NotifierError>;
            async fn notify_account(&self, account: Account) -> Result<(), crate::domain::errors::NotifierError>;
        }
    }

    mock! {
        ShutdownClone {}
        impl Clone for ShutdownClone {
            fn clone(&self) -> Self {
                self.clone()
            }
        }

        unsafe impl Send for ShutdownClone {}
        unsafe impl Sync for ShutdownClone {}

        impl Shutdown for ShutdownClone {
            fn subscribe(&self) -> broadcast::Receiver<()>;
        }
    }

    #[tokio::test]
    async fn test_process_block() {
        let mut bc_client = MockBcClientClone::new();
        let mut notifier = MockNotifierClone::new();
        let slot = 12345;

        // Mock the get_block method
        bc_client
            .expect_get_block()
            .with(eq(slot))
            .times(1)
            .returning(|_| Ok(create_mock_block()));

        notifier.expect_clone().times(1).returning(|| {
            let mut cloned_notifier = MockNotifierClone::new();
            cloned_notifier
                .expect_notify_account()
                .times(2)
                .returning(|_| Ok(()));
            cloned_notifier
        });

        let result = SolanaAggregator::<MockBcClientClone, MockNotifierClone, MockShutdownClone>::process_block(bc_client, notifier, slot).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_create_account() {
        let mut notifier = MockNotifierClone::new();
        let instruction = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1],
            data: vec![],
        };
        let account_keys = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let lamports = 1000000;
        let slot = 12345;

        notifier
            .expect_notify_account()
            .times(1)
            .returning(|_| Ok(()));

        let result = SolanaAggregator::<MockBcClientClone, MockNotifierClone, MockShutdownClone>::process_create_account(
            notifier,
            &instruction,
            &account_keys,
            lamports,
            slot,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_account_balances() {
        let mut notifier = MockNotifierClone::new();
        let account_keys = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let meta = UiTransactionStatusMeta {
            status: Ok(()),
            fee: 5000,
            pre_balances: vec![1000000, 2000000],
            post_balances: vec![990000, 2010000],
            inner_instructions: OptionSerializer::none(),
            log_messages: OptionSerializer::none(),
            pre_token_balances: OptionSerializer::none(),
            post_token_balances: OptionSerializer::none(),
            rewards: OptionSerializer::none(),
            loaded_addresses: OptionSerializer::skip(),
            return_data: OptionSerializer::skip(),
            compute_units_consumed: OptionSerializer::skip(),
            err: None,
        };
        let slot = 12345;

        notifier
            .expect_notify_account()
            .times(2)
            .returning(|_| Ok(()));

        let result = SolanaAggregator::<MockBcClientClone, MockNotifierClone, MockShutdownClone>::update_account_balances(
            notifier,
            &account_keys,
            &meta,
            slot,
        )
        .await;

        assert!(result.is_ok());
    }

    fn create_mock_block() -> solana_transaction_status::UiConfirmedBlock {
        let system_instruction =
            bincode::serialize(&SystemInstruction::Transfer { lamports: 1000000 }).unwrap();
        let block_hash = Hash::default();
        let keypair = Keypair::new();
        let sender = keypair.pubkey();
        let recipient = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let instructions = Instruction::new_with_borsh(
            program_id,
            &system_instruction,
            vec![
                AccountMeta::new(sender, true),
                AccountMeta::new(recipient, false),
            ],
        );
        let message = solana_sdk::message::Message::new(&[instructions], Some(&sender));

        let mut transaction = solana_sdk::transaction::Transaction::new_unsigned(message);
        transaction.sign(&[&keypair], block_hash);

        let tx_with_meta = EncodedTransactionWithStatusMeta {
            version: None,
            transaction: transaction.encode(UiTransactionEncoding::Base64),
            meta: Some(UiTransactionStatusMeta {
                status: Ok(()),
                fee: 5000,
                pre_balances: vec![2000000, 1000000],
                post_balances: vec![995000, 2000000],
                inner_instructions: OptionSerializer::Some(vec![UiInnerInstructions {
                    index: 0,
                    instructions: vec![UiInstruction::Compiled(UiCompiledInstruction {
                        program_id_index: 0,
                        accounts: vec![0, 1],
                        data: bs58::encode(&system_instruction).into_string(),
                        stack_height: None,
                    })],
                }]),
                log_messages: OptionSerializer::Some(vec![]),
                pre_token_balances: OptionSerializer::Some(vec![]),
                post_token_balances: OptionSerializer::Some(vec![]),
                rewards: OptionSerializer::Some(vec![]),
                err: None,
                loaded_addresses: OptionSerializer::Skip,
                return_data: OptionSerializer::skip(),
                compute_units_consumed: OptionSerializer::skip(),
            }),
        };

        solana_transaction_status::UiConfirmedBlock {
            signatures: Some(
                transaction
                    .signatures
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ),
            num_reward_partitions: Some(1),
            transactions: Some(vec![tx_with_meta]),
            block_height: Some(12345),
            previous_blockhash: "".to_string(),
            blockhash: block_hash.to_string(),
            parent_slot: 12345,
            block_time: Some(1234567890),
            rewards: Some(vec![]),
        }
    }
}
