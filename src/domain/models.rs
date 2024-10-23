use super::errors::{DataStorageError, NotifierError};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub id: String,
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub amount: u64,
    pub fee: u64,
    pub slot: u64,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Account {
    pub address: Pubkey,
    pub balance: u64,
    pub last_slot_updated: u64,
}

#[async_trait::async_trait]
pub trait DataStorage {
    async fn store_transaction(&self, transaction: Transaction) -> Result<(), DataStorageError>;
    async fn store_account(&self, account: Account) -> Result<(), DataStorageError>;
    async fn get_transactions(&self) -> Result<Vec<Transaction>, DataStorageError>;
    async fn get_accounts(&self) -> Result<Vec<Account>, DataStorageError>;
    async fn get_transaction(&self, id: String) -> Result<Transaction, DataStorageError>;
    async fn get_account(&self, address: Pubkey) -> Result<Account, DataStorageError>;
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

#[async_trait::async_trait]
pub trait Notifier {
    async fn notify_transaction(&self, transaction: Transaction) -> Result<(), NotifierError>;
    async fn notify_account(&self, account: Account) -> Result<(), NotifierError>;
}
