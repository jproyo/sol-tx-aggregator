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
}

#[async_trait::async_trait]
pub trait DataStorage {
    async fn store_transaction(&self, transaction: Transaction) -> Result<(), DataStorageError>;
    async fn store_account(&self, account: Account) -> Result<(), DataStorageError>;
}

#[async_trait::async_trait]
pub trait Notifier {
    async fn notify_transaction(&self, transaction: Transaction) -> Result<(), NotifierError>;
    async fn notify_account(&self, account: Account) -> Result<(), NotifierError>;
}
