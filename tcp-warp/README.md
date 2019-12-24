# A userspace tunnel between two hosts mapping ports on client machine to addresses reachable from server machine

[![Linux build status](https://travis-ci.org/tcp-warp/tcp-warp.svg)](https://travis-ci.org/tcp-warp/tcp-warp)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/tcp-warp/tcp-warp?svg=true)](https://ci.appveyor.com/project/tcp-warp/tcp-warp)
[![Crates.io](https://img.shields.io/crates/v/tcp-warp.svg)](https://crates.io/crates/tcp-warp)
[![Packaging status](https://repology.org/badge/tiny-repos/tcp-warp.svg)](https://repology.org/project/tcp-warp/badges)

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

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
    Accept: */*
    ```
