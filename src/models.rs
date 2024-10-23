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
