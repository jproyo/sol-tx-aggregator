use crate::models::{Account, Transaction};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use yellowstone_grpc_client::{GeyserGrpcBuilder, GeyserGrpcClient};
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::{prelude::*, tonic::transport::ClientTlsConfig};

pub struct Aggregator {
    client: GeyserGrpcBuilder,
    transactions: Arc<RwLock<Vec<Transaction>>>,
    accounts: Arc<RwLock<Vec<Account>>>,
}

impl Aggregator {
    pub async fn new() -> Result<Self> {
        let endpoint = "https://devnet.helius-rpc.com";
        let api_key = "883a58ea-8640-456c-ad09-802120787faf";
        let client = GeyserGrpcClient::build_from_shared(endpoint)?
            .x_token(Some(api_key))?
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .tls_config(ClientTlsConfig::default())?;

        Ok(Self {
            client,
            transactions: Arc::new(RwLock::new(Vec::new())),
            accounts: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut block_stream = self
            .client
            .connect()
            .await?
            .subscribe_with_request(Some(SubscribeRequest {
                accounts: HashMap::new(),
                blocks: HashMap::new(),
                accounts_data_slice: vec![],
                blocks_meta: HashMap::new(),
                commitment: None,
                entry: HashMap::new(),
                ping: None,
                slots: HashMap::new(),
                transactions: HashMap::new(),
                transactions_status: HashMap::new(),
            }))
            .await?;

        while let Some(block_update) = block_stream.message().await? {
            match block_update.block {
                Some(block) => {
                    self.process_block(block).await?;
                }
                None => {
                    tracing::warn!("Received empty block update");
                }
            }
        }

        Ok(())
    }

    async fn process_block(&self, block: UpdateOneof) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        let mut accounts = self.accounts.write().await;

        // Process transactions
        for tx in block.transactions {
            transactions.push(Transaction {
                id: tx.signature,
                sender: tx.message.unwrap().account_keys[0].clone(),
                receiver: tx.message.unwrap().account_keys[1].clone(),
                amount: 0, // You'll need to extract this from the instruction data
                timestamp: block.block_time.unwrap().timestamp,
            });
        }

        // Process account updates
        for account in block.accounts {
            accounts.push(Account {
                address: account.pubkey,
                balance: account.lamports,
            });
        }

        Ok(())
    }

    pub async fn get_transactions(&self) -> Vec<Transaction> {
        self.transactions.read().await.clone()
    }

    pub async fn get_accounts(&self) -> Vec<Account> {
        self.accounts.read().await.clone()
    }
}
