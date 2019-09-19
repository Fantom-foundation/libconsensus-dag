#![feature(try_trait)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[macro_use]
extern crate crossbeam_channel;
#[macro_use]
extern crate log;
extern crate libconsensus;
use crate::conf::DAGconfig;
use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use crate::peer::DAGPeerList;
use crate::peer::Frame;
use crate::peer::GossipList;
use crate::store::DAGstore;
use crate::store_sled::SledStore;
use crate::sync::{SyncReply, SyncReq};
use crate::transactions::InternalTransaction;
use crossbeam_channel::tick;
use futures::executor::block_on;
use futures::stream::Stream;
use futures::stream::StreamExt;
use futures::task::Context;
use futures::task::Poll;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::errors::Result as BaseResult;
use libconsensus::Consensus;
use libtransport::Transport;
use libtransport::TransportReceiver;
use libtransport::TransportSender;
use libtransport_tcp::receiver::TCPreceiver;
use libtransport_tcp::sender::TCPsender;
use libtransport_tcp::TCPtransport;
use log::error;
use std::pin::Pin;
use std::sync::mpsc::{self, Sender};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

// DAG node structure
pub struct DAG<P, T>
where
    T: DataType,
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
    Data: DataType,
    P: PeerId,
{
    conf: Arc<RwLock<DAGconfig<P, Data>>>,
    store: Arc<RwLock<dyn DAGstore<Data, P>>>,
    tx_pool: Vec<Data>,
    internal_tx_pool: Vec<InternalTransaction<P>>,
    lamport_time: LamportTime,
    current_frame: Frame,
    last_finalised_frame: Option<Frame>,
    //    sync_request_transport: Box<dyn Transport<P, SyncReq<P>, Error, DAGPeerList<P>> + 'a>,
    //    sync_reply_transport: Box<dyn Transport<P, SyncReply<P>, Error, DAGPeerList<P>> + 'a>,
}

fn listener<P, Data: 'static>(core: Arc<RwLock<DAGcore<P, Data>>>, quit_rx: Receiver<()>)
where
    Data: DataType,
    P: PeerId,
{
    let config = { core.read().unwrap().conf.clone() };
    // FIXME: what we do with unwrap() in threads?
    loop {
        // check if quit channel got message
        let mut cfg = config.write().unwrap();
        match quit_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                cfg.shutdown = true;
                break;
            }
            Err(TryRecvError::Empty) => {}
        }
        // allow to pool again if waker is set
        if let Some(waker) = cfg.waker.take() {
            waker.wake()
        }
    }
}

// Procedure A of DAG consensus
fn procedure_a<P, D>(core: Arc<RwLock<DAGcore<P, D>>>)
where
    D: DataType + 'static,
    P: PeerId + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    let ticker = {
        let cfg = config.read().unwrap();
        tick(Duration::from_millis(cfg.heartbeat))
    };
    let (transport_type, reply_bind_address) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.reply_addr.clone())
    };
    // setup TransportSender for Sync Request.
    let mut sync_req_sender = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPsender::<P, SyncReq<P>, errors::Error, peer::DAGPeerList<P>>::new().unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    let mut sync_reply_receiver = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPreceiver::<P, SyncReply<D, P>, Error, DAGPeerList<P>>::new(reply_bind_address)
                    .unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
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
        // choose the next peer and send Sync Request to it.
        let peer = cfg.peers.next_peer();
        let gossip_list: GossipList<P> = cfg.peers.get_gossip_list();
        let request = SyncReq {
            from: cfg.peers[0].id.clone(), // FIXME: we assume creator is the peer of index 0
            to: peer.id,
            gossip_list,
            lamport_time: { core.read().unwrap().lamport_time.clone() },
        };
        match sync_req_sender.send(peer.request_addr, request) {
            Ok(()) => {}
            Err(e) => error!("error sending sync request: {:?}", e),
        }

        // Receive Sync Reply and process it.
        // NB: it may not be from the very same peer we have sent Sync Request above.
        block_on(async {
            if let Some(sync_reply) = sync_reply_receiver.next().await {
                debug!(
                    "{} Sync Reply from {}",
                    sync_reply.to.clone(),
                    sync_reply.from.clone()
                );
                // update Lamport timestamp of the node
                {
                    core.write()
                        .unwrap()
                        .update_lamport_time(sync_reply.lamport_time);
                }
                // process unknown events
                for event in sync_reply.events.iter() {
                    {
                        config
                            .write()
                            .unwrap()
                            .peers
                            .find_peer_mut(event.get_creator())
                            .unwrap()
                            .update_lamport_time_and_height(
                                event.get_lamport_time(),
                                event.get_height(),
                            );
                    }
                }
            }
        });

        // create new event if needed referring remote peer as other-parent
        // FIXME: need to be implemented
        {
            let mut core_loc = core.write().unwrap();
        }

        // wait until hearbeat interval expires
        select! {
            recv(ticker) -> _ => {},
        }
    }
}

