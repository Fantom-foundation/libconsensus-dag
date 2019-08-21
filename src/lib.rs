#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[macro_use]
extern crate crossbeam_channel;
#[macro_use]
extern crate libconsensus;
use crate::conf::DAGconfig;
use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use crate::peer::Frame;
use crate::transactions::InternalTransaction;
use crossbeam_channel::tick;
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::Consensus;
use os_pipe::PipeWriter;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

// DAG node structure
pub struct DAG<T> {
    conf: DAGconfig<T>,
    tx_pool: Vec<T>,
    internal_tx_pool: Vec<InternalTransaction>,
    quit_tx: Sender<()>,
    lamport_time: LamportTime,
    current_frame: Frame,
    last_finalised_frame: Option<Frame>,
}

impl<D> Consensus for DAG<D>
where
    D: std::convert::AsRef<u8>,
{
    type Configuration = DAGconfig<D>;
    type Data = D;

    fn new(mut cfg: DAGconfig<D>) -> DAG<D> {
        let (tx, rx) = mpsc::channel();
        cfg.set_quit_rx(rx);
        return DAG {
            conf: cfg,
            tx_pool: Vec::with_capacity(1),
            internal_tx_pool: Vec::with_capacity(1),
            quit_tx: tx,
            lamport_time: 0,
            current_frame: 0,
            last_finalised_frame: None,
        };
    }

    // Basically run() method spawn Procedure B of DAG0 and execute loop of
    // procedure A of DAG0 until terminated with shutdown()
    fn run(&mut self) {
        // FIXME: need to be implemented!
        let ticker = tick(Duration::from_millis(self.conf.heartbeat));
        // DAG0 procedure A loop
        loop {
            // check if shutdown() has been called
            if self.conf.check_quit() {
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

    // Terminates procedures A and B of DAG0 started with run() method.
    fn shutdown(&mut self) {
        let _ = self.quit_tx.send(());
    }

    fn send_transaction(&mut self, data: Self::Data) -> bool {
        // Vec::push() panics when number of elements overflows `usize`
        if self.tx_pool.len() == std::usize::MAX {
            return false;
        }
        self.tx_pool.push(data);
        true
    }
}

impl<D> DAG<D>
where
    D: std::convert::AsRef<u8>,
{
    fn send_internal_transaction(&mut self, tx: InternalTransaction) -> Result<()> {
        if self.internal_tx_pool.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        self.internal_tx_pool.push(tx);
        Ok(())
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
mod transactions;
