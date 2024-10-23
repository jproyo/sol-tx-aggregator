use super::models::{Account, Transaction};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Error, Debug)]
pub enum AggregatorError {
    #[error("Failed to process block {0}. Retrying on next loop")]
    ToProcessBlock(u64),
    #[error("Failed solana rpc client {0}")]
    SolanaRpcClient(#[from] solana_client::client_error::ClientError),
    #[error("Failed to notify notifier {0}")]
    Notifier(#[from] NotifierError),
    #[error("Failed with blockchain communication client {0}")]
    BcClient(#[from] BcClientError),
}

#[derive(Error, Debug)]
pub enum BcClientError {
    #[error("Failed to get current slot {0}")]
    GettingCurrentSlot(String),
    #[error("Failed to get blocks {0}")]
    RetrievingBlocks(String),
    #[error("Failed to get block {0}")]
    GetBlock(String),
}

#[derive(Error, Debug)]
pub enum DataStorageError {
    #[error("Failed to store transaction")]
    StoreTransaction,
    #[error("Failed to store account")]
    ToStoreAccount,
    #[error("Invalid date on transaction {0}")]
    InvalidDate(String),
    #[error("Transaction not found {0}")]
    TransactionNotFound(String),
    #[error("Account not found {0}")]
    AccountNotFound(String),
}

#[derive(Error, Debug)]
pub enum NotifierError {
    #[error("Failed to notify transaction {0}")]
    OnTransaction(#[from] SendError<Transaction>),
    #[error("Failed to notify account {0}")]
    Account(#[from] SendError<Account>),
}
