use crate::domain::{
    errors::NotifierError,
    models::{Account, DataStorage, Notifier, Transaction},
};
use std::marker::PhantomData;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct ChannelNotifier<D> {
    transactions: mpsc::Sender<Transaction>,
    accounts: mpsc::Sender<Account>,
    _phantom: PhantomData<D>,
}

impl<D> ChannelNotifier<D>
where
    D: DataStorage + Send + Sync + 'static + Clone,
{
    pub fn new(database: D) -> Self {
        let (tx_transactions, rx_transactions) = mpsc::channel(100);
        let (tx_accounts, rx_accounts) = mpsc::channel(100);

        listen_for_transactions(rx_transactions, database.clone());
        listen_for_accounts(rx_accounts, database.clone());

        Self {
            transactions: tx_transactions,
            accounts: tx_accounts,
            _phantom: PhantomData,
        }
    }
}

fn listen_for_accounts<D>(rx_accounts: mpsc::Receiver<Account>, database: D)
where
    D: DataStorage + Send + Sync + 'static + Clone,
{
    tokio::spawn(async move {
        let mut rx_accounts = rx_accounts;
        while let Some(account) = rx_accounts.recv().await {
            database.store_account(account).await.unwrap();
        }
    });
}

fn listen_for_transactions<D>(rx_transactions: mpsc::Receiver<Transaction>, database: D)
where
    D: DataStorage + Send + Sync + 'static + Clone,
{
    tokio::spawn(async move {
        let mut rx_transactions = rx_transactions;
        while let Some(transaction) = rx_transactions.recv().await {
            database.store_transaction(transaction).await.unwrap();
        }
    });
}

#[async_trait::async_trait]
impl<D> Notifier for ChannelNotifier<D>
where
    D: DataStorage + Send + Sync + 'static + Clone,
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
