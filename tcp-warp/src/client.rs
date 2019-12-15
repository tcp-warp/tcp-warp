use super::*;

pub struct TcpWarpClient {
    bind_address: IpAddr,
    server_address: SocketAddr,
    map: Vec<TcpWarpPortMap>,
}

impl TcpWarpClient {
    pub fn new(bind_address: IpAddr, server_address: SocketAddr, map: Vec<TcpWarpPortMap>) -> Self {
        Self {
            bind_address,
            server_address,
            map,
        }
    }

    pub fn client_port(&self, host_port: u16) -> Option<u16> {
        self.map
            .iter()
            .find(|x| x.host_port == host_port)
            .map(|x| x.client_port)
    }

    pub fn host_port(&self, client_port: u16) -> Option<u16> {
        self.map
            .iter()
            .find(|x| x.client_port == client_port)
            .map(|x| x.host_port)
    }

    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        let stream = TcpStream::connect(&self.server_address).await?;
        let mut transport = Framed::new(stream, TcpWarpProto);

        while let Some(message) = transport.next().await {
            match message? {
                TcpWarpMessage::AddPorts(host_ports) => {
                    for host_port in host_ports {
                        let port = self.client_port(host_port).unwrap_or(host_port);

                        let bind_address = SocketAddr::new(self.bind_address, port);

                        spawn(async move {
                            let mut listener = TcpListener::bind(bind_address).await?;
                            info!("listen: {:?}", bind_address);
                            let mut incoming = listener.incoming();
                            while let Some(Ok(stream)) = incoming.next().await {
                                spawn(async move {
                                    if let Err(e) = process(stream).await {
                                        info!("failed to process connection; error = {}", e);
                                    }
                                });
                            }

                            info!("done listen: {:?}", bind_address);

                            Ok::<(), io::Error>(())
                        });
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
}

async fn process(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    Ok(())
}
