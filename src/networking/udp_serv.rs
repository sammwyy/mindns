use async_lock::Mutex;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::error::Error;
use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use super::peer::{UDPPeer, UdpPeer, UdpReader};
use net2::{UdpBuilder, UdpSocketExt};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::unbounded_channel;

///The maximum size of a single UDP packet is 4096 by default. The MTU is generally not more than 1500 on the Internet
///If the LAN is likely to be larger, 4096 is generally enough
pub const BUFF_MAX_SIZE: usize = 4096;

/// UDP Context
/// each bind will create a
pub struct UdpContext {
    pub id: usize,
    recv: Arc<UdpSocket>,
    pub peers: Mutex<HashMap<SocketAddr, UDPPeer>>,
}

unsafe impl Send for UdpContext {}
unsafe impl Sync for UdpContext {}

/// UDP Server listen
pub struct UdpServer<I, T> {
    udp_contexts: Vec<Arc<UdpContext>>,
    input: Arc<I>,
    _ph: PhantomData<T>,
    clean_sec: Option<u64>,
}

impl<I, R, T> UdpServer<I, T>
where
    I: Fn(UDPPeer, UdpReader, T) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Box<dyn Error>>> + Send + 'static,
    T: Sync + Send + Clone + 'static,
{
    /// new udp server
    pub fn new<A: ToSocketAddrs>(addr: A, input: I) -> io::Result<Self> {
        let udp_list = create_udp_socket_list(&addr, get_cpu_count())?;
        let udp_contexts = udp_list
            .into_iter()
            .enumerate()
            .map(|(id, socket)| {
                Arc::new(UdpContext {
                    id,
                    recv: Arc::new(socket),
                    peers: Default::default(),
                })
            })
            .collect();
        Ok(UdpServer {
            udp_contexts,
            input: Arc::new(input),
            _ph: Default::default(),
            clean_sec: None,
        })
    }

    /// set how long the packet is not obtained and close the udp peer
    #[inline]
    pub fn set_peer_timeout_sec(mut self, sec: u64) -> UdpServer<I, T> {
        assert!(sec > 0);
        self.clean_sec = Some(sec);
        self
    }

    /// start server
    #[inline]
    pub async fn start(&self, inner: T) -> io::Result<()> {
        let need_check_timeout = {
            if let Some(clean_sec) = self.clean_sec {
                let clean_sec = clean_sec as i64;
                let contexts = self.udp_contexts.clone();
                tokio::spawn(async move {
                    loop {
                        let current = chrono::Utc::now().timestamp();
                        for context in contexts.iter() {
                            context.peers.lock().await.values().for_each(|peer| {
                                if current - peer.get_last_recv_sec() > clean_sec {
                                    peer.close();
                                }
                            });
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await
                    }
                });
                true
            } else {
                false
            }
        };

        let (tx, mut rx) = unbounded_channel();
        for (index, udp_listen) in self.udp_contexts.iter().enumerate() {
            let create_peer_tx = tx.clone();
            let udp_context = udp_listen.clone();
            tokio::spawn(async move {
                log::debug!("start udp listen:{index}");
                let mut buff = [0; BUFF_MAX_SIZE];
                loop {
                    match udp_context.recv.recv_from(&mut buff).await {
                        Ok((size, addr)) => {
                            let peer = {
                                udp_context
                                    .peers
                                    .lock()
                                    .await
                                    .entry(addr)
                                    .or_insert_with(|| {
                                        let (peer, reader) =
                                            UdpPeer::new(index, udp_context.recv.clone(), addr);
                                        log::trace!("create udp listen:{index} udp peer:{addr}");
                                        if let Err(err) =
                                            create_peer_tx.send((peer.clone(), reader, index, addr))
                                        {
                                            panic!("create_peer_tx err:{}", err);
                                        }
                                        peer
                                    })
                                    .clone()
                            };

                            if need_check_timeout {
                                if let Err(err) = peer
                                    .push_data_and_update_instant(buff[..size].to_vec())
                                    .await
                                {
                                    log::error!("peer push data and update instant is error:{err}");
                                }
                            } else if let Err(err) = peer.push_data(buff[..size].to_vec()) {
                                log::error!("peer push data is error:{err}");
                            }
                        }
                        Err(err) => {
                            log::trace!("udp:{index} recv_from error:{err}");
                        }
                    }
                }
            });
        }
        drop(tx);

        while let Some((peer, reader, index, addr)) = rx.recv().await {
            let inner = inner.clone();
            let input_fn = self.input.clone();
            let context = self
                .udp_contexts
                .get(index)
                .expect("not found context")
                .clone();
            tokio::spawn(async move {
                if let Err(err) = (input_fn)(peer, reader, inner).await {
                    log::error!("udp input error:{err}")
                }
                context.peers.lock().await.remove(&addr);
            });
        }
        Ok(())
    }
}

///Create udp socket for windows
#[cfg(target_os = "windows")]
fn make_udp_client(addr: SocketAddr) -> io::Result<std::net::UdpSocket> {
    if addr.is_ipv4() {
        Ok(UdpBuilder::new_v4()?.reuse_address(true)?.bind(addr)?)
    } else if addr.is_ipv6() {
        Ok(UdpBuilder::new_v6()?.reuse_address(true)?.bind(addr)?)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "not address AF_INET"))
    }
}

