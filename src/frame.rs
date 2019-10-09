use crate::lamport_time::LamportTime;
use libhash_sha3::Hash as EventHash;
use serde::{Deserialize, Serialize};

// A frame record for a single event; must contains all fields used in
// final ordering calculation
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct FrameRecord {
    pub(crate) hash: EventHash,
    pub(crate) lamport_time: LamportTime,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Frame {
    pub(crate) events: Vec<FrameRecord>,
}

impl Default for Frame {
    fn default() -> Frame {
        Frame {
            events: Vec::with_capacity(1),
        }
    }
}
