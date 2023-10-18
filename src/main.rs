use protocol::Result;

use crate::networking::server::run_server_on;

pub mod config;
pub mod dns;
pub mod networking;
pub mod protocol;
pub mod rules;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting DNS server.");
    run_server_on("127.0.0.1:53").await
}
