// Config module

use crate::peer::DAGPeerList;
use futures::task::Waker;
use libcommon_rs::peer::PeerList;
use libconsensus::ConsensusConfiguration;
use libtransport::TransportType;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, TryRecvError};

pub struct DAGconfig<Data> {
    pub(crate) request_addr: String,
    pub(crate) reply_addr: String,
    shutdown: bool,
    pub(crate) transport_type: TransportType,
    // heartbeat duration in milliseconds
    pub(crate) heartbeat: u64,
    pub(crate) quit_rx: Option<Receiver<()>>,
    pub(crate) waker: Option<Waker>,
    peers: DAGPeerList,
    phantom: PhantomData<Data>,
}

impl<Data> DAGconfig<Data> {
    pub fn set_quit_rx(&mut self, rx: Receiver<()>) {
        self.quit_rx = Some(rx);
    }
    pub fn check_quit(&mut self) -> bool {
        if !self.shutdown {
            match &self.quit_rx {
                None => return false,
                Some(ch) => match ch.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => self.shutdown = true,
                    Err(TryRecvError::Empty) => {}
                },
            }
        }
        self.shutdown
    }
}

impl<Data> ConsensusConfiguration<Data> for DAGconfig<Data> {
    fn new() -> Self {
        return DAGconfig {
            request_addr: "localhost:9000".to_string(),
            reply_addr: "localhost:12000".to_string(),
            heartbeat: 1000,
            shutdown: false,
            transport_type: TransportType::Unknown,
            quit_rx: None,
            waker: None,
            peers: DAGPeerList::new(),
            phantom: PhantomData,
        };
    }
}
