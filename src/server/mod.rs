mod connection;
use connection::PeerConnection;

mod ledger_wrapper;
use ledger_wrapper::LedgerWrapper;

use clap::{Arg, App};

use tokio::net::TcpListener;

use tokio::{spawn};

use std::net::{ToSocketAddrs};
use std::sync::Arc;
use log::{info,error};

use serde::{Serialize};
use serde::de::{DeserializeOwned};

pub async fn main_thread<Operation:
    Send+Sync+Clone+Serialize+DeserializeOwned+'static> () {
    let arg_matches = App::new("blockchain-sim")
        .author("Kai Mast <kaimast@cs.cornell.edu>")
        .version("0.1")
        .about("Simulates a blockchain network using a single process")
        .arg(Arg::with_name("listen")
                .takes_value(true)
                .long("listen")
                .short("l")
                .help("The address to bind to")
                .default_value("0.0.0.0")
            )
        .arg(Arg::with_name("throughput")
                .takes_value(true)
                .long("throughput")
                .default_value("1000")
                .help("The maximum throughput of the chain (in tx/s)")
            )
        .arg(Arg::with_name("latency")
                .takes_value(true)
                .long("latency")
                .default_value("100")
                .help("The transaction confirmation delay (in ms)")
            )
        .get_matches();

    let throughput: u32 = arg_matches.value_of("throughput").unwrap().parse::<u32>().expect("Failed to parse command line argument");

    if throughput == 0 {
        panic!("Throughput cannot be 0");
    }

    let latency: u32 = arg_matches.value_of("latency").unwrap().parse::<u32>().expect("Failed to parse command line argument");

    let host = arg_matches.value_of("listen").unwrap();
    let addr_str = format!("{}:8080", host);
    let addr = addr_str.to_socket_addrs().unwrap().next().unwrap();

    info!("Listening for connections on {}", addr_str);

    let ledger = Arc::new( LedgerWrapper::<Operation>::new(throughput, latency) );
    let mut listener = TcpListener::bind(&addr).await.expect("Failed to bind socket!");

    let mut next_id: u32 = 1;

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("Got new connection from {}", addr);
                let id = next_id;
                next_id += 1;
 
                let (c, read_socket) = PeerConnection::new(id, ledger.clone(), socket);

                let conn = Arc::new(c);
                ledger.register_peer(id, conn.clone());

                spawn(async move {
                    conn.run(read_socket).await;
                });
            },
            Err(e) => {
                error!("Falied to accept new connection: {}", e);
            }
        }
    }
}