///It is used to create udp sockets for non-windows. The difference from windows is that reuse_port
#[cfg(not(target_os = "windows"))]
fn make_udp_client(addr: SocketAddr) -> io::Result<std::net::UdpSocket> {
    use net2::unix::UnixUdpBuilderExt;
    if addr.is_ipv4() {
        Ok(UdpBuilder::new_v4()?
            .reuse_address(true)?
            .reuse_port(true)?
            .bind(addr)?)
    } else if addr.is_ipv6() {
        Ok(UdpBuilder::new_v6()?
            .reuse_address(true)?
            .reuse_port(true)?
            .bind(addr)?)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "not address AF_INET"))
    }
}

///Create a udp socket and set the buffer size
fn create_udp_socket<A: ToSocketAddrs>(addr: &A) -> io::Result<std::net::UdpSocket> {
    let addr = {
        let mut addrs = addr.to_socket_addrs()?;
        let addr = match addrs.next() {
            Some(addr) => addr,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "no socket addresses could be resolved",
                ))
            }
        };
        if addrs.next().is_none() {
            Ok(addr)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "more than one address resolved",
            ))
        }
    };
    let res = make_udp_client(addr?)?;
    res.set_send_buffer_size(1784 * 10000)?;
    res.set_recv_buffer_size(1784 * 10000)?;
    Ok(res)
}

/// From std socket create tokio udp socket
fn create_async_udp_socket<A: ToSocketAddrs>(addr: &A) -> io::Result<UdpSocket> {
    let std_sock = create_udp_socket(&addr)?;
    std_sock.set_nonblocking(true)?;
    let sock = UdpSocket::try_from(std_sock)?;
    Ok(sock)
}

/// create tokio UDP socket list
/// listen_count indicates how many UDP SOCKETS to listen
fn create_udp_socket_list<A: ToSocketAddrs>(
    addr: &A,
    listen_count: usize,
) -> io::Result<Vec<UdpSocket>> {
    log::debug!("cpus:{listen_count}");
    let mut listens = Vec::with_capacity(listen_count);
    for _ in 0..listen_count {
        let sock = create_async_udp_socket(addr)?;
        listens.push(sock);
    }
    Ok(listens)
}

#[cfg(not(target_os = "windows"))]
fn get_cpu_count() -> usize {
    num_cpus::get()
}

#[cfg(target_os = "windows")]
fn get_cpu_count() -> usize {
    1
}
