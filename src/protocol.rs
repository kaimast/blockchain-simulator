use crate::transactions::Transaction;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

#[ derive(Serialize, Deserialize, Debug, Clone) ]
pub enum Message<OpType: Serialize+Debug> {
    LedgerUpdate { transaction: Transaction<OpType> },
    TransactionRequest { transaction: Transaction<OpType> },
}

