use crate::domain::errors::AggregatorError;

pub mod aggregator;
pub mod app;
/// The `Aggregator` trait defines the core functionality for transaction aggregation.
///
/// Implementors of this trait are responsible for running the aggregation process,
/// which typically involves collecting, processing, and potentially storing
/// transaction data from a blockchain network.
///
/// The `run` method returns a `Result` where the `Err` variant is an `AggregatorError`.
/// This allows for proper error handling and propagation throughout the application.
#[async_trait::async_trait]
pub trait Aggregator {
    async fn run(mut self) -> Result<(), AggregatorError>;
}
