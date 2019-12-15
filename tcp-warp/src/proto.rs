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
            TcpWarpMessage::HostConnect {
                connection_id,
                host_port,
            } => {
                dst.reserve(1 + 16 + 2);
                dst.put_u8(2);
                dst.put_u128(connection_id.as_u128());
                dst.put_u16(host_port);
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
            Some(2) if src.len() > (16 + 2) => {
                src.advance(1);
                let connection_id = Uuid::from_slice(&src[0..16]).unwrap();
                let host_port = u16::from_be_bytes(src[16..18].try_into().unwrap());
                Some(TcpWarpMessage::HostConnect {
                    connection_id,
                    host_port,
                })
            }
            _ => None,
        })
    }
}

/// Command types:
/// 1 - add ports
/// 2 - host connect u128 u16
#[derive(Debug)]
pub enum TcpWarpMessage {
    AddPorts(Vec<u16>),
    BytesClient {
        connection_id: Uuid,
        data: BytesMut,
    },
    BytesServer {
        data: BytesMut,
    },
    Connect {
        connection_id: Uuid,
        host_port: u16,
        sender: Sender<TcpWarpMessage>,
    },
    HostConnect {
        connection_id: Uuid,
        host_port: u16,
    },
    Disconnect {
        connection_id: Uuid,
    },
}

pub struct TcpWarpProtoClient {
    pub connection_id: Uuid,
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
        if src.is_empty() {
            return Ok(None);
        }

        Ok(Some(TcpWarpMessage::BytesClient {
            connection_id: self.connection_id,
            data: src.split(),
        }))
    }
}
/*
pub struct TcpWarpProtoHost {
    pub connection_id: Uuid,
}

impl Encoder for TcpWarpProtoHost {
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

impl Decoder for TcpWarpProtoHost {
    type Item = TcpWarpMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<TcpWarpMessage>> {
        if src.is_empty() {
            return Ok(None);
        }

        Ok(Some(TcpWarpMessage::BytesClient {
            connection_id: self.connection_id,
            data: src.split(),
        }))
    }
}

*/
