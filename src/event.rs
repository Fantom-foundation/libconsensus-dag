use crate::event_hash::EventHash;
use crate::flag_table::FlagTable;
use crate::lamport_time::LamportTime;
use crate::peer::Frame;
use crate::transactions::InternalTransaction;
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Event<P> {
    pub(crate) creator: P,
    height: u64,
    self_parent: EventHash,
    other_parent: EventHash,
    lamport_timestamp: LamportTime,
    transactions: Vec<Vec<u8>>,
    internal_transactions: Vec<InternalTransaction<P>>,
    pub(crate) hash: EventHash,
    frame_number: Frame,
    ft: FlagTable,
}

impl<P> Stub for Event<P> where P: PeerId {}
