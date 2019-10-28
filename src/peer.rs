use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use core::fmt::Display;
use core::fmt::Formatter;
use core::slice::{Iter, IterMut};
use libcommon_rs::peer::{Peer, PeerId, PeerList};
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::BaseConsensusPeer;
use libsignature::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::ops::{Index, IndexMut};

pub(crate) type FrameNumber = usize;
pub(crate) type Height = usize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gossip {
    pub(crate) lamport_time: LamportTime,
    pub(crate) height: Height,
}

impl Display for Gossip {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "({};{})", self.lamport_time, self.height)
    }
}

pub(crate) type GossipList<P> = HashMap<P, Gossip>;
pub(crate) type SuspectList<P> = HashMap<P, LamportTime>;

// Peer attributes
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DAGPeer<P, PK> {
    #[serde(rename = "PubKeyHex")]
    pub pub_key: PK,
    #[serde(rename = "ID")]
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

impl<P, PK> From<BaseConsensusPeer<P, PK>> for DAGPeer<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    fn from(bp: BaseConsensusPeer<P, PK>) -> DAGPeer<P, PK> {
        let mut socket: SocketAddr = bp.net_addr.clone().parse().unwrap();
        let port = socket.port();
        socket.set_port(port + 1);
        DAGPeer {
            pub_key: bp.pub_key,
            id: bp.id,
            request_addr: bp.net_addr,
            reply_addr: socket.to_string(),
            height: 0,
            lamport_time: 0,
        }
    }
}

impl<P, PK> Peer<P, Error> for DAGPeer<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    fn new(id: P, net_addr: String) -> Self {
        let mut socket: SocketAddr = net_addr.clone().parse().unwrap();
        let port = socket.port();
        socket.set_port(port + 1);
        DAGPeer {
            pub_key: PK::default(),
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

impl<P, PK> DAGPeer<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    pub(crate) fn get_public_key(&self) -> PK {
        self.pub_key.clone()
    }
    pub(crate) fn set_public_key(&mut self, key: PK) {
        self.pub_key = key;
    }
    pub(crate) fn update_lamport_time(&mut self, time: LamportTime) {
        if self.lamport_time < time {
            self.lamport_time = time;
        }
    }
    pub(crate) fn get_lamport_time(&self) -> LamportTime {
        self.lamport_time
    }
    pub(crate) fn get_next_height(&mut self) -> Height {
        self.height += 1;
        self.height
    }
    pub(crate) fn get_height(&self) -> Height {
        self.height
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

impl<P, PK> Display for DAGPeer<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let mut formatted = String::new();
        formatted.push_str(&format!(
            "id:{}; pub_key:{}; request_addr:{}; reply_addr:{}; height:{}; lamport_time:{}.",
            self.id.clone(),
            self.pub_key.clone(),
            self.request_addr.clone(),
            self.reply_addr.clone(),
            self.height,
            self.lamport_time
        ));
        write!(f, "{}", formatted)
    }
}

#[derive(Clone)]
pub struct DAGPeerList<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    peers: Vec<DAGPeer<P, PK>>,
    // number of peers; the size of the peer list
    n: usize,
    // round robin number
    r: usize,
    // creator ID for the current node
    creator: P,
    // index of creator in the peers
    current: usize,
}

impl<P, PK> Default for DAGPeerList<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    fn default() -> DAGPeerList<P, PK> {
        DAGPeerList {
            peers: Vec::with_capacity(5),
            n: 0,
            r: 0,
            creator: Default::default(),
            current: 0,
        }
    }
}

impl<P, PK> Index<usize> for DAGPeerList<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    type Output = DAGPeer<P, PK>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.peers[index]
    }
}

impl<P, PK> IndexMut<usize> for DAGPeerList<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.peers[index]
    }
}

