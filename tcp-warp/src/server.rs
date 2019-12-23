use super::*;

pub struct TcpWarpServer {
    listen_address: SocketAddr,
    connect_address: IpAddr,
    ports: Vec<u16>,
}

impl TcpWarpServer {
    pub fn new(listen_address: SocketAddr, connect_address: IpAddr, ports: Vec<u16>) -> Self {
        Self {
            listen_address,
            connect_address,
            ports,
        }
    }

    pub async fn listen(&self) -> Result<(), Box<dyn Error>> {
        let mut listener = TcpListener::bind(&self.listen_address).await?;
        let mut incoming = listener.incoming();
        let connect_address = self.connect_address;

        while let Some(Ok(stream)) = incoming.next().await {
            let ports = self.ports.clone();
            spawn(async move {
                if let Err(e) = process(stream, connect_address, ports).await {
                    println!("failed to process connection; error = {}", e);
                }
            });
        }
        Ok(())
    }
}

async fn process(
    stream: TcpStream,
    connect_address: IpAddr,
    ports: Vec<u16>,
) -> Result<(), Box<dyn Error>> {
    let mut transport = Framed::new(stream, TcpWarpProto);

    transport.send(TcpWarpMessage::AddPorts(ports)).await?;

    let (mut wtransport, mut rtransport) = transport.split();

    let (sender, mut receiver) = channel(100);

    let mut connections = HashMap::new();

    let forward_task = async move {
        debug!("in receiver task process");
        while let Some(message) = receiver.next().await {
            debug!("received in fw message: {:?}", message);
            let message = match message {
                TcpWarpMessage::ConnectForward {
                    connection_id,
                    sender,
                    connected_sender,
                } => {
                    debug!("adding connection: {}", connection_id);
                    if let Err(err) = connected_sender.send(Ok(())) {
                        error!("connected sender errored: {:?}", err);
                    }
                    connections.insert(connection_id.clone(), sender.clone());
                    TcpWarpMessage::Connected { connection_id }
                }
                TcpWarpMessage::DisconnectClient { ref connection_id } => {
                    debug!(
                        "{} client connection disconnected, handle server disconnect",
                        connection_id
                    );
                    if let Some(mut sender) = connections.remove(connection_id) {
                        if let Err(err) = sender.send(message).await {
                            error!("cannot send to channel: {}", err);
                        }
                    } else {
                        error!("connection not found: {}", connection_id);
                    }
                    debug!("connections in pool: {}", connections.len());
                    continue;
                }
                TcpWarpMessage::BytesClient {
                    connection_id,
                    data,
                } => {
                    if let Some(sender) = connections.get_mut(&connection_id) {
                        debug!(
                            "forward message to host port of connection: {}",
                            connection_id
                        );
                        if let Err(err) = sender.send(TcpWarpMessage::BytesServer { data }).await {
                            error!("cannot send to channel: {}", err);
                        };
                    } else {
                        error!("connection not found: {}", connection_id);
                    }
                    continue;
                }
                regular_message => regular_message,
            };
            debug!("sending message {:?} from server to tunnel client", message);
            wtransport.send(message).await?
        }

        debug!("no more messages, closing forward to tunnel client task");
        wtransport.close().await?;
        receiver.close();

        Ok::<(), io::Error>(())
    };

    let processing_task = async move {
        while let Some(Ok(message)) = rtransport.next().await {
            debug!("server received from tunnel client {:?}", message);
            if let Err(err) =
                process_client_to_host_message(message, sender.clone(), connect_address).await
            {
                error!("error in processing: {}", err);
            }
        }

        debug!("processing task for client to host tunnel finished");

        Ok::<(), io::Error>(())
    };

    let (_, _) = try_join!(forward_task, processing_task)?;

    debug!("finished process of tunnel connection");

    Ok(())
}

async fn process_client_to_host_message(
    message: TcpWarpMessage,
    mut client_sender: Sender<TcpWarpMessage>,
    connect_address: IpAddr,
) -> Result<(), io::Error> {
    match message {
        TcpWarpMessage::HostConnect {
            connection_id,
            host,
            port,
        } => {
            let client_sender_ = client_sender.clone();
            spawn(async move {
                let connect_address = connect_address.to_string();
                let socket_address = format!(
                    "{}:{}",
                    host.unwrap_or_else(|| connect_address.to_string()),
                    port
                );
                debug!("host connection to {}", socket_address);
                if let Err(err) =
                    process_host_connection(client_sender_, connection_id, socket_address).await
                {
                    error!(
                        "failed connection {} {}: {}",
                        connect_address, connection_id, err
                    );
                }
            });
        }
        TcpWarpMessage::DisconnectClient { .. } => {
            if let Err(err) = client_sender.send(message).await {
                error!(
                    "cannot send message DisconnectClient to forward channel: {}",
                    err
                );
            }
        }
        TcpWarpMessage::BytesClient { .. } => {
            if let Err(err) = client_sender.send(message).await {
                error!(
                    "cannot send message BytesClient to forward channel: {}",
                    err
                );
            }
        }
        other_message => warn!("unsupported message: {:?}", other_message),
    }
    Ok(())
}

async fn process_host_connection<S: ToSocketAddrs>(
    mut client_sender: Sender<TcpWarpMessage>,
    connection_id: Uuid,
    socket_address: S,
) -> Result<(), Box<dyn Error>> {
    debug!("{} new connection", connection_id);
    let stream = TcpStream::connect(socket_address).await?;

    let (mut wtransport, mut rtransport) =
        Framed::new(stream, TcpWarpProtoHost { connection_id }).split();

    let (host_sender, mut host_receiver) = channel(100);

    let forward_task = async move {
        debug!("{} in receiver task process_host_connection", connection_id);

        while let Some(message) = host_receiver.next().await {
            debug!("{} just received a message: {:?}", connection_id, message);
            match message {
                TcpWarpMessage::DisconnectClient { .. } => break,
                TcpWarpMessage::BytesServer { data } => wtransport.send(data).await?,
                _ => (),
            }
        }

        debug!(
            "{} no more messages, closing process host forward task",
            connection_id
        );
        wtransport.close().await?;
        host_receiver.close();
        debug!("{} closed write transport", connection_id);

        Ok::<(), io::Error>(())
    };

    let (connected_sender, connected_receiver) = oneshot::channel();

    client_sender
        .send(TcpWarpMessage::ConnectForward {
            connection_id,
            sender: host_sender,
            connected_sender,
        })
        .await?;

    debug!("{} sended connect to client", connection_id);

    let mut client_sender_ = client_sender.clone();

    let processing_task = async move {
        if let Err(err) = connected_receiver.await {
            error!("{} connection error: {}", connection_id, err);
        }
        while let Some(Ok(message)) = rtransport.next().await {
            if let Err(err) = client_sender_.send(message).await {
                error!("{} {}", connection_id, err);
            }
        }

        let message = TcpWarpMessage::DisconnectHost { connection_id };

        debug!("{} sending disconnect host message", connection_id);

        if let Err(err) = client_sender_.send(message).await {
            error!("{} err: {}", connection_id, err);
        }

        debug!("{} host connection processing task done", connection_id);

        Ok::<(), io::Error>(())
    };

    try_join!(forward_task, processing_task)?;

    debug!("{} disconnect, processing task done", connection_id);

    Ok(())
}
