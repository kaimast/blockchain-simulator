use tokio::net::TcpStream;
use tokio::io::split;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::sync::Mutex as FMutex;

use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::codec::length_delimited::LengthDelimitedCodec;

use futures::stream::StreamExt;
use futures::sink::SinkExt;

use bincode;
use serde::{Serialize};
use serde::de::{DeserializeOwned};

use bytes::Bytes;

use std::sync::Arc;
use log::{info, error};

use crate::server::ledger_wrapper::LedgerWrapper;
use crate::protocol::Message;

pub type PeerReadSocket = FramedRead<ReadHalf<TcpStream>, LengthDelimitedCodec>;
pub type PeerWriteSocket = FramedWrite<WriteHalf<TcpStream>, LengthDelimitedCodec>;

pub struct PeerConnection<Operation: Clone+Sync+Send+Serialize+DeserializeOwned+'static> {
    identifier: u32,
    ledger: Arc<LedgerWrapper<Operation>>,
    write_framed: FMutex<PeerWriteSocket>
}

impl<Operation: Clone+Sync+Send+Serialize+DeserializeOwned+'static> PeerConnection<Operation> {   pub fn new(identifier: u32, ledger: Arc<LedgerWrapper<Operation>>, socket: TcpStream) -> (Self, PeerReadSocket) {
        let (read_socket, write_socket) = split(socket);

        let read_framed = FramedRead::new(read_socket, LengthDelimitedCodec::new());
        let write_framed = FMutex::new(FramedWrite::new(write_socket, LengthDelimitedCodec::new()));

        return (Self{identifier, ledger, write_framed}, read_framed);
    }

    pub async fn run(&self, mut read_framed: PeerReadSocket) {
        while let Some(result) = read_framed.next().await {
            match result {
                Ok(data) => {
                   self.handle_message(data.freeze()).await;
                }
                Err(e) => {
                     println!("Error on decoding from socket; error = {:?}", e);
                     break;
                }
            }
        }

        info!("Peer {} disconnected from blockchain-sim", self.identifier);
        self.ledger.unregister_peer(&self.identifier);
    }

    pub async fn handle_message(&self, data: Bytes) {
        let msg = bincode::deserialize(&data).unwrap();

        match msg {
            Message::LedgerUpdate{transaction: _} => { panic!("Should not get ledger update!"); },
            Message::TransactionRequest{transaction} => {
                self.ledger.insert(transaction).await;
            }
        }
    }

    pub async fn send(&self, msg: &Message<Operation>) {
        let data = bincode::serialize(msg).expect("Failed to serialize data");
        let mut framed = self.write_framed.lock().await;
        let result = framed.send(data.into()).await;
        
        match result {
            Ok(()) => {},
            Err(e) => { error!("Failed to send data to peer: {}", e); }
        }
    }
}
