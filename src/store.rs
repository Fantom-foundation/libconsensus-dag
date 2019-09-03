// DAG Consensus Store trait

use crate::errors::Result;
use crate::event::Event;
use crate::event_hash::EventHash;
use crate::flag_table::FlagTable;
use libcommon_rs::peer::PeerId;

pub(crate) trait DAGstore<P>
where
    P: PeerId,
{
    // Create new stoirage for DAG Consensus
    fn new(path: &str) -> Result<Self>
    where
        Self: std::marker::Sized;

    // Writes Event into storage
    fn set_event(&mut self, e: Event<P>) -> Result<()>;

    // Read Event with EventHash
    fn get_event(&mut self, ex: &EventHash) -> Result<Event<P>>;

    // Writes FlagTable into storage for specifid EventHash
    fn set_flag_table(&mut self, ex: &EventHash, ft: FlagTable) -> Result<()>;

    // Read FlagTable with EventHash
    fn get_flag_table(&mut self, ex: &EventHash) -> Result<FlagTable>;
}
