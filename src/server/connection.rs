use tokio::net::TcpStream;
use tokio::sync::Mutex as FMutex;

use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::codec::length_delimited::LengthDelimitedCodec;

use futures::stream::StreamExt;
use futures::sink::SinkExt;

use bytes::Bytes;

use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::server::ledger_wrapper::LedgerWrapper;
use crate::protocol::Message;
use crate::transactions::Transaction;
use crate::OpTrait;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

pub type PeerReadSocket = FramedRead<OwnedReadHalf, LengthDelimitedCodec>;
pub type PeerWriteSocket = FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>;

pub trait Callback<Operation: OpTrait>: Sync+Send {
    fn validate_transaction(&self, tx: &Transaction<Operation>) -> bool;

    fn notify_new_transaction(&self, tx: &Transaction<Operation>);
}

/// Callback that does not do anything
pub struct NullCallback{}

impl<Operation: OpTrait> Callback<Operation> for NullCallback {
    fn validate_transaction(&self, _: &Transaction<Operation>) -> bool { true }

    fn notify_new_transaction(&self, _: &Transaction<Operation>) {}
}

pub struct PeerConnection<Operation: OpTrait> {
    identifier: u32,
    ledger: Arc<LedgerWrapper<Operation>>,
    write_framed: FMutex<PeerWriteSocket>,
    callback: Arc<dyn Callback<Operation>>
}

impl<Operation: OpTrait+Serialize+DeserializeOwned> PeerConnection<Operation> {
    pub fn new(identifier: u32, ledger: Arc<LedgerWrapper<Operation>>, callback: Arc<dyn Callback<Operation>>, socket: TcpStream) -> (Self, PeerReadSocket) {
        let (read_socket, write_socket) = socket.into_split();

        let read_framed = FramedRead::new(read_socket, LengthDelimitedCodec::new());
        let write_framed = FMutex::new(FramedWrite::new(write_socket, LengthDelimitedCodec::new()));

        (Self{identifier, callback, ledger, write_framed}, read_framed)
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

        log::info!("Peer {} disconnected from blockchain-sim", self.identifier);
        self.ledger.unregister_peer(self.identifier).await;
    }

    pub async fn handle_message(&self, data: Bytes) {
        let msg = bincode::deserialize(&data).unwrap();

        match msg {
            Message::TransactionRequest{transaction} => {
                if self.callback.validate_transaction(&transaction) {
                    self.callback.notify_new_transaction(&transaction);
                    self.ledger.insert(transaction).await;
                } else {
                    log::debug!("Discarded transaction because validation failed: {transaction:?}");
                }
            }
            _ => {
                panic!("Server got unexpected message: {msg:?}");
            }
        }
    }

    pub async fn send(&self, msg: &Message<Operation>) {
        let data = bincode::serialize(msg).expect("Failed to serialize data");
        let mut framed = self.write_framed.lock().await;
        let result = framed.send(data.into()).await;

        if let Err(err) = result {
            log::error!("Failed to send data to peer: {err}");
        }
    }
}
