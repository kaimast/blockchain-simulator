use std::sync::Arc;
use std::time::{Duration,Instant};
use std::collections::{HashMap};
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::spawn;
use tokio::time::delay_for;
use tokio::sync::Mutex;

use crate::protocol::{EpochId, Message};
use crate::server::connection::PeerConnection;
use crate::{Ledger, OpTrait, Epoch};
use crate::transactions::Transaction;

use serde::Serialize;
use serde::de::DeserializeOwned;

use log::*;

/// This adds some server-side functionality to the ledger class
pub struct LedgerWrapper<OpType: OpTrait> {
    ledger: Arc<Ledger<OpType>>,
    peers: Mutex<HashMap<u32, Arc<PeerConnection<OpType>>>>,
    min_interval: Duration,
    latency : Duration,
    last_tx : Mutex<Instant>,
    next_epoch_id: AtomicU32
}

impl<OpType: OpTrait+Serialize+DeserializeOwned> LedgerWrapper<OpType> {
    pub fn new(throughput: f64, latency_ms: u32) -> Self {
        let ledger = Arc::new( Ledger::default() );
        let peers = Mutex::new( HashMap::new() );

        let min_interval = Duration::from_secs_f64((1.0/throughput).into());
        let latency = Duration::from_millis(latency_ms.into());

        let last_tx = Mutex::new( Instant::now() );

        let next_epoch_id = AtomicU32::new(0);

        Self{ ledger, peers, min_interval, latency, last_tx, next_epoch_id }
    }

    pub async fn register_peer(&self, identifier: u32, peer: Arc<PeerConnection<OpType>>) {
        // Hold lock throughout function to avoid sending messages twicey
        let mut peers = self.peers.lock().await;

        // First send all previous transactions / epochs
        let num_epochs = self.ledger.num_epochs();
        for i in 0..num_epochs {
            let eid = i as EpochId;
            let epoch = self.ledger.get_epoch(eid);
            let msg = Message::SyncEpoch{ identifier: eid, epoch };

            peer.send(&msg).await;
        }

        peers.insert(identifier, peer);
    }

    pub async fn unregister_peer(&self, identifier: u32) {
        self.peers.lock().await.remove(&identifier);
    }

    #[ allow(dead_code) ]
    pub fn num_epochs(&self) -> usize {
        self.ledger.num_epochs()
    }

    #[ allow(dead_code) ]
    pub fn get_epoch(&self, identifier: EpochId) -> Epoch<OpType> {
        self.ledger.get_epoch(identifier)
    }

    pub async fn start_new_epoch(&self) {
        let identifier = self.next_epoch_id.fetch_add(1, Ordering::SeqCst);

        let now = chrono::offset::Utc::now();
        let timestamp = now.timestamp();

        info!("Starting new blockchain epoch (id={} timestamp={})", identifier, timestamp);

        // Lock peers before ledger
        let peers = self.peers.lock().await;
        self.ledger.create_new_epoch(identifier, timestamp);
        let peers = peers.clone();

        spawn(async move {
            let msg = Message::NewEpochStarted{ identifier, timestamp };
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

    pub async fn insert(&self, transaction: Transaction<OpType>) {
        {
            let mut last_tx = self.last_tx.lock().await;
            let now = Instant::now();
            let diff = now.duration_since(*last_tx);

            if diff < self.min_interval {
                delay_for(self.min_interval - diff).await;
            }

            *last_tx = now;
        }

        let latency = self.latency;

        // Lock peers before ledger
        let peers = self.peers.lock().await;
        self.ledger.insert(transaction.clone());
        let peers = peers.clone();

        spawn(async move {
            delay_for(latency).await;

            trace!("Adding new transaction to the ledger");

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
