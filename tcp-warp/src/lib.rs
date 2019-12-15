use bytes::{Buf, BufMut, Bytes, BytesMut};
use failure::Fail;
use futures::prelude::*;
use log::*;
use std::{
    convert::TryInto,
    error::Error,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};
use tokio::{
    net::{TcpListener, TcpStream},
    prelude::*,
    spawn,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

mod client;
mod proto;
mod server;

pub use client::TcpWarpClient;
pub use proto::{TcpWarpMessage, TcpWarpProto};
pub use server::TcpWarpServer;

#[derive(Debug)]
pub struct TcpWarpPortMap {
    host_port: u16,
    client_port: u16,
}

#[derive(Debug, Fail)]
#[fail(display = "cannot parse port mapping")]
pub struct TcpWarpParseError;

impl FromStr for TcpWarpPortMap {
    type Err = TcpWarpParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':').map(FromStr::from_str);

        match (parts.next(), parts.next(), parts.next()) {
            (Some(Ok(host_port)), Some(Ok(client_port)), None) => Ok(TcpWarpPortMap {
                host_port,
                client_port,
            }),
            _ => Err(TcpWarpParseError),
        }
    }
}
