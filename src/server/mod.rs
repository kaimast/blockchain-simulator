mod connection;
use connection::PeerConnection;
pub use connection::{Callback, NullCallback};

mod ledger_wrapper;
use ledger_wrapper::LedgerWrapper;

use clap::{Arg, Command};

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

pub async fn main_thread<OpType: OpTrait + Serialize + DeserializeOwned>(
    callback: Arc<dyn Callback<OpType>>,
) {
    let arg_matches = Command::new("blockchain-sim")
        .author("Kai Mast <kaimast@cs.cornell.edu>")
        .version("0.1")
        .about("Simulates a blockchain network using a single process")
        .arg(
            Arg::new("listen")
                .long("listen-address")
                .short('l')
                .help("The address to bind to")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("throughput")
                .long("throughput")
                .default_value("1000.0")
                .help("The maximum throughput of the chain (in tx/s)"),
        )
        .arg(
            Arg::new("latency")
                .long("latency")
                .default_value("100")
                .help("The transaction confirmation delay (in ms)"),
        )
        .arg(
            Arg::new("epoch_length")
                .long("epoch_length")
                .default_value("60")
                .help("Length of an epoch (in s)"),
        )
        .get_matches();

    let throughput: f64 = *arg_matches
        .get_one("throughput")
        .expect("Failed to parse command line argument");

    if throughput == 0.0 {
        panic!("Throughput cannot be 0");
    }

    let latency: u32 = *arg_matches
        .get_one("latency")
        .expect("Failed to parse command line argument");

    info!(
        "Ledger throughput set to {}tx/s and latency set to {}ms",
        throughput, latency
    );

    let addr_str: &String = arg_matches.get_one("listen").unwrap();
    let addr = parse_address(addr_str, DEFAULT_BLOCKCHAIN_PORT);
    info!("Listening for connections on {:?}", addr);

    let ledger = Arc::new(LedgerWrapper::new(throughput, latency));
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind socket!");

    let l2 = ledger.clone();

    let elength_arg: u64 = *arg_matches.get_one("epoch_length").unwrap();
    let epoch_length = Duration::from_secs(elength_arg);

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
