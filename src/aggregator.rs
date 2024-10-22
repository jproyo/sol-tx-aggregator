use crate::models::{Account, Transaction};
use anyhow::Result;
use futures::{sink::SinkExt, stream::StreamExt};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use yellowstone_grpc_client::{GeyserGrpcBuilder, GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::{prelude::*, tonic::transport::ClientTlsConfig};

pub struct Aggregator {
    endpoint: String,
    api_key: String,
    transactions: Arc<RwLock<Vec<Transaction>>>,
    accounts: Arc<RwLock<Vec<Account>>>,
}

impl Aggregator {
    pub async fn new() -> Result<Self> {
        let endpoint = "https://devnet.helius-rpc.com";
        let api_key = "883a58ea-8640-456c-ad09-802120787faf";
        Ok(Self {
            endpoint: endpoint.to_string(),
            api_key: api_key.to_string(),
            transactions: Arc::new(RwLock::new(Vec::new())),
            accounts: Arc::new(RwLock::new(Vec::new())),
        })
    }

    async fn connect(&self) -> Result<GeyserGrpcClient<impl Interceptor>> {
        GeyserGrpcBuilder::from_shared(self.endpoint.clone())?
            .x_token(Some(self.api_key.clone()))?
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .tls_config(ClientTlsConfig::default())?
            .connect()
            .await
            .map_err(anyhow::Error::from)
    }

    pub async fn run(&self) -> Result<()> {
        let (mut subscribe_tx, mut stream) = self.connect().await?.subscribe().await?;

        subscribe_tx
            .send(SubscribeRequest {
                slots: HashMap::new(),
                accounts: HashMap::new(),
                transactions: HashMap::new(),
                transactions_status: HashMap::new(),
                entry: HashMap::new(),
                blocks: vec![(
                    "blocks".to_string(),
                    SubscribeRequestFilterBlocks {
                        account_include: vec![],
                        include_transactions: Some(true),
                        include_accounts: Some(true),
                        include_entries: Some(false),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
                blocks_meta: HashMap::new(),
                commitment: Some(CommitmentLevel::Confirmed as i32),
                accounts_data_slice: vec![],
                ping: None,
            })
            .await?;

        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => match msg.update_oneof {
                    Some(UpdateOneof::Block(block)) => {
                        self.process_block(block).await?;
                    }
                    _ => {
                        tracing::warn!(
                            "Non expected - Non subscribe message: {:?}",
                            msg.update_oneof
                        );
                    }
                },
                Err(error) => {
                    tracing::error!("stream error: {error:?}");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn process_block(&self, block: SubscribeUpdateBlock) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        let mut accounts = self.accounts.write().await;

        // Process transactions
        for tx in block.transactions {
            transactions.push(Transaction {
                id: tx.signature.clone(),
                sender: tx
                    .transaction
                    .as_ref()
                    .unwrap()
                    .message
                    .as_ref()
                    .unwrap()
                    .account_keys[0]
                    .clone(),
                receiver: tx
                    .transaction
                    .as_ref()
                    .unwrap()
                    .message
                    .as_ref()
                    .unwrap()
                    .account_keys[1]
                    .clone(),
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
