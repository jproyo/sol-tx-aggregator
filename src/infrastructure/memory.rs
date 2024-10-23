use crate::domain::{
    errors::DataStorageError,
    models::{Account, DataStorage, Transaction},
};
use dashmap::DashMap;

#[derive(Clone, Default)]
pub struct InMemoryDatabase {
    transactions: DashMap<String, Transaction>,
    accounts: DashMap<String, Account>,
}
#[async_trait::async_trait]
impl DataStorage for InMemoryDatabase {
    async fn store_transaction(&self, transaction: Transaction) -> Result<(), DataStorageError> {
        self.transactions
            .insert(transaction.id.to_string(), transaction);
        Ok(())
    }

    async fn store_account(&self, account: Account) -> Result<(), DataStorageError> {
        self.accounts.insert(account.address.to_string(), account);
        Ok(())
    }
}
