use autonomi::Multiaddr;
use std::str::FromStr;

// todo: make this user configurable
// random prod peer
const SAFE_PEER: &str =
    "/ip4/127.0.0.1/udp/61507/quic-v1/p2p/12D3KooWAMSnvW7T2JXwq53mMnLPEjgWxSoMCJpVHNjdBbXV57Y4";

pub async fn client() -> autonomi::Client {
    autonomi::client::Client::connect(&[
        Multiaddr::from_str(SAFE_PEER).expect("Could not parse SAFE_PEER")
    ])
    .await
    .unwrap()
}
