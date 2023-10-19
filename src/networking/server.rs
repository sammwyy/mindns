use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    config::Config,
    networking::handler::handle_request,
    protocol::{byte_packet_buffer::BytePacketBuffer, Result},
    rules::Rule,
};

pub async fn run_server(config: &Config, rules: &Vec<Rule>) -> Result<()> {
    let server = &config.server;

    // Bind server address.
    let raw_addr = format!("{}:{}", server.bind, server.port);
    let addr = raw_addr.parse::<SocketAddr>()?;
    let socket = UdpSocket::bind(addr).await?;

    loop {
        let mut buffer = BytePacketBuffer::new();
        let (size, peer) = socket.recv_from(&mut buffer.buf).await?;
        buffer.pos = 0;
        println!("Received {} bytes from {}", size, peer);
        handle_request(config, rules, &socket, peer, &mut buffer).await?;
    }
}
