use thiserror::Error;

#[derive(Error, Debug)]
pub enum AggregatorError {
    #[error("Failed to process block {0}. Retrying on next loop")]
    FailedToProcessBlock(u64),
    #[error("Failed solana rpc client")]
    FailedSolanaRpcClient(#[from] solana_client::client_error::ClientError),
    #[error("Failed to notify notifier")]
    FailedToNotifyNotifier(#[from] NotifierError),
}

#[derive(Error, Debug)]
pub enum DataStorageError {
    #[error("Failed to store transaction")]
    FailedToStoreTransaction,
    #[error("Failed to store account")]
    FailedToStoreAccount,
}

#[derive(Error, Debug)]
pub enum NotifierError {
    #[error("Failed to notify transaction")]
    FailedToNotifyTransaction,
    #[error("Failed to notify account")]
    FailedToNotifyAccount,
}
