use libcommon_rs::data::DataType;
use libsignature::PublicKey;
use libsignature::Signature;
use log::{error, warn};

use crate::errors::Error;
use crate::peer::FrameNumber;
use crate::store::DAGstore;
use libcommon_rs::peer::PeerId;
use libhash_sha3::Hash as EventHash;
use std::collections::HashMap;

// FlagTable is a map from EventHash into Frame number
pub(crate) type FlagTable = HashMap<EventHash, FrameNumber>;
// CreatorFlagTable is a map from PeerId into Frame number (Frame)
pub(crate) type CreatorFlagTable<PeerId> = HashMap<PeerId, FrameNumber>;

// Strict flag table merging procedure takes two flag tables and the frame number
// and forms a new flag table which contains only those entries from any of source
// flag tables whose corresponding frame number is equal to the frame number specified.
pub(crate) fn strict_merge_flag_table(
    first: &FlagTable,
    second: &FlagTable,
    frame_number: FrameNumber,
) -> FlagTable {
    let mut result = FlagTable::new();
    for (key, value) in first.iter() {
        if *value == frame_number {
            result.insert(*key, *value);
        }
    }
    for (key, value) in second.iter() {
        if *value == frame_number {
            result.insert(*key, *value);
        }
    }
    result
}

// Open flag table merging procedure takes two flag tables and the frame number
// and forms a new flagtable which contains only those entries from any of source
// flag tables whose corresponding frame number is equal or greater to the frame
// number specified.
pub(crate) fn open_merge_flag_table(
    first: &FlagTable,
    second: &FlagTable,
    frame_number: FrameNumber,
) -> FlagTable {
    let mut result = FlagTable::new();
    for (key, value) in first.iter() {
        if *value >= frame_number {
            result.insert((*key).clone(), (*value).clone());
        }
    }
    for (key, value) in second.iter() {
        match first.get(key) {
            Some(number) => {
                if *number > *value {
                    result.insert((*key).clone(), (*value).clone());
                }
            }
            _ => {
                result.insert((*key).clone(), (*value).clone());
            }
        }
    }
    result
}

pub(crate) fn min_frame(ft: &FlagTable) -> FrameNumber {
    let mut res: Option<FrameNumber> = None;
    for (_, frame) in ft.iter() {
        match res {
            None => res = Some(*frame),
            Some(x) => {
                if x > *frame {
                    res = Some(*frame)
                }
            }
        }
    }
    match res {
        None => 0,
        Some(x) => x,
    }
}
