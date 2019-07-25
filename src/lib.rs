#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

extern crate libconsensus;
use crate::conf::DAGconfig;
use libconsensus::Consensus;
use os_pipe::PipeWriter;
use std::sync::mpsc::Sender;

pub struct DAG<T> {
    conf: DAGconfig,
    tx_pool: Vec<T>,
    callback_pool: Vec<fn(data: T) -> bool>,
    channel_pool: Vec<Sender<T>>,
    pipe_pool: Vec<PipeWriter>,
}

impl<D> Default for DAG<D> {
    fn default() -> DAG<D> {
        DAG {
            conf: DAGconfig::default(),
            tx_pool: Vec::with_capacity(1),
            callback_pool: Vec::with_capacity(1),
            channel_pool: Vec::with_capacity(1),
            pipe_pool: Vec::with_capacity(1),
        }
    }
}

impl<D> Consensus for DAG<D>
where
    D: std::convert::AsRef<u8>,
{
    type Configuration = DAGconfig;
    type Data = D;

    fn new(cfg: DAGconfig) -> DAG<D> {
        return DAG {
            conf: cfg,
            tx_pool: Vec::with_capacity(1),
            callback_pool: Vec::with_capacity(1),
            channel_pool: Vec::with_capacity(1),
            pipe_pool: Vec::with_capacity(1),
        };
    }

    // Basically run() method spawn Procedure B of DAG0 and execute loop of
    // procedure A of DAG0 until terminated with shutdown()
    fn run() {
        // FIXME: need to be implemented!
    }

    // Terminates procedures A and B of DAG0 started with run() method.
    fn shutdown() {
        // FIXME: need to be implemented!
    }

    fn send_transaction(&mut self, data: Self::Data) -> bool {
        // Vec::push() panics when number of elements overflows `usize`
        if self.tx_pool.len() == std::usize::MAX {
            return false;
        }
        self.tx_pool.push(data);
        true
    }

    fn register_callback(&mut self, callback: fn(data: Self::Data) -> bool) -> bool {
        // Vec::push() panics when number of elements overflows `usize`
        if self.callback_pool.len() == std::usize::MAX {
            return false;
        }
        self.callback_pool.push(callback);
        true
    }

    fn set_callback_timeout(&mut self, timeout: u64) {
        self.conf.callback_timeout = timeout;
    }

    fn register_channel(&mut self, sender: Sender<Self::Data>) -> bool {
        // Vec::push() panics when number of elements overflows `usize`
        if self.channel_pool.len() == std::usize::MAX {
            return false;
        }
        self.channel_pool.push(sender);
        true
    }

    fn register_os_pipe(&mut self, sender: PipeWriter) -> bool {
        // Vec::push() panics when number of elements overflows `usize`
        if self.pipe_pool.len() == std::usize::MAX {
            return false;
        }
        self.pipe_pool.push(sender);
        true
    }
}

pub fn test() {
    println!("Test!");
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
