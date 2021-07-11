use crate::peer::Peer;
use log::info;
use std::{net::SocketAddrV4, time::Duration};
use tokio::{
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    time::interval,
};

use self::runner::PeersRunner;

mod runner;
pub use runner::{Ping, PingPeriodProvider};

pub async fn start(
    bind_addr: SocketAddrV4,
    initial_peer: Option<SocketAddrV4>,
    create_ping_period: PingPeriodProvider,
    get_remote_peers_interval: Duration,
    pings_sink: UnboundedSender<Ping>,
) -> anyhow::Result<()> {
    let socket = TcpListener::bind(bind_addr).await?;
    let (peers_tx, mut peers_rx) = unbounded_channel();
    let peers_runner =
        PeersRunner::new(bind_addr, create_ping_period, peers_tx.clone(), pings_sink);
    if let Some(initial) = initial_peer {
        peers_runner.run_peer(Peer::connect(initial, bind_addr).await?);
    }
    let mut get_remote_peers_interval = interval(get_remote_peers_interval);
    loop {
        tokio::select! {
            Ok((connection, _)) = socket.accept() => {
                let peers_tx = peers_tx.clone();
                tokio::spawn(async move {
                    let peer = Peer::connected(connection).await;
                    match peer {
                        Ok(peer) => peers_tx.send(peer).unwrap(),
                        Err(err) => info!("Can't connect accepted peer due to: {:?}", err),
                    }
                });
            }

            Some(peer) = peers_rx.recv() => {
                peers_runner.run_peer(peer);
            }

            _ = get_remote_peers_interval.tick() => {
                peers_runner.request_remote_peers()
            }
        }
    }
}
