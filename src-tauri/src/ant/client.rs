use autonomi::client::ConnectError;
use autonomi::Client;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct SharedClient {
    client: RwLock<Option<Client>>,
}

impl SharedClient {
    pub async fn connect(&self) -> Result<Client, ConnectError> {
        let client = Client::init().await?;
        *self.client.write().await = Some(client.clone());
        Ok(client)
    }

    #[expect(dead_code)]
    pub async fn disconnect(&self) {
        *self.client.write().await = None;
    }

    pub async fn get_client(&self) -> Result<Client, ConnectError> {
        if let Some(client) = self.client.read().await.clone() {
            return Ok(client);
        }

        self.connect().await
    }
}
