#![feature(try_trait)]
#![recursion_limit = "1024000"]
#![allow(clippy::type_complexity)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[macro_use]
extern crate failure;
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate libconsensus;
extern crate syslog;
pub use crate::conf::DAGconfig;
use crate::core::DAGcore;
use crate::errors::Error;
// reserved for DAG1
//use crate::errors::{Result};
use crate::event::Event;
pub use crate::peer::DAGPeer;
pub use crate::peer::DAGPeerList;
use crate::peer::FrameNumber;
use crate::peer::GossipList;
use crate::sync::{SyncReply, SyncReq};
// reserved for DAG1
//use crate::transactions::InternalTransaction;
use futures::executor::block_on;
use futures::stream::Stream;
use futures::stream::StreamExt;
use futures::task::Context;
use futures::task::Poll;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::Peer;
use libcommon_rs::peer::PeerId;
use libconsensus::errors::Result as BaseResult;
use libconsensus::Consensus;
use libhash_sha3::Hash as EventHash;
use libsignature::Signature;
use libsignature::{PublicKey, SecretKey};
use libtransport::TransportReceiver;
use libtransport::TransportSender;
use libtransport_tcp::receiver::TCPreceiver;
use libtransport_tcp::sender::TCPsender;
use log::error;
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
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK>,
{
    core: Arc<RwLock<DAGcore<P, T, SK, PK, Sig>>>,
    listener_handle: Option<JoinHandle<()>>,
    proc_a_handle: Option<JoinHandle<()>>,
    proc_b_handle: Option<JoinHandle<()>>,

    quit_txs: Vec<Sender<()>>,
}

fn listener<P, Data, SK, PK, Sig>(
    core: Arc<RwLock<DAGcore<P, Data, SK, PK, Sig>>>,
    quit_rx: Receiver<()>,
    sync_reply_receiver: &mut dyn TransportReceiver<
        P,
        SyncReply<Data, P, PK, Sig>,
        Error,
        DAGPeerList<P, PK>,
    >,
) where
    Data: DataType + 'static,
    P: PeerId + 'static,
    SK: SecretKey,
    PK: PublicKey + 'static,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK> + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    // FIXME: what we do with unwrap() in threads?

    let me = { core.read().unwrap().me_a() };

    debug!("l {}: listener started", me.clone());

    loop {
        // check if quit channel got message
        //debug!("l {}: listener loop start", me.clone());
        match quit_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                core.write().unwrap().set_shutdown(true);
                break;
            }
            Err(TryRecvError::Empty) => {}
        }

        // Receive Sync Reply and process it.
        // NB: it may not be from the very same peer we have sent Sync Request above.
        block_on(async {
            if let Some(sync_reply) = sync_reply_receiver.next().await {
                debug!(
                    "l {} Sync Reply from {}",
                    me.clone(),
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
                        let event: Event<Data, P, PK, Sig> = ev.into();
                        // check if event is valid
                        if !{ core.read().unwrap().check_event(&event).unwrap() } {
                            error!("l {}: Event {} is not valid", me.clone(), event);
                            continue;
                        }
                        let lamport_time = event.get_lamport_time();
                        let height = event.get_height();
                        let creator = event.get_creator();
                        debug!("l {}: insert event: {}", me.clone(), event.clone());
                        // insert event into node DB
                        {
                            core.write().unwrap().insert_event(event).unwrap();
                        }
                        debug!(
                            "l {}: updating lamport time {} and height {}",
                            me.clone(),
                            lamport_time.clone(),
                            height.clone()
                        );
                        // update lamport time and height of the event creator's peer
                        config
                            .write()
                            .unwrap()
                            .peers
                            .find_peer_mut(&creator)
                            .unwrap()
                            .update_lamport_time_and_height(lamport_time, height);
                    }
                }
                debug!(
                    "l {} Sync Reply from {} done",
                    me.clone(),
                    sync_reply.from.clone()
                );
            }
        });
        // allow to pool again if waker is set
        //if let Some(waker) = config.write().unwrap().waker.take() {
        //    waker.wake()
        //}
        //debug!("{}: listener loop end", me.clone());
    }
}

