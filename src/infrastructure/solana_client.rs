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
}

impl SolanaClient {
    /// Creates a new `SolanaClient` instance from the given RPC URL.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The URL of the Solana RPC endpoint.
    ///
    /// # Returns
    ///
    /// A new `SolanaClient` instance.
    pub fn from_url(rpc_url: &str) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            )),
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
        let retry_strategy = ExponentialBackoff::from_millis(500).map(jitter).take(3);

        let result = Retry::spawn(retry_strategy, || self.rpc_client.get_slot())
            .await
            .map_err(|e| BcClientError::FailedToGetCurrentSlot(e.to_string()))?;
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
        let retry_strategy = ExponentialBackoff::from_millis(500).map(jitter).take(3);

        let result = Retry::spawn(retry_strategy, || {
            self.rpc_client.get_blocks_with_commitment(
                start_slot,
                Some(end_slot),
                CommitmentConfig::confirmed(),
            )
        })
        .await
        .map_err(|e| BcClientError::FailedToGetBlocks(e.to_string()))?;
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
        let retry_strategy = ExponentialBackoff::from_millis(500).map(jitter).take(3);

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
        .map_err(|e| BcClientError::FailedToGetBlock(e.to_string()))?;
        Ok(result)
    }
}
