use crate::lamport_time::LamportTime;
use libhash_sha3::Hash as EventHash;
use serde::{Deserialize, Serialize};

// A frame record for a single event; must contains all fields used in
// final ordering calculation
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

impl Frame {
    pub(crate) fn finalise(&mut self) {
        self.events.sort_by(|a, b| {
            use std::cmp::Ordering;
            if a.lamport_time < b.lamport_time {
                Ordering::Less
            } else if a.lamport_time > b.lamport_time {
                Ordering::Greater
            } else if a.hash < b.hash {
                Ordering::Less
            } else if a.hash > b.hash {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
    }
}
