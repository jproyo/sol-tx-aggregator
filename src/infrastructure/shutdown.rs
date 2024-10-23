use tokio::sync::broadcast;

pub trait Shutdown {
    fn subscribe(&self) -> broadcast::Receiver<()>;
}

#[derive(Clone)]
pub struct ShutdownChannel {
    shutdown: broadcast::Sender<()>,
}

impl ShutdownChannel {
    pub fn new(shutdown: broadcast::Sender<()>) -> Self {
        Self { shutdown }
    }
}

impl Shutdown for ShutdownChannel {
    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown.subscribe()
    }
}
