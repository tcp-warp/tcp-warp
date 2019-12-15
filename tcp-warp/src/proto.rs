use super::*;
use std::io;

pub struct TcpWarpProto;

impl Encoder for TcpWarpProto {
    type Item = TcpWarpMessage;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> io::Result<()> {
        match item {
            TcpWarpMessage::AddPorts(ports) => {
                dst.reserve(1 + 2 + ports.len() * 2);
                dst.put_u8(1);
                dst.put_u16(ports.len() as u16);
                for port in ports {
                    dst.put_u16(port);
                }
            }
            _ => {
                error!("unknown message");
            }
        }
        Ok(())
    }
}

impl Decoder for TcpWarpProto {
    type Item = TcpWarpMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<TcpWarpMessage>> {
        Ok(match src.get(0) {
            Some(1) if src.len() > 3 => {
                let len = u16::from_be_bytes(src[1..3].try_into().unwrap());
                if len as usize * 2 + 3 >= src.len() {
                    src.advance(3);
                    let data = src.split_to(len as usize * 2);
                    let ports = data
                        .chunks_exact(2)
                        .map(|x| u16::from_be_bytes(x.try_into().unwrap()))
                        .collect();
                    Some(TcpWarpMessage::AddPorts(ports))
                } else {
                    None
                }
            }
            _ => None,
        })
    }
}

/// Command types:
/// 1 - add ports
#[derive(Debug)]
pub enum TcpWarpMessage {
    AddPorts(Vec<u16>),
    BytesClient {
        connection_id: Uuid,
        host_port: u16,
        client_port: u16,
        data: BytesMut,
    },
    BytesServer {
        data: BytesMut,
    },
    Connect {
        connection_id: Uuid,
        host_port: u16,
        client_sender: Sender<TcpWarpMessage>,
    },
    Disconnect {
        connection_id: Uuid,
    },
}

pub struct TcpWarpProtoClient {
    pub connection_id: Uuid,
    pub host_port: u16,
    pub client_port: u16,
}

impl Encoder for TcpWarpProtoClient {
    type Item = TcpWarpMessage;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> io::Result<()> {
        match item {
            TcpWarpMessage::BytesServer { data } => {
                dst.extend_from_slice(&data);
            }
            _ => {
                error!("unsupported message");
            }
        }
        Ok(())
    }
}

impl Decoder for TcpWarpProtoClient {
    type Item = TcpWarpMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<TcpWarpMessage>> {
        Ok(Some(TcpWarpMessage::BytesClient {
            connection_id: self.connection_id,
            host_port: self.host_port,
            client_port: self.client_port,
            data: src.split(),
        }))
    }
}
