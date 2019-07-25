// DAG Consensus Store trait

use crate::errors::Result;
use crate::event::Event;
use crate::event_hash::EventHash;

pub(crate) trait DAGstore {
    // Create new stoirage for DAG Consensus
    fn new(path: &str) -> Result<Self>
    where
        Self: std::marker::Sized;

    // Writes Event into storage; returns True on success and
    // False on failure
    fn set_event(&mut self, e: Event) -> Result<()>;

    // Read Event with EventHash
    fn get_event(&mut self, ex: &EventHash) -> Result<Event>;
}
