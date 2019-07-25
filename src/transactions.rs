// contains TransactionType definition
use crate::peer::BaseConsensusPeer;
use libconsensus::TransactionType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InternalTransaction {
    transaction_type: TransactionType,
    peer: BaseConsensusPeer,
}
