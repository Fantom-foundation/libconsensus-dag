#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[macro_use]
extern crate crossbeam_channel;
#[macro_use]
extern crate libconsensus;
use crate::conf::DAGconfig;
use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use crate::peer::DAGPeerList;
use crate::peer::Frame;
use crate::store::DAGstore;
use crate::store_sled::SledStore;
use crate::sync::{SyncReply, SyncReq};
use crate::transactions::InternalTransaction;
use crossbeam_channel::tick;
use futures::stream::Stream;
use futures::task::Context;
use futures::task::Poll;
use libcommon_rs::peer::PeerId;
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::errors::Result as BaseResult;
use libconsensus::Consensus;
use libtransport::Transport;
use libtransport::TransportSender;
use libtransport_tcp::sender::TCPsender;
use libtransport_tcp::TCPtransport;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::pin::Pin;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

// DAG node structure
pub struct DAG<P, T>
where
    P: PeerId,
{
    //    conf: Arc<Mutex<DAGconfig<P, T>>>,
    core: Arc<RwLock<DAGcore<P, T>>>,
    listener_handle: Option<JoinHandle<()>>,
    procA_handle: Option<JoinHandle<()>>,
    procB_handle: Option<JoinHandle<()>>,

    //    tx_pool: Vec<T>,
    //    internal_tx_pool: Vec<InternalTransaction<P>>,
    quit_tx: Sender<()>,
    //    lamport_time: LamportTime,
    //    current_frame: Frame,
    //    last_finalised_frame: Option<Frame>,
    //    sync_request_transport: Box<dyn Transport<P, SyncReq<P>, Error, DAGPeerList<P>> + 'a>,
    //    sync_reply_transport: Box<dyn Transport<P, SyncReply<P>, Error, DAGPeerList<P>> + 'a>,
}

pub(crate) struct DAGcore<P, Data>
where
    P: PeerId,
{
    conf: Arc<RwLock<DAGconfig<P, Data>>>,
    tx_pool: Vec<Data>,
    internal_tx_pool: Vec<InternalTransaction<P>>,
    lamport_time: LamportTime,
    current_frame: Frame,
    last_finalised_frame: Option<Frame>,
    //    sync_request_transport: Box<dyn Transport<P, SyncReq<P>, Error, DAGPeerList<P>> + 'a>,
    //    sync_reply_transport: Box<dyn Transport<P, SyncReply<P>, Error, DAGPeerList<P>> + 'a>,
}

fn listener<P, Data: 'static>(core: Arc<RwLock<DAGcore<P, Data>>>)
where
    Data: Serialize + DeserializeOwned + Send + Clone,
    P: PeerId,
{
    let config = { core.read().unwrap().conf.clone() };
    // FIXME: what we do with unwrap() in threads?
    loop {
        // check if quit channel got message
        let mut cfg = config.write().unwrap();
        match &cfg.quit_rx {
            None => {}
            Some(ch) => {
                if ch.try_recv().is_ok() {
                    break;
                }
            }
        }
        // allow to pool again if waker is set
        if let Some(waker) = cfg.waker.take() {
            waker.wake()
        }
    }
}

