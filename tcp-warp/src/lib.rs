/*!
# A userspace tunnel between two hosts mapping ports on client machine to addresses reachable from server machine

```bash
tcp-warp server
tcp-warp client -c 8080:towel.blinkenlights.nl:23
nc 127.0.0.1 8080
```

## Features

1. A userspace tunnel to connect ports on client network with connections available on server side.
1. Uses only single port.
1. Client push of addresses to connect from server.

## Installation

With [cargo](https://www.rust-lang.org/learn/get-started):

    cargo install tcp-warp-cli

## Usage

To create a tunnel we need to start a server listening on some port and then connect to it with a client.

### Docker usage for server part

```bash
docker run --rm -d -p 18000:18000 tcpwarp/tcpwarp
```

or with custom listen port (ex: 18234):

```bash
docker run --rm -d -p 18234:18234 tcpwarp/tcpwarp tcp-warp server --listen=0.0.0.0:18234
```

### Simple local running port remapper

1. Start server:

    ```bash
    tcp-warp server
    ```

1. Start client:

    ```bash
    tcp-warp client -c 8080:towel.blinkenlights.nl:23
    ```

1. Enjoy the show:

    ```bash
    nc 127.0.0.1 8080
    ```

1. This example uses default listen and connect interfaces. In a real life scenario you need at least provide -t / --tunnel parameter to client:

    ```bash
    tcp-warp client -t host:port ...
    ```

Both client and server have address on which they listen for incoming connections and client additionally have parameter to specify connection address.

Next we look at more specific example.

### Use case: running Docker on machine without Docker daemon installed with Docker daemon behind SSH

Background:

- client: client machine runs on Windows, has Windows version of `tcp-warp` and Docker CLI installed. Client cannot run Docker daemon.
- public: master node accessible with SSH from which Docker daemon node can be accessed.
- docker: docker daemon node accessible with SSH.

Target:

Run Docker over tcp transport, allowing `client` to build and run containers. Environment should be available for each developer independent of other.

Solution:

Run on `docker` machine Docker-in-Docker container (`dind`) using tcp host protocol. Use `DOCKER_HOST` environment variable on `client` to connect to `dind`. `dind` is bindet to host port on `docker` host and forwarded via `public` with SSH port-forwarding.

The sequence of commands can be following:

#### Initial sequence (installation)

1. Go to `docker` node and start required containers:

    ```bash
    user@client $ ssh user1@public
    user1@public $ ssh user2@docker
    user2@docker $ docker run --rm --privileged -p 2375:2375 -p 18000:18000 -d --name some-docker docker:dind dockerd --host=tcp://0.0.0.0:2375
    user2@docker $ DOCKER_HOST=tcp://127.0.0.1:2375 docker run --rm -p 18000:18000 -d --name some-docker-tcp-warp tcpwarp/tcpwarp
    ```

1. Disconnect from `docker` and `public` nodes.

#### Normal sequence (usage)

1. Connect to `public` node with `ssh` and forward port for `tcp-warp`:

    ```bash
    ssh -L 18000:docker:18000 user1@public
    ```

1. Connect to Docker daemon with `tcp-warp client` on `client` machine:

    ```bash
    tcp-warp client -c 10001:172.18.0.1:2375
    ```

    `172.18.0.1` here is the address of host node in `dind`.

1. Export DOCKER_HOST environment variable on `client` machine:

    ```bash
    export DOCKER_HOST=tcp://127.0.0.1:10001
    ```

1. Run docker commands from `client`:

    ```bash
    docker ps
    docker run hello-world
    docker run -it alpine ash
    ```

#### Additional services

We can start additional services and relaunch `tcp-warp client` with additional `-c` for these services.

Simple example with `whoami` service:

1. Create network to use for hostname resolution. Start `whoami` service with all above steps done. Connect tcp-warp container to new network:

    ```bash
    docker network create our-network
    docker run --rm -d --net our-network --name whoami containous/whoami
    docker network connect our-network some-docker-tcp-warp
    ```

1. Stop `tcp-warp client`. Start it with additional port mapping for `whoami` service:

    ```bash
    tcp-warp client -c 10001:172.18.0.1:2375 -c 8080:whoami:80
    ```

1. Test `whoami` service:

    ```bash
    $ curl http://localhost:8080/
    Hostname: 9fe704cf0e87
    IP: 127.0.0.1
    IP: 172.18.0.3
    IP: 172.19.0.3
    RemoteAddr: 172.19.0.2:44612
    GET / HTTP/1.1
    Host: localhost:8080
    User-Agent: curl/7.64.1
    ```

*/
use bytes::{Buf, BufMut, BytesMut};
use futures::{
    future::{abortable, AbortHandle},
    prelude::*,
    try_join,
};
use log::*;
use std::{
    collections::HashMap,
    convert::TryInto,
    error::Error,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    spawn,
    sync::{
        mpsc::{channel, Sender},
        oneshot,
    },
    time::delay_for,
};
use tokio_util::codec::{Decoder, Encoder, Framed};
use uuid::Uuid;

mod client;
mod proto;
mod server;

pub use client::TcpWarpClient;
pub use proto::{TcpWarpMessage, TcpWarpProto, TcpWarpProtoClient, TcpWarpProtoHost};
pub use server::TcpWarpServer;

#[derive(Debug, Clone, PartialEq)]
pub struct TcpWarpPortConnection {
    client_port: Option<u16>,
    host: Option<String>,
    port: u16,
}

impl FromStr for TcpWarpPortConnection {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');

        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(client_port), host, Some(port), None) => {
                match (client_port.parse(), port.parse()) {
                    (Ok(client_port), Ok(port)) => Ok(TcpWarpPortConnection {
                        client_port: Some(client_port),
                        host: host.map(str::to_owned),
                        port,
                    }),
                    _ => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "cannot parse port mapping",
                    )),
                }
            }
            (Some(client_port_or_host), Some(port), None, None) => {
                let port = match port.parse() {
                    Ok(port) => port,
                    Err(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "cannot parse port mapping",
                        ))
                    }
                };
                let client_port: Result<u16, _> = client_port_or_host.parse();
                if let Ok(client_port) = client_port {
                    Ok(TcpWarpPortConnection {
                        client_port: Some(client_port),
                        host: None,
                        port,
                    })
                } else {
                    Ok(TcpWarpPortConnection {
                        client_port: None,
                        host: Some(client_port_or_host.to_owned()),
                        port,
                    })
                }
            }
            (Some(port), None, None, None) => {
                let port = match port.parse() {
                    Ok(port) => port,
                    Err(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "cannot parse port mapping",
                        ))
                    }
                };
                Ok(TcpWarpPortConnection {
                    client_port: None,
                    host: None,
                    port,
                })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "cannot parse port mapping",
            )),
        }
    }
}

pub struct TcpWarpConnection {
    sender: Sender<TcpWarpMessage>,
    connected_sender: Option<oneshot::Sender<Result<(), io::Error>>>,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn connection_from_str() {
        assert_eq!(
            Ok(TcpWarpPortConnection {
                client_port: None,
                host: None,
                port: 8080
            }),
            "8080".parse().map_err(|_| ())
        );
        assert_eq!(
            Ok(TcpWarpPortConnection {
                client_port: Some(8081),
                host: None,
                port: 8080
            }),
            "8081:8080".parse().map_err(|_| ())
        );
        assert_eq!(
            Ok(TcpWarpPortConnection {
                client_port: Some(8081),
                host: Some("localhost".into()),
                port: 8080
            }),
            "8081:localhost:8080".parse().map_err(|_| ())
        );
    }
}
