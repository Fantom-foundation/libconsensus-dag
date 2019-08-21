// Config module

use libconsensus::errors::{Error, Error::AtMaxVecCapacity, Result};
use libconsensus::ConsensusConfiguration;
use os_pipe::PipeWriter;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

pub struct DAGconfig<D> {
    pub(crate) inner_port: u16,
    pub(crate) service_port: u16,
    pub(crate) callback_timeout: u64,
    // heartbeat duration in milliseconds
    pub(crate) heartbeat: u64,
    callback_pool: Vec<fn(data: D) -> bool>,
    channel_pool: Vec<Sender<D>>,
    pipe_pool: Vec<PipeWriter>,
    quit_rx: Option<Receiver<()>>,
}

impl<Data> DAGconfig<Data> {
    pub fn set_quit_rx(&mut self, rx: Receiver<()>) {
        self.quit_rx = Some(rx);
    }
    pub fn check_quit(&mut self) -> bool {
        match &self.quit_rx {
            None => return false,
            Some(ch) => match ch.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => return true,
                Err(TryRecvError::Empty) => return false,
            },
        }
    }
}

impl<Data> ConsensusConfiguration<Data> for DAGconfig<Data> {
    fn new() -> Self {
        return DAGconfig {
            inner_port: 9000,
            service_port: 12000,
            callback_timeout: 100,
            heartbeat: 1000,
            callback_pool: Vec::with_capacity(1),
            channel_pool: Vec::with_capacity(1),
            pipe_pool: Vec::with_capacity(1),
            quit_rx: None,
        };
    }

    fn register_channel(&mut self, sender: Sender<Data>) -> Result<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.channel_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity);
        }
        self.channel_pool.push(sender);
        Ok(())
    }

    fn register_os_pipe(&mut self, sender: PipeWriter) -> Result<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.pipe_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity);
        }
        self.pipe_pool.push(sender);
        Ok(())
    }

    fn register_callback(&mut self, callback: fn(data: Data) -> bool) -> Result<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.callback_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity);
        }
        self.callback_pool.push(callback);
        Ok(())
    }

    fn set_callback_timeout(&mut self, timeout: u64) {
        self.callback_timeout = timeout;
    }
}
