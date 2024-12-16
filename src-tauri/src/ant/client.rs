use std::str::FromStr;
use autonomi::Multiaddr;

// todo: make this user configurable
// random prod peer
const SAFE_PEER: &str = "/ip4/167.71.140.72/tcp/37462/ws/p2p/12D3KooWMrQ95QFLzKkyTc2X3aEFqGhH5cPmsf1QkCuAD4PWNqyW";

pub fn client() -> autonomi::Client {
    autonomi::client::Client::connect(&*vec![Multiaddr::from_str(SAFE_PEER).expect("Could not parse SAFE_PEER")])
}