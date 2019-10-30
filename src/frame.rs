use crate::lamport_time::LamportTime;
use core::fmt::Display;
use core::fmt::Formatter;
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

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "[")?;
        for e in self.events.iter() {
            write!(f, "({};{})", e.hash, e.lamport_time)?;
        }
        write!(f, "[")
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
