use autonomi::client::ConnectError;
use autonomi::Client;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct SharedClient {
    client: RwLock<Option<Client>>,
}

impl SharedClient {
    pub async fn connect(&self) -> Result<Client, ConnectError> {
        let mut client_lock = self.client.write().await;

        // check if another thread already connected the client in the meanwhile
        if let Some(client) = client_lock.as_ref() {
            return Ok(client.clone());
        }

        let client = Client::init().await?;
        *client_lock = Some(client.clone());

        Ok(client)
    }

    #[expect(dead_code)]
    pub async fn disconnect(&self) {
        *self.client.write().await = None;
    }

    pub async fn get_client(&self) -> Result<Client, ConnectError> {
        if let Some(client) = self.client.read().await.as_ref() {
            return Ok(client.clone());
        }

        self.connect().await
    }
}
