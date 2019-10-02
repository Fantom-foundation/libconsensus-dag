#![feature(try_trait)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[macro_use]
extern crate failure;
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate libconsensus;
use crate::conf::DAGconfig;
use crate::core::DAGcore;
use crate::errors::{Error, Result};
use crate::event::Event;
use crate::peer::DAGPeerList;
use crate::peer::GossipList;
use crate::sync::{SyncReply, SyncReq};
use crate::transactions::InternalTransaction;
use futures::executor::block_on;
use futures::stream::Stream;
use futures::stream::StreamExt;
use futures::task::Context;
use futures::task::Poll;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libconsensus::errors::Result as BaseResult;
use libconsensus::Consensus;
use libhash_sha3::Hash as EventHash;
use libsignature::Signature;
use libsignature::{PublicKey, SecretKey};
use libtransport::Transport;
use libtransport::TransportReceiver;
use libtransport::TransportSender;
use libtransport_tcp::receiver::TCPreceiver;
use libtransport_tcp::sender::TCPsender;
use libtransport_tcp::TCPtransport;
use log::error;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::mpsc::{self, Sender};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

// DAG node structure
pub struct DAG<P, T, SK, PK, Sig>
where
    T: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
{
    //    conf: Arc<Mutex<DAGconfig<P, T>>>,
    core: Arc<RwLock<DAGcore<P, T, SK, PK, Sig>>>,
    listener_handle: Option<JoinHandle<()>>,
    proc_a_handle: Option<JoinHandle<()>>,
    proc_b_handle: Option<JoinHandle<()>>,

    //    tx_pool: Vec<T>,
    //    internal_tx_pool: Vec<InternalTransaction<P>>,
    quit_tx: Sender<()>,
    //    lamport_time: LamportTime,
    //    current_frame: Frame,
    //    last_finalised_frame: Option<Frame>,
    //    sync_request_transport: Box<dyn Transport<P, SyncReq<P>, Error, DAGPeerList<P>> + 'a>,
    //    sync_reply_transport: Box<dyn Transport<P, SyncReply<P>, Error, DAGPeerList<P>> + 'a>,
}

fn listener<P, Data: 'static, SK, PK, Sig>(
    core: Arc<RwLock<DAGcore<P, Data, SK, PK, Sig>>>,
    quit_rx: Receiver<()>,
) where
    Data: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
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
fn procedure_a<P, D, SK, PK, Sig>(core: Arc<RwLock<DAGcore<P, D, SK, PK, Sig>>>)
where
    D: DataType + 'static,
    P: PeerId + 'static,
    SK: SecretKey,
    PK: PublicKey + 'static,
    Sig: Signature + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    let mut ticker = {
        let cfg = config.read().unwrap();
        async_timer::Interval::platform_new(Duration::from_millis(cfg.heartbeat))
    };
    let (transport_type, reply_bind_address) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.reply_addr.clone())
    };
    // setup TransportSender for Sync Request.
    let mut sync_req_sender = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPsender::<P, SyncReq<P>, errors::Error, peer::DAGPeerList<P, PK>>::new().unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    let mut sync_reply_receiver = {
        match transport_type {
            libtransport::TransportType::TCP => {
                let x: TCPreceiver<P, SyncReply<D, P, PK, Sig>, Error, DAGPeerList<P, PK>> =
                    TCPreceiver::new(reply_bind_address).unwrap();
                //                TCPreceiver::<P, SyncReply<D, P, PK, Sig>, Error, DAGPeerList<P, PK>>::new(
                //                    reply_bind_address,
                //                )
                //                .unwrap()
                x
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
            lamport_time: { core.read().unwrap().get_lamport_time() },
        };
        drop(cfg);
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
                for ev in sync_reply.events.into_iter() {
                    {
                        let event: Event<D, P, PK, Sig> = ev.into();
                        // check if event is valid
                        if !core.read().unwrap().check_event(&event).unwrap() {
                            error!("Event {:?} is not valid", event);
                            continue;
                        }
                        let lamport_time = event.get_lamport_time();
                        let height = event.get_height();
                        let creator = event.get_creator();
                        // insert event into node DB
                        core.write().unwrap().insert_event(event).unwrap();
                        // update lamport time and height of the event creator's peer
                        config
                            .write()
                            .unwrap()
                            .peers
                            .find_peer_mut(creator)
                            .unwrap()
                            .update_lamport_time_and_height(lamport_time, height);
                    }
                }
            }
        });

        // create new event if needed referring remote peer as other-parent
        // FIXME: need to be implemented
        {
            let mut local_core = core.write().unwrap();
            let creator = local_core.conf.read().unwrap().get_creator();
            let height = config
                .read()
                .unwrap()
                .peers
                .find_peer_mut(creator)
                .unwrap()
                .get_next_height();
            let (self_parent_event, other_parent_event) = {
                let store = local_core.store.read().unwrap();
                (
                    store
                        .get_event_of_creator(
                            peer.id,
                            config
                                .read()
                                .unwrap()
                                .peers
                                .find_peer_mut(peer.id)
                                .unwrap()
                                .get_height(),
                        )
                        .unwrap(),
                    store.get_event_of_creator(creator, height - 1).unwrap(),
                )
            };
            let self_parent = self_parent_event.hash;
            let other_parent = other_parent_event.hash;
            let lamport_timestamp = local_core.get_next_lamport_time();
            let transactions = local_core.next_transactions();
            let internal_transactions = local_core.next_internal_transactions();
            let event: Event<D, P, PK, Sig> = Event {
                creator,
                height,
                self_parent,
                other_parent,
                lamport_timestamp,
                transactions,
                internal_transactions,
                hash: EventHash::default(),
                signatures: HashMap::new(),
                frame_number,
                ft,
            };
        }

        // wait until hearbeat interval expires
        block_on(async {
            ticker.as_mut().await;
        });
    }
}

