use tokio::net::TcpStream;
use tokio::sync::Mutex;

use tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tokio_util::codec::{FramedRead, FramedWrite};

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::thread::sleep;

use futures_util::{SinkExt, StreamExt};

use blockchain_simulator::{
    generate_key_pair, to_account_id, Ledger, TestOperation, Transaction, DEFAULT_BLOCKCHAIN_PORT,
};

use blockchain_simulator::protocol::Message;

const NUM_TRANSACTIONS: usize = 1000;

fn main() {
    let mode = {
        let mut args = std::env::args();

        if args.len() != 2 {
            panic!("Got invalid number of arguments");
        }

        // Discard first argument
        args.next().unwrap();

        args.next().unwrap()
    };

    let mut rt = tokio::runtime::Builder::new()
        .enable_io()
        .enable_time()
        .threaded_scheduler()
        .build()
        .expect("Failed to start worker threads");

    rt.block_on(async move {
        let addr = format!("localhost:{}", DEFAULT_BLOCKCHAIN_PORT)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        let tstream = TcpStream::connect(addr).await.unwrap();

        let (read_stream, write_stream) = tstream.into_split();
        let (private_key, public_key) = generate_key_pair();

        let account_id = to_account_id(&public_key);

        let ledger = Arc::new(Ledger::<TestOperation>::default());
        let l2 = ledger.clone();

        let mut read_framed = FramedRead::new(read_stream, LengthDelimitedCodec::new());
        let write_framed = Arc::new(Mutex::new(FramedWrite::new(
            write_stream,
            LengthDelimitedCodec::new(),
        )));

        // Receive loop
        tokio::spawn(async move {
            while let Some(res) = read_framed.next().await {
                match res {
                    Ok(data) => {
                        let msg = bincode::deserialize(&data).unwrap();

                        match msg {
                            Message::NewEpochStarted {
                                identifier,
                                timestamp,
                            } => {
                                ledger.create_new_epoch(identifier, timestamp);
                            }
                            Message::SyncEpoch { identifier, epoch } => {
                                ledger.synchronize_epoch(identifier, epoch);
                            }
                            Message::LedgerUpdate { transaction } => {
                                ledger.insert(transaction.clone());
                            }
                            _ => {
                                panic!("Got unexpected message from blockchain: {:?}", msg);
                            }
                        }
                    }
                    Err(e) => panic!("Failed to receive data from blockchain: {}", e),
                }
            }
        });

        if mode == "send_transactions" {
            for _ in 0..NUM_TRANSACTIONS {
                let transaction =
                    Transaction::new(account_id, TestOperation::Empty {}, &private_key);

                let request = Message::TransactionRequest { transaction };

                let data = bincode::serialize(&request).expect("Serialize message");
                let mut sock = write_framed.lock().await;

                match sock.send(data.into()).await {
                    Ok(()) => {}
                    Err(e) => {
                        println!("Failed to write data to socket: {}", e)
                    }
                }
            }
        } else if mode == "count_transactions" {
            //FIXME need to figure out a better way to synchronize
            sleep(std::time::Duration::from_secs(10));

            let num_txs = l2.num_transactions();

            if num_txs == NUM_TRANSACTIONS {
                println!("Transaction count is correct.");
            } else {
                panic!(
                    "Invalid transactions count: got {} but expected {}",
                    num_txs, NUM_TRANSACTIONS
                );
            }
        } else {
            panic!("Got unexpected mode: {}", mode);
        }
    });
}
