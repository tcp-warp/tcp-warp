/*!
# tcp-warp-cli description

## Features

## Usage

```bash
cargo install tcp-warp-cli
```

*/
use env_logger::Builder as LoggerBuilder;
use std::{error::Error, sync::Arc, time::Duration};
use structopt::StructOpt;
use tcpwarp::{TcpWarpClient, TcpWarpServer};

mod cli;

use cli::{Cli, Command::*};

const DEFAULT_CLIENT_BIND: &str = "0.0.0.0";
const DEFAULT_CLIENT_SERVER: &str = "127.0.0.1:18000";
const DEFAULT_SERVER_LISTEN: &str = DEFAULT_CLIENT_SERVER;
const DEFAULT_SERVER_CONNECT: &str = "127.0.0.1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::from_args();

    cli.verbose
        .log_level()
        .map(|x| {
            LoggerBuilder::new()
                .filter(None, x.to_level_filter())
                .format_timestamp_nanos()
                .try_init()
        })
        .transpose()?;

    match cli.command {
        Client {
            bind,
            tunnel,
            connection,
            retry,
            retry_interval,
            keep_connections,
        } => {
            let client = TcpWarpClient::new(
                bind.unwrap_or_else(|| DEFAULT_CLIENT_BIND.into()).parse()?,
                tunnel
                    .unwrap_or_else(|| DEFAULT_CLIENT_SERVER.into())
                    .parse()?,
            );
            if retry {
                client
                    .connect_loop(
                        Duration::from_secs(retry_interval.unwrap_or(5)),
                        keep_connections,
                        Arc::new(connection),
                    )
                    .await?;
            } else {
                client.connect(connection).await?;
            }
        }
        Server { listen, connect } => {
            TcpWarpServer::new(
                listen
                    .unwrap_or_else(|| DEFAULT_SERVER_LISTEN.into())
                    .parse()?,
                connect
                    .unwrap_or_else(|| DEFAULT_SERVER_CONNECT.into())
                    .parse()?,
            )
            .listen()
            .await?;
        }
    }

    Ok(())
}
