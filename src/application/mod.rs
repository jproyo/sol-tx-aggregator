use crate::domain::errors::AggregatorError;

pub mod aggregator;
/// The `Aggregator` trait defines the core functionality for transaction aggregation.
///
/// Implementors of this trait are responsible for running the aggregation process,
/// which typically involves collecting, processing, and potentially storing
/// transaction data from a blockchain network.
///
/// # Examples
///
/// ```no_run
/// use crate::application::Aggregator;
/// use crate::domain::errors::AggregatorError;
///
/// struct MyAggregator;
///
/// #[async_trait]
/// impl Aggregator for MyAggregator {
///     async fn run(&self) -> Result<(), AggregatorError> {
///         // Implement aggregation logic here
///         Ok(())
///     }
/// }
/// ```
///
/// # Errors
///
/// The `run` method returns a `Result` where the `Err` variant is an `AggregatorError`.
/// This allows for proper error handling and propagation throughout the application.
#[async_trait::async_trait]
pub trait Aggregator {
    async fn run(&self) -> Result<(), AggregatorError>;
}
