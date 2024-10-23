use crate::domain::{
    errors::DataStorageError,
    models::{Account, DataStorage, Transaction},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;

#[derive(Clone, Default)]
pub struct InMemoryDatabase {
    transactions: DashMap<String, Transaction>,
    accounts: DashMap<String, Account>,
    // date -> transaction_id. `%Y-%m-%d` format is the key
    transactions_by_date: DashMap<String, Vec<String>>,
    // account -> date -> transaction_id. `%Y-%m-%d` format is the key
    transactions_by_account: DashMap<String, DashMap<String, Vec<String>>>,
}

#[async_trait::async_trait]
impl DataStorage for InMemoryDatabase {
    async fn store_transaction(&self, transaction: Transaction) -> Result<(), DataStorageError> {
        let transaction_id = transaction.id.to_string();
        let date = transaction.timestamp;
        let sender = transaction.sender.to_string();
        let receiver = transaction.receiver.to_string();

        // Store transaction by ID
        self.transactions
            .insert(transaction_id.clone(), transaction);

        // Store transaction by date
        let date = DateTime::<Utc>::from_timestamp(date, 0)
            .ok_or(DataStorageError::InvalidDate(date.to_string()))?;
        let date = date.format("%Y-%m-%d").to_string();
        self.transactions_by_date
            .entry(date.clone())
            .or_default()
            .push(transaction_id.clone());

        // Store transaction for sender
        self.transactions_by_account
            .entry(sender)
            .or_default()
            .entry(date.clone())
            .or_default()
            .push(transaction_id.clone());

        // Store transaction for receiver
        self.transactions_by_account
            .entry(receiver)
            .or_default()
            .entry(date)
            .or_default()
            .push(transaction_id);

        Ok(())
    }

    async fn store_account(&self, account: Account) -> Result<(), DataStorageError> {
        self.accounts.insert(account.address.to_string(), account);
        Ok(())
    }
}
