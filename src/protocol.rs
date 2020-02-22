use crate::transactions::Transaction;
use serde::{Serialize, Deserialize};

#[ derive(Serialize, Deserialize) ]
pub enum Message<OpType: Serialize> {
    LedgerUpdate { transaction: Transaction<OpType> },
    TransactionRequest { transaction: Transaction<OpType> },
}

