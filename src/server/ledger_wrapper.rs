use log::trace;
use std::sync::{Arc,Mutex};
use std::time::{Duration,Instant};
use std::collections::{HashMap};
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::spawn;
use tokio::time::delay_for;
use tokio::sync::Mutex as FMutex;

use crate::protocol::{Message};
use crate::server::connection::{PeerConnection, OpTrait};
use crate::Ledger;
use crate::transactions::Transaction;

/// This adds some server-side functionality to the ledger class
pub struct LedgerWrapper<Operation: OpTrait> {
    ledger: Arc<Ledger<Operation>>,
    peers: Mutex<HashMap<u32, Arc<PeerConnection<Operation>>>>,
    min_interval: Duration,
    latency : Duration,
    last_tx : FMutex<Instant>,
    next_epoch_id: AtomicU32
}

impl<Operation: OpTrait> LedgerWrapper<Operation> {
    pub fn new(throughput: u32, latency_ms: u32) -> Self {
        let ledger = Arc::new( Ledger::default() );
        let peers = Mutex::new( HashMap::new() );

        let min_interval = Duration::from_millis((1000/throughput).into());
        let latency = Duration::from_millis(latency_ms.into());

        let last_tx = FMutex::new( Instant::now() );

        let next_epoch_id = AtomicU32::new(1);

        return Self{ ledger, peers, min_interval, latency, last_tx, next_epoch_id };
    }

    pub fn register_peer(&self, identifier: u32, peer: Arc<PeerConnection<Operation>>) {
        self.peers.lock().unwrap().insert(identifier, peer);
    }

    pub fn unregister_peer(&self, identifier: u32) {
        self.peers.lock().unwrap().remove(&identifier);
    }

    pub fn start_new_epoch(&self) {
        // we're not actually modifying the ledger, just sending a message to peers
        let peers = self.peers.lock().unwrap().clone();

        let identifier = self.next_epoch_id.fetch_add(1, Ordering::SeqCst);

        spawn(async move {
            trace!("Starting new epoch");

            let msg = Message::NewEpochStarted{ identifier };
            let mut futures = Vec::new();

            // broadcast
            for peer in peers.values() {
                futures.push(peer.send(&msg));
            }

            for future in futures.drain(..) {
                future.await;
            }
        });
    }

    pub async fn insert(&self, transaction: Transaction<Operation>) {
        {
            let mut last_tx = self.last_tx.lock().await;
            let now = Instant::now();
            let diff = now.duration_since(*last_tx);

            if diff < self.min_interval {
                delay_for(self.min_interval - diff).await;
            }

            *last_tx = now;
        }

        let ledger = self.ledger.clone();
        let latency = self.latency;
        let peers = self.peers.lock().unwrap().clone();

        spawn(async move {
            delay_for(latency).await;

            trace!("Adding new transaction to the ledger");

            ledger.insert(transaction.clone());

            let msg = Message::LedgerUpdate{ transaction };
            let mut futures = Vec::new();

            // broadcast
            for peer in peers.values() {
                futures.push(peer.send(&msg));
            }

            for future in futures.drain(..) {
                future.await;
            }
        });
    }
}
