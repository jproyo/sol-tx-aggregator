use crate::domain::errors::BcClientError;
use solana_transaction_status::UiConfirmedBlock;

/// A trait representing a blockchain client for interacting with the Solana network.
#[async_trait::async_trait]
pub trait BcClient {
    /// Retrieves the current slot number from the Solana network.
    ///
    /// # Returns
    ///
    /// * `Result<u64, BcClientError>` - The current slot number if successful, or an error if the operation fails.
    async fn get_current_slot(&self) -> Result<u64, BcClientError>;

    /// Retrieves a list of block slot numbers within a specified range.
    ///
    /// # Arguments
    ///
    /// * `start_slot` - The starting slot number of the range.
    /// * `end_slot` - The ending slot number of the range.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u64>, BcClientError>` - A vector of slot numbers if successful, or an error if the operation fails.
    async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, BcClientError>;

    /// Retrieves detailed information about a specific block.
    ///
    /// # Arguments
    ///
    /// * `slot` - The slot number of the block to retrieve.
    ///
    /// # Returns
    ///
    /// * `Result<UiConfirmedBlock, BcClientError>` - The block information if successful, or an error if the operation fails.
    async fn get_block(&self, slot: u64) -> Result<UiConfirmedBlock, BcClientError>;
}
