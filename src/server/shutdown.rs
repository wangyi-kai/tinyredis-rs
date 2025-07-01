use tokio::sync::broadcast;

pub struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    pub fn new(notify: broadcast::Receiver<()>) -> Self {
        Self {
            shutdown: false,
            notify,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub fn shutdown(&mut self) {
        self.shutdown = true
    }

    pub async fn receiver(&mut self) {
        if self.shutdown {
            return;
        }
        let _ = self.notify.recv().await;
        self.shutdown = true;
    }
}