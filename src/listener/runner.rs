use std::{
    collections::HashSet,
    net::SocketAddrV4,
    pin::Pin,
    sync::{Arc, RwLock},
};

use log::info;
use tokio::sync::{mpsc::UnboundedSender, Notify};
use tokio_stream::{Stream, StreamExt};

use crate::peer::{Peer, PeerMessage};

pub type PingPeriodProvider =
    Arc<dyn Fn() -> Pin<Box<dyn Stream<Item = ()> + Send + 'static>> + Send + Sync + 'static>;

#[derive(Debug)]
pub struct Ping {
    pub ping: String,
    pub addr: SocketAddrV4,
}

#[derive(Clone)]
pub struct PeersRunner {
    remote_peers: Arc<RwLock<HashSet<SocketAddrV4>>>,
    self_addr: SocketAddrV4,
    create_ping_period: PingPeriodProvider,
    get_remote_peers_signal: Arc<Notify>,
    new_peers: UnboundedSender<Peer>,
    pings: UnboundedSender<Ping>,
}

impl PeersRunner {
    pub fn new(
        bind_addr: SocketAddrV4,
        create_ping_period: PingPeriodProvider,
        new_peers: UnboundedSender<Peer>,
        pings: UnboundedSender<Ping>,
    ) -> Self {
        PeersRunner {
            remote_peers: Arc::new(RwLock::new(HashSet::new())),
            self_addr: bind_addr,
            create_ping_period,
            get_remote_peers_signal: Arc::new(Notify::new()),
            new_peers,
            pings,
        }
    }

    pub fn run_peer(&self, peer: Peer) {
        let mut remote_peers = self.remote_peers.write().unwrap();
        if remote_peers.insert(peer.remote_addr) {
            let runner = self.clone();
            tokio::spawn(async move {
                let mut disconnected = Disconnected {
                    runner: &runner,
                    addr: peer.remote_addr,
                    reason: None,
                };
                let res = runner.clone().create_peer_task(peer).await;
                disconnected.reason = Some(res);
            });
        } else {
            info!("Dublicated peer '{}' dropping", peer.remote_addr);
        }
    }

    pub fn request_remote_peers(&self) {
        self.get_remote_peers_signal.notify_one();
    }

    async fn create_peer_task(self, mut peer: Peer) -> anyhow::Result<()> {
        info!("Peer '{}' connected", peer.remote_addr);
        let mut ping_interval = (&self.create_ping_period)();
        loop {
            tokio::select! {
                message = peer.read_msg() => {
                    self.handle_peer_message(&mut peer, message?).await?;
                }
                _ = self.get_remote_peers_signal.notified() => {
                    info!("Asking peers from {}", peer.remote_addr);
                    peer.write_msg(PeerMessage::GetPeersRequest).await?;
                }
                Some(_) = ping_interval.next() => {
                    peer.write_msg(PeerMessage::Ping("Random message".to_owned())).await?;
                }
            }
        }
    }

    async fn handle_peer_message(
        &self,
        peer: &mut Peer,
        message: PeerMessage,
    ) -> anyhow::Result<()> {
        match message {
            PeerMessage::Ping(ping) => {
                self.pings
                    .send(Ping {
                        ping,
                        addr: peer.remote_addr,
                    })
                    .unwrap();
            }
            PeerMessage::GetPeersRequest => {
                let peers = self.remote_peers.read().unwrap().iter().copied().collect();
                peer.write_msg(PeerMessage::GetPeersResponse(peers)).await?;
            }
            PeerMessage::GetPeersResponse(peers) => {
                let remote_peers = self.remote_peers.read().unwrap();
                for peer in peers {
                    if self.self_addr != peer && !remote_peers.contains(&peer) {
                        let peers = self.new_peers.clone();
                        let self_addr = self.self_addr;
                        tokio::spawn(async move {
                            let peer = Peer::connect(peer, self_addr).await;
                            match peer {
                                Ok(peer) => peers.send(peer).unwrap(),
                                Err(err) => info!("Can't connect new peer due to: {:?}", err),
                            }
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

struct Disconnected<'a> {
    runner: &'a PeersRunner,
    addr: SocketAddrV4,
    reason: Option<anyhow::Result<()>>,
}

impl<'a> Drop for Disconnected<'a> {
    fn drop(&mut self) {
        let mut remote_peers = self.runner.remote_peers.write().unwrap();
        remote_peers.remove(&self.addr);
        info!("Peer {} disconnected due to {:#?}", self.addr, self.reason);
    }
}