// Procedure A of DAG consensus
fn procedure_a<P, D, SK, PK, Sig>(core: Arc<RwLock<DAGcore<P, D, SK, PK, Sig>>>)
where
    D: DataType + 'static,
    P: PeerId + 'static,
    SK: SecretKey,
    PK: PublicKey + 'static,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK> + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    let store = { core.read().unwrap().store.clone() };
    let creator = { config.read().unwrap().get_creator() };
    let mut ticker = {
        let cfg = config.read().unwrap();
        thread::sleep(Duration::from_millis(cfg.get_proc_a_delay()));
        async_timer::Interval::platform_new(Duration::from_millis(cfg.heartbeat))
    };
    let (transport_type, reply_bind_address) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.reply_addr.clone())
    };
    let me = { core.read().unwrap().me_a() };
    debug!("procedure_a, reply_bind_addr: {}", reply_bind_address);
    // setup TransportSender for Sync Request.
    let mut sync_req_sender = {
        match transport_type {
            libtransport::TransportType::TCP => {
                TCPsender::<P, SyncReq<P>, errors::Error, peer::DAGPeerList<P, PK>>::new().unwrap()
            }
            libtransport::TransportType::Unknown => panic!("unknown transport"),
        }
    };
    // DAG procedure A loop
    loop {
        debug!("{}: proc_a loop", me.clone());
        // check if shutdown() has been called
        debug!("{} checking quit condition", me.clone());
        let quit = { core.read().unwrap().check_quit() };
        if quit {
            debug!("{}: terminating proc_a", me);
            // terminating
            // FIXME: need to be implemented
            break;
        }
        debug!("{} locking cfg", me.clone());
        let mut cfg = config.write().unwrap();
        // choose the next peer and send Sync Request to it.
        debug!("{} getting next peer", me.clone());
        let peer = cfg.peers.next_peer();
        debug!("{} got next peer: {}", me.clone(), peer.clone());
        let gossip_list: GossipList<P> = cfg.peers.get_gossip_list();
        drop(cfg);
        debug!("{} got gossip list", me.clone());
        let request = SyncReq {
            from: creator.clone(),
            to: peer.id.clone(),
            gossip_list,
            lamport_time: { core.read().unwrap().get_lamport_time() },
        };
        debug!(
            "{}: sending SyncReq to {} ==> {}",
            me.clone(),
            peer.request_addr.clone(),
            request.clone()
        );
        match sync_req_sender.send(peer.request_addr.clone(), request) {
            Ok(()) => {}
            Err(e) => error!(
                "error sending sync request to {}: {:?}",
                peer.request_addr, e
            ),
        }
        debug!("{}: SyncReq sent", me.clone());

        // Sync Reply receiver was here

        // create new event if needed referring remote peer as other-parent
        debug!("{}: create new event", me);
        let height = {
            config
                .write()
                .unwrap()
                .peers
                .find_peer_mut(&creator)
                .unwrap()
                .get_next_height()
        };
        let other_height = {
            config
                .read()
                .unwrap()
                .peers
                .find_peer(&peer.id)
                .unwrap()
                .get_height()
        };
        debug!(
            "{}: heights; self[{}]: {}; other[{}]: {}",
            me.clone(),
            creator.clone(),
            height,
            peer.id.clone(),
            other_height,
        );
        let (other_parent_event, self_parent_event) = {
            let store_local = store.read().unwrap();
            (
                store_local
                    .get_event_of_creator(peer.id.clone(), other_height)
                    .unwrap(),
                store_local
                    .get_event_of_creator(creator.clone(), height - 1)
                    .unwrap(),
            )
        };
        debug!("{}: parent events read", me.clone());
        let self_parent = self_parent_event.hash;
        let other_parent = other_parent_event.hash;
        let (lamport_timestamp, transactions, internal_transactions) = {
            let mut local_core = core.write().unwrap();
            (
                local_core.get_next_lamport_time(),
                local_core.next_transactions(),
                local_core.next_internal_transactions(),
            )
        };
        let mut event: Event<D, P, PK, Sig> = Event::new(
            creator.clone(),
            height,
            self_parent,
            other_parent,
            lamport_timestamp,
            transactions,
            internal_transactions,
        );
        debug!("{}: event formed: {}", me.clone(), event.clone());
        let ex = event.event_hash().unwrap();
        let rc = { core.write().unwrap().insert_event(event).unwrap() };
        if !rc {
            error!("Error inserting new event {:?}", ex);
        }

        // wait until heartbeat interval expires
        debug!("{}: wait heartbeat expires", me.clone());
        block_on(async {
            ticker.as_mut().await;
        });
        debug!("{}: heartbeat finished", me.clone());
    }
}

