use crate::models::{Account, Transaction};
use anyhow::Result;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::{RpcBlockConfig, RpcBlockSubscribeConfig, RpcBlockSubscribeFilter},
};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, system_instruction::SystemInstruction,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{
    EncodedConfirmedBlock, EncodedTransactionWithStatusMeta, TransactionDetails, UiConfirmedBlock,
    UiTransactionEncoding,
};
use std::{str::FromStr, sync::Arc};
use tokio::sync::RwLock;

pub struct Aggregator {
    transactions: Arc<RwLock<Vec<Transaction>>>,
    accounts: Arc<RwLock<Vec<Account>>>,
    client: RpcClient,
}

impl Aggregator {
    pub async fn new() -> Result<Self> {
        let endpoint =
            "https://devnet.helius-rpc.com/?api-key=883a58ea-8640-456c-ad09-802120787faf"
                .to_string();
        let client = RpcClient::new(endpoint);
        Ok(Self {
            client,
            transactions: Arc::new(RwLock::new(Vec::new())),
            accounts: Arc::new(RwLock::new(Vec::new())),
        })
    }
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Subscribing to blocks");
        let slot = self.client.get_slot().await?;
        tracing::info!("Current slot: {}", slot);
        let blocks = self
            .client
            .get_blocks_with_commitment(slot - 10, None, CommitmentConfig::confirmed())
            .await?;
        tracing::info!("Blocks: {:?}", blocks);
        for slot_block in blocks {
            let block = self
                .client
                .get_block_with_config(
                    slot_block,
                    RpcBlockConfig {
                        transaction_details: Some(TransactionDetails::Full),
                        encoding: Some(UiTransactionEncoding::Binary),
                        rewards: Some(false),
                        commitment: Some(CommitmentConfig::confirmed()),
                        max_supported_transaction_version: Some(0),
                        ..Default::default()
                    },
                )
                .await?;
            self.process_block(block).await?;
            tracing::info!("Transactions: {:?}", self.transactions.read().await.len());
            tracing::info!("Accounts: {:?}", self.accounts.read().await.len());
        }

        Ok(())
    }

    async fn process_block(&self, block: UiConfirmedBlock) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        let mut accounts = self.accounts.write().await;

        // Process transactions
        if let Some(txs) = block.transactions {
            for tx in txs {
                if let Some(decode_tx) = tx.transaction.decode() {
                    let hash = decode_tx.verify_and_hash_message()?;
                    if let Some((sender, receiver, amount)) = Self::get_transfer_details(&decode_tx)
                    {
                        transactions.push(Transaction {
                            id: hash.to_bytes(),
                            sender,
                            receiver,
                            amount,
                            timestamp: block.block_time.unwrap(),
                        });
                    }
                }

                if let Some(meta) = &tx.meta {
                    for (account_index, account_key) in
                        Self::get_accounts_keys(&tx)?.iter().enumerate()
                    {
                        if let Some(post_balance) = meta.post_balances.get(account_index) {
                            if let Some(pre_balance) = meta.pre_balances.get(account_index) {
                                if post_balance != pre_balance {
                                    accounts.push(Account {
                                        address: *account_key,
                                        balance: *post_balance,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_accounts_keys(transaction: &EncodedTransactionWithStatusMeta) -> Result<Vec<Pubkey>> {
        if let Some(message) = transaction.transaction.decode() {
            return match message.message {
                solana_sdk::message::VersionedMessage::Legacy(msg) => Ok(msg.account_keys),
                solana_sdk::message::VersionedMessage::V0(msg) => Ok(msg.account_keys),
            };
        } else {
            match &transaction.transaction {
                solana_transaction_status::EncodedTransaction::Json(json_tx) => {
                    let accounts = match &json_tx.message {
                        solana_transaction_status::UiMessage::Parsed(parsed_message) => {
                            parsed_message
                                .account_keys
                                .iter()
                                .map(|a| a.pubkey.clone())
                                .collect()
                        }
                        solana_transaction_status::UiMessage::Raw(raw_message) => {
                            raw_message.account_keys.clone()
                        }
                    };
                    accounts
                        .iter()
                        .map(|a| Pubkey::from_str(a.as_str()))
                        .collect::<Result<Vec<Pubkey>, _>>()
                        .map_err(|_| anyhow::anyhow!("Failed to parse account key"))
                }
                solana_transaction_status::EncodedTransaction::Accounts(accounts) => accounts
                    .account_keys
                    .iter()
                    .map(|a| Pubkey::from_str(a.pubkey.as_str()))
                    .collect::<Result<Vec<Pubkey>, _>>()
                    .map_err(|_| anyhow::anyhow!("Failed to parse account key")),
                _ => Ok(vec![]),
            }
        }
    }

    fn get_transfer_details(transaction: &VersionedTransaction) -> Option<(Pubkey, Pubkey, u64)> {
        let message = transaction.message.clone();

        let (instructions, account_keys) = match message {
            solana_sdk::message::VersionedMessage::Legacy(msg) => {
                (msg.instructions, msg.account_keys)
            }
            solana_sdk::message::VersionedMessage::V0(msg) => (msg.instructions, msg.account_keys),
        };

        for instruction in instructions {
            if let Ok(system_instruction) =
                bincode::deserialize::<SystemInstruction>(&instruction.data)
            {
                match system_instruction {
                    SystemInstruction::Transfer { lamports } => {
                        let sender = account_keys[instruction.accounts[0] as usize];
                        let receiver = account_keys[instruction.accounts[1] as usize];
                        return Some((sender, receiver, lamports));
                    }
                    _ => continue,
                }
            }
        }

        None
    }

    pub async fn get_transactions(&self) -> Vec<Transaction> {
        self.transactions.read().await.clone()
    }

    pub async fn get_accounts(&self) -> Vec<Account> {
        self.accounts.read().await.clone()
    }
}
