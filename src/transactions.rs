// contains TransactionType definition
use libcommon_rs::peer::PeerId;
use libconsensus::BaseConsensusPeer;
use libconsensus::TransactionType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InternalTransaction<P>
where
    P: PeerId,
{
    transaction_type: TransactionType,
    peer: BaseConsensusPeer<P>,
}
