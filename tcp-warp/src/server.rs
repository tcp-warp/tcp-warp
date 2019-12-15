use super::*;

pub struct TcpWarpServer {
    listen_address: SocketAddr,
    connect_address: IpAddr,
}

impl TcpWarpServer {
    pub fn new(listen_address: SocketAddr, connect_address: IpAddr) -> Self {
        Self {
            listen_address,
            connect_address,
        }
    }

    pub async fn listen(&self) -> Result<(), Box<dyn Error>> {
        let mut listener = TcpListener::bind(&self.listen_address).await?;
        let mut incoming = listener.incoming();
        let connect_address = self.connect_address;

        while let Some(Ok(stream)) = incoming.next().await {
            spawn(async move {
                if let Err(e) = process(stream, connect_address).await {
                    println!("failed to process connection; error = {}", e);
                }
            });
        }
        Ok(())
    }
}

async fn process(stream: TcpStream, connect_address: IpAddr) -> Result<(), Box<dyn Error>> {
    let mut transport = Framed::new(stream, TcpWarpProto);

    transport
        .send(TcpWarpMessage::AddPorts(vec![8081, 8082]))
        .await?;

    let (mut wtransport, mut rtransport) = transport.split();

    let (sender, mut receiver) = channel(100);

    let mut connections = HashMap::new();

    let forward_task = async move {
        debug!("in receiver task process");
        while let Some(message) = receiver.next().await {
            match message {
                TcpWarpMessage::Connect {
                    connection_id,
                    sender,
                    host_port: _,
                } => {
                    debug!("adding connection: {}", connection_id);
                    connections.insert(connection_id.clone(), sender.clone());
                    continue;
                }
                TcpWarpMessage::Disconnect { ref connection_id } => {
                    connections.remove(connection_id);
                    break;
                }
                _ => (),
            }
            wtransport.send(message).await?;
        }
        Ok::<(), io::Error>(())
    };

    let processing_task = async move {
        while let Some(Ok(message)) = rtransport.next().await {
            debug!("received {:?}", message);
            spawn(process_client_to_host_message(
                message,
                sender.clone(),
                connect_address,
            ));
        }

        Ok::<(), io::Error>(())
    };
    let (_, _) = try_join!(forward_task, processing_task)?;

    Ok(())
}

async fn process_client_to_host_message(
    message: TcpWarpMessage,
    client_sender: Sender<TcpWarpMessage>,
    connect_address: IpAddr,
) -> Result<(), io::Error> {
    match message {
        TcpWarpMessage::HostConnect {
            connection_id,
            host_port,
        } => {
            let connect_address = SocketAddr::new(connect_address, host_port);
            let client_sender_ = client_sender.clone();
            spawn(async move {
                if let Err(err) = process_host_connection(
                    connect_address,
                    client_sender_,
                    connection_id,
                    host_port,
                )
                .await
                {
                    error!(
                        "failed connection {} {}: {}",
                        connect_address, connection_id, err
                    );
                }
            });
        }
        other_message => warn!("unsupported message: {:?}", other_message),
    }
    Ok(())
}

async fn process_host_connection(
    connect_address: SocketAddr,
    mut client_sender: Sender<TcpWarpMessage>,
    connection_id: Uuid,
    host_port: u16,
) -> Result<(), Box<dyn Error>> {
    debug!("new connection: {}", connection_id);
    let stream = TcpStream::connect(connect_address).await?;

    let (mut wtransport, mut rtransport) =
        Framed::new(stream, TcpWarpProtoClient { connection_id }).split();

    let (host_sender, mut host_receiver) = channel(100);

    let forward_task = async move {
        debug!("in receiver task process_host_connection");

        while let Some(message) = host_receiver.next().await {
            debug!("just received a message: {:?}", message);
            wtransport.send(message).await?;
        }

        debug!("no more messages, closing forward task");

        Ok::<(), io::Error>(())
    };

    client_sender
        .send(TcpWarpMessage::Connect {
            connection_id,
            host_port,
            sender: host_sender,
        })
        .await?;

    let mut client_sender_ = client_sender.clone();
    let processing_task = async move {
        while let Some(Ok(message)) = rtransport.next().await {
            if let Err(err) = client_sender_.send(message).await {
                error!("{}", err);
            }
        }
        Ok::<(), io::Error>(())
    };

    try_join!(forward_task, processing_task)?;

    client_sender
        .send(TcpWarpMessage::Disconnect { connection_id })
        .await?;

    Ok(())
}
