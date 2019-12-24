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
                host,
                port,
            } => {
                let len = host.as_ref().map_or(0, |x| x.len());
                dst.reserve(1 + 2 + 16 + 2 + len);
                dst.put_u8(2);
                dst.put_u16(len as u16);
                dst.put_u128(connection_id.as_u128());
                dst.put_u16(port);
                if let Some(data) = host {
                    dst.put_slice(data.as_bytes());
                }
            }
            TcpWarpMessage::BytesClient {
                connection_id,
                data,
            } => {
                dst.reserve(1 + 16 + 4 + data.len());
                dst.put_u8(3);
                dst.put_u128(connection_id.as_u128());
                dst.put_u32(data.len() as u32);
                dst.put_slice(&data);
            }
            TcpWarpMessage::BytesHost {
                connection_id,
                data,
            } => {
                dst.reserve(1 + 16 + 4 + data.len());
                dst.put_u8(4);
                dst.put_u128(connection_id.as_u128());
                dst.put_u32(data.len() as u32);
                dst.put_slice(&data);
            }
            TcpWarpMessage::Connected { connection_id } => {
                dst.reserve(1 + 16);
                dst.put_u8(5);
                dst.put_u128(connection_id.as_u128());
            }
            TcpWarpMessage::DisconnectHost { connection_id } => {
                dst.reserve(1 + 16);
                dst.put_u8(6);
                dst.put_u128(connection_id.as_u128());
            }
            TcpWarpMessage::DisconnectClient { connection_id } => {
                dst.reserve(1 + 16);
                dst.put_u8(7);
                dst.put_u128(connection_id.as_u128());
            }
            TcpWarpMessage::ConnectFailure { connection_id } => {
                dst.reserve(1 + 16);
                dst.put_u8(8);
                dst.put_u128(connection_id.as_u128());
            }
            other => {
                error!("unknown message: {:?}", other);
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
            Some(1) if src.len() > 2 => {
                let len = u16::from_be_bytes(src[1..3].try_into().unwrap());
                if src.len() == 3 && len == 0 {
                    src.advance(3);
                    return Ok(Some(TcpWarpMessage::AddPorts(vec![])));
                }
                if len as usize * 2 + 3 <= src.len() {
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
                let len = u16::from_be_bytes(src[1..3].try_into().unwrap()) as usize;
                if 2 + 16 + 2 + len < src.len() {
                    src.advance(3);
                    let header = src.split_to(18);
                    let connection_id = Uuid::from_slice(&header[0..16]).unwrap();
                    let port = u16::from_be_bytes(header[16..18].try_into().unwrap());
                    let host = if len > 0 {
                        String::from_utf8(src.split_to(len).to_vec()).ok()
                    } else {
                        None
                    };
                    Some(TcpWarpMessage::HostConnect {
                        connection_id,
                        host,
                        port,
                    })
                } else {
                    None
                }
            }
            Some(3) if src.len() > (16 + 4 + 1) => {
                let len = u32::from_be_bytes(src[17..21].try_into().unwrap()) as usize;
                if len as usize + 16 + 4 < src.len() {
                    src.advance(1);
                    let header = src.split_to(20);
                    let connection_id = Uuid::from_slice(&header[0..16]).unwrap();
                    let data = src.split_to(len);
                    Some(TcpWarpMessage::BytesClient {
                        connection_id,
                        data,
                    })
                } else {
                    None
                }
            }
            Some(4) if src.len() > (16 + 4 + 1) => {
                let len = u32::from_be_bytes(src[17..21].try_into().unwrap()) as usize;
                if len as usize + 16 + 4 < src.len() {
                    src.advance(1);
                    let header = src.split_to(20);
                    let connection_id = Uuid::from_slice(&header[0..16]).unwrap();
                    let data = src.split_to(len);
                    Some(TcpWarpMessage::BytesHost {
                        connection_id,
                        data,
                    })
                } else {
                    None
                }
            }
            Some(5) if src.len() > 16 => {
                src.advance(1);
                let header = src.split_to(16);
                let connection_id = Uuid::from_slice(&header).unwrap();
                Some(TcpWarpMessage::Connected { connection_id })
            }
            Some(6) if src.len() > 16 => {
                src.advance(1);
                let header = src.split_to(16);
                let connection_id = Uuid::from_slice(&header).unwrap();
                Some(TcpWarpMessage::DisconnectHost { connection_id })
            }
            Some(7) if src.len() > 16 => {
                src.advance(1);
                let header = src.split_to(16);
                let connection_id = Uuid::from_slice(&header).unwrap();
                Some(TcpWarpMessage::DisconnectClient { connection_id })
            }
            Some(8) if src.len() > 16 => {
                src.advance(1);
                let header = src.split_to(16);
                let connection_id = Uuid::from_slice(&header).unwrap();
                Some(TcpWarpMessage::ConnectFailure { connection_id })
            }
            _ => {
                debug!("looks like data is wrong [{}] {:?}", src.len(), src);
                None
            } // _ => None,
        })
    }
}

/// Command types.
///
/// Serialization scheme:
/// - 1 - add ports u16 len * u16
/// - 2 - host connect u16=(len + 2) u128 u16 len * u8
/// - 3 - bytes client u128 u32 len * u8
/// - 4 - bytes host u128 u32 len * u8
/// - 5 - connected u128
/// - 6 - disconnect host u128
/// - 7 - disconnect client u128
#[derive(Debug)]
pub enum TcpWarpMessage {
    AddPorts(Vec<u16>),
    Connected {
        connection_id: Uuid,
    },
    BytesClient {
        connection_id: Uuid,
        data: BytesMut,
    },
    BytesServer {
        data: BytesMut,
    },
    BytesHost {
        connection_id: Uuid,
        data: BytesMut,
    },
    Connect {
        connection_id: Uuid,
        connection: TcpWarpPortConnection,
        sender: Sender<TcpWarpMessage>,
        connected_sender: oneshot::Sender<Result<(), io::Error>>,
    },
    ConnectForward {
        connection_id: Uuid,
        sender: Sender<TcpWarpMessage>,
        connected_sender: oneshot::Sender<Result<(), io::Error>>,
    },
    ConnectFailure {
        connection_id: Uuid,
    },
    Disconnect,
    Listener(AbortHandle),
    HostConnect {
        connection_id: Uuid,
        host: Option<String>,
        port: u16,
    },
    DisconnectHost {
        connection_id: Uuid,
    },
    DisconnectClient {
        connection_id: Uuid,
    },
}

pub struct TcpWarpProtoClient {
    pub connection_id: Uuid,
}

impl Encoder for TcpWarpProtoClient {
    type Item = BytesMut;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> io::Result<()> {
        dst.extend_from_slice(&item);
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

pub struct TcpWarpProtoHost {
    pub connection_id: Uuid,
}

impl Encoder for TcpWarpProtoHost {
    type Item = BytesMut;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> io::Result<()> {
        dst.extend_from_slice(&item);
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

        Ok(Some(TcpWarpMessage::BytesHost {
            connection_id: self.connection_id,
            data: src.split(),
        }))
    }
}