// Procedure B of DAG consensus
fn procedure_b<P, D>(core: Arc<RwLock<DAGcore<P, D>>>)
where
    D: DataType + 'static,
    P: PeerId + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    let (transport_type, request_bind_address) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.request_addr.clone())
    };
    let mut sync_req_receiver = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPreceiver::<P, SyncReq<P>, Error, DAGPeerList<P>>::new(request_bind_address)
                    .unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    let mut sync_reply_sender = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPsender::<P, SyncReply<D, P>, Error, DAGPeerList<P>>::new().unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    let store = { core.read().unwrap().store.clone() };
    block_on(async {
        while let Some(sync_req) = sync_req_receiver.next().await {
            debug!("{} Sync request from {}", sync_req.to, sync_req.from);
            {
                core.write()
                    .unwrap()
                    .update_lamport_time(sync_req.lamport_time);
            }
            match store
                .read()
                .unwrap()
                .get_events_for_gossip(&sync_req.gossip_list)
            {
                Err(e) => error!("Procedure B: get_events_for_gossip() error: {:?}", e),
                Ok(events) => {
                    let gossip_list: GossipList<P> = config.read().unwrap().peers.get_gossip_list();
                    let reply = SyncReply::<D, P> {
                        from: sync_req.to,
                        to: sync_req.from,
                        gossip_list,
                        lamport_time: { core.read().unwrap().lamport_time.clone() },
                        events,
                    };
                    match {
                        config
                            .write()
                            .unwrap()
                            .peers
                            .find_peer_with_lamport_time_update(
                                reply.to.clone(),
                                sync_req.lamport_time,
                            )
                    } {
                        Ok(peer) => {
                            let address = peer.reply_addr.clone();
                            let res = sync_reply_sender.send(address, reply);
                            match res {
                                Ok(()) => {}
                                Err(e) => error!("error sendinf sync reply: {:?}", e),
                            }
                        }
                        Err(e) => error!("peer {} find error: {:?}", reply.to, e),
                    }
                }
            }
        }
    });
}

impl<P, D> Consensus<'_, D> for DAG<P, D>
where
    P: PeerId + 'static,
    D: DataType + 'static,
{
    type Configuration = DAGconfig<P, D>;

    fn new(mut cfg: DAGconfig<P, D>) -> BaseResult<DAG<P, D>> {
        let (tx, rx) = mpsc::channel();
        //cfg.set_quit_rx(rx);
        let bind_addr = cfg.request_addr.clone();
        let reply_addr = cfg.reply_addr.clone();
        let transport_type = cfg.transport_type.clone();
        let store_type = cfg.store_type.clone();
        let mut sr_transport = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    TCPtransport::<P, SyncReq<P>, Error, DAGPeerList<P>>::new(bind_addr)?
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };
        let mut sp_transport = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    TCPtransport::<P, SyncReply<D, P>, Error, DAGPeerList<P>>::new(reply_addr)?
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };
        let store = {
            match store_type {
                crate::store::StoreType::Unknown => panic!("unknown DAG store"),
                crate::store::StoreType::Sled => {
                    // FIXME: we should use a configurable parameter for store location instead of "./sled_store"
                    <SledStore as DAGstore<D, P>>::new("./sled_store").unwrap()
                }
            }
        };

        let core = Arc::new(RwLock::new(DAGcore {
            conf: Arc::new(RwLock::new(cfg)),
            store: Arc::new(RwLock::new(store)),
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
        let listener_core = core.clone();
        let handle = thread::spawn(|| listener(listener_core, rx));
        //        let configA = Arc::clone(&cfg_mutexed);
        let core_a = core.clone();
        let proc_a_handle = thread::spawn(|| procedure_a(core_a));
        //        let configB = Arc::clone(&cfg_mutexed);
        let core_b = core.clone();
        let proc_b_handle = thread::spawn(|| procedure_b(core_b));
        return Ok(DAG {
            core: core,
            quit_tx: tx,
            listener_handle: Some(handle),
            procA_handle: Some(proc_a_handle),
            procB_handle: Some(proc_b_handle),
        });
    }

    // Terminates procedures A and B of DAG0 started with run() method.
    fn shutdown(&mut self) -> BaseResult<()> {
        let _ = self.quit_tx.send(());
        Ok(())
    }

    fn send_transaction(&mut self, data: D) -> BaseResult<()> {
        let mut core = self.core.write().unwrap();
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
    D: DataType,
    P: PeerId,
{
    fn drop(&mut self) {
        self.quit_tx.send(()).unwrap();
    }
}

impl<P, D> DAG<P, D>
where
    D: DataType,
    P: PeerId,
{
    // send internal transaction
    fn send_internal_transaction(&mut self, tx: InternalTransaction<P>) -> Result<()> {
        let mut core = self.core.write().unwrap();
        // Vec::push() panics when number of elements overflows `usize`
        if core.internal_tx_pool.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        core.internal_tx_pool.push(tx);
        Ok(())
    }
}

impl<P, D> Unpin for DAG<P, D>
where
    D: DataType,
    P: PeerId,
{
}

impl<P, Data> Stream for DAG<P, Data>
where
    P: PeerId,
    Data: DataType,
{
    type Item = Data;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let myself = Pin::get_mut(self);
        let config = {
            let core = myself.core.write().unwrap();
            Arc::clone(&core.conf)
        };
        let mut cfg = config.write().unwrap();
        // FIXME: need to be implemented
        cfg.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl<P, Data> DAGcore<P, Data>
where
    P: PeerId,
    Data: DataType,
{
    fn update_lamport_time(&mut self, time: LamportTime) {
        if self.lamport_time < time {
            self.lamport_time = time;
        }
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
