use log::{error, warn};

use crate::errors::Error;
use crate::event_hash::EventHash;
use crate::peer::Frame;
use crate::store::DAGstore;
use libconsensus::PeerId;
use std::collections::HashMap;

// FlagTable is a map from EventHash into Frame number
pub(crate) type FlagTable = HashMap<EventHash, Frame>;
// CreatorFlagTable is a map from PeerId into Frame number (Frame)
pub(crate) type CreatorFlagTable = HashMap<PeerId, Frame>;

// Strict flag table merging procedure takes two flag tables and the frame number
// and forms a new flag table which contains only those entries from any of source
// flag tables whose corresponding frame number is equal to the frame number specified.
fn strict_merge_flag_table(
    first: &FlagTable,
    second: &FlagTable,
    frame_number: Frame,
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
fn open_merge_flag_table(first: &FlagTable, second: &FlagTable, frame_number: Frame) -> FlagTable {
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

// This procedure takes a store and a flag table as input
// and produces a map which stores creator's hashes of visible roots;
// for each root it stores minimal frame number.
fn derive_creator_table<S: DAGstore>(store: &mut S, ft: &FlagTable) -> CreatorFlagTable {
    let mut result = CreatorFlagTable::new();
    for (key, value) in ft.iter() {
        match store.get_event(key) {
            Err(Error::Base(none_error!())) => warn!("Event {:?} not found", key),
            Err(e) => error!("Error {:?} encountered while retrieving event {:?}", e, key),
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
