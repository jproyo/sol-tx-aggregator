use crate::domain::errors::BcClientError;
use solana_transaction_status::UiConfirmedBlock;

#[async_trait::async_trait]
pub trait BcClient {
    async fn get_current_slot(&self) -> Result<u64, BcClientError>;
    async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, BcClientError>;
    async fn get_block(&self, slot: u64) -> Result<UiConfirmedBlock, BcClientError>;
}
