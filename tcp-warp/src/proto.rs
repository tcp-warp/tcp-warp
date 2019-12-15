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
pub enum TcpWarpMessage {
    AddPorts(Vec<u16>),
    BytesClient { port: u16, client: u16, data: Bytes },
    BytesServer,
    CloseConnection,
}
