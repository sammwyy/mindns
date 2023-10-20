use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub type UDPPeer = Arc<UdpPeer>;
pub type UdpSender = UnboundedSender<io::Result<Vec<u8>>>;
pub type UdpReader = UnboundedReceiver<io::Result<Vec<u8>>>;

/// UDP Peer
/// each address+port is equal to one UDP peer
pub struct UdpPeer {
    pub socket_id: usize,
    pub udp_sock: Arc<UdpSocket>,
    pub addr: SocketAddr,
    sender: UdpSender,
    last_read_time: AtomicI64,
}

impl Drop for UdpPeer {
    fn drop(&mut self) {
        log::trace!("drop udp peer:{}", self.addr);
    }
}

impl UdpPeer {
    #[inline]
    pub fn new(
        socket_id: usize,
        udp_sock: Arc<UdpSocket>,
        addr: SocketAddr,
    ) -> (UDPPeer, UdpReader) {
        let (tx, rx) = unbounded_channel();
        (
            Arc::new(Self {
                socket_id,
                udp_sock,
                addr,
                sender: tx,
                last_read_time: AtomicI64::new(timestamp_sec()),
            }),
            rx,
        )
    }

    /// get last recv sec
    #[inline]
    pub(crate) fn get_last_recv_sec(&self) -> i64 {
        self.last_read_time.load(Ordering::Acquire)
    }

    /// push data to read tx
    #[inline]
    pub(crate) fn push_data(&self, buf: Vec<u8>) -> io::Result<()> {
        if let Err(err) = self.sender.send(Ok(buf)) {
            Err(io::Error::new(ErrorKind::Other, err))
        } else {
            Ok(())
        }
    }

    /// push data to read tx and update instant
    #[inline]
    pub(crate) async fn push_data_and_update_instant(&self, buf: Vec<u8>) -> io::Result<()> {
        self.last_read_time
            .store(timestamp_sec(), Ordering::Release);
        self.push_data(buf)
    }

    #[inline]
    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    /// send buf to peer
    #[inline]
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.udp_sock.send_to(buf, &self.addr).await
    }

    #[inline]
    pub fn close(&self) {
        if let Err(err) = self.sender.send(Err(io::Error::new(
            ErrorKind::TimedOut,
            "udp peer need close",
        ))) {
            log::error!("send timeout to udp peer:{} error:{err}", self.get_addr());
        }
    }
}

#[inline]
fn timestamp_sec() -> i64 {
    chrono::Utc::now().timestamp()
}
