// DAG Consensus Store trait

use crate::errors::Result;
use crate::event::Event;
use crate::event_hash::EventHash;
use crate::flag_table::FlagTable;
use crate::peer::GossipList;
use crate::peer::Height;
use libcommon_rs::peer::PeerId;

#[derive(Clone)]
pub enum StoreType {
    Unknown,
    Sled,
}

pub(crate) trait DAGstore<P>: Send + Sync
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

    // Read Event with Creator and Height
    fn get_event_of_creator(&self, creator: P, height: Height) -> Result<Event<P>>;

    // Writes FlagTable into storage for specifid EventHash
    fn set_flag_table(&mut self, ex: &EventHash, ft: FlagTable) -> Result<()>;

    // Read FlagTable with EventHash
    fn get_flag_table(&mut self, ex: &EventHash) -> Result<FlagTable>;

    fn get_events_for_gossip(&self, gossip: &GossipList<P>) -> Result<Vec<Event<P>>>;
}
