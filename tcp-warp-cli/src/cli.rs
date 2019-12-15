use structopt::StructOpt;
use tcp_warp::TcpWarpPortMap;

/// Allows to create userspace tunnel between two hosts
/// mapping ports on client machine to hosts and ports
/// accessible from server machine using only single port.
#[derive(StructOpt)]
pub struct Cli {
    #[structopt(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
    #[structopt(flatten)]
    pub command: Command,
}

#[derive(StructOpt)]
pub enum Command {
    /// Client mode. Runs on machine, to which ports are mapped.
    Client {
        /// Address to bind
        /// Format: IP
        /// Example: --bind 127.0.0.1
        /// Default: 0.0.0.0
        #[structopt(long)]
        bind: Option<String>,
        #[structopt(long)]
        /// Server to connect
        /// Format: IP:PORT
        /// Example: --server 192.168.0.1:18000
        /// Default: 127.0.0.1:18000
        server: Option<String>,
        /// Port map
        /// Format: host_port:client_port
        /// Example: --map 8081:18081 --map 8082:18082
        #[structopt(long)]
        map: Vec<TcpWarpPortMap>,
    },
    /// Server mode. Runs on machine, from which mapped addresses are available.
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
    },
}
