#![ feature(trait_alias) ]

pub mod protocol;
use protocol::EpochId;

mod transactions;
pub use transactions::*;

use std::fmt::Debug;
use std::sync::{Mutex};
use std::collections::{LinkedList, HashMap, BTreeMap};

#[ cfg(feature="server") ]
pub mod server;

use serde::{Serialize};

pub const DEFAULT_BLOCKCHAIN_PORT: u16 = 8080;

mod crypto_helper;
pub use crypto_helper::{PublicKey, AccountId, PrivateKey, generate_key_pair, to_account_id};

pub struct Identity {
    #[allow(dead_code)]
    public_key: PublicKey
}

struct EpochInfo {
    #[ allow(dead_code) ]
    timestamp: i64
}

pub struct Ledger<OpType: Serialize+Debug> {
    #[allow(dead_code)]
    identities: Mutex<HashMap<AccountId, Identity>>,
    #[allow(dead_code)]
    transactions: Mutex<LinkedList<Transaction<OpType>>>,

    epochs: Mutex<BTreeMap<EpochId, EpochInfo>>
}

impl<Operation: Serialize+Debug> Default for Ledger<Operation> {
    fn default() -> Self {
        let transactions = Mutex::new( LinkedList::default() );
        let identities = Mutex::new( HashMap::default() );
        let epochs = Mutex::new( BTreeMap::default() );

        Self{ identities, transactions, epochs }
    }
}

impl<Operation: Serialize+Debug> Ledger<Operation> {
    pub fn insert(&self, tx: Transaction<Operation>) {
     /*   if tx.op_type == OpType::CreateAccount {
            self.identity_mgr.create_account(&tx);
        }*/

        self.transactions.lock().unwrap().push_back(tx);
    }

    pub fn size(&self) -> usize {
        return self.transactions.lock().unwrap().len();
    }

    pub fn notify_new_epoch(&self, identifier: EpochId, timestamp: i64) {
        let mut epochs = self.epochs.lock().unwrap();
        epochs.insert(identifier, EpochInfo{ timestamp });
    }

    pub fn has_gaps(&self) -> bool {
        let epochs = self.epochs.lock().unwrap();

        for (pos, key) in epochs.keys().enumerate() {
            let expected = (pos+1) as EpochId;

            if expected != *key {
                return true
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{generate_key_pair, to_account_id, Transaction, Ledger};

    use serde::Serialize;

    #[ derive(Serialize, Debug) ]
    enum TestOperation {
        Empty{}
    }

    #[test]
    fn size() {
        let ledger = Ledger::default();

        assert_eq!(ledger.size(), 0);

        let (skey, pkey) = generate_key_pair();
        let account = to_account_id(&pkey);

        let tx = Transaction::new(account, TestOperation::Empty{}, &skey);
        ledger.insert(tx);

        assert_eq!(ledger.size(), 1);
    }
}
