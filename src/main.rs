use listener::start;
use log::info;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::interval;
use tokio_stream::{wrappers::IntervalStream, Stream, StreamExt};

mod listener;
mod peer;

#[derive(Debug, StructOpt)]
#[structopt(name = "p2p", about = "A p2p test task.")]
struct Opts {
    #[structopt(short, long)]
    ping_period_ms: u64,

    #[structopt(short, long)]
    bind: SocketAddrV4,

    #[structopt(short, long)]
    connect: Option<SocketAddrV4>,
}

#[tokio::main]
async fn main() {
    let opt = Opts::from_args();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let (pings_tx, mut pings_rx) = unbounded_channel();
    let ping_period_ms = opt.ping_period_ms;
    let pings_provider = Arc::new(move || {
        Box::pin(IntervalStream::new(interval(Duration::from_millis(ping_period_ms))).map(|_| ()))
            as Pin<Box<dyn Stream<Item = ()> + Send + 'static>>
    });
    let peer_handle = start(
        opt.bind,
        opt.connect,
        pings_provider,
        Duration::from_secs(10),
        pings_tx,
    );
    let pings_handle = async move {
        loop {
            let ping = pings_rx.recv().await.unwrap();
            info!("Got ping '{}' from {}", ping.ping, ping.addr);
        }
    };
    let (peer, _ping) = tokio::join!(peer_handle, pings_handle);
    peer.unwrap();
}

#[cfg(test)]
mod tests {
    use super::start;
    use futures::{stream::StreamExt, Stream};
    use std::{net::SocketAddrV4, pin::Pin};
    use std::{sync::Arc, time::Duration};
    use tokio::sync::mpsc::unbounded_channel;

    #[tokio::test]
    async fn test_multi_peers() {
        let first_addr = "127.0.0.1:2555".parse().unwrap();
        let (pings_tx, mut pings_rx) = unbounded_channel();
        let pings_count = 500;
        let pings = Arc::new(move || {
            Box::pin(futures::stream::repeat(()).take(pings_count))
                as Pin<Box<dyn Stream<Item = ()> + Send + 'static>>
        });
        let first_peer = tokio::spawn(start(
            first_addr,
            None,
            pings.clone(),
            Duration::from_millis(1),
            pings_tx.clone(),
        ));
        tokio::task::yield_now().await;
        let mut peers = vec![first_peer];
        for port in 2556..2565 {
            peers.push(tokio::spawn(start(
                SocketAddrV4::new(*first_addr.ip(), port),
                Some(SocketAddrV4::new(*first_addr.ip(), port - 1)),
                pings.clone(),
                Duration::from_nanos(1),
                pings_tx.clone(),
            )))
        }
        let mut count = 0;
        let expected = pings_count * peers.len() * (peers.len() - 1);

        loop {
            pings_rx.recv().await;
            count += 1;
            if count == expected {
                return;
            }
        }
    }
}
