// DAG Consensus Store trait

use crate::errors::{Error, Result};
use crate::event::Event;
use crate::event::NetEvent;
use crate::flag_table::{CreatorFlagTable, FlagTable};
use crate::frame::Frame;
use crate::peer::FrameNumber;
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
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
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
    fn set_flag_table(&mut self, ex: &EventHash, ft: &FlagTable) -> Result<()>;

    // Read FlagTable with EventHash
    fn get_flag_table(&self, ex: &EventHash) -> Result<FlagTable>;

    fn get_frame(&self, frame: FrameNumber) -> Result<Frame>;

    fn get_events_for_gossip(
        &self,
        gossip: &GossipList<P>,
    ) -> Result<Vec<NetEvent<Data, P, PK, Sig>>>;

    // This procedure takes a flag table as input
    // and produces a map which stores creator's hashes of visible roots;
    // for each root it stores minimal frame number.
    fn derive_creator_flag_table(&self, ft: &FlagTable) -> CreatorFlagTable<P> {
        let mut result = CreatorFlagTable::<P>::new();
        for (key, value) in ft.iter() {
            match self.get_event(key) {
                Err(e) => match e.downcast::<Error>() {
                    Ok(err) => {
                        if err == Error::NoneError {
                            warn!("Event {:?} not found", key)
                        } else {
                            error!(
                                "Error {:?} encountered while retrieving event {:?}",
                                err, key
                            )
                        }
                    }
                    Err(erx) => error!(
                        "Error {:?} encountered while retrieving event {:?}",
                        erx, key
                    ),
                },
                Ok(e) => match result.get(&e.creator) {
                    Some(frame) => {
                        if *frame > *value {
                            result.insert(e.creator, *value);
                        }
                    }
                    _ => {
                        result.insert(e.creator, *value);
                    }
                },
            };
        }
        result
    }
}
