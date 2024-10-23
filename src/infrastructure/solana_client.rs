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

#[derive(Clone)]
pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
}

impl SolanaClient {
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
    async fn get_current_slot(&self) -> Result<u64, BcClientError> {
        let retry_strategy = ExponentialBackoff::from_millis(500).map(jitter).take(3);

        let result = Retry::spawn(retry_strategy, || self.rpc_client.get_slot())
            .await
            .map_err(|e| BcClientError::FailedToGetCurrentSlot(e.to_string()))?;
        Ok(result)
    }

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
                    ..Default::default()
                },
            )
        })
        .await
        .map_err(|e| BcClientError::FailedToGetBlock(e.to_string()))?;
        Ok(result)
    }
}
