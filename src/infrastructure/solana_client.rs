use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{TransactionDetails, UiConfirmedBlock, UiTransactionEncoding};
use std::sync::Arc;
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    Retry,
};

use crate::domain::errors::BcClientError;

use super::bc_client::BcClient;

/// A client for interacting with the Solana blockchain.
#[derive(Clone)]
pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
    num_retries: usize,
}

impl SolanaClient {
    /// Creates a new `SolanaClient` instance from the given RPC URL and number of retries.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The URL of the Solana RPC endpoint.
    /// * `num_retries` - The number of retries for the RPC client.
    /// # Returns
    ///
    /// A new `SolanaClient` instance.
    pub(crate) fn new(rpc_url: String, num_retries: usize) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new_with_commitment(
                rpc_url,
                CommitmentConfig::confirmed(),
            )),
            num_retries,
        }
    }
}

#[async_trait::async_trait]
impl BcClient for SolanaClient {
    /// Retrieves the current slot number from the Solana blockchain.
    ///
    /// # Returns
    ///
    /// The current slot number as a `u64`, or a `BcClientError` if the operation fails.
    async fn get_current_slot(&self) -> Result<u64, BcClientError> {
        let retry_strategy = ExponentialBackoff::from_millis(500)
            .factor(2)
            .map(jitter)
            .take(self.num_retries);

        let result = Retry::spawn(retry_strategy, || self.rpc_client.get_slot())
            .await
            .map_err(|e| BcClientError::GettingCurrentSlot(e.to_string()))?;
        Ok(result)
    }

    /// Retrieves a range of block numbers from the Solana blockchain.
    ///
    /// # Arguments
    ///
    /// * `start_slot` - The starting slot number.
    /// * `end_slot` - The ending slot number.
    ///
    /// # Returns
    ///
    /// A vector of block numbers (slots) within the specified range, or a `BcClientError` if the operation fails.
    async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, BcClientError> {
        let retry_strategy = ExponentialBackoff::from_millis(500)
            .factor(2)
            .map(jitter)
            .take(self.num_retries);

        let result = Retry::spawn(retry_strategy, || {
            self.rpc_client.get_blocks_with_commitment(
                start_slot,
                Some(end_slot),
                CommitmentConfig::confirmed(),
            )
        })
        .await
        .map_err(|e| BcClientError::RetrievingBlocks(e.to_string()))?;
        Ok(result)
    }

    /// Retrieves detailed information about a specific block from the Solana blockchain.
    ///
    /// # Arguments
    ///
    /// * `slot` - The slot number of the block to retrieve.
    ///
    /// # Returns
    ///
    /// A `UiConfirmedBlock` containing detailed information about the requested block,
    /// or a `BcClientError` if the operation fails.
    async fn get_block(&self, slot: u64) -> Result<UiConfirmedBlock, BcClientError> {
        let retry_strategy = ExponentialBackoff::from_millis(500)
            .factor(2)
            .map(jitter)
            .take(self.num_retries);

        let result = Retry::spawn(retry_strategy, || {
            self.rpc_client.get_block_with_config(
                slot,
                RpcBlockConfig {
                    transaction_details: Some(TransactionDetails::Full),
                    encoding: Some(UiTransactionEncoding::Base64),
                    rewards: Some(false),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
        })
        .await
        .map_err(|e| BcClientError::GetBlock(e.to_string()))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_current_slot_with_retries() {
        let mock_client = RpcClient::new("http://localhost:8899".to_string());
        let solana_client = SolanaClient {
            rpc_client: Arc::new(mock_client),
            num_retries: 1,
        };

        let result = solana_client.get_current_slot().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_blocks_with_retries() {
        let mock_client = RpcClient::new("http://localhost:8899".to_string());

        let solana_client = SolanaClient {
            rpc_client: Arc::new(mock_client),
            num_retries: 1,
        };

        let result = solana_client.get_blocks(10, 20).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_block_with_retries() {
        let mock_client = RpcClient::new("http://localhost:8899".to_string());

        let solana_client = SolanaClient {
            rpc_client: Arc::new(mock_client),
            num_retries: 1,
        };

        let result = solana_client.get_block(100).await;
        assert!(result.is_err());
    }
}
