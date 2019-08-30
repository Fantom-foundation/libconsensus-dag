use libconsensus::errors::Error::AtMaxVecCapacity;
use std::collections::HashMap;

use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use core::slice::{Iter, IterMut};
use libcommon_rs::peer::{Peer, PeerList};
use libconsensus::{BaseConsensusPeer, PeerId};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::ops::{Index, IndexMut};

pub(crate) type Frame = usize;
pub(crate) type Height = usize;

pub(crate) type GossipList = HashMap<PeerId, LamportTime>;
pub(crate) type SuspectList = HashMap<PeerId, LamportTime>;

// Peer attributes
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DAGPeer {
    #[serde(rename = "PubKeyHex")]
    id: PeerId,
    #[serde(rename = "NetAddr")]
    net_addr: String,
    #[serde(skip, default)]
    height: Height,
}

impl From<BaseConsensusPeer> for DAGPeer {
    fn from(bp: BaseConsensusPeer) -> DAGPeer {
        DAGPeer {
            id: bp.id,
            net_addr: bp.net_addr,
            height: 0,
        }
    }
}

impl Peer<PeerId> for DAGPeer {
    fn new(id: PeerId, net_addr: String) -> Self {
        DAGPeer {
            id,
            net_addr,
            height: 0,
        }
    }
    fn get_id(&self) -> PeerId {
        self.id.clone()
    }
    fn get_net_addr(&self) -> String {
        self.net_addr.clone()
    }
}

pub(crate) struct DAGPeerList {
    peers: Vec<DAGPeer>,
    // number of peers; the size of the peer list
    n: usize,
    // round robin number
    r: usize,
}

impl Default for DAGPeerList {
    fn default() -> DAGPeerList {
        DAGPeerList {
            peers: Vec::with_capacity(5),
            n: 0,
            r: 0,
        }
    }
}

impl Index<usize> for DAGPeerList {
    type Output = DAGPeer;
    fn index(&self, index: usize) -> &Self::Output {
        &self.peers[index]
    }
}

impl IndexMut<usize> for DAGPeerList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.peers[index]
    }
}

impl PeerList<PeerId, Error> for DAGPeerList {
    type P = DAGPeer;
    fn new() -> Self {
        DAGPeerList {
            peers: Vec::with_capacity(5),
            n: 0,
            r: 0,
        }
    }
    fn get_peers_from_file(&mut self, json_peer_path: String) -> Result<()> {
        let mut file = File::open(json_peer_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let mut v: Vec<DAGPeer> = serde_json::from_str(&data)?;
        self.peers.append(&mut v);
        Ok(())
    }

    fn add(&mut self, p: DAGPeer) -> Result<()> {
        if self.peers.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        self.peers.push(p.into());
        self.n = self.peers.len();
        self.r = self.n >> 1;
        Ok(())
    }
    fn iter(&self) -> Iter<'_, Self::P> {
        self.peers.iter()
    }
    fn iter_mut(&mut self) -> IterMut<'_, Self::P> {
        self.peers.iter_mut()
    }
}

impl DAGPeerList {
    pub fn next_peer(&mut self) -> DAGPeer {
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
