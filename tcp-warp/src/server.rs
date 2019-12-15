use super::*;

pub struct TcpWarpServer {
    listen_address: SocketAddr,
}

impl TcpWarpServer {
    pub fn new(listen_address: SocketAddr) -> Self {
        Self { listen_address }
    }

    pub async fn listen(&self) -> Result<(), Box<dyn Error>> {
        let mut listener = TcpListener::bind(&self.listen_address).await?;
        let mut incoming = listener.incoming();

        while let Some(Ok(stream)) = incoming.next().await {
            spawn(async move {
                if let Err(e) = process(stream).await {
                    println!("failed to process connection; error = {}", e);
                }
            });
        }
        Ok(())
    }
}

async fn process(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut transport = Framed::new(stream, TcpWarpProto);
    transport
        .send(TcpWarpMessage::AddPorts(vec![8081, 8082]))
        .await?;

    while let Some(request) = transport.next().await {
        match request? {
            _ => (),
        }
    }

    Ok(())
}
