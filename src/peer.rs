use std::collections::HashMap;

use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use libconsensus::{BaseConsensusPeer, PeerId};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

pub(crate) type Frame = usize;
pub(crate) type Height = usize;

type GossipList = HashMap<PeerId, LamportTime>;
type SuspectList = HashMap<PeerId, LamportTime>;

// Peer attributes
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DAG0Peer {
    #[serde(rename = "PubKeyHex")]
    id: PeerId,
    #[serde(rename = "NetAddr")]
    net_addr: String,
    #[serde(skip, default)]
    height: Height,
}

impl From<BaseConsensusPeer> for DAG0Peer {
    fn from(bp: BaseConsensusPeer) -> DAG0Peer {
        DAG0Peer {
            id: bp.id,
            net_addr: bp.net_addr,
            height: 0,
        }
    }
}

pub(crate) struct PeerList {
    peers: Vec<DAG0Peer>,
    // number of peers; the size of the peer list
    n: usize,
    // round robin number
    r: usize,
}

impl Default for PeerList {
    fn default() -> PeerList {
        PeerList {
            peers: Vec::with_capacity(5),
            n: 0,
            r: 0,
        }
    }
}

impl PeerList {
    fn get_peers_from_file(&mut self, json_peer_path: String) -> Result<()> {
        let mut file = File::open(json_peer_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let mut v: Vec<DAG0Peer> = serde_json::from_str(&data)?;
        self.peers.append(&mut v);
        Ok(())
    }

    pub fn add(&mut self, p: BaseConsensusPeer) -> Result<()> {
        if self.peers.len() == std::usize::MAX {
            return Err(Error::AtMaxVecCapacity);
        }
        self.peers.push(p.into());
        self.n = self.peers.len();
        self.r = self.n >> 1;
        Ok(())
    }

    pub fn next_peer(&mut self) -> DAG0Peer {
        // we assume the very first peer in the vector is one
        // cotrresponding to the current node, so the value of `current`
        // is always 0 and omitted here.
        let next = 1 + self.r % (self.n - 1);
        if self.r > 0 {
            self.r >>= 1;
        } else {
            self.r = self.n >> 1
        }
        return self.peers[next].clone();
    }
}