// Procedure B of DAG consensus
fn procedure_b<P, D, SK, PK, Sig>(
    core: Arc<RwLock<DAGcore<P, D, SK, PK, Sig>>>,
    sync_req_receiver: &mut dyn TransportReceiver<P, SyncReq<P>, Error, DAGPeerList<P, PK>>,
) where
    D: DataType + 'static,
    P: PeerId + 'static,
    SK: SecretKey,
    PK: PublicKey + 'static,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK> + 'static,
{
    let config = { core.read().unwrap().conf.clone() };
    let (transport_type, request_bind_address) = {
        let cfg = config.read().unwrap();
        (cfg.transport_type.clone(), cfg.request_addr.clone())
    };
    let me = { core.read().unwrap().me_b() };
    debug!("procedure_b, request_bind_addr: {}", request_bind_address);
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
        debug!("{}: waiting for Sync request", me.clone());
        while let Some(sync_req) = sync_req_receiver.next().await {
            debug!(
                "{} Sync request from {} <== {}",
                me.clone(),
                {
                    config
                        .read()
                        .unwrap()
                        .peers
                        .find_peer(&sync_req.from)
                        .unwrap()
                        .get_base_addr()
                },
                sync_req.clone()
            );
            {
                core.write()
                    .unwrap()
                    .update_lamport_time(sync_req.lamport_time);
            }
            debug!("{}: lamport time update: {}", me.clone(), {
                core.read().unwrap().get_lamport_time()
            });
            let events_for_gossip = {
                store
                    .read()
                    .unwrap()
                    .get_events_for_gossip(&sync_req.gossip_list)
            };
            match events_for_gossip {
                Err(e) => error!("Procedure B: get_events_for_gossip() error: {:?}", e),
                Ok(events) => {
                    debug!("{}: got events for gossip", me.clone());
                    let gossip_list: GossipList<P> =
                        { config.read().unwrap().peers.get_gossip_list() };
                    debug!("{}: got gossip list", me.clone());
                    let reply = SyncReply::<D, P, PK, Sig> {
                        from: sync_req.to,
                        to: sync_req.from,
                        gossip_list,
                        lamport_time: { core.read().unwrap().get_lamport_time() },
                        events,
                    };
                    debug!("{}: SyncReply formed: {}", me.clone(), reply.clone());
                    match {
                        config
                            .write()
                            .unwrap()
                            .peers
                            .find_peer_with_lamport_time_update(&reply.to, sync_req.lamport_time)
                    } {
                        Ok(peer) => {
                            let address = peer.reply_addr.clone();
                            debug!("{}: sending SyncReply to {}", me.clone(), address.clone());
                            let res = sync_reply_sender.send(address, reply);
                            match res {
                                Ok(()) => {}
                                Err(e) => error!("error sending sync reply: {:?}", e),
                            }
                            debug!("{}: SyncReply sent", me.clone());
                        }
                        Err(e) => error!("peer {} find error: {:?}", reply.to, e),
                    }
                }
            }
        }
        debug!("{}: exit proc_b loop!", me.clone());
    });
}

