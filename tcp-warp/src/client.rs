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

    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        let stream = TcpStream::connect(&self.server_address).await?;
        let (mut wtransport, mut rtransport) = Framed::new(stream, TcpWarpProto).split();

        let (sender, mut receiver) = unbounded_channel();

        let receiver_task = async move {
            debug!("in receiver task");
            while let Some(message) = receiver.next().await {
                debug!("just received a message: {:?}", message);
                wtransport.send(message).await?;
            }

            debug!("no more messages, closing receiver task");

            Ok::<(), io::Error>(())
        };

        let mapping = self.map.clone();
        let bind_address = self.bind_address;

        let processing_task = async move {
            while let Some(Ok(message)) = rtransport.next().await {
                process_host_to_client_message(message, sender.clone(), &mapping, bind_address)
                    .await?;
            }

            Ok::<(), io::Error>(())
        };

        let (_, _) = try_join!(receiver_task, processing_task)?;

        Ok(())
    }
}

async fn process_host_to_client_message(
    message: TcpWarpMessage,
    sender: UnboundedSender<TcpWarpMessage>,
    mapping: &[TcpWarpPortMap],
    bind_address: IpAddr,
) -> Result<(), io::Error> {
    match message {
        TcpWarpMessage::AddPorts(host_ports) => {
            for host_port in host_ports {
                let port = client_port(mapping, host_port).unwrap_or(host_port);

                let bind_address = SocketAddr::new(bind_address, port);
                let sender_ = sender.clone();
                spawn(async move {
                    let mut listener = TcpListener::bind(bind_address).await?;
                    info!("listen: {:?}", bind_address);
                    let mut incoming = listener.incoming();

                    while let Some(Ok(stream)) = incoming.next().await {
                        let sender__ = sender_.clone();
                        spawn(async move {
                            if let Err(e) = process(stream, sender__, host_port, port).await {
                                info!("failed to process connection; error = {}", e);
                            }
                        });
                    }

                    info!("done listen: {:?}", bind_address);

                    Ok::<(), io::Error>(())
                });
            }
        }
        other_message => warn!("unsupported message: {:?}", other_message),
    }
    Ok(())
}

fn client_port(mapping: &[TcpWarpPortMap], host_port: u16) -> Option<u16> {
    mapping
        .iter()
        .find(|x| x.host_port == host_port)
        .map(|x| x.client_port)
}

fn host_port(mapping: &[TcpWarpPortMap], client_port: u16) -> Option<u16> {
    mapping
        .iter()
        .find(|x| x.client_port == client_port)
        .map(|x| x.host_port)
}

async fn process(
    stream: TcpStream,
    sender: UnboundedSender<TcpWarpMessage>,
    host_port: u16,
    client_port: u16,
) -> Result<(), Box<dyn Error>> {
    sender.send(TcpWarpMessage::CloseConnection)?;
    Ok(())
}
