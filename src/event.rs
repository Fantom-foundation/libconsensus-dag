use crate::event_hash::EventHash;
use crate::flag_table::FlagTable;
use crate::lamport_time::LamportTime;
use crate::peer::{Frame, PeerId};
use crate::transactions::InternalTransaction;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Event {
    pub(crate) creator: PeerId,
    height: u64,
    self_parent: EventHash,
    other_parent: EventHash,
    lamport_timestamp: LamportTime,
    transactions: Vec<Vec<u8>>,
    internal_transactions: Vec<InternalTransaction>,
    pub(crate) hash: EventHash,
    frame_number: Frame,
    ft: FlagTable,
}