impl<Pid, PK> PeerList<Pid, Error> for DAGPeerList<Pid, PK>
where
    Pid: PeerId,
    PK: PublicKey,
{
    type P = DAGPeer<Pid, PK>;
    fn new() -> Self {
        DAGPeerList {
            peers: Vec::with_capacity(5),
            n: 0,
            r: 0,
            creator: Default::default(),
            current: 0,
        }
    }
    fn get_peers_from_file(&mut self, json_peer_path: String) -> std::result::Result<(), Error> {
        let mut file = File::open(json_peer_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let mut v: Vec<DAGPeer<Pid, PK>> = serde_json::from_str(&data)?;
        self.peers.append(&mut v);
        self.sort_peers();
        Ok(())
    }

    fn add(&mut self, p: DAGPeer<Pid, PK>) -> std::result::Result<(), Error> {
        if self.peers.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        self.peers.push(p);
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

impl<P, PK> DAGPeerList<P, PK>
where
    P: PeerId,
    PK: PublicKey,
{
    pub fn new_with_content(peers: Vec<DAGPeer<P, PK>>) -> Self {
        let mut n = DAGPeerList::new();
        n.peers = peers;
        n
    }
    fn peers_mut(&mut self) -> &mut Vec<DAGPeer<P, PK>> {
        &mut self.peers
    }
    fn sort_peers(&mut self) {
//        let creator = self.creator.clone();
        self.peers_mut().sort_by(|a, b| {
            use std::cmp::Ordering;
            if a.id < b.id {
                Ordering::Less
            } else if a.id > b.id {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
            //            let a_cmp = a.id == creator;
            //            let b_cmp = b.id == creator;
            //            if a_cmp {
            //                if b_cmp {
            //                    Ordering::Equal
            //                } else {
            //                    Ordering::Less
            //                }
            //            } else if b_cmp {
            //                Ordering::Greater
            //            } else {
            //                Ordering::Equal
            //            }
        });
        self.current = match self.peers.iter().position(|x| x.id == self.creator) {
            Some(p) => p,
            None => 0, //panic!("creator not found in the peers!"),
        }
    }
    pub fn set_creator(&mut self, creator: P) {
        self.creator = creator;
        self.sort_peers();
    }
    pub fn next_peer(&mut self) -> DAGPeer<P, PK> {
        // we assume the very first peer in the vector is one
        // cotrresponding to the current node, so the value of `current`
        // is always 0 and omitted here.
        //let next = 1 + self.r % (self.n - 1);
        let next = (self.current + self.r) % self.n;
        if self.r > 1 {
            self.r >>= 1;
        } else {
            self.r = self.n >> 1
        }
        self.peers[next].clone()
    }

    pub fn get_gossip_list(&self) -> GossipList<P> {
        let mut g = GossipList::<P>::new();
        for (_i, p) in self.peers.iter().enumerate() {
            g.insert(
                p.id.clone(),
                Gossip {
                    lamport_time: p.lamport_time,
                    height: p.height,
                },
            );
        }
        g
    }

    // Find a peer for read only operations
    pub fn find_peer(&self, id: &P) -> Result<DAGPeer<P, PK>> {
        match self.peers.iter().find(|&x| x.id == *id) {
            None => Err(Error::NoneError.into()),
            Some(p_ref) => Ok(p_ref.clone()),
        }
    }

    // Find a peer for read only operations and update its lamport time if needed
    pub fn find_peer_with_lamport_time_update(
        &mut self,
        id: &P,
        time: LamportTime,
    ) -> Result<DAGPeer<P, PK>> {
        match self.peers.iter_mut().find(|x| x.id == *id) {
            None => Err(Error::NoneError.into()),
            Some(p_ref) => {
                if p_ref.lamport_time < time {
                    p_ref.lamport_time = time;
                }
                Ok(p_ref.clone())
            }
        }
    }

    // Find a peer for read/write operations
    pub(crate) fn find_peer_mut(&mut self, id: &P) -> Result<&mut DAGPeer<P, PK>> {
        match self.peers.iter_mut().find(|x| x.id == *id) {
            None => Err(Error::NoneError.into()),
            Some(p_ref) => Ok(p_ref),
        }
    }

    /// Return RootMajority value
    pub(crate) fn root_majority(&self) -> usize {
        2 * self.peers.len() / 3 + 1
    }

    pub(crate) fn len(&self) -> usize {
        self.peers.len()
    }

    pub(crate) fn get_creator_id(&self) -> P {
        self.peers[self.current].id.clone()
    }
}
