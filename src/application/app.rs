use std::sync::Arc;

use super::aggregator::SolanaAggregator;
use super::Aggregator;
use crate::domain::errors::{AggregatorError, DataStorageError};
use crate::domain::models::{Account, DataStorage, Transaction};
use crate::infrastructure::channel_notifier::ChannelNotifier;
use crate::infrastructure::memory::InMemoryDatabase;
use crate::infrastructure::shutdown::ShutdownChannel;
use crate::infrastructure::solana_client::SolanaClient;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::broadcast;

#[async_trait::async_trait]
pub trait Application {
    async fn run_aggregator(
        &self,
        endpoint: &str,
        shutdown: broadcast::Sender<()>,
    ) -> Result<(), AggregatorError>;
    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError>;
    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError>;
    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError>;
    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError>;
    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError>;
    async fn get_transactions_by_slot(
        &self,
        slot: u64,
    ) -> Result<Vec<Transaction>, DataStorageError>;
    async fn get_transactions_by_date(
        &self,
        date: String,
    ) -> Result<Vec<Transaction>, DataStorageError>;
}

#[derive(Clone)]
pub struct App<D> {
    database: Arc<D>,
}

impl App<InMemoryDatabase> {
    pub fn new() -> Self {
        let database = Arc::new(InMemoryDatabase::default());
        Self { database }
    }
}

#[async_trait::async_trait]
impl<D> Application for App<D>
where
    D: DataStorage + Clone + Send + Sync + 'static,
{
    async fn run_aggregator(
        &self,
        endpoint: &str,
        shutdown: broadcast::Sender<()>,
    ) -> Result<(), AggregatorError> {
        let shutdown_channel = ShutdownChannel::new(shutdown);
        let bc_client = SolanaClient::from_url(endpoint);
        let aggregator = SolanaAggregator::builder()
            .bc_client(bc_client)
            .notifier(ChannelNotifier::new(
                self.database.clone(),
                shutdown_channel.clone(),
            ))
            .shutdown(shutdown_channel)
            .build();
        tracing::info!("Running aggregator ...");
        aggregator.run().await
    }

    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting all transactions ...");
        self.database.get_transactions().await
    }

    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError> {
        tracing::info!("Getting all accounts ...");
        self.database.get_accounts().await
    }

    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError> {
        tracing::info!("Getting transaction by id: {}", id);
        self.database.get_transaction(id).await
    }

    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transaction by sender {}", sender);
        self.database.get_transactions_by_sender(sender).await
    }

    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transaction by receiver {}", receiver);
        self.database.get_transactions_by_receiver(receiver).await
    }

    async fn get_transactions_by_slot(
        &self,
        slot: u64,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transaction by slot {}", slot);
        self.database.get_transactions_by_slot(slot).await
    }

    async fn get_transactions_by_date(
        &self,
        date: String,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transactions by date {}", date);
        self.database.get_transactions_by_date(date).await
    }
}
