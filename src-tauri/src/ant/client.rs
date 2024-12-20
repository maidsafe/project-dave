pub async fn client() -> autonomi::Client {
    autonomi::client::Client::init()
        .await
        .expect("Failed to initialize client")
}
