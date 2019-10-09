extern crate sled;

use crate::event::Event;
use crate::event::NetEvent;
use crate::frame::Frame;
use crate::frame::FrameRecord;
use crate::peer::FrameNumber;
use crate::peer::GossipList;
use crate::peer::Height;
use crate::store::*;
use bincode::{deserialize, serialize};
use libcommon_rs::data::DataType;
use libhash_sha3::Hash as EventHash;
use libsignature::{PublicKey, Signature};
//use log::warn;
use crate::errors::{Error, Result};
use crate::flag_table::FlagTable;
use libcommon_rs::peer::PeerId;

use std::path::Path;
use to_vec::ToVec;

pub(crate) struct SledStore {
    event: sled::Db,
    flag_table: sled::Db,
    frame: sled::Db,
    sync: bool,
}

impl<P, D, PK, Sig> DAGstore<D, P, PK, Sig> for SledStore
where
    D: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature,
{
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
        let frame_config = sled::ConfigBuilder::new()
            .path(Path::new(base_path).join("frames").as_path())
            .print_profile_on_drop(true) // if true, gives summary of latency historgrams
            .build();

        Ok(SledStore {
            event: sled::Db::start(event_config)?,
            flag_table: sled::Db::start(ft_config)?,
            frame: sled::Db::start(frame_config)?,
            sync: true, // let be synchronous in writing, though it's slow
        })
    }

    // function set_event() writes Event into storage; returns True on success and
    // False on failure
    fn set_event(&mut self, e: Event<D, P, PK, Sig>) -> Result<()> {
        let e_bytes = serialize(&e)?;
        // Store serialized event with hash as a key.
        let key = e.hash.clone().to_vec();
        self.event.insert(key, e_bytes.clone())?;
        // Store serialized event with creator and creator's height as a key.
        let key2 = format!("{}-{}", e.creator, e.height).into_bytes();
        self.event.insert(key2, e_bytes)?;
        if self.sync {
            self.event.flush()?;
        }
        let frame_key = format!("{}", e.frame_number).into_bytes();
        let mut frame: Frame = match self.frame.get(&*frame_key)? {
            Some(x) => deserialize::<Frame>(&x)?,
            None => Frame::default(),
        };
        let record = FrameRecord {
            hash: e.get_hash(),
            lamport_time: e.lamport_timestamp,
        };
        frame.events.push(record);
        let f_bytes = serialize(&frame)?;
        self.frame.insert(frame_key, f_bytes)?;
        if self.sync {
            self.frame.flush()?;
        }
        Ok(())
    }

    fn get_event(&self, ex: &EventHash) -> Result<Event<D, P, PK, Sig>> {
        let key = ex.to_vec();
        match self.event.get(&*key)? {
            Some(x) => Ok(deserialize::<Event<D, P, PK, Sig>>(&x)?),
            None => Err(Error::NoneError.into()),
        }
    }

    fn get_frame(&self, frame_number: FrameNumber) -> Result<Frame> {
        let frame_key = format!("{}", frame_number).into_bytes();
        match self.frame.get(&*frame_key)? {
            Some(x) => Ok(deserialize::<Frame>(&x)?),
            None => Ok(Frame::default()),
        }
    }

    fn set_flag_table(&mut self, ex: &EventHash, ft: &FlagTable) -> Result<()> {
        let key = ex.clone().to_vec();
        let e_bytes = serialize(&ft)?;
        self.flag_table.insert(key, e_bytes)?;
        if self.sync {
            self.flag_table.flush()?;
        }
        Ok(())
    }

    fn get_flag_table(&self, ex: &EventHash) -> Result<FlagTable> {
        let key = ex.to_vec();
        match self.flag_table.get(&*key)? {
            Some(x) => Ok(deserialize::<FlagTable>(&x)?),
            None => Err(Error::NoneError.into()),
        }
    }

    fn get_event_of_creator(&self, creator: P, height: Height) -> Result<Event<D, P, PK, Sig>> {
        let key = format!("{}-{}", creator, height).into_bytes();
        match self.event.get(&*key)? {
            Some(x) => Ok(deserialize::<Event<D, P, PK, Sig>>(&x)?),
            None => Err(Error::NoneError.into()),
        }
    }

    fn get_events_for_gossip(
        &self,
        gossip: &GossipList<P>,
    ) -> Result<Vec<NetEvent<D, P, PK, Sig>>> {
        let mut events: Vec<NetEvent<D, P, PK, Sig>> = Vec::with_capacity(1);
        for (peer, gossip) in gossip.iter() {
            let mut height = gossip.height + 1;
            loop {
                let event = match self.get_event_of_creator(peer.clone(), height.clone()) {
                    Err(e) => match e.downcast::<Error>() {
                        Ok(err) => {
                            if err == Error::NoneError {
                                break;
                            } else {
                                return Err(err.into());
                            }
                        }
                        Err(erx) => return Err(erx),
                    },
                    Ok(event) => event,
                };

                events.push(event.into());
                height += 1;
            }
        }
        Ok(events)
    }
}
