use crate::transactions::Transaction;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

#[ derive(Serialize, Deserialize, Debug, Clone) ]
pub enum Message<OpType: Serialize+Debug> {
    // A new epoch has started
    // (e.g. a new key block was mined)
    NewEpochStarted {},

    // A new transaction was added to the chain
    LedgerUpdate { transaction: Transaction<OpType> },

    // Send by clients 
    TransactionRequest { transaction: Transaction<OpType> },
}

