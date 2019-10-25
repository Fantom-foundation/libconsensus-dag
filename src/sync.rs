use crate::event::NetEvent;
use crate::lamport_time::LamportTime;
use crate::peer::GossipList;
use core::fmt::Display;
use core::fmt::Formatter;
use core::hash::Hash;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use libhash_sha3::Hash as EventHash;
use libsignature::PublicKey;
use libsignature::Signature;
use serde::{Deserialize, Serialize};

// Sync request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReq<P: Hash + Eq> {
    pub(crate) from: P,
    pub(crate) to: P,
    pub(crate) gossip_list: GossipList<P>,
    pub(crate) lamport_time: LamportTime,
}

impl<P> Stub for SyncReq<P> where P: PeerId {}

// Sync Reply
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReply<Data, P: Hash + Eq, PK, Sig>
where
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    pub(crate) from: P,
    pub(crate) to: P,
    pub(crate) gossip_list: GossipList<P>,
    pub(crate) lamport_time: LamportTime,
    #[serde(bound(deserialize = "Data: Deserialize<'de>"))]
    pub(crate) events: Vec<NetEvent<Data, P, PK, Sig>>,
}

impl<'a, Data, P, PK, Sig> Stub for SyncReply<Data, P, PK, Sig>
where
    Data: Serialize + Deserialize<'a> + Send + Clone,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
}

impl<P> Display for SyncReq<P>
where
    P: PeerId,
{
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let mut formatted = String::new();
        formatted.push_str(&format!(
            "from:{}; to:{}; lamport_time:{}; gossip_list:[",
            self.from.clone(),
            self.to.clone(),
            self.lamport_time
        ));
        for (k, v) in self.gossip_list.iter() {
            formatted.push_str(&format!("({}:{})", k, v));
        }
        formatted.push_str("]");
        write!(f, "{}", formatted)
    }
}

impl<'a, Data, P, PK, Sig> Display for SyncReply<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let mut formatted = String::new();
        formatted.push_str(&format!(
            "from:{}; to:{}; lamport_time:{}; gossip_list:[",
            self.from.clone(),
            self.to.clone(),
            self.lamport_time
        ));
        for (k, v) in self.gossip_list.iter() {
            formatted.push_str(&format!("({}:{})", k, v));
        }
        formatted.push_str("]; events:[");
        for e in self.events.iter() {
            formatted.push_str(&format!("({})", e));
        }
        formatted.push_str("]");
        write!(f, "{}", formatted)
    }
}
