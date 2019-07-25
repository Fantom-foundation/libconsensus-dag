// DAG Consensus Store trait

use crate::errors::Result;
use crate::event::Event;
use crate::event_hash::EventHash;
use crate::flag_table::FlagTable;

pub(crate) trait DAGstore {
    // Create new stoirage for DAG Consensus
    fn new(path: &str) -> Result<Self>
    where
        Self: std::marker::Sized;

    // Writes Event into storage
    fn set_event(&mut self, e: Event) -> Result<()>;

    // Read Event with EventHash
    fn get_event(&mut self, ex: &EventHash) -> Result<Event>;

    // Writes FlagTable into storage for specifid EventHash
    fn set_flag_table(&mut self, ex: &EventHash, ft: FlagTable) -> Result<()>;

    // Read FlagTable with EventHash
    fn get_flag_table(&mut self, ex: &EventHash) -> Result<FlagTable>;
}
