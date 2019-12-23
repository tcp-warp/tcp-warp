use structopt::StructOpt;
use tcpwarp::TcpWarpPortConnection;

/// An utility to create userspace tunnel between two hosts
/// mapping ports on client machine to hosts and ports
/// accessible from server machine using single port.
#[derive(StructOpt)]
pub struct Cli {
    #[structopt(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
    #[structopt(flatten)]
    pub command: Command,
}

#[derive(StructOpt)]
pub enum Command {
    /// Client mode.
    ///
    /// Runs on machine, to which ports are mapped.
    Client {
        /// Address to bind
        ///
        /// Format: IP
        ///
        /// Example: --bind 127.0.0.1
        ///
        /// Default: 0.0.0.0
        #[structopt(long)]
        bind: Option<String>,
        #[structopt(long, short)]
        /// Server to connect
        ///
        /// Format: IP:PORT
        ///
        /// Example: --tunnel 192.168.0.1:18000
        ///
        /// Default: 127.0.0.1:18000
        tunnel: Option<String>,
        /// Connections
        ///
        /// Format: [client_port:][host:]host_port
        ///
        /// Example: --connection 8080 --connection 18081:8081 --connection 18082:127.0.0.1:8082
        #[structopt(long, short)]
        connection: Vec<TcpWarpPortConnection>,
        /// Retry connection on failure or disconnect
        #[structopt(long)]
        retry: bool,
        /// Retry interval in seconds
        ///
        /// Default: 5 secs
        #[structopt(long)]
        retry_interval: Option<u64>,
        /// Keep connections between reconnect attempts
        #[structopt(long)]
        keep_connections: bool,
    },
    /// Server mode.
    ///
    /// Runs on machine, from which mapped addresses are available.
    Server {
        /// Address of target host with mapped ports
        /// Format: IP
        /// Example: --connect 172.24.0.1
        /// Default: 127.0.0.1
        #[structopt(long)]
        connect: Option<String>,
        #[structopt(long)]
        /// Address to listen
        /// Format: IP:PORT
        /// Example: --server 192.168.0.1:18000
        /// Default: 127.0.0.1:18000
        listen: Option<String>,
        #[structopt(long, short)]
        port: Vec<u16>,
    },
}
