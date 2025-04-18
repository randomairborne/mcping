//! Implementation of the Java Minecraft ping protocol.
//! [Server List Ping](https://wiki.vg/Server_List_Ping)

use std::{
    io::{self, Cursor},
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use super::Pinger;
use crate::{Error, Java, JavaResponse, java::Packet, tokio::AsyncPingable};

impl AsyncPingable for Java {
    type Response = JavaResponse;

    async fn ping(self, pinger: &Pinger) -> Result<(u64, Self::Response), Error> {
        let mut conn = Connection::new(&self.server_address, self.timeout, pinger).await?;

        // Handshake
        conn.send_packet(Packet::Handshake {
            version: 47,
            host: conn.host.clone(),
            port: conn.port,
            next_state: 1,
        })
        .await?;

        // Request
        conn.send_packet(Packet::Request {}).await?;

        let resp = match conn.read_packet().await? {
            Packet::Response { response } => {
                tracing::trace!(
                    response,
                    "Got Minecraft: Java Edition ping response payload"
                );

                serde_json::from_str(&response)?
            }
            _ => return Err(Error::InvalidPacket),
        };

        // Ping Request
        let r = rand::random();
        conn.send_packet(Packet::Ping { payload: r }).await?;

        let ping = match conn.read_packet().await? {
            Packet::Pong { payload } if payload == r => {
                Instant::now().elapsed().as_millis().try_into()?
            }
            _ => return Err(Error::InvalidPacket),
        };

        Ok((ping, resp))
    }
}

trait AsyncReadJavaExt: AsyncRead + AsyncReadExt + Unpin {
    async fn read_varint(&mut self) -> io::Result<i32> {
        let mut res = 0i32;
        for i in 0..5u8 {
            let part = self.read_u8().await?;
            res |= (i32::from(part) & 0x7F) << (7 * i);
            if part & 0x80 == 0 {
                return Ok(res);
            }
        }
        Err(io::Error::other("VarInt too big!"))
    }

    async fn read_string(&mut self) -> io::Result<String> {
        let len: usize = self.read_varint().await?.try_into().map_err(|_v| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Netty string length varint cannot be negative",
            )
        })?;
        if len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Netty string length varint cannot be zero",
            ));
        }
        let mut buf = vec![0; len];
        self.read_exact(&mut buf).await?;
        String::from_utf8(buf)
            .map_err(|_v| io::Error::new(io::ErrorKind::InvalidData, "Bad UTF-8 in Netty string"))
    }
}

impl<T> AsyncReadJavaExt for T where T: AsyncRead + AsyncReadExt + Unpin {}

trait AsyncWriteJavaExt: AsyncWrite + AsyncWriteExt + Unpin {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    async fn write_varint(&mut self, mut val: i32) -> io::Result<()> {
        for _ in 0..5 {
            if val & !0x7F == 0 {
                self.write_u8(val as u8).await?;
                return Ok(());
            }
            self.write_u8((val & 0x7F | 0x80) as u8).await?;
            val >>= 7;
        }
        Err(io::Error::other("VarInt too big!"))
    }

    async fn write_string(&mut self, s: &str) -> io::Result<()> {
        let len_i32 = s.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Tried to write out of bounds usize as i32 varint",
            )
        })?;
        self.write_varint(len_i32).await?;
        self.write_all(s.as_bytes()).await?;
        Ok(())
    }
}

impl<T> AsyncWriteJavaExt for T where T: AsyncWrite + AsyncWriteExt + Unpin {}

struct Connection {
    stream: TcpStream,
    host: String,
    port: u16,
}

impl Connection {
    async fn new(address: &str, timeout: Option<Duration>, pinger: &Pinger) -> Result<Self, Error> {
        // Split the address up into it's parts, saving the host and port for later and converting the
        // potential domain into an ip
        let mut parts = address.split(':');

        let host = parts.next().ok_or(Error::InvalidAddress)?.to_string();

        // If a port exists we want to try and parse it and if not we will
        // default to 25565 (Minecraft)
        let mut port = if let Some(port) = parts.next() {
            port.parse::<u16>().map_err(|_| Error::InvalidAddress)?
        } else {
            25565
        };

        // Determine what host to lookup by doing the following:
        // - Lookup the SRV record for the domain, if it exists perform a lookup of the ip from the target
        //   and grab the port pointed at by the record.
        //
        //   Note: trust_dns_resolver should do a recursive lookup for an ip but it doesn't seem to at
        //   the moment.
        //
        // - If the above failed in any way fall back to the normal ip lookup from the host provided
        //   and use the provided port.

        let srv_lookup = pinger
            .resolver
            .srv_lookup(format!("_minecraft._tcp.{}.", &host))
            .await
            .ok();
        let ip: IpAddr = match srv_lookup {
            Some(lookup) => match lookup.into_iter().next() {
                Some(record) => {
                    port = record.port();
                    pinger
                        .resolver
                        .lookup_ip(record.target().to_string())
                        .await
                        .ok()
                        .and_then(|lookup_ip| lookup_ip.into_iter().next())
                }
                None => None,
            },
            None => pinger
                .resolver
                .lookup_ip(host.clone())
                .await
                .ok()
                .and_then(|lookup_ip| lookup_ip.into_iter().next()),
        }
        .ok_or(Error::DnsLookupFailed)?;
        let socket_addr = SocketAddr::new(ip, port);

        let stream = TcpStream::connect(&socket_addr).await?.into_std()?;

        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;

        Ok(Self {
            stream: TcpStream::from_std(stream)?,
            host,
            port,
        })
    }

    async fn send_packet(&mut self, p: Packet) -> Result<(), Error> {
        let mut buf = Vec::new();
        match p {
            Packet::Handshake {
                version,
                host,
                port,
                next_state,
            } => {
                buf.write_varint(0x00).await?;
                buf.write_varint(version).await?;
                buf.write_string(&host).await?;
                buf.write_u16(port).await?;
                buf.write_varint(next_state).await?;
            }
            Packet::Request {} => {
                buf.write_varint(0x00).await?;
            }
            Packet::Ping { payload } => {
                buf.write_varint(0x01).await?;
                buf.write_u64(payload).await?;
            }
            _ => return Err(Error::InvalidPacket),
        }
        self.stream.write_varint(buf.len().try_into()?).await?;
        self.stream.write_all(&buf).await?;
        Ok(())
    }

    async fn read_packet(&mut self) -> Result<Packet, Error> {
        let len = self.stream.read_varint().await?.try_into()?;
        let mut buf = vec![0; len];
        self.stream.read_exact(&mut buf).await?;
        let mut c = Cursor::new(buf);

        Ok(match c.read_varint().await? {
            0x00 => Packet::Response {
                response: c.read_string().await?,
            },
            0x01 => Packet::Pong {
                payload: c.read_u64().await?,
            },
            _ => return Err(Error::InvalidPacket),
        })
    }
}
