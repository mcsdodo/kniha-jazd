use serde::Serialize;
use std::sync::Mutex;
use tokio::sync::oneshot;

pub struct ServerManager {
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    status: Mutex<ServerStatus>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub url: Option<String>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Mutex::new(None),
            status: Mutex::new(ServerStatus {
                running: false,
                port: None,
                url: None,
            }),
        }
    }

    pub fn set_running(&self, port: u16, url: String, tx: oneshot::Sender<()>) {
        *self.shutdown_tx.lock().unwrap() = Some(tx);
        *self.status.lock().unwrap() = ServerStatus {
            running: true,
            port: Some(port),
            url: Some(url),
        };
    }

    pub fn stop(&self) -> Result<(), String> {
        let tx = self.shutdown_tx.lock().unwrap().take();
        match tx {
            Some(tx) => {
                let _ = tx.send(());
                *self.status.lock().unwrap() = ServerStatus {
                    running: false,
                    port: None,
                    url: None,
                };
                Ok(())
            }
            None => Err("Server is not running".to_string()),
        }
    }

    pub fn status(&self) -> ServerStatus {
        self.status.lock().unwrap().clone()
    }
}
