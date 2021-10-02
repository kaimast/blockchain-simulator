use crate::transactions::Transaction;
use crate::{Epoch, OpTrait};

use serde::{Serialize, Deserialize};
use std::fmt::Debug;

pub type EpochId = u32;

#[ derive(Serialize, Deserialize, Debug, Clone) ]
pub enum Message<OpType: OpTrait> {
    // Send an entire epoch. Only done during initial connection setup
    SyncEpoch { identifier: EpochId, epoch: Epoch<OpType> },

    // A new epoch has started
    // (e.g. a new key block was mined)
    NewEpochStarted { identifier: EpochId, timestamp: i64 },

    // A new transaction was added to the chain
    LedgerUpdate { transaction: Transaction<OpType> },

    // Send by clients 
    TransactionRequest { transaction: Transaction<OpType> },
}

