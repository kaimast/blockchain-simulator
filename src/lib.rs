#![feature(trait_alias)]

pub mod protocol;
use protocol::EpochId;

mod transactions;
pub use transactions::*;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::sync::{Mutex, RwLock};

#[cfg(feature = "server")]
pub mod server;

use serde::{Deserialize, Serialize};

pub const DEFAULT_BLOCKCHAIN_PORT: u16 = 8080;

mod crypto_helper;
pub use crypto_helper::{generate_key_pair, to_account_id, AccountId, PrivateKey, PublicKey};

pub trait OpTrait = Clone + Debug + Sync + Send + 'static;

pub struct Identity {
    #[allow(dead_code)]
    public_key: PublicKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Epoch<OpType: OpTrait> {
    timestamp: i64,
    transactions: Vec<Transaction<OpType>>,
}

impl<OpType: OpTrait> Epoch<OpType> {
    pub fn new(timestamp: i64) -> Self {
        Self {
            timestamp,
            transactions: Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.transactions.len()
    }

    pub fn get_transactions(&self) -> &Vec<Transaction<OpType>> {
        &self.transactions
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }
}

type EpochMap<OpType> = BTreeMap<EpochId, Mutex<Epoch<OpType>>>;

pub struct Ledger<OpType: OpTrait> {
    #[allow(dead_code)]
    identities: Mutex<HashMap<AccountId, Identity>>,
    epochs: RwLock<EpochMap<OpType>>,
}

impl<OpType: OpTrait> Default for Ledger<OpType> {
    fn default() -> Self {
        let identities = Mutex::new(HashMap::default());
        let epochs = RwLock::new(EpochMap::default());

        Self { identities, epochs }
    }
}

impl<OpType: OpTrait> Ledger<OpType> {
    pub fn insert(&self, tx: Transaction<OpType>) {
        /*   if tx.op_type == OpType::CreateAccount {
            self.identity_mgr.create_account(&tx);
        }*/

        // Hold the lock to this throughout the entire modification to avoid race conditions
        let epochs = self.epochs.read().unwrap();

        let epoch = match epochs.last_key_value() {
            Some((_, v)) => v,
            None => {
                panic!("Cannot insert transaction before starting the first epoch!");
            }
        };

        let mut lock = epoch.lock().unwrap();
        lock.transactions.push(tx);
    }

    pub fn get_epoch_timestamp(&self, identifier: EpochId) -> i64 {
        let epochs = self.epochs.read().unwrap();
        let epoch = epochs.get(&identifier).unwrap();

        let lock = epoch.lock().unwrap();
        lock.get_timestamp()
    }

    // Returns a copy of an epoch
    pub fn get_epoch(&self, identifier: EpochId) -> Epoch<OpType> {
        let epochs = self.epochs.read().unwrap();
        let epoch = epochs.get(&identifier).expect("No such epoch");

        let lock = epoch.lock().unwrap();
        lock.clone()
    }

    pub fn num_transactions(&self) -> usize {
        let mut result = 0;
        let epochs = self.epochs.read().unwrap();

        for epoch in epochs.values() {
            let epoch = epoch.lock().unwrap();
            result += epoch.size();
        }

        result
    }

    pub fn num_epochs(&self) -> usize {
        let epochs = self.epochs.read().unwrap();
        epochs.len()
    }

    pub fn create_new_epoch(&self, identifier: EpochId, timestamp: i64) {
        let mut epochs = self.epochs.write().unwrap();
        let result = epochs.insert(identifier, Mutex::new(Epoch::new(timestamp)));

        if result.is_some() {
            panic!("Epoch {} was created more than once", identifier);
        }
    }

    pub fn has_gaps(&self) -> bool {
        let epochs = self.epochs.read().unwrap();

        for (pos, key) in epochs.keys().enumerate() {
            let expected = pos as EpochId;

            if expected != *key {
                return true;
            }
        }

        false
    }

    pub fn synchronize_epoch(&self, identifier: EpochId, epoch: Epoch<OpType>) {
        let mut epochs = self.epochs.write().unwrap();
        let result = epochs.insert(identifier, Mutex::new(epoch));

        if result.is_some() {
            panic!("Epoch {} was created more than once", identifier);
        }
    }

    pub fn get_current_epoch(&self) -> EpochId {
        let epochs = self.epochs.read().unwrap();
        let (k, _) = epochs.last_key_value().unwrap();

        *k
    }
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub enum TestOperation {
    Empty {},
}

#[cfg(test)]
mod tests {
    use crate::{generate_key_pair, to_account_id, Ledger, TestOperation, Transaction};

    #[test]
    fn size() {
        let ledger = Ledger::default();

        assert_eq!(ledger.num_epochs(), 0);

        let (skey, pkey) = generate_key_pair();
        let account = to_account_id(&pkey);

        let tx = Transaction::new(account, TestOperation::Empty {}, &skey);
        ledger.create_new_epoch(0, 5);
        ledger.insert(tx);

        let epoch = ledger.get_epoch(0);
        assert_eq!(epoch.size(), 1);
    }

    #[test]
    fn has_gaps() {
        let ledger = Ledger::<TestOperation>::default();
        ledger.create_new_epoch(1, 5);

        assert_eq!(true, ledger.has_gaps());

        ledger.create_new_epoch(0, 1);

        assert_eq!(false, ledger.has_gaps());
        assert_eq!(1, ledger.get_current_epoch());
    }

    #[test]
    fn sync_epochs() {
        let ledger = Ledger::default();
        let copy = Ledger::default();

        let (skey, pkey) = generate_key_pair();
        let account = to_account_id(&pkey);

        let tx = Transaction::new(account, TestOperation::Empty {}, &skey);
        ledger.create_new_epoch(0, 5);
        ledger.insert(tx);

        let epoch = ledger.get_epoch(0);
        copy.synchronize_epoch(0, epoch);

        let ecopy = copy.get_epoch(0);

        assert_eq!(copy.num_epochs(), 1);
        assert_eq!(ecopy.size(), 1);
        assert_eq!(copy.num_transactions(), 1);
    }
}
