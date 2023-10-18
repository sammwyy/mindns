use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    networking::handler::handle_query,
    protocol::{byte_packet_buffer::BytePacketBuffer, Result},
};

pub async fn run_server(addr: SocketAddr) -> Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    let mut buffer = BytePacketBuffer::new();

    loop {
        let (size, peer) = socket.recv_from(&mut buffer.buf).await?;
        buffer.pos = 0;
        println!("Received {} bytes from {}", size, peer);
        handle_query(&socket, peer, &mut buffer).await?;
    }
}

pub async fn run_server_on(addr: &str) -> Result<()> {
    let addr = addr.parse::<SocketAddr>()?;
    run_server(addr).await
}
