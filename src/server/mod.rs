mod connection;
use connection::PeerConnection;
pub use connection::{Callback, NullCallback};

mod ledger_wrapper;
use ledger_wrapper::LedgerWrapper;

use clap::Parser;

use tokio::net::TcpListener;
use tokio::spawn;

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use log::{error, info};

use crate::{OpTrait, DEFAULT_BLOCKCHAIN_PORT};

fn parse_address(addr_str: &str, default_port: u16) -> SocketAddr {
    if addr_str.contains(':') {
        addr_str.to_socket_addrs().unwrap().next().unwrap()
    } else {
        let addr_str = format!("{addr_str}:{default_port}");
        addr_str.to_socket_addrs().unwrap().next().unwrap()
    }
}

#[derive(Parser)]
#[clap(about = "Simulates a blockchain network using a single process")]
struct Args {
    #[clap(
        long,
        short = 'l',
        help = "The address to listen for client connections",
        default_value = "0.0.0.0"
    )]
    listen_address: String,
    #[clap(
        long,
        help = "The maximum throughput of the chain (in tx/s)",
        default_value_t = 1000.0
    )]
    throughput: f64,
    #[clap(
        long,
        help = "The transaction confirmation delay (in ms)",
        default_value_t = 100
    )]
    latency: u32,
    #[clap(long, help = "Length of an epoch (in s)", default_value_t = 60)]
    epoch_length: u64,
}

pub async fn main_thread<OpType: OpTrait + Serialize + DeserializeOwned>(
    callback: Arc<dyn Callback<OpType>>,
) {
    let args = Args::parse();
    if args.throughput <= 0.0 {
        panic!("Throughput cannot be <=0");
    }

    info!(
        "Ledger throughput set to {}tx/s and latency set to {}ms",
        args.throughput, args.latency
    );

    let addr = parse_address(&args.listen_address, DEFAULT_BLOCKCHAIN_PORT);
    info!("Listening for connections on {addr:?}");

    let ledger = Arc::new(LedgerWrapper::new(args.throughput, args.latency));
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind socket!");

    let l2 = ledger.clone();

    let epoch_length = Duration::from_secs(args.epoch_length);

    tokio::spawn(async move {
        loop {
            l2.start_new_epoch().await;
            tokio::time::sleep(epoch_length).await;
        }
    });

    let mut next_id: u32 = 1;

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("Got new connection from {addr}");
                let id = next_id;
                next_id += 1;

                let (c, read_socket) =
                    PeerConnection::new(id, ledger.clone(), callback.clone(), socket);

                let conn = Arc::new(c);
                ledger.register_peer(id, conn.clone()).await;

                spawn(async move {
                    conn.run(read_socket).await;
                });
            }
            Err(err) => {
                error!("Failed to accept new connection: {err}");
            }
        }
    }
}
