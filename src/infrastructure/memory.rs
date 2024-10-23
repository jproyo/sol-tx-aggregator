use crate::domain::{
    errors::DataStorageError,
    models::{Account, DataStorage, Transaction},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Default)]
pub struct InMemoryDatabase {
    transactions: DashMap<String, Transaction>,
    accounts: DashMap<String, Account>,
    // date -> [transaction_id]. `%Y-%m-%d` format date is the key
    transactions_by_date: DashMap<String, Vec<String>>,
    // account -> (date -> [transaction_id]). `%Y-%m-%d` format date is the key of the second map
    transactions_by_account: DashMap<String, DashMap<String, Vec<String>>>,
    // slot -> [transaction_id]
    transactions_by_slot: DashMap<u64, Vec<String>>,
}

#[async_trait::async_trait]
impl DataStorage for InMemoryDatabase {
    async fn store_transaction(&self, transaction: Transaction) -> Result<(), DataStorageError> {
        let transaction_id = transaction.id.to_string();
        let date = transaction.timestamp;
        let sender = transaction.sender.to_string();
        let receiver = transaction.receiver.to_string();
        let slot = transaction.slot;

        if self.transactions.contains_key(&transaction_id) {
            tracing::warn!("Transaction already exists: {}", transaction_id);
            return Ok(());
        }

        // Store transaction by ID
        self.transactions
            .entry(transaction_id.clone())
            .or_insert(transaction);

        // Store transaction by date
        let date = DateTime::<Utc>::from_timestamp(date, 0)
            .ok_or(DataStorageError::InvalidDate(date.to_string()))?;
        let date = date.format("%Y-%m-%d").to_string();
        self.transactions_by_date
            .entry(date.clone())
            .or_default()
            .push(transaction_id.clone());

        self.transactions_by_slot
            .entry(slot)
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
        if let Some(ref mut existing_account) = self.accounts.get_mut(&account.address.to_string())
        {
            if existing_account.last_slot_updated < account.last_slot_updated {
                existing_account.balance = account.balance;
                existing_account.last_slot_updated = account.last_slot_updated;
            }
        } else {
            self.accounts.insert(account.address.to_string(), account);
        }
        Ok(())
    }

    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError> {
        Ok(self
            .transactions
            .iter()
            .map(|v| v.value().clone())
            .collect())
    }

    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError> {
        Ok(self.accounts.iter().map(|v| v.value().clone()).collect())
    }

    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError> {
        self.transactions
            .get(&id)
            .map(|v| v.value().clone())
            .ok_or(DataStorageError::TransactionNotFound(id))
    }

    async fn get_account(&self, address: Pubkey) -> Result<Account, DataStorageError> {
        self.accounts
            .get(&address.to_string())
            .map(|v| v.value().clone())
            .ok_or(DataStorageError::AccountNotFound(address.to_string()))
    }

    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        self.get_transactions_by_account(sender)
    }

    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        self.get_transactions_by_account(receiver)
    }

    async fn get_transactions_by_slot(
        &self,
        slot: u64,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        Ok(self
            .transactions_by_slot
            .get(&slot)
            .map(|v| v.value().clone())
            .unwrap_or_default()
            .iter()
            .map(|v| self.transactions.get(v).unwrap().value().clone())
            .collect())
    }

    async fn get_transactions_by_date(
        &self,
        date: String,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        Ok(self
            .transactions_by_date
            .get(&date)
            .map(|v| v.value().clone())
            .unwrap_or_default()
            .iter()
            .map(|v| self.transactions.get(v).unwrap().value().clone())
            .collect())
    }
}

impl InMemoryDatabase {
    fn get_transactions_by_account(
        &self,
        account: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        Ok(self
            .transactions_by_account
            .get(&account.to_string())
            .map(|v| v.value().clone())
            .unwrap_or_default()
            .into_read_only()
            .values()
            .flatten()
            .map(|v| self.transactions.get(v).unwrap().value().clone())
            .collect())
    }
}
