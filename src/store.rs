// DAG Consensus Store trait

use crate::errors::Result;
use crate::event::Event;
use crate::event::NetEvent;
use crate::flag_table::FlagTable;
use crate::peer::GossipList;
use crate::peer::Height;
use libcommon_rs::peer::PeerId;
use libhash_sha3::Hash as EventHash;
use libsignature::PublicKey;
use libsignature::Signature;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub enum StoreType {
    Unknown,
    Sled,
}

pub(crate) trait DAGstore<Data, P, PK, Sig>: Send + Sync
where
    Data: Serialize + DeserializeOwned + Send + Clone,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature,
{
    // Create new stoirage for DAG Consensus
    fn new(path: &str) -> Result<Self>
    where
        Self: std::marker::Sized;

    // Writes Event into storage
    fn set_event(&mut self, e: Event<Data, P, PK, Sig>) -> Result<()>;

    // Read Event with EventHash
    fn get_event(&self, ex: &EventHash) -> Result<Event<Data, P, PK, Sig>>;

    // Read Event with Creator and Height
    fn get_event_of_creator(&self, creator: P, height: Height) -> Result<Event<Data, P, PK, Sig>>;

    // Writes FlagTable into storage for specifid EventHash
    fn set_flag_table(&mut self, ex: &EventHash, ft: FlagTable) -> Result<()>;

    // Read FlagTable with EventHash
    fn get_flag_table(&self, ex: &EventHash) -> Result<FlagTable>;

    fn get_events_for_gossip(
        &self,
        gossip: &GossipList<P>,
    ) -> Result<Vec<NetEvent<Data, P, PK, Sig>>>;
}
