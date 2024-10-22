use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Vec<u8>,
    pub sender: Vec<u8>,
    pub receiver: Vec<u8>,
    pub amount: u64,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: Vec<u8>,
    pub balance: u64,
}
