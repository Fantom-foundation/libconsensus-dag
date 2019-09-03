use crate::lamport_time::LamportTime;
use crate::peer::GossipList;
use libcommon_rs::peer::PeerId;
use serde::{Deserialize, Serialize};

// Sync request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReq<P>
where
    P: PeerId,
{
    from: P,
    to: P,
    gossip_list: GossipList<P>,
    lamport_time: LamportTime,
}
