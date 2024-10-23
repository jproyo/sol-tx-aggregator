use super::shutdown::Shutdown;
use crate::domain::{
    errors::NotifierError,
    models::{Account, DataStorage, Notifier, Transaction},
};
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::mpsc;

/// A notifier implementation that uses channels to communicate updates.
///
/// This struct provides a way to notify about new transactions and account updates
/// using tokio channels. It spawns separate tasks to listen for these updates and
/// store them in the provided database.
#[derive(Clone)]
pub struct ChannelNotifier<D, S> {
    /// Channel sender for new transactions
    transactions: mpsc::Sender<Transaction>,
    /// Channel sender for account updates
    accounts: mpsc::Sender<Account>,
    /// Phantom data to hold the shutdown type
    _shutdown: PhantomData<S>,
    /// Phantom data to hold the database type
    _phantom: PhantomData<D>,
}

impl<D, S> ChannelNotifier<D, S>
where
    D: DataStorage + Send + Sync + 'static,
    S: Shutdown + Send + Sync + 'static + Clone,
{
    /// Creates a new ChannelNotifier instance.
    ///
    /// This method sets up the channels and spawns listener tasks for both
    /// transactions and accounts.
    ///
    /// # Arguments
    ///
    /// * `database` - An Arc-wrapped instance of the database implementation
    /// * `shutdown` - An instance of the shutdown mechanism
    ///
    /// # Returns
    ///
    /// A new instance of ChannelNotifier
    pub fn new(database: Arc<D>, shutdown: S) -> Self {
        let (tx_transactions, rx_transactions) = mpsc::channel(100);
        let (tx_accounts, rx_accounts) = mpsc::channel(100);

        listen_for_transactions(rx_transactions, database.clone(), shutdown.clone());
        listen_for_accounts(rx_accounts, database.clone(), shutdown);

        Self {
            transactions: tx_transactions,
            accounts: tx_accounts,
            _shutdown: PhantomData,
            _phantom: PhantomData,
        }
    }
}

/// Spawns a task to listen for account updates and store them in the database.
///
/// This function creates an asynchronous task that continuously listens for account
/// updates on the provided channel. It will store each received account in the database
/// until a shutdown signal is received.
///
/// # Arguments
///
/// * `rx_accounts` - The receiving end of the account update channel
/// * `database` - An Arc-wrapped instance of the database implementation
/// * `shutdown` - An instance of the shutdown mechanism
fn listen_for_accounts<D, S>(
    mut rx_accounts: mpsc::Receiver<Account>,
    database: Arc<D>,
    shutdown: S,
) where
    D: DataStorage + Send + Sync + 'static,
    S: Shutdown + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut shutdown_recv = shutdown.subscribe();
        loop {
            tokio::select! {
                _ = shutdown_recv.recv() => {
                    tracing::info!("Received shutdown signal, stopping account listener");
                    break;
                }
                Some(account) = rx_accounts.recv() => {
                    database.store_account(account).await.unwrap();
                }
            }
        }
    });
}

/// Spawns a task to listen for new transactions and store them in the database.
///
/// This function creates an asynchronous task that continuously listens for new
/// transactions on the provided channel. It will store each received transaction
/// in the database until a shutdown signal is received.
///
/// # Arguments
///
/// * `rx_transactions` - The receiving end of the new transaction channel
/// * `database` - An Arc-wrapped instance of the database implementation
/// * `shutdown` - An instance of the shutdown mechanism
fn listen_for_transactions<D, S>(
    mut rx_transactions: mpsc::Receiver<Transaction>,
    database: Arc<D>,
    shutdown: S,
) where
    D: DataStorage + Send + Sync + 'static,
    S: Shutdown + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut shutdown_recv = shutdown.subscribe();
        loop {
            tokio::select! {
                _ = shutdown_recv.recv() => {
                    tracing::info!("Received shutdown signal, stopping transaction listener");
                    break;
                }
                Some(transaction) = rx_transactions.recv() => {
                    database.store_transaction(transaction).await.unwrap();
                }
            }
        }
    });
}

#[async_trait::async_trait]
impl<D, S> Notifier for ChannelNotifier<D, S>
where
    D: DataStorage + Send + Sync + 'static + Clone,
    S: Shutdown + Send + Sync + 'static,
{
    /// Notifies the system about a new transaction.
    ///
    /// This method sends the new transaction through the channel to be processed
    /// and stored in the database.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The new transaction to be notified
    ///
    /// # Returns
    ///
    /// A Result indicating success or a NotifierError if the notification failed
    async fn notify_transaction(&self, transaction: Transaction) -> Result<(), NotifierError> {
        self.transactions.send(transaction).await?;
        Ok(())
    }

    /// Notifies the system about an account update.
    ///
    /// This method sends the account update through the channel to be processed
    /// and stored in the database.
    ///
    /// # Arguments
    ///
    /// * `account` - The updated account information to be notified
    ///
    /// # Returns
    ///
    /// A Result indicating success or a NotifierError if the notification failed
    async fn notify_account(&self, account: Account) -> Result<(), NotifierError> {
        self.accounts.send(account).await?;
        Ok(())
    }
}
