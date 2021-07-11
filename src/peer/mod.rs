use connection::{Connection, Message};
use std::net::SocketAddrV4;
use tokio::net::TcpStream;

mod connection;

#[derive(Debug)]
pub enum PeerMessage {
    Ping(String),
    GetPeersRequest,
    GetPeersResponse(Vec<SocketAddrV4>),
}

#[derive(Debug)]
pub struct Peer {
    pub remote_addr: SocketAddrV4,
    connection: Connection<TcpStream>,
}

impl Peer {
    pub async fn connect(addr: SocketAddrV4, self_addr: SocketAddrV4) -> anyhow::Result<Peer> {
        let mut connection = Connection::new(TcpStream::connect(addr).await?);
        connection.write_msg(Message::Connected(self_addr)).await?;
        Ok(Peer {
            remote_addr: addr,
            connection,
        })
    }

    pub async fn connected(connection: TcpStream) -> anyhow::Result<Peer> {
        let mut connection = Connection::new(connection);

        let message = connection.read_msg().await?;
        if let Message::Connected(remote_addr) = message {
            Ok(Peer {
                remote_addr,
                connection,
            })
        } else {
            Err(anyhow::anyhow!("Bad hello message: {:?}", message))
        }
    }

    pub async fn read_msg(&mut self) -> anyhow::Result<PeerMessage> {
        Ok(match self.connection.read_msg().await? {
            Message::GetPeersRequest => PeerMessage::GetPeersRequest,
            Message::Ping(ping) => PeerMessage::Ping(ping),
            Message::GetPeersResponse(addrs) => PeerMessage::GetPeersResponse(addrs),
            msg => return Err(anyhow::anyhow!("Got bad message {:?}", msg)),
        })
    }

    pub async fn write_msg(&mut self, msg: PeerMessage) -> anyhow::Result<()> {
        let msg = match msg {
            PeerMessage::Ping(ping) => Message::Ping(ping),
            PeerMessage::GetPeersRequest => Message::GetPeersRequest,
            PeerMessage::GetPeersResponse(addrs) => Message::GetPeersResponse(addrs),
        };
        self.connection.write_msg(msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::Peer;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connect() {
        let addr = "127.0.0.1:2050".parse().unwrap();
        let self_addr = "127.0.0.1:2055".parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        let _peer = Peer::connect(addr, self_addr).await.unwrap();
        let (accepted, _) = listener.accept().await.unwrap();

        let connected_peer = Peer::connected(accepted).await.unwrap();

        assert_eq!(connected_peer.remote_addr, self_addr);
    }
}
