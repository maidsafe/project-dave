use autonomi::client::ConnectError;
use autonomi::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SharedClient {
    pub client: Arc<Mutex<Client>>,
}

impl SharedClient {
    pub async fn init() -> Self {
        let client = client().await.expect("Failed to connect client");

        Self {
            client: Arc::new(Mutex::new(client)),
        }
    }
}

pub async fn client() -> Result<Client, ConnectError> {
    Client::init().await
}
