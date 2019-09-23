use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use core::slice::{Iter, IterMut};
use libcommon_rs::peer::{Peer, PeerId, PeerList};
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::BaseConsensusPeer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::ops::{Index, IndexMut};

pub(crate) type Frame = usize;
pub(crate) type Height = usize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Gossip {
    pub(crate) lamport_time: LamportTime,
    pub(crate) height: Height,
}

pub(crate) type GossipList<P> = HashMap<P, Gossip>;
pub(crate) type SuspectList<P> = HashMap<P, LamportTime>;

// Peer attributes
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DAGPeer<P> {
    #[serde(rename = "PubKeyHex")]
    pub(crate) id: P,
    #[serde(rename = "NetAddr")]
    pub(crate) request_addr: String,
    #[serde(skip, default)]
    pub(crate) reply_addr: String,
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
        let mut socket: SocketAddr = bp.net_addr.clone().parse().unwrap();
        let port = socket.port();
        socket.set_port(port + 1);
        DAGPeer {
            id: bp.id,
            request_addr: bp.net_addr,
            reply_addr: socket.to_string(),
            height: 0,
            lamport_time: 0,
        }
    }
}

impl<P> Peer<P, Error> for DAGPeer<P>
where
    P: PeerId,
{
    fn new(id: P, net_addr: String) -> Self {
        let mut socket: SocketAddr = net_addr.clone().parse().unwrap();
        let port = socket.port();
        socket.set_port(port + 1);
        DAGPeer {
            id,
            request_addr: net_addr,
            reply_addr: socket.to_string(),
            height: 0,
            lamport_time: 0,
        }
    }
    fn get_id(&self) -> P {
        self.id.clone()
    }
    fn get_base_addr(&self) -> String {
        self.request_addr.clone()
    }
    fn get_net_addr(&self, n: usize) -> String {
        match n {
            0 => self.request_addr.clone(),
            1 => self.reply_addr.clone(),
            // FIXME: should we return Err() in that case?
            _ => self.request_addr.clone(),
        }
    }
    fn set_net_addr(&mut self, n: usize, addr: String) -> std::result::Result<(), Error> {
        match n {
            0 => {
                self.request_addr = addr;
                Ok(())
            }
            1 => {
                self.reply_addr = addr;
                Ok(())
            }
            _ => Err(Error::Base(AtMaxVecCapacity)),
        }
    }
}

impl<P> DAGPeer<P>
where
    P: PeerId,
{
    pub(crate) fn update_lamport_time(&mut self, time: LamportTime) {
        if self.lamport_time < time {
            self.lamport_time = time;
        }
    }
    pub(crate) fn update_lamport_time_and_height(&mut self, time: LamportTime, height: Height) {
        if self.lamport_time < time {
            self.lamport_time = time;
        }
        if self.height < height {
            self.height = height;
        }
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
    // creator ID for the current node
    creator: P,
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
            creator: Default::default(),
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
            creator: Default::default(),
        }
    }
    fn get_peers_from_file(&mut self, json_peer_path: String) -> Result<()> {
        let mut file = File::open(json_peer_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let mut v: Vec<DAGPeer<Pid>> = serde_json::from_str(&data)?;
        self.peers.append(&mut v);
        self.sort_peers();
        Ok(())
    }

    fn add(&mut self, p: DAGPeer<Pid>) -> Result<()> {
        if self.peers.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        self.peers.push(p.into());
        self.n = self.peers.len();
        self.r = self.n >> 1;
        self.sort_peers();
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
    fn peers_mut(&mut self) -> &mut Vec<DAGPeer<P>> {
        &mut self.peers
    }
    fn sort_peers(&mut self) {
        let creator = self.creator.clone();
        self.peers_mut().sort_by(|a, b| {
            use std::cmp::Ordering;
            let a_cmp = a.id == creator;
            let b_cmp = b.id == creator;
            if a_cmp {
                if b_cmp {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            } else {
                if b_cmp {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            }
        })
    }
    pub fn set_creator(&mut self, creator: P) {
        self.creator = creator;
        self.sort_peers();
    }
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
            g.insert(
                p.id.clone(),
                Gossip {
                    lamport_time: p.lamport_time.clone(),
                    height: p.height.clone(),
                },
            );
        }
        g
    }

    // Find a peer for read only operations
    pub fn find_peer(&self, id: P) -> Result<DAGPeer<P>> {
        let p_ref = self.peers.iter().find(|&x| x.id == id)?;
        Ok(p_ref.clone())
    }

    // Find a peer for read only operations and update its lamport time if needed
    pub fn find_peer_with_lamport_time_update(
        &mut self,
        id: P,
        time: LamportTime,
    ) -> Result<DAGPeer<P>> {
        let mut p_ref = self.peers.iter_mut().find(|x| x.id == id)?;
        if p_ref.lamport_time < time {
            p_ref.lamport_time = time;
        }
        Ok(p_ref.clone())
    }

    // Find a peer for read/write operations
    pub fn find_peer_mut(&mut self, id: P) -> Result<&mut DAGPeer<P>> {
        let mut p_ref = self.peers.iter_mut().find(|x| x.id == id)?;
        Ok(p_ref)
    }
}
