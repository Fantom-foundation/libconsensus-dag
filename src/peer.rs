use std::collections::HashMap;

use crate::lamport_time::LamportTime;
use serde::{Deserialize, Serialize};

pub(crate) type PeerId = Vec<u8>;
pub(crate) type Frame = usize;
pub(crate) type Height = usize;

type GossipList = HashMap<PeerId, LamportTime>;
type SuspectList = HashMap<PeerId, LamportTime>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BaseConsensusPeer {
    #[serde(rename = "PubKeyHex")]
    id: PeerId,
    #[serde(rename = "NetAddr")]
    net_addr: String,
}

// Peer attributes
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DAG0Peer {
    #[serde(rename = "PubKeyHex")]
    id: PeerId,
    #[serde(rename = "NetAddr")]
    net_addr: String,
    #[serde(skip, default)]
    lamport_time: LamportTime,
    #[serde(skip, default)]
    height: u64,
    #[serde(skip, default)]
    current_frame: Frame,
    #[serde(skip, default)]
    last_finalised_frame: Frame,
}
