use bytes::{Buf, BytesMut};
use std::{io::Cursor, net::SocketAddrV4, usize};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Connect,
    Connected(SocketAddrV4),
    GetPeersRequest,
    Ping(String),
    GetPeersResponse(Vec<SocketAddrV4>),
}

#[derive(Debug)]
pub struct Connection<RW> {
    connection: BufWriter<RW>,
    buf: BytesMut,
}

impl<RW: AsyncWrite + AsyncRead + Unpin> Connection<RW> {
    pub fn new(connection: RW) -> Self {
        Self {
            connection: BufWriter::with_capacity(1024, connection),
            buf: BytesMut::with_capacity(1024),
        }
    }

    pub async fn read_msg(&mut self) -> anyhow::Result<Message> {
        loop {
            if let Some(msg) = self.parse_msg()? {
                return Ok(msg);
            }
            if self.connection.read_buf(&mut self.buf).await? == 0 {
                return Err(anyhow::anyhow!("Connection closed"));
            }
        }
    }

    fn parse_msg(&mut self) -> anyhow::Result<Option<Message>> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        let mut cursor = Cursor::new(&self.buf[..]);

        let msg = match cursor.get_u8() {
            0 => Message::GetPeersRequest,
            1 => {
                if cursor.remaining() < 8 {
                    return Ok(None);
                }
                let msg_size = cursor.get_u64() as usize;
                if cursor.remaining() < msg_size {
                    return Ok(None);
                }
                let position = cursor.position() as usize;
                let ping_msg =
                    std::str::from_utf8(&cursor.get_ref()[position..msg_size + position])?
                        .to_owned();
                let msg = Message::Ping(ping_msg);
                cursor.advance(msg_size);
                msg
            }
            2 => {
                if cursor.remaining() < 8 {
                    return Ok(None);
                }
                let addrs_count = cursor.get_u64() as usize;
                let serialized_size = addrs_count * 6;
                if cursor.remaining() < serialized_size {
                    return Ok(None);
                }
                let mut addrs = Vec::with_capacity(addrs_count);
                for _ in 0..addrs_count {
                    let ip = [
                        cursor.get_u8(),
                        cursor.get_u8(),
                        cursor.get_u8(),
                        cursor.get_u8(),
                    ];
                    let port = cursor.get_u16();
                    addrs.push(SocketAddrV4::new(ip.into(), port))
                }
                Message::GetPeersResponse(addrs)
            }
            3 => Message::Connect,
            4 => {
                if cursor.remaining() < 6 {
                    return Ok(None);
                }
                let ip = [
                    cursor.get_u8(),
                    cursor.get_u8(),
                    cursor.get_u8(),
                    cursor.get_u8(),
                ];
                let port = cursor.get_u16();
                Message::Connected(SocketAddrV4::new(ip.into(), port))
            }
            _ => unreachable!(),
        };
        let pos = cursor.position() as usize;
        self.buf.advance(pos);
        Ok(Some(msg))
    }

    pub async fn write_msg(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::GetPeersRequest => {
                self.connection.write_u8(0).await?;
            }
            Message::Ping(ping) => {
                self.connection.write_u8(1).await?;
                let ping_bytes = ping.as_bytes();
                self.connection.write_u64(ping_bytes.len() as u64).await?;
                self.connection.write_all(&ping_bytes).await?;
            }
            Message::GetPeersResponse(peers) => {
                self.connection.write_u8(2).await?;
                self.connection.write_u64(peers.len() as u64).await?;
                for peer in peers {
                    self.connection.write_all(&peer.ip().octets()).await?;
                    self.connection.write_u16(peer.port()).await?
                }
            }
            Message::Connect => {
                self.connection.write_u8(3).await?;
            }
            Message::Connected(ip) => {
                self.connection.write_u8(4).await?;
                self.connection.write_all(&ip.ip().octets()).await?;
                self.connection.write_u16(ip.port()).await?;
            }
        }
        self.connection.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Connection, Message};
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;
    use std::io::Cursor;
    use tokio::runtime::Runtime;

    impl Arbitrary for Message {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let slice = &[
                Message::Connect,
                Message::Connected(Arbitrary::arbitrary(g)),
                Message::GetPeersRequest,
                Message::GetPeersResponse(Arbitrary::arbitrary(g)),
                Message::Ping(Arbitrary::arbitrary(g)),
            ];
            g.choose(slice).unwrap().clone()
        }
    }

    #[quickcheck]
    fn test_read_write(message: Message) -> bool {
        let mut connection = Connection::new(Cursor::new(vec![]));
        let received_msg = async {
            connection.write_msg(message.clone()).await.unwrap();
            connection.connection.get_mut().set_position(0);
            connection.read_msg().await.unwrap()
        };
        let received_msg = Runtime::new().unwrap().block_on(received_msg);

        message == received_msg
    }
}
