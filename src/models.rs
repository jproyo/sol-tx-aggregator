use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: [u8; 32],
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: Pubkey,
    pub balance: u64,
}
