use bytes::{Buf, BufMut, BytesMut};
use futures::{
    future::{abortable, AbortHandle},
    prelude::*,
    try_join,
};
use log::*;
use std::{
    collections::HashMap,
    convert::TryInto,
    error::Error,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    spawn,
    sync::{
        mpsc::{channel, Sender},
        oneshot,
    },
    time::delay_for,
};
use tokio_util::codec::{Decoder, Encoder, Framed};
use uuid::Uuid;

mod client;
mod proto;
mod server;

pub use client::TcpWarpClient;
pub use proto::{TcpWarpMessage, TcpWarpProto, TcpWarpProtoClient, TcpWarpProtoHost};
pub use server::TcpWarpServer;

#[derive(Debug, Clone, PartialEq)]
pub struct TcpWarpPortConnection {
    client_port: Option<u16>,
    host: Option<String>,
    port: u16,
}

impl FromStr for TcpWarpPortConnection {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');

        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(client_port), host, Some(port), None) => {
                match (client_port.parse(), port.parse()) {
                    (Ok(client_port), Ok(port)) => Ok(TcpWarpPortConnection {
                        client_port: Some(client_port),
                        host: host.map(str::to_owned),
                        port,
                    }),
                    _ => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "cannot parse port mapping",
                    )),
                }
            }
            (Some(client_port_or_host), Some(port), None, None) => {
                let port = match port.parse() {
                    Ok(port) => port,
                    Err(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "cannot parse port mapping",
                        ))
                    }
                };
                let client_port: Result<u16, _> = client_port_or_host.parse();
                if let Ok(client_port) = client_port {
                    Ok(TcpWarpPortConnection {
                        client_port: Some(client_port),
                        host: None,
                        port,
                    })
                } else {
                    Ok(TcpWarpPortConnection {
                        client_port: None,
                        host: Some(client_port_or_host.to_owned()),
                        port,
                    })
                }
            }
            (Some(port), None, None, None) => {
                let port = match port.parse() {
                    Ok(port) => port,
                    Err(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "cannot parse port mapping",
                        ))
                    }
                };
                Ok(TcpWarpPortConnection {
                    client_port: None,
                    host: None,
                    port,
                })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "cannot parse port mapping",
            )),
        }
    }
}

pub struct TcpWarpConnection {
    sender: Sender<TcpWarpMessage>,
    connected_sender: Option<oneshot::Sender<Result<(), io::Error>>>,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn connection_from_str() {
        assert_eq!(
            Ok(TcpWarpPortConnection {
                client_port: None,
                host: None,
                port: 8080
            }),
            "8080".parse().map_err(|_| ())
        );
        assert_eq!(
            Ok(TcpWarpPortConnection {
                client_port: Some(8081),
                host: None,
                port: 8080
            }),
            "8081:8080".parse().map_err(|_| ())
        );
        assert_eq!(
            Ok(TcpWarpPortConnection {
                client_port: Some(8081),
                host: Some("localhost".into()),
                port: 8080
            }),
            "8081:localhost:8080".parse().map_err(|_| ())
        );
    }
}
