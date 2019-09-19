use crate::event_hash::EventHash;
use crate::flag_table::FlagTable;
use crate::lamport_time::LamportTime;
use crate::peer::Frame;
use crate::peer::Height;
use crate::transactions::InternalTransaction;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libcommon_rs::Stub;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Event<Data, P> {
    pub(crate) creator: P,
    pub(crate) height: Height,
    self_parent: EventHash,
    other_parent: EventHash,
    lamport_timestamp: LamportTime,
    transactions: Vec<Data>,
    internal_transactions: Vec<InternalTransaction<P>>,
    #[serde(skip, default)]
    pub(crate) hash: EventHash,
    #[serde(skip, default)]
    frame_number: Frame,
    #[serde(skip, default)]
    ft: FlagTable,
}

impl<Data, P> Stub for Event<Data, P>
where
    Data: DataType,
    P: PeerId,
{
}

impl<Data, P> Event<Data, P>
where
    Data: DataType,
    P: PeerId,
{
    pub(crate) fn new(
        creator: P,
        self_parent: EventHash,
        other_parent: EventHash,
        lamport_timestamp: LamportTime,
        transactions: Vec<Data>,
        internal_transactions: Vec<InternalTransaction<P>>,
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
            frame_number: Frame::default(),
            ft: FlagTable::new(),
        };
        event
    }
    pub(crate) fn get_creator(&self) -> P {
        self.creator.clone()
    }
    pub(crate) fn get_lamport_time(&self) -> LamportTime {
        self.lamport_timestamp.clone()
    }
    pub(crate) fn get_height(&self) -> Height {
        self.height.clone()
    }
}
