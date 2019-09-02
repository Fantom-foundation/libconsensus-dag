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
use crate::sync::SyncReq;
use crate::transactions::InternalTransaction;
use crossbeam_channel::tick;
use futures::stream::Stream;
use futures::task::Context;
use futures::task::Poll;
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::errors::Result as BaseResult;
use libconsensus::{Consensus, PeerId};
use libtransport::Transport;
use libtransport_tcp::TCPtransport;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::pin::Pin;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

// DAG node structure
pub struct DAG<'a, T> {
    conf: Arc<Mutex<DAGconfig<T>>>,
    listener_handle: Option<JoinHandle<()>>,
    procA_handle: Option<JoinHandle<()>>,
    procB_handle: Option<JoinHandle<()>>,
    tx_pool: Vec<T>,
    internal_tx_pool: Vec<InternalTransaction>,
    quit_tx: Sender<()>,
    lamport_time: LamportTime,
    current_frame: Frame,
    last_finalised_frame: Option<Frame>,
    sync_request_transport: &'a (dyn Transport<PeerId, SyncReq, Error, DAGPeerList> + 'a),
}

fn listener<Data: 'static>(cfg_mutexed: Arc<Mutex<DAGconfig<Data>>>)
where
    Data: Serialize + DeserializeOwned + Send + Clone,
{
    // FIXME: what we do with unwrap() in threads?
    let config = Arc::clone(&cfg_mutexed);
    loop {
        // check if quit channel got message
        let mut cfg = config.lock().unwrap();
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

fn procedureA<D>(config: Arc<Mutex<DAGconfig<D>>>) {}

fn procedureB<D>(config: Arc<Mutex<DAGconfig<D>>>) {}

impl<D> Consensus<D> for DAG<'_, D>
where
    D: Serialize + DeserializeOwned + Send + Clone + 'static,
    //libtransport_tcp::TCPtransport<sync::SyncReq>: libtransport::Transport<
    //    std::vec::Vec<u8>,
    //    sync::SyncReq,
    //    errors::Error,
    //    peer::DAGPeerList,
    //    dyn TransportConfiguration<SyncReq>,
    //>,
{
    type Configuration = DAGconfig<D>;
    //    type Data = D;

    fn new(mut cfg: DAGconfig<D>) -> BaseResult<DAG<'static, D>> {
        let (tx, rx) = mpsc::channel();
        cfg.set_quit_rx(rx);
        let cfg_mutexed = Arc::new(Mutex::new(cfg));
        let config = Arc::clone(&cfg_mutexed);
        let handle = thread::spawn(|| listener(config));
        let configA = Arc::clone(&cfg_mutexed);
        let procA_handle = thread::spawn(|| procedureA(configA));
        let configB = Arc::clone(&cfg_mutexed);
        let procB_handle = thread::spawn(|| procedureB(configB));
        let mut sr_transport = {
            match cfg.transport_type {
                TCP => {
                    //                    let mut tcp_cfg =
                    //                        TCPtransportCfg::<SyncReq>::new(cfg.request_addr.clone()).unwrap();
                    TCPtransport::<SyncReq>::new(cfg.request_addr.clone())?
                }
                Unknown => panic!("unknown transport"),
            }
        };
        return Ok(DAG {
            conf: cfg_mutexed,
            tx_pool: Vec::with_capacity(1),
            internal_tx_pool: Vec::with_capacity(1),
            quit_tx: tx,
            lamport_time: 0,
            current_frame: 0,
            last_finalised_frame: None,
            listener_handle: Some(handle),
            sync_request_transport: &sr_transport,
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
        // Vec::push() panics when number of elements overflows `usize`
        if self.tx_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity);
        }
        self.tx_pool.push(data);
        Ok(())
    }
}

impl<D> Drop for DAG<'_, D> {
    fn drop(&mut self) {
        self.quit_tx.send(()).unwrap();
    }
}

impl<D> DAG<'_, D> {
    // Basically run() method spawn Procedure B of DAG0 and execute loop of
    // procedure A of DAG0 until terminated with shutdown()
    fn run(&mut self) {
        // FIXME: need to be implemented!
        let config = Arc::clone(&self.conf);
        let ticker = {
            let mut cfg = config.lock().unwrap();
            tick(Duration::from_millis(cfg.heartbeat))
        };
        // DAG0 procedure A loop
        loop {
            // check if shutdown() has been called
            let mut cfg = config.lock().unwrap();
            if cfg.check_quit() {
                // terminating
                // FIXME: need to be implemented
                break;
            }

            // wait until hearbeat interval expires
            select! {
                recv(ticker) -> _ => {},
            }
        }
    }

    // send internal transaction
    fn send_internal_transaction(&mut self, tx: InternalTransaction) -> Result<()> {
        if self.internal_tx_pool.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        self.internal_tx_pool.push(tx);
        Ok(())
    }
}

impl<D> Unpin for DAG<'_, D> {}

impl<Data> Stream for DAG<'_, Data>
where
    Data: Serialize + DeserializeOwned,
{
    type Item = Data;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let myself = Pin::get_mut(self);
        let config = Arc::clone(&myself.conf);
        let mut cfg = config.lock().unwrap();
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
