use crate::errors::Result;
use crate::lamport_time::LamportTime;
use crate::peer::FrameNumber;
use crate::peer::Height;
use crate::transactions::InternalTransaction;
use core::fmt::Formatter;
use core::hash::Hash;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use libhash::Hash as OtherHash;
use libhash_sha3::Hash as EventHash;
use libsignature::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

/// Event structure used locally on the node; all fields are serialised/deserialised
/// to be stored into the Store
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Event<Data, P, PK, Sig>
where
    P: Eq + Hash,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    pub(crate) creator: P,
    pub(crate) height: Height,
    pub(crate) self_parent: EventHash,
    pub(crate) other_parent: EventHash,
    pub(crate) lamport_timestamp: LamportTime,
    pub(crate) transactions: Vec<Data>,
    #[serde(bound = "")]
    pub(crate) internal_transactions: Vec<InternalTransaction<P, PK>>,
    pub(crate) hash: EventHash,
    #[serde(bound = "")]
    pub(crate) signatures: HashMap<P, Sig>,
    pub(crate) frame_number: FrameNumber,
}

/// NetEvent is the event transferred between nodes over the network;
/// Thus some fields are not serialised/deserialised.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NetEvent<Data, P, PK, Sig>
where
    P: Eq + Hash,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    pub(crate) creator: P,
    pub(crate) height: Height,
    self_parent: EventHash,
    other_parent: EventHash,
    lamport_timestamp: LamportTime,
    transactions: Vec<Data>,
    #[serde(bound = "")]
    internal_transactions: Vec<InternalTransaction<P, PK>>,
    #[serde(bound = "")]
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
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
}

impl<Data, P, PK, Sig> From<Event<Data, P, PK, Sig>> for HashEvent<Data, P, PK>
where
    P: Eq + Hash,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    fn from(ev: Event<Data, P, PK, Sig>) -> HashEvent<Data, P, PK> {
        HashEvent {
            creator: ev.creator,
            height: ev.height,
            self_parent: ev.self_parent,
            other_parent: ev.other_parent,
            lamport_timestamp: ev.lamport_timestamp,
            transactions: ev.transactions,
            internal_transactions: ev.internal_transactions,
        }
    }
}

impl<Data, P, PK, Sig> From<Event<Data, P, PK, Sig>> for NetEvent<Data, P, PK, Sig>
where
    P: Eq + Hash,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    fn from(ev: Event<Data, P, PK, Sig>) -> NetEvent<Data, P, PK, Sig> {
        NetEvent {
            creator: ev.creator,
            height: ev.height,
            self_parent: ev.self_parent,
            other_parent: ev.other_parent,
            lamport_timestamp: ev.lamport_timestamp,
            transactions: ev.transactions,
            internal_transactions: ev.internal_transactions,
            signatures: ev.signatures,
        }
    }
}

impl<Data, P, PK, Sig> From<NetEvent<Data, P, PK, Sig>> for Event<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
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

impl<Data, P, PK, Sig> Display for Event<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "creator:{}; height:{}; self-parent:{}; other-parent:{}; lamport_time:{}; hash:{}; frame:{}; signatures: {:?}; transactions: {:#?}.",
        self.creator, self.height, self.self_parent, self.other_parent,
        self.lamport_timestamp, self.hash, self.frame_number,
        self.signatures, self.transactions)
    }
}

impl<Data, P, PK, Sig> Event<Data, P, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    PK: PublicKey,
    Sig: Signature<Hash = EventHash, PublicKey = PK>,
{
    pub(crate) fn new(
        creator: P,
        height: usize,
        self_parent: EventHash,
        other_parent: EventHash,
        lamport_timestamp: LamportTime,
        transactions: Vec<Data>,
        internal_transactions: Vec<InternalTransaction<P, PK>>,
    ) -> Self {
        Event {
            creator,
            height,
            self_parent,
            other_parent,
            lamport_timestamp,
            transactions,
            internal_transactions,
            hash: EventHash::default(),
            signatures: HashMap::new(),
            frame_number: FrameNumber::default(),
        }
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
