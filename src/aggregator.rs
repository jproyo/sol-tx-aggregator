use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::{Account, Transaction};

pub struct Aggregator {
    client: RpcClient,
    transactions: Arc<RwLock<Vec<Transaction>>>,
    accounts: Arc<RwLock<Vec<Account>>>,
}

impl Aggregator {
    pub async fn new() -> Result<Self> {
        let rpc_url = "https://api.devnet.solana.com".to_string(); // Use devnet for this example
        let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        Ok(Self {
            client,
            transactions: Arc::new(RwLock::new(Vec::new())),
            accounts: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            self.fetch_and_process_data().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }

    async fn fetch_and_process_data(&self) -> Result<()> {
        // Implement logic to fetch and process blockchain data
        // This is a placeholder and needs to be implemented
        Ok(())
    }

    pub async fn get_transactions(&self) -> Vec<Transaction> {
        self.transactions.read().await.clone()
    }

    pub async fn get_accounts(&self) -> Vec<Account> {
        self.accounts.read().await.clone()
    }
}
