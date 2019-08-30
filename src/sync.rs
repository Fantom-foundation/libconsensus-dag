use crate::lamport_time::LamportTime;
use crate::peer::GossipList;
use libconsensus::PeerId;
use serde::{Deserialize, Serialize};

// Sync request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReq {
    from: PeerId,
    to: PeerId,
    gossip_list: GossipList,
    lamport_time: LamportTime,
}
