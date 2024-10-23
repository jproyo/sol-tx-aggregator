use super::errors::{DataStorageError, NotifierError};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Represents a transaction on the Solana blockchain.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Transaction {
    /// Unique identifier for the transaction
    pub id: String,
    /// Public key of the transaction sender
    pub sender: Pubkey,
    /// Public key of the transaction receiver
    pub receiver: Pubkey,
    /// Amount of tokens transferred
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Slot number in which the transaction was processed
    pub slot: u64,
    /// Unix timestamp of the transaction
    pub timestamp: i64,
}

/// Represents an account on the Solana blockchain.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Account {
    /// Public key of the account
    pub address: Pubkey,
    /// Current balance of the account
    pub balance: u64,
    /// The last slot in which this account was updated
    pub last_slot_updated: u64,
}

/// Trait for data storage operations.
#[async_trait::async_trait]
pub trait DataStorage {
    /// Stores a transaction in the database.
    async fn store_transaction(&self, transaction: Transaction) -> Result<(), DataStorageError>;

    /// Stores an account in the database.
    async fn store_account(&self, account: Account) -> Result<(), DataStorageError>;

    /// Retrieves all transactions from the database.
    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all accounts from the database.
    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError>;

    /// Retrieves a specific transaction by its ID.
    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError>;

    /// Retrieves a specific account by its address.
    async fn get_account(&self, address: Pubkey) -> Result<Account, DataStorageError>;

    /// Retrieves all transactions sent by a specific address.
    async fn get_transactions_by_sender(
        &self,
        sender: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all transactions received by a specific address.
    async fn get_transactions_by_receiver(
        &self,
        receiver: Pubkey,
    ) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all transactions processed in a specific slot.
    async fn get_transactions_by_slot(
        &self,
        slot: u64,
    ) -> Result<Vec<Transaction>, DataStorageError>;

    /// Retrieves all transactions processed on a specific date.
    async fn get_transactions_by_date(
        &self,
        date: String,
    ) -> Result<Vec<Transaction>, DataStorageError>;
}

/// Trait for notifying about new transactions and account updates.
#[async_trait::async_trait]
pub trait Notifier {
    /// Notifies about a new transaction.
    async fn notify_transaction(&self, transaction: Transaction) -> Result<(), NotifierError>;

    /// Notifies about an account update.
    async fn notify_account(&self, account: Account) -> Result<(), NotifierError>;
}
