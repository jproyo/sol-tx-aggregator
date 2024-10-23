use crate::domain::{
    errors::DataStorageError,
    models::{Account, DataStorage, Transaction},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;

/// An in-memory database implementation for storing and retrieving blockchain data.
///
/// This struct provides a thread-safe, concurrent-access storage solution using `DashMap`s
/// to store transactions and accounts, as well as various indices for efficient querying.
#[derive(Clone, Default)]
pub struct InMemoryDatabase {
    /// Stores transactions by their ID
    transactions: DashMap<String, Transaction>,
    /// Stores accounts by their address
    accounts: DashMap<String, Account>,
    /// Stores transaction IDs by date (YYYY-MM-DD format)
    transactions_by_date: DashMap<String, Vec<String>>,
    /// Stores transaction IDs by account and date
    transactions_by_account: DashMap<String, DashMap<String, Vec<String>>>,
    /// Stores transaction IDs by slot number
    transactions_by_slot: DashMap<u64, Vec<String>>,
}

#[async_trait::async_trait]
impl DataStorage for InMemoryDatabase {
    /// Stores a transaction in the database.
    ///
    /// This method adds the transaction to various indices for efficient retrieval later.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to store
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transaction was stored successfully
    /// * `Err(DataStorageError)` if there was an error storing the transaction
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

    /// Stores or updates an account in the database.
    ///
    /// If the account already exists, it updates the balance and last updated slot
    /// if the new account has a more recent slot number.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to store or update
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the account was stored or updated successfully
    /// * `Err(DataStorageError)` if there was an error storing the account
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

    /// Retrieves all transactions from the database.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Transaction>)` containing all stored transactions
    /// * `Err(DataStorageError)` if there was an error retrieving the transactions
    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError> {
        Ok(self
            .transactions
            .iter()
            .map(|v| v.value().clone())
            .collect())
    }

    /// Retrieves all accounts from the database.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Account>)` containing all stored accounts
    /// * `Err(DataStorageError)` if there was an error retrieving the accounts
    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError> {
        Ok(self.accounts.iter().map(|v| v.value().clone()).collect())
    }

    /// Retrieves a specific transaction by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the transaction to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Transaction)` if the transaction was found
    /// * `Err(DataStorageError::TransactionNotFound)` if the transaction was not found
    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError> {
        self.transactions
            .get(&id)
            .map(|v| v.value().clone())
            .ok_or(DataStorageError::TransactionNotFound(id))
    }

    /// Retrieves a specific account by its address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address of the account to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` if the account was found
    /// * `Err(DataStorageError::AccountNotFound)` if the account was not found
    async fn get_account(&self, address: Pubkey) -> Result<Account, DataStorageError> {
        self.accounts
            .get(&address.to_string())
            .map(|v| v.value().clone())
            .ok_or(DataStorageError::AccountNotFound(address.to_string()))
    }

    /// Retrieves all transactions where the given address is the sender.
    ///
    /// # Arguments
    ///
    /// * `sender` - The address of the sender
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Transaction>)` containing all matching transactions
    /// * `Err(DataStorageError)` if there was an error retrieving the transactions
    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        self.get_transactions_by_account(sender)
    }

    /// Retrieves all transactions where the given address is the receiver.
    ///
    /// # Arguments
    ///
    /// * `receiver` - The address of the receiver
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Transaction>)` containing all matching transactions
    /// * `Err(DataStorageError)` if there was an error retrieving the transactions
    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError> {
        self.get_transactions_by_account(receiver)
    }

    /// Retrieves all transactions for a specific slot.
    ///
    /// # Arguments
    ///
    /// * `slot` - The slot number to query
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Transaction>)` containing all transactions in the given slot
    /// * `Err(DataStorageError)` if there was an error retrieving the transactions
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

    /// Retrieves all transactions for a specific date.
    ///
    /// # Arguments
    ///
    /// * `date` - The date to query in YYYY-MM-DD format
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Transaction>)` containing all transactions on the given date
    /// * `Err(DataStorageError)` if there was an error retrieving the transactions
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
    /// Helper method to retrieve transactions for a given account (either as sender or receiver).
    ///
    /// # Arguments
    ///
    /// * `account` - The account address to query
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Transaction>)` containing all transactions involving the given account
    /// * `Err(DataStorageError)` if there was an error retrieving the transactions
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
