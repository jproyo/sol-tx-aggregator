use crate::domain::{
    errors::NotifierError,
    models::{Account, DataStorage, Notifier, Transaction},
};
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::mpsc;

use super::shutdown::Shutdown;

#[derive(Clone)]
pub struct ChannelNotifier<D, S> {
    transactions: mpsc::Sender<Transaction>,
    accounts: mpsc::Sender<Account>,
    _shutdown: PhantomData<S>,
    _phantom: PhantomData<D>,
}

impl<D, S> ChannelNotifier<D, S>
where
    D: DataStorage + Send + Sync + 'static,
    S: Shutdown + Send + Sync + 'static + Clone,
{
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
    async fn notify_transaction(&self, transaction: Transaction) -> Result<(), NotifierError> {
        self.transactions.send(transaction).await?;
        Ok(())
    }

    async fn notify_account(&self, account: Account) -> Result<(), NotifierError> {
        self.accounts.send(account).await?;
        Ok(())
    }
}
