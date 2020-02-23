pub mod protocol;

mod transactions;
pub use transactions::Transaction;

use std::sync::{Mutex};
use std::collections::{LinkedList, HashMap};

#[ cfg(feature="server") ]
mod server;

#[ cfg(feature="server") ]
pub use server::main_thread;

use serde::{Serialize};

mod crypto_helper;
pub use crypto_helper::{PublicKey, AccountId, PrivateKey, generate_key_pair, to_account_id};

pub struct Identity {
    #[allow(dead_code)]
    public_key: PublicKey
}

pub struct Ledger<OpType: Serialize> {
    #[allow(dead_code)]
    identities: Mutex<HashMap<AccountId, Identity>>,
    #[allow(dead_code)]
    transactions: Mutex<LinkedList<Transaction<OpType>>>,
}

impl<Operation: Serialize> Default for Ledger<Operation> {
    fn default() -> Self {
        let transactions = Mutex::new( LinkedList::new() );
        let identities = Mutex::new( HashMap::new() );

        Self{ identities, transactions }
    }
}

impl<Operation: Serialize> Ledger<Operation> {
    pub fn insert(&self, tx: Transaction<Operation>) {
     /*   if tx.op_type == OpType::CreateAccount {
            self.identity_mgr.create_account(&tx);
        }*/

        self.transactions.lock().unwrap().push_back(tx);
    }

    pub fn size(&self) -> usize {
        return self.transactions.lock().unwrap().len();
    }
}

#[cfg(test)]
mod tests {
    use crate::{generate_key_pair, to_account_id, Transaction, Ledger};

    use crate::protocol::Message;

    use serde::Serialize;
    use bytes::Bytes;

    #[ derive(Serialize) ]
    enum TestOperation {
        Empty{}
    }

    #[test]
    fn size() {
        let ledger = Ledger::new();

        assert_eq!(ledger.size(), 0);

        let (skey, pkey) = generate_key_pair();
        let account = to_account_id(&pkey);

        let tx = Transaction::new(account, TestOperation::Empty{}, &skey);
        ledger.insert(tx);

        assert_eq!(ledger.size(), 1);
    }
}