impl<P, D, SK, PK, Sig> Consensus<'_, D, P> for DAG<P, D, SK, PK, Sig>
where
    P: PeerId + 'static,
    D: DataType + 'static,
    SK: SecretKey + 'static,
    PK: PublicKey + 'static,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK> + 'static,
{
    type Configuration = DAGconfig<P, D, SK, PK>;

    fn new(cfg: DAGconfig<P, D, SK, PK>) -> BaseResult<DAG<P, D, SK, PK, Sig>> {
        let (tx, rx) = mpsc::channel();

        let (transport_type, reply_bind_address, request_bind_address) = (
            cfg.transport_type.clone(),
            cfg.reply_addr.clone(),
            cfg.request_addr.clone(),
        );
        let mut sync_reply_receiver = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    let x: TCPreceiver<P, SyncReply<D, P, PK, Sig>, Error, DAGPeerList<P, PK>> =
                        TCPreceiver::new(reply_bind_address).unwrap();
                    x
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };
        let srr_tx = sync_reply_receiver.get_quit_tx();

        let mut sync_req_receiver = {
            match transport_type {
                libtransport::TransportType::TCP => {
                    TCPreceiver::<P, SyncReq<P>, Error, DAGPeerList<P, PK>>::new(
                        request_bind_address,
                    )
                    .unwrap()
                }
                libtransport::TransportType::Unknown => panic!("unknown transport"),
            }
        };
        let syr_tx = sync_req_receiver.get_quit_tx();

        let core = Arc::new(RwLock::new(DAGcore::new(cfg)));

        let handle = {
            let listener_core = core.clone();
            thread::Builder::new()
                .name("listener".to_string())
                .stack_size(1024 * 1024)
                .spawn(move || listener(listener_core, rx, &mut sync_reply_receiver))?
        };
        //        let configA = Arc::clone(&cfg_mutexed);
        let core_a = core.clone();
        let proc_a_handle = thread::Builder::new()
            .name("procedure_a".to_string())
            .stack_size(4 * 1024 * 1024)
            .spawn(move || procedure_a(core_a))?;
        //        let configB = Arc::clone(&cfg_mutexed);
        let core_b = core.clone();
        let proc_b_handle = thread::Builder::new()
            .name("procedure_b".to_string())
            .stack_size(4 * 1024 * 1024 * 1024)
            .spawn(move || procedure_b(core_b, &mut sync_req_receiver))?;
        let mut dag = DAG {
            core,
            listener_handle: Some(handle),
            proc_a_handle: Some(proc_a_handle),
            proc_b_handle: Some(proc_b_handle),
            quit_txs: Vec::with_capacity(3),
        };
        dag.set_quit_tx(tx);
        match srr_tx {
            None => {}
            Some(x) => dag.set_quit_tx(x),
        };
        match syr_tx {
            None => {}
            Some(x) => dag.set_quit_tx(x),
        };
        Ok(dag)
    }

    // Terminates procedures A and B of DAG0 started with run() method.
    fn shutdown(&mut self) -> BaseResult<()> {
        for tx in self.quit_txs.iter() {
            let _ = tx.send(());
        }
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
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK>,
{
    fn drop(&mut self) {
        let me = self.core.read().unwrap().me_a();
        for tx in self.quit_txs.iter() {
            let _ = tx.send(());
        }
        debug!("d {}: shutting down listener", me.clone());
        if let Some(listener_handle) = self.listener_handle.take() {
            listener_handle
                .join()
                .expect("Couldn't join on the listener thread.");
        }
        debug!("d {}: shutting down procedure A", me.clone());
        if let Some(proc_a_handle) = self.proc_a_handle.take() {
            proc_a_handle
                .join()
                .expect("Couldn't join on the procedure A thread.");
        }
        if let Some(waker) = self
            .core
            .write()
            .unwrap()
            .conf
            .write()
            .unwrap()
            .waker
            .take()
        {
            debug!("d {}: calling waker", me.clone());
            waker.wake();
        }
        debug!("d {}: shutting down procedure B", me);
        if let Some(proc_b_handle) = self.proc_b_handle.take() {
            proc_b_handle
                .join()
                .expect("Couldn't join on the procedure B thread.");
        }
    }
}

impl<P, D, SK, PK, Sig> DAG<P, D, SK, PK, Sig>
where
    D: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK>,
{
    // FIXME: reserved for DAG1
    /// Sends internal transaction
    //    fn send_internal_transaction(&mut self, tx: InternalTransaction<P, PK>) -> Result<()> {
    //        let mut core = self.core.write().unwrap();
    //        core.add_internal_transaction(tx)
    //    }
    pub(crate) fn set_quit_tx(&mut self, tx: Sender<()>) {
        self.quit_txs.push(tx);
    }
}

impl<P, D, SK, PK, Sig> Unpin for DAG<P, D, SK, PK, Sig>
where
    D: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK>,
{
}

impl<P, Data, SK, PK, Sig> Stream for DAG<P, Data, SK, PK, Sig>
where
    P: PeerId,
    Data: DataType,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK, SecretKey = SK>,
{
    type Item = (Data, P);
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let myself = Pin::get_mut(self);
        let me = {
            let core = myself.core.read().unwrap();
            let me = core.me_a();
            if core.check_quit() {
                debug!("o {}: terminating stream", me);
                return Poll::Ready(None);
            }
            me
        };
        let mut data: Option<Self::Item> = None;
        debug!("o {}: check last finalised frame", me.clone());
        let last_finalised_frame: FrameNumber = {
            let core = myself.core.read().unwrap();
            match core.last_finalised_frame {
                None => {
                    core.conf.write().unwrap().waker = Some(cx.waker().clone());
                    debug!("o {}: poll pending", me);
                    return Poll::Pending;
                }
                Some(x) => x,
            }
        };

        loop {
            let mut core = myself.core.write().unwrap();
            debug!("o {}: check current frame", me.clone());
            let mut current_frame: FrameNumber = match core.current_frame {
                None => 0,
                Some(x) => x,
            };
            debug!(
                "o {}: current_frame:{}; last_finalised_frame:{}",
                me.clone(),
                current_frame,
                last_finalised_frame
            );
            let mut current_event = match core.current_event {
                None => {
                    if current_frame >= last_finalised_frame {
                        core.conf.write().unwrap().waker = Some(cx.waker().clone());
                        debug!("o {}: no more finalised frames yet", me);
                        return Poll::Pending;
                    }
                    current_frame += 1;
                    0
                }
                Some(x) => x,
            };
            let mut current_tx = match core.current_tx {
                None => 0,
                Some(x) => x,
            };

            let frame = { core.store.read().unwrap().get_frame(current_frame).unwrap() };
            let n_events = frame.events.len();

            let event_record = frame.events[current_event];
            let mut event = {
                core.store
                    .read()
                    .unwrap()
                    .get_event(&event_record.hash)
                    .unwrap()
            };
            debug!("o {}: current event: {}", me.clone(), event.clone());

            let n_tx = event.transactions.len();
            debug!("o {}: n_tx:{}", me.clone(), n_tx);
            if n_tx > 0 {
                data = Some((event.transactions.swap_remove(current_tx), event.creator));
            } else {
                debug!("o {}: event with no txs", me);
            }
            current_tx += 1;
            if current_tx < n_tx {
                core.current_tx = Some(current_tx);
            } else {
                core.current_tx = Some(0);
                current_event += 1;
                if current_event < n_events {
                    core.current_event = Some(current_event);
                } else {
                    core.current_event = None;
                    core.current_frame = Some(current_frame + 1);
                }
            }
            if data != None {
                break;
            }
        }

        myself.core.read().unwrap().conf.write().unwrap().waker = Some(cx.waker().clone());
        debug!("o {}: delivering data: {:#?}", me, data.clone());
        Poll::Ready(data)
    }
}

