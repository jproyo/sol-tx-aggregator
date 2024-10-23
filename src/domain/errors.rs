use super::models::{Account, Transaction};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Error, Debug)]
pub enum AggregatorError {
    #[error("Failed to process block {0}. Retrying on next loop")]
    FailedToProcessBlock(u64),
    #[error("Failed solana rpc client {0}")]
    FailedSolanaRpcClient(#[from] solana_client::client_error::ClientError),
    #[error("Failed to notify notifier {0}")]
    FailedToNotifyNotifier(#[from] NotifierError),
    #[error("Failed with blockchain communication client {0}")]
    FailedBcClient(#[from] BcClientError),
}

#[derive(Error, Debug)]
pub enum BcClientError {
    #[error("Failed to get current slot {0}")]
    FailedToGetCurrentSlot(String),
    #[error("Failed to get blocks {0}")]
    FailedToGetBlocks(String),
    #[error("Failed to get block {0}")]
    FailedToGetBlock(String),
}

#[derive(Error, Debug)]
pub enum DataStorageError {
    #[error("Failed to store transaction")]
    FailedToStoreTransaction,
    #[error("Failed to store account")]
    FailedToStoreAccount,
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
    FailedToNotifyTransaction(#[from] SendError<Transaction>),
    #[error("Failed to notify account {0}")]
    FailedToNotifyAccount(#[from] SendError<Account>),
}
