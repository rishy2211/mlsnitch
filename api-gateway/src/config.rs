//! API gateway configuration.
//!
//! For now this only configures the HTTP listen address. The underlying
//! chain configuration is taken from `chain::ChainConfig::default()`.

use std::net::SocketAddr;

/// Configuration for the API gateway HTTP server.
#[derive(Clone, Debug)]
pub struct ApiConfig {
    /// Address to bind the HTTP server to.
    pub listen_addr: SocketAddr,
}

impl Default for ApiConfig {
    fn default() -> Self {
        // Safe to unwrap: fixed, valid address literal.
        let addr: SocketAddr = "127.0.0.1:8081"
            .parse()
            .expect("hard-coded API listen address should parse");
        Self { listen_addr: addr }
    }
}
