use crate::errors::Result;
use crate::flag_table::FlagTable;
use crate::lamport_time::LamportTime;
use crate::peer::FrameNumber;
use crate::peer::Height;
use crate::transactions::InternalTransaction;
use core::hash::Hash;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use libhash::Hash as OtherHash;
use libhash_sha3::Hash as EventHash;
use libsignature::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event structure used locally on the node; all fields are serialised/deserialised
/// to be stored into the Store
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Event<Data, P, PK, Sig>
where
    P: Eq + Hash,
{
    pub(crate) creator: P,
    pub(crate) height: Height,
    pub(crate) self_parent: EventHash,
    pub(crate) other_parent: EventHash,
    pub(crate) lamport_timestamp: LamportTime,
    pub(crate) transactions: Vec<Data>,
    pub(crate) internal_transactions: Vec<InternalTransaction<P, PK>>,
    pub(crate) hash: EventHash,
    pub(crate) signatures: HashMap<P, Sig>,
    pub(crate) frame_number: FrameNumber,
}

/// NetEvent is the event transferred between nodes over the network;
/// Thus some fields are not serialised/deserialised.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NetEvent<Data, P, PK, Sig>
where
    P: Eq + Hash,
{
    pub(crate) creator: P,
    pub(crate) height: Height,
    self_parent: EventHash,
    other_parent: EventHash,
    lamport_timestamp: LamportTime,
    transactions: Vec<Data>,
    internal_transactions: Vec<InternalTransaction<P, PK>>,
    pub(crate) signatures: HashMap<P, Sig>,
}

/// HashEvent is a structure used to calculate Event's hash;
/// Based on Event structure; some fields are not serialised/deserialised.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Hash)]
struct HashEvent<Data, P, PK>
where
    P: Eq + Hash,
{
    pub(crate) creator: P,
    pub(crate) height: Height,
    self_parent: EventHash,
    other_parent: EventHash,
    lamport_timestamp: LamportTime,
    transactions: Vec<Data>,
    internal_transactions: Vec<InternalTransaction<P, PK>>,
}

impl<Data, P, PK, Sig> Stub for Event<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature,
{
}

impl<Data, P, PK, Sig> From<Event<Data, P, PK, Sig>> for HashEvent<Data, P, PK>
where
    P: Eq + Hash,
{
    fn from(ev: Event<Data, P, PK, Sig>) -> HashEvent<Data, P, PK> {
        return HashEvent {
            creator: ev.creator,
            height: ev.height,
            self_parent: ev.self_parent,
            other_parent: ev.other_parent,
            lamport_timestamp: ev.lamport_timestamp,
            transactions: ev.transactions,
            internal_transactions: ev.internal_transactions,
        };
    }
}

impl<Data, P, PK, Sig> From<Event<Data, P, PK, Sig>> for NetEvent<Data, P, PK, Sig>
where
    P: Eq + Hash,
{
    fn from(ev: Event<Data, P, PK, Sig>) -> NetEvent<Data, P, PK, Sig> {
        return NetEvent {
            creator: ev.creator,
            height: ev.height,
            self_parent: ev.self_parent,
            other_parent: ev.other_parent,
            lamport_timestamp: ev.lamport_timestamp,
            transactions: ev.transactions,
            internal_transactions: ev.internal_transactions,
            signatures: ev.signatures,
        };
    }
}

impl<Data, P, PK, Sig> From<NetEvent<Data, P, PK, Sig>> for Event<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature,
{
    fn from(ev: NetEvent<Data, P, PK, Sig>) -> Event<Data, P, PK, Sig> {
        let mut ex: Event<Data, P, PK, Sig> = Event {
            creator: ev.creator,
            height: ev.height,
            self_parent: ev.self_parent,
            other_parent: ev.other_parent,
            lamport_timestamp: ev.lamport_timestamp,
            transactions: ev.transactions,
            internal_transactions: ev.internal_transactions,
            hash: EventHash::default(),
            signatures: ev.signatures,
            frame_number: FrameNumber::default(),
        };
        let _ = ex.event_hash().unwrap();
        ex
    }
}

impl<Data, P, PK, Sig> Event<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature,
{
    pub(crate) fn new(
        creator: P,
        self_parent: EventHash,
        other_parent: EventHash,
        lamport_timestamp: LamportTime,
        transactions: Vec<Data>,
        internal_transactions: Vec<InternalTransaction<P, PK>>,
    ) -> Self {
        let mut event = Event {
            creator,
            height: 0,
            self_parent,
            other_parent,
            lamport_timestamp,
            transactions,
            internal_transactions,
            hash: EventHash::default(),
            signatures: HashMap::new(),
            frame_number: FrameNumber::default(),
        };
        event
    }
    pub(crate) fn get_creator(&self) -> P {
        self.creator.clone()
    }
    pub(crate) fn get_lamport_time(&self) -> LamportTime {
        self.lamport_timestamp
    }
    pub(crate) fn get_height(&self) -> Height {
        self.height
    }
    pub(crate) fn get_hash(&self) -> EventHash {
        self.hash
    }
    pub(crate) fn event_hash(&mut self) -> Result<EventHash> {
        let ev: HashEvent<Data, P, PK> = (*self).clone().into();
        let hash = EventHash::new(&ev)?;
        self.hash = hash;
        Ok(hash)
    }
}
