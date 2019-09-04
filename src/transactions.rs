// contains TransactionType definition
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use libconsensus::BaseConsensusPeer;
use libconsensus::TransactionType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InternalTransaction<P> {
    transaction_type: TransactionType,
    peer: BaseConsensusPeer<P>,
}

impl<P> Stub for InternalTransaction<P> where P: PeerId {}
