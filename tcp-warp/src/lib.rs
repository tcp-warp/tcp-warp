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
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
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

#[derive(Debug, Clone)]
pub struct TcpWarpPortMap {
    host_port: u16,
    client_port: u16,
}

impl FromStr for TcpWarpPortMap {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':').map(FromStr::from_str);

        match (parts.next(), parts.next(), parts.next()) {
            (Some(Ok(host_port)), Some(Ok(client_port)), None) => Ok(TcpWarpPortMap {
                host_port,
                client_port,
            }),
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
