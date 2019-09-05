use libconsensus::errors::Error::AtMaxVecCapacity;
use std::collections::HashMap;

use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use core::slice::{Iter, IterMut};
use libcommon_rs::peer::{Peer, PeerId, PeerList};
use libconsensus::BaseConsensusPeer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::ops::{Index, IndexMut};

pub(crate) type Frame = usize;
pub(crate) type Height = usize;

pub(crate) type GossipList<P> = HashMap<P, LamportTime>;
pub(crate) type SuspectList<P> = HashMap<P, LamportTime>;

// Peer attributes
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DAGPeer<P> {
    #[serde(rename = "PubKeyHex")]
    id: P,
    #[serde(rename = "NetAddr")]
    net_addr: String,
    #[serde(skip, default)]
    height: Height,
    #[serde(skip, default)]
    lamport_time: LamportTime,
}

impl<P> From<BaseConsensusPeer<P>> for DAGPeer<P>
where
    P: PeerId,
{
    fn from(bp: BaseConsensusPeer<P>) -> DAGPeer<P> {
        DAGPeer {
            id: bp.id,
            net_addr: bp.net_addr,
            height: 0,
            lamport_time: 0,
        }
    }
}

impl<P> Peer<P> for DAGPeer<P>
where
    P: PeerId,
{
    fn new(id: P, net_addr: String) -> Self {
        DAGPeer {
            id,
            net_addr,
            height: 0,
            lamport_time: 0,
        }
    }
    fn get_id(&self) -> P {
        self.id.clone()
    }
    fn get_net_addr(&self) -> String {
        self.net_addr.clone()
    }
}

pub(crate) struct DAGPeerList<P>
where
    P: PeerId,
{
    peers: Vec<DAGPeer<P>>,
    // number of peers; the size of the peer list
    n: usize,
    // round robin number
    r: usize,
}

impl<P> Default for DAGPeerList<P>
where
    P: PeerId,
{
    fn default() -> DAGPeerList<P> {
        DAGPeerList {
            peers: Vec::with_capacity(5),
            n: 0,
            r: 0,
        }
    }
}

impl<P> Index<usize> for DAGPeerList<P>
where
    P: PeerId,
{
    type Output = DAGPeer<P>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.peers[index]
    }
}

impl<P> IndexMut<usize> for DAGPeerList<P>
where
    P: PeerId,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.peers[index]
    }
}

impl<Pid> PeerList<Pid, Error> for DAGPeerList<Pid>
where
    Pid: PeerId,
{
    type P = DAGPeer<Pid>;
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
        let mut v: Vec<DAGPeer<Pid>> = serde_json::from_str(&data)?;
        self.peers.append(&mut v);
        Ok(())
    }

    fn add(&mut self, p: DAGPeer<Pid>) -> Result<()> {
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

impl<P> DAGPeerList<P>
where
    P: PeerId,
{
    pub fn next_peer(&mut self) -> DAGPeer<P> {
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

    pub fn get_gossip_list(&self) -> GossipList<P> {
        let mut g = GossipList::<P>::new();
        for (_i, p) in self.peers.iter().enumerate() {
            g.insert(p.id.clone(), p.lamport_time.clone());
        }
        g
    }
}
