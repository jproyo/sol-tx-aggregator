/// Module containing the application layer implementation.
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

/// Defines the interface for the application layer.
#[async_trait::async_trait]
pub trait Application {
    /// Runs the Solana blockchain aggregator.
    ///
    /// # Arguments
    /// * `endpoint` - The Solana RPC endpoint URL.
    /// * `shutdown` - A broadcast channel for signaling shutdown.
    ///
    /// # Returns
    /// A Result containing () on success, or an AggregatorError on failure.
    async fn run_aggregator(
        &self,
        endpoint: &str,
        shutdown: broadcast::Sender<()>,
    ) -> Result<(), AggregatorError>;

    /// Retrieves all transactions from the database.
    ///
    /// # Returns
    /// A Result containing a vector of Transactions on success, or a DataStorageError on failure.
    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all accounts from the database.
    ///
    /// # Returns
    /// A Result containing a vector of Accounts on success, or a DataStorageError on failure.
    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError>;

    /// Retrieves a specific account by its public key.
    ///
    /// # Arguments
    /// * `address` - The public key of the account to retrieve.
    ///
    /// # Returns
    /// A Result containing the Account on success, or a DataStorageError on failure.
    async fn get_account(&self, address: Pubkey) -> Result<Account, DataStorageError>;

    /// Retrieves a specific transaction by its ID.
    ///
    /// # Arguments
    /// * `id` - The ID of the transaction to retrieve.
    ///
    /// # Returns
    /// A Result containing the Transaction on success, or a DataStorageError on failure.
    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError>;

    /// Retrieves all transactions sent by a specific address.
    ///
    /// # Arguments
    /// * `sender` - The public key of the sender.
    ///
    /// # Returns
    /// A Result containing a vector of Transactions on success, or a DataStorageError on failure.
    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all transactions received by a specific address.
    ///
    /// # Arguments
    /// * `receiver` - The public key of the receiver.
    ///
    /// # Returns
    /// A Result containing a vector of Transactions on success, or a DataStorageError on failure.
    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all transactions processed in a specific slot.
    ///
    /// # Arguments
    /// * `slot` - The slot number.
    ///
    /// # Returns
    /// A Result containing a vector of Transactions on success, or a DataStorageError on failure.
    async fn get_transactions_by_slot(
        &self,
        slot: u64,
    ) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all transactions processed on a specific date.
    ///
    /// # Arguments
    /// * `date` - The date in string format.
    ///
    /// # Returns
    /// A Result containing a vector of Transactions on success, or a DataStorageError on failure.
    async fn get_transactions_by_date(
        &self,
        date: String,
    ) -> Result<Vec<Transaction>, DataStorageError>;
}

/// Represents the main application struct.
#[derive(Clone)]
pub struct App<D> {
    /// The database used for storing and retrieving data.
    database: Arc<D>,
}

impl Default for App<InMemoryDatabase> {
    /// Creates a default App instance with an in-memory database.
    fn default() -> Self {
        Self::new()
    }
}

impl App<InMemoryDatabase> {
    /// Creates a new App instance with an in-memory database.
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
    /// Runs the Solana blockchain aggregator.
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

    /// Retrieves all transactions from the database.
    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting all transactions ...");
        self.database.get_transactions().await
    }

    /// Retrieves all accounts from the database.
    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError> {
        tracing::info!("Getting all accounts ...");
        self.database.get_accounts().await
    }

    /// Retrieves a specific account by its public key.
    async fn get_account(&self, address: Pubkey) -> Result<Account, DataStorageError> {
        tracing::info!("Getting account by address: {}", address);
        self.database.get_account(address).await
    }

    /// Retrieves a specific transaction by its ID.
    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError> {
        tracing::info!("Getting transaction by id: {}", id);
        self.database.get_transaction(id).await
    }

    /// Retrieves all transactions sent by a specific address.
    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transaction by sender {}", sender);
        self.database.get_transactions_by_sender(sender).await
    }

    /// Retrieves all transactions received by a specific address.
    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transaction by receiver {}", receiver);
        self.database.get_transactions_by_receiver(receiver).await
    }

    /// Retrieves all transactions processed in a specific slot.
    async fn get_transactions_by_slot(
        &self,
        slot: u64,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transaction by slot {}", slot);
        self.database.get_transactions_by_slot(slot).await
    }

    /// Retrieves all transactions processed on a specific date.
    async fn get_transactions_by_date(
        &self,
        date: String,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        tracing::info!("Getting transactions by date {}", date);
        self.database.get_transactions_by_date(date).await
    }
}
