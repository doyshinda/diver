//! [D'iver](https://malazan.fandom.com/wiki/D%27ivers) acts a Divider proxy for a TCP stream.
//!
//! It will send all TCP traffic it receives to two different endpoints (designated 'real'
//! and 'test'). The response from the 'real' endpoint is returned upstream to the client,
//! and the 'test' response is dropped.
//!
//! This allows for direct, live comparison of the effect on an application when introducing new
//! features, bug fixes, optimizations, etc.
//!
//! # Usage
//! * Create a `config.yaml` file:
//! ```yaml
//! real: real.endpoint.com:80
//! test: test.endpoint.com:80
//! port: 80
//! ```
//! * Run the binary:
//! `./target/release/diver`
//! * Point clients at the IP address + port of the current machine
use config::{Config, ConfigError, File};
use env_logger;
use serde::Deserialize;

#[doc(hidden)]
mod diver;

#[derive(Deserialize)]
pub struct AppConfig {
    /// The 'real' endpoint, the response from this endpoint is what gets returned upstream.
    real: String,

    /// The 'test' endpoint, the response from this endpoint will be dropped.
    test: String,

    /// The port to listen for TCP connections.
    port: String,

    /// The maximum number of concurrent connections D'iver will accept (default: 1000).
    max_conn: Option<usize>,
}

impl AppConfig {
    fn load() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("config.yaml"))?;
        s.try_into()
    }
}

#[doc(hidden)]
fn main() {
    env_logger::init();
    let cfg = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => panic!("Unable to parse config: {:?}", e),
    };

    diver::run(cfg);
}
