use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
}