// Procedure B of DAG consensus
fn procedure_b<P, D, SK, PK, Sig>(core: Arc<RwLock<DAGcore<P, D, SK, PK, Sig>>>)
where
    D: DataType + 'static,
    P: PeerId + 'static,
    SK: SecretKey,
    PK: PublicKey + 'static,
    Sig: Signature + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    let (transport_type, request_bind_address) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.request_addr.clone())
    };
    let mut sync_req_receiver = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPreceiver::<P, SyncReq<P>, Error, DAGPeerList<P, PK>>::new(request_bind_address)
                    .unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    let mut sync_reply_sender = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPsender::<P, SyncReply<D, P, PK, Sig>, Error, DAGPeerList<P, PK>>::new().unwrap()
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
                    let reply = SyncReply::<D, P, PK, Sig> {
                        from: sync_req.to,
                        to: sync_req.from,
                        gossip_list,
                        lamport_time: { core.read().unwrap().get_lamport_time() },
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

impl<P, D, SK, PK, Sig> Consensus<'_, D> for DAG<P, D, SK, PK, Sig>
where
    P: PeerId + 'static,
    D: DataType + 'static,
    SK: SecretKey + 'static,
    PK: PublicKey + 'static,
    Sig: Signature + 'static,
{
    type Configuration = DAGconfig<P, D, SK, PK>;

    fn new(mut cfg: DAGconfig<P, D, SK, PK>) -> BaseResult<DAG<P, D, SK, PK, Sig>> {
        let (tx, rx) = mpsc::channel();
        //cfg.set_quit_rx(rx);
        let bind_addr = cfg.request_addr.clone();
        let reply_addr = cfg.reply_addr.clone();
        let transport_type = cfg.transport_type.clone();
        let mut sr_transport = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    TCPtransport::<P, SyncReq<P>, Error, DAGPeerList<P, PK>>::new(bind_addr)?
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };
        let mut sp_transport = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    TCPtransport::<P, SyncReply<D, P, PK, Sig>, Error, DAGPeerList<P, PK>>::new(
                        reply_addr,
                    )?
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };

        let core = Arc::new(RwLock::new(DAGcore::new(cfg)));

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
            core,
            quit_tx: tx,
            listener_handle: Some(handle),
            proc_a_handle: Some(proc_a_handle),
            proc_b_handle: Some(proc_b_handle),
        });
    }

    // Terminates procedures A and B of DAG0 started with run() method.
    fn shutdown(&mut self) -> BaseResult<()> {
        let _ = self.quit_tx.send(());
        Ok(())
    }

    fn send_transaction(&mut self, data: D) -> BaseResult<()> {
        let mut core = self.core.write().unwrap();
        core.add_transaction(data)
    }
}

impl<P, D, SK, PK, Sig> Drop for DAG<P, D, SK, PK, Sig>
where
    D: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
{
    fn drop(&mut self) {
        self.quit_tx.send(()).unwrap();
    }
}

impl<P, D, SK, PK, Sig> DAG<P, D, SK, PK, Sig>
where
    D: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
{
    // send internal transaction
    fn send_internal_transaction(&mut self, tx: InternalTransaction<P, PK>) -> Result<()> {
        let mut core = self.core.write().unwrap();
        core.add_internal_transaction(tx)
    }
}

impl<P, D, SK, PK, Sig> Unpin for DAG<P, D, SK, PK, Sig>
where
    D: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
{
}

impl<P, Data, SK, PK, Sig> Stream for DAG<P, Data, SK, PK, Sig>
where
    P: PeerId,
    Data: DataType,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod conf;
mod core;
mod errors;
mod event;
mod flag_table;
mod lamport_time;
mod peer;
mod store;
mod store_sled;
mod sync;
mod transactions;
