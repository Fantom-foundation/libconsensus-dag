// contains TransactionType definition
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use libconsensus::BaseConsensusPeer;
use libconsensus::TransactionType;
use libsignature::PublicKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InternalTransaction<P, PK> {
    transaction_type: TransactionType,
    peer: BaseConsensusPeer<P, PK>,
}

impl<P, PK> Stub for InternalTransaction<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
}
