extern crate sled;

use crate::event::Event;
use crate::event_hash::EventHash;
use crate::store::*;
use bincode::{deserialize, serialize};
//use log::warn;
use crate::errors::{Error, Result};
use crate::flag_table::FlagTable;

use std::path::Path;
use to_vec::ToVec;

pub(crate) struct SledStore {
    event: sled::Db,
    flag_table: sled::Db,
    sync: bool,
}

impl DAGstore for SledStore {
    // function new() creates a new Sled based Storage
    fn new(base_path: &str) -> Result<SledStore> {
        let event_config = sled::ConfigBuilder::new()
            .path(Path::new(base_path).join("events").as_path())
            .print_profile_on_drop(true) // if true, gives summary of latency historgrams
            .build();
        let ft_config = sled::ConfigBuilder::new()
            .path(Path::new(base_path).join("flag_tables").as_path())
            .print_profile_on_drop(true) // if true, gives summary of latency historgrams
            .build();

        Ok(SledStore {
            event: sled::Db::start(event_config)?,
            flag_table: sled::Db::start(ft_config)?,
            sync: true, // let be synchronous in writing, though it's slow
        })
    }

    // function set_event() writes Event into storage; returns True on success and
    // False on failure
    fn set_event(&mut self, e: Event) -> Result<()> {
        let key = e.hash.clone().to_vec();
        let e_bytes = serialize(&e)?;
        self.event.insert(key, e_bytes)?;
        if self.sync {
            self.event.flush()?;
        }
        Ok(())
    }

    fn get_event(&mut self, ex: &EventHash) -> Result<Event> {
        let key = ex.to_vec();
        match self.event.get(&*key)? {
            Some(x) => Ok(deserialize::<Event>(&x)?),
            None => Err(Error::Base(none_error!())),
        }
    }

    fn set_flag_table(&mut self, ex: &EventHash, ft: FlagTable) -> Result<()> {
        let key = ex.clone().to_vec();
        let e_bytes = serialize(&ft)?;
        self.flag_table.insert(key, e_bytes)?;
        if self.sync {
            self.flag_table.flush()?;
        }
        Ok(())
    }

    fn get_flag_table(&mut self, ex: &EventHash) -> Result<FlagTable> {
        let key = ex.to_vec();
        match self.flag_table.get(&*key)? {
            Some(x) => Ok(deserialize::<FlagTable>(&x)?),
            None => Err(Error::Base(none_error!())),
        }
    }
}
