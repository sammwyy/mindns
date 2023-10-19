use protocol::Result;

use crate::networking::server::run_server;

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

    for rule in &rules {
        println!("{:?}", rule);
    }

    println!(
        "Starting DNS server at udp://{}:{}",
        config.server.bind, config.server.port
    );

    run_server(&config, &rules).await
}
