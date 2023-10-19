use protocol::Result;

use crate::networking::server::run_server_on;

pub mod config;
pub mod dns;
pub mod networking;
pub mod protocol;
pub mod rules;
pub mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::load_config_relative("./config/mindns.toml");
    println!("Loaded configuration file.");

    let rules = rules::parse_rules_config(&config.rules);
    println!("Loaded {} rules.", rules.len());

    println!("Starting DNS server.");
    run_server_on("127.0.0.1:53").await
}