// Procedure A of DAG consensus
fn procedureA<P: 'static, D>(core: Arc<RwLock<DAGcore<P, D>>>)
where
    P: PeerId,
{
    let config = { core.read().unwrap().conf.clone() };
    let ticker = {
        let cfg = config.read().unwrap();
        tick(Duration::from_millis(cfg.heartbeat))
    };
    let (transport_type, store_type) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.store_type.clone())
    };
    // setup TransportSender for Sync Request.
    let sync_req_sender = {
        match transport_type {
            libtransport::TransportType::TCP => {
                <TCPsender<SyncReq<P>> as libtransport::TransportSender<
                    P,
                    sync::SyncReq<P>,
                    errors::Error,
                    peer::DAGPeerList<P>,
                >>::new()
                .unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    // setup Store for procedure A
    let store = {
        match store_type {
            crate::store::StoreType::Unknown => panic!("unknown DAG store"),
            crate::store::StoreType::Sled => {
                // FIXME: we should use a configurable parameter for store location instead of "./sled_store"
                <SledStore as DAGstore<P>>::new("./sled_store").unwrap()
            }
        }
    };
    // DAG procedure A loop
    loop {
        // check if shutdown() has been called
        let mut cfg = config.write().unwrap();
        if cfg.check_quit() {
            // terminating
            // FIXME: need to be implemented
            break;
        }
        let peer = cfg.peers.next_peer();
        let gossip_list = cfg.peers.get_gossip_list();
        let request = SyncReq {
            from: cfg.peers[0].id.clone(), // we assume creator is the peer of index 0
            to: peer.id,
            gossip_list,
            lamport_time: { core.read().unwrap().lamport_time.clone() },
        };

        // wait until hearbeat interval expires
        select! {
            recv(ticker) -> _ => {},
        }
    }
}

// Procedure B of DAG consensus
fn procedureB<P, D>(core: Arc<RwLock<DAGcore<P, D>>>)
where
    P: PeerId,
{
    let config = { core.read().unwrap().conf.clone() };
}

impl<P, D> Consensus<'_, D> for DAG<P, D>
where
    P: PeerId + 'static,
    D: Serialize + DeserializeOwned + Send + Clone + 'static,
{
    type Configuration = DAGconfig<P, D>;

    fn new(mut cfg: DAGconfig<P, D>) -> BaseResult<DAG<P, D>> {
        let (tx, rx) = mpsc::channel();
        cfg.set_quit_rx(rx);
        let bind_addr = cfg.request_addr.clone();
        let reply_addr = cfg.reply_addr.clone();
        let transport_type = cfg.transport_type.clone();
        let mut sr_transport = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    <TCPtransport<SyncReq<P>> as libtransport::Transport<
                        P,
                        sync::SyncReq<P>,
                        errors::Error,
                        peer::DAGPeerList<P>,
                    >>::new(bind_addr)?
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };
        let mut sp_transport = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    <TCPtransport<SyncReply<P>> as libtransport::Transport<
                        P,
                        sync::SyncReply<P>,
                        errors::Error,
                        peer::DAGPeerList<P>,
                    >>::new(reply_addr)?
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };

        let core = Arc::new(RwLock::new(DAGcore {
            conf: Arc::new(RwLock::new(cfg)),
            tx_pool: Vec::with_capacity(1),
            internal_tx_pool: Vec::with_capacity(1),
            lamport_time: 0,
            current_frame: 0,
            last_finalised_frame: None,
            //            sync_request_transport: Box::new(sr_transport)
            //                as Box<
            //                    dyn libtransport::Transport<
            //                        P,
            //                        sync::SyncReq<P>,
            //                        errors::Error,
            //                        peer::DAGPeerList<P>,
            //                    >,
            //                >,
            //            sync_reply_transport: Box::new(sp_transport)
            //                as Box<
            //                    dyn libtransport::Transport<
            //                        P,
            //                        sync::SyncReply<P>,
            //                        errors::Error,
            //                        peer::DAGPeerList<P>,
            //                    >,
            //                >,
        }));

        //        let cfg_mutexed = Arc::new(Mutex::new(cfg));
        //        let config = Arc::clone(&cfg_mutexed);
        let handle = thread::spawn(|| listener(core.clone()));
        //        let configA = Arc::clone(&cfg_mutexed);
        let procA_handle = thread::spawn(|| procedureA(core.clone()));
        //        let configB = Arc::clone(&cfg_mutexed);
        let procB_handle = thread::spawn(|| procedureB(core.clone()));
        return Ok(DAG {
            core: core,
            quit_tx: tx,
            listener_handle: Some(handle),
            procA_handle: Some(procA_handle),
            procB_handle: Some(procB_handle),
        });
    }

    // Terminates procedures A and B of DAG0 started with run() method.
    fn shutdown(&mut self) -> BaseResult<()> {
        let _ = self.quit_tx.send(());
        Ok(())
    }

    fn send_transaction(&mut self, data: D) -> BaseResult<()> {
        let core = self.core.write().unwrap();
        // Vec::push() panics when number of elements overflows `usize`
        if core.tx_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity);
        }
        core.tx_pool.push(data);
        Ok(())
    }
}

impl<P, D> Drop for DAG<P, D>
where
    P: PeerId,
{
    fn drop(&mut self) {
        self.quit_tx.send(()).unwrap();
    }
}

impl<P, D> DAG<P, D>
where
    P: PeerId,
{
    // send internal transaction
    fn send_internal_transaction(&mut self, tx: InternalTransaction<P>) -> Result<()> {
        let core = self.core.write().unwrap();
        // Vec::push() panics when number of elements overflows `usize`
        if core.internal_tx_pool.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        core.internal_tx_pool.push(tx);
        Ok(())
    }
}

impl<P, D> Unpin for DAG<P, D> where P: PeerId {}

impl<P, Data> Stream for DAG<P, Data>
where
    P: PeerId,
    Data: Serialize + DeserializeOwned,
{
    type Item = Data;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let myself = Pin::get_mut(self);
        let config = {
            let core = self.core.write().unwrap();
            Arc::clone(&core.conf)
        };
        let mut cfg = config.write().unwrap();
        // FIXME: need to be implemented
        cfg.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod conf;
mod errors;
mod event;
mod event_hash;
mod flag_table;
mod lamport_time;
mod peer;
mod store;
mod store_sled;
mod sync;
mod transactions;