mod conf;
mod core;
mod errors;
mod event;
mod flag_table;
mod frame;
mod lamport_time;
mod peer;
mod store;
mod store_sled;
mod sync;
mod transactions;

#[cfg(test)]
mod tests {
    use core::fmt::{Display, Formatter};

    use futures::executor::block_on;
    use futures::StreamExt;
    use libcommon_rs::peer::Peer;
    use libcommon_rs::peer::PeerList;
    use libhash_sha3::Hash as EventHash;
    use libsignature::Signature as LibSignature;
    use libsignature_ed25519_dalek::{PublicKey, SecretKey, Signature};
    use serde::{Deserialize, Serialize};

    use crate::conf::DAGconfig;
    use crate::libconsensus::Consensus;
    use crate::libconsensus::ConsensusConfiguration;
    pub use crate::peer::DAGPeer;
    pub use crate::peer::DAGPeerList;
    use crate::DAG;

    type Id = PublicKey;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Hash, Copy)]
    struct Data {
        byte: i8,
    }

    impl Display for Data {
        fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
            let mut formatted = String::new();
            formatted.push_str(&self.byte.to_string());
            write!(f, "{}", formatted)
        }
    }

    impl From<usize> for Data {
        fn from(i: usize) -> Data {
            Data { byte: i as i8 }
        }
    }

    #[test]
    fn test_vectorised_network() {
        env_logger::init();
        //        syslog::init(
        //            syslog::Facility::LOG_USER,
        //            log::LevelFilter::Debug,
        //            Some("test"),
        //        )
        //        .unwrap();
        const N: usize = 5;
        const BASE_PORT: usize = 10001;
        let mut kp: Vec<(PublicKey, SecretKey)> = Vec::with_capacity(N);
        let mut peer_list = DAGPeerList::<Id, PublicKey>::default();
        let mut dag: Vec<DAG<Id, Data, SecretKey, PublicKey, Signature<EventHash>>> =
            Vec::with_capacity(N);
        let mut data: Vec<Data> = Vec::with_capacity(N);

        for i in 0..N {
            kp.push(Signature::<EventHash>::generate_key_pair().unwrap());
            let mut peer = DAGPeer::<Id, PublicKey>::new(
                kp[i].0.clone(),
                format!("127.0.0.1:{}", BASE_PORT + 2 * i).to_string(),
            );
            peer.set_public_key(kp[i].0.clone());
            peer_list.add(peer).unwrap();
            data.push(i.into());
        }
        for i in 0..N {
            let mut consensus_config = DAGconfig::<Id, Data, SecretKey, PublicKey>::new();
            consensus_config.request_addr = format!("127.0.0.1:{}", BASE_PORT + 2 * i).to_string();
            consensus_config.reply_addr =
                format!("127.0.0.1:{}", BASE_PORT + 2 * i + 1).to_string();
            consensus_config.transport_type = libtransport::TransportType::TCP;
            consensus_config.store_type = libcommon_rs::store::StoreType::Sled;
            consensus_config.creator = kp[i].0.clone();
            consensus_config.public_key = kp[i].0.clone();
            consensus_config.secret_key = kp[i].1.clone();
            consensus_config.peers = peer_list.clone();
            dag.push(
                DAG::<Id, Data, SecretKey, PublicKey, Signature<EventHash>>::new(consensus_config)
                    .unwrap(),
            );
        }

        for i in 0..N {
            dag[i].send_transaction(data[i].clone()).unwrap();
            println!("d{} transaction sent", data[i]);
        }

        let mut res1: Vec<Data> = vec![Data { byte: 0 }; N];

        block_on(async {
            for i in 0..N {
                match i {
                    0 => {
                        for i in 0..N {
                            match dag[0].next().await {
                                Some((d, _p)) => {
                                    println!("DAG1: data[{}] OK", i);
                                    res1[i] = d;
                                }
                                None => panic!("unexpected None"),
                            }
                        }
                    }
                    x => {
                        for i in 0..N {
                            // check DAG2
                            match dag[x].next().await {
                                Some((d, _p)) => assert_eq!(d, res1[i]),
                                None => panic!("unexpected None in dags[{}]", i),
                            };
                        }
                    }
                }
            }
        });

        println!("Result: {:?}", res1);

        println!("Shutting down DAGs");

        for i in 0..N {
            dag[i].shutdown().unwrap();
        }
    }
}
