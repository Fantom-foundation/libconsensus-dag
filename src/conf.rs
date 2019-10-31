// Config module

use crate::peer::DAGPeerList;
use futures::task::Waker;
use libcommon_rs::peer::{PeerId, PeerList};
use libcommon_rs::store::StoreType;
use libconsensus::ConsensusConfiguration;
use libsignature::PublicKey;
use libsignature::SecretKey;
use libtransport::TransportType;
use std::marker::PhantomData;
use std::path::PathBuf;

pub struct DAGconfig<P, Data, SK, PK>
where
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
{
    pub request_addr: String,
    pub reply_addr: String,
    pub transport_type: TransportType,
    pub store_type: StoreType,
    pub store_dir: PathBuf,
    // heartbeat duration in milliseconds
    pub heartbeat: u64,
    pub(crate) proc_a_delay: u64,
    pub(crate) waker: Option<Waker>,
    pub peers: DAGPeerList<P, PK>,
    pub creator: P,
    pub secret_key: SK,
    pub public_key: PK,
    phantom: PhantomData<Data>,
}

impl<P, Data, SK, PK> DAGconfig<P, Data, SK, PK>
where
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
{
    pub fn set_heartbeat(&mut self, heartbeat: u64) {
        self.heartbeat = heartbeat;
    }
    pub fn set_store_type(&mut self, store_type: StoreType) {
        self.store_type = store_type;
    }
    pub fn set_transport_type(&mut self, transport_type: TransportType) {
        self.transport_type = transport_type;
    }
    pub fn set_reply_addr(&mut self, reply_addr: String) {
        self.reply_addr = reply_addr;
    }
    pub fn set_request_addr(&mut self, request_addr: String) {
        self.request_addr = request_addr;
    }
    pub fn get_creator(&self) -> P {
        self.creator.clone()
    }
    pub fn get_secret_key(&self) -> SK {
        self.secret_key.clone()
    }
    pub fn get_public_key(&self) -> PK {
        self.public_key.clone()
    }
    pub fn get_proc_a_delay(&self) -> u64 {
        self.proc_a_delay
    }
    pub fn get_request_addr(&self) -> String {
        self.request_addr.clone()
    }
}

impl<P, Data, SK, PK> ConsensusConfiguration<Data> for DAGconfig<P, Data, SK, PK>
where
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
{
    fn new() -> Self {
        DAGconfig {
            request_addr: "localhost:9000".to_string(),
            reply_addr: "localhost:12000".to_string(),
            heartbeat: 1000,
            proc_a_delay: 3000,
            transport_type: TransportType::Unknown,
            store_type: StoreType::Unknown,
            store_dir: PathBuf::from("./sled_store"),
            waker: None,
            peers: DAGPeerList::new(),
            creator: Default::default(),
            secret_key: SK::default(),
            public_key: PK::default(),
            phantom: PhantomData,
        }
    }
}
