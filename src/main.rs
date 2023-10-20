use protocol::Result;

use crate::config::Config;
use crate::logs::setup_logger;
use crate::networking::handler::handle_request;
use crate::networking::udp_serv::UdpServer;
use crate::protocol::byte_packet_buffer::BytePacketBuffer;
use crate::rules::Rule;

mod config;
mod dns;
mod logs;
mod networking;
mod protocol;
mod rules;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration file.
    let config = config::load_config_relative("./mindns.toml");
    setup_logger(&config.logs);
    log::info!("Loaded configuration file.");

    // Load rules.
    let rules = rules::parse_rules_config(&config.rules);
    log::info!("Loaded {} rules.", rules.len());

    // Start DNS server.
    let raw_addr = format!("{}:{}", config.server.bind, config.server.port);
    log::info!("Starting DNS server at udp://{}", raw_addr);

    UdpServer::new(
        raw_addr,
        |peer, mut reader, (config, rules): (Config, Vec<Rule>)| async move {
            let mut buffer = BytePacketBuffer::new();
            while let Some(Ok(data)) = reader.recv().await {
                buffer.pos = 0;
                buffer.buf[..data.len()].copy_from_slice(&data);

                handle_request(&config, &rules, &peer, &mut buffer).await?;
            }

            Ok(())
        },
    )?
    .set_peer_timeout_sec(20)
    .start((config, rules))
    .await?;

    Ok(())
}
