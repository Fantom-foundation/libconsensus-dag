use crate::event::Event;
use crate::lamport_time::LamportTime;
use crate::peer::GossipList;
use core::hash::Hash;
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use serde::{Deserialize, Serialize};

// Sync request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReq<P: Hash + Eq> {
    from: P,
    to: P,
    gossip_list: GossipList<P>,
    lamport_time: LamportTime,
}

impl<P> Stub for SyncReq<P> where P: PeerId {}

// Sync Reply
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReply<P: Hash + Eq> {
    from: P,
    to: P,
    gossip_list: GossipList<P>,
    lamport_time: LamportTime,
    events: Vec<Event<P>>,
}
