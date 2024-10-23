use tokio::sync::broadcast;

/// Trait for types that can provide a shutdown mechanism.
pub trait Shutdown {
    /// Subscribe to the shutdown signal.
    ///
    /// # Returns
    /// A `broadcast::Receiver<()>` that can be used to await the shutdown signal.
    fn subscribe(&self) -> broadcast::Receiver<()>;
}

/// A channel for broadcasting shutdown signals.
#[derive(Clone)]
pub struct ShutdownChannel {
    shutdown: broadcast::Sender<()>,
}

impl ShutdownChannel {
    /// Create a new `ShutdownChannel`.
    ///
    /// # Arguments
    /// * `shutdown` - A `broadcast::Sender<()>` used to send shutdown signals.
    ///
    /// # Returns
    /// A new instance of `ShutdownChannel`.
    pub fn new(shutdown: broadcast::Sender<()>) -> Self {
        Self { shutdown }
    }
}

impl Shutdown for ShutdownChannel {
    /// Subscribe to the shutdown signal.
    ///
    /// # Returns
    /// A `broadcast::Receiver<()>` that can be used to await the shutdown signal.
    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown.subscribe()
    }
}
