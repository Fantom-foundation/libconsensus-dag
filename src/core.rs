use crate::conf::DAGconfig;
use crate::errors::Result;
use crate::event::Event;
use crate::flag_table::{min_frame, open_merge_flag_table, strict_merge_flag_table};
use crate::lamport_time::LamportTime;
use crate::peer::Frame;
use crate::store::DAGstore;
use crate::store_sled::SledStore;
use crate::transactions::InternalTransaction;
use core::mem::swap;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::errors::Result as BaseResult;
use libsignature::PublicKey;
use libsignature::SecretKey;
use libsignature::Signature;
use std::sync::Arc;
use std::sync::RwLock;

pub(crate) struct DAGcore<P, Data, SK, PK, Sig>
where
    Data: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
{
    pub(crate) conf: Arc<RwLock<DAGconfig<P, Data, SK, PK>>>,
    pub(crate) store: Arc<RwLock<dyn DAGstore<Data, P, PK, Sig>>>,
    tx_pool: Vec<Data>,
    internal_tx_pool: Vec<InternalTransaction<P, PK>>,
    lamport_time: LamportTime,
    current_frame: Frame,
    last_finalised_frame: Option<Frame>,
}

impl<P, Data, SK, PK, Sig> DAGcore<P, Data, SK, PK, Sig>
where
    P: PeerId,
    Data: DataType,
    SK: SecretKey,
    PK: PublicKey,
    Sig: Signature,
{
    /// Defines maximum number of transactions in a single event
    const TRANSACTIONS_LIMIT: usize = 16000;

    pub(crate) fn new(conf: DAGconfig<P, Data, SK, PK>) -> DAGcore<P, Data, SK, PK, Sig> {
        let store_type = conf.store_type.clone();
        let store = {
            match store_type {
                crate::store::StoreType::Unknown => panic!("unknown DAG store"),
                crate::store::StoreType::Sled => {
                    // FIXME: we should use a configurable parameter for store location instead of "./sled_store"
                    <SledStore as DAGstore<Data, P, PK, Sig>>::new("./sled_store").unwrap()
                }
            }
        };
        DAGcore {
            conf: Arc::new(RwLock::new(conf)),
            store: Arc::new(RwLock::new(store)),
            tx_pool: Vec::with_capacity(1),
            internal_tx_pool: Vec::with_capacity(1),
            lamport_time: 0,
            current_frame: 0,
            last_finalised_frame: None,
        }
    }
    pub(crate) fn get_lamport_time(&self) -> LamportTime {
        self.lamport_time
    }
    pub(crate) fn get_next_lamport_time(&mut self) -> LamportTime {
        self.lamport_time = self.lamport_time + 1;
        self.lamport_time
    }
    pub(crate) fn add_transaction(&mut self, data: Data) -> BaseResult<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.tx_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity.into());
        }
        self.tx_pool.push(data);
        Ok(())
    }
    pub(crate) fn next_transactions(&mut self) -> Vec<Data> {
        let mut len = self.tx_pool.len();
        if len > Self::TRANSACTIONS_LIMIT {
            len = Self::TRANSACTIONS_LIMIT;
        }
        let mut new_trx = self.tx_pool.split_off(len);
        swap(&mut self.tx_pool, &mut new_trx);
        new_trx
    }
    pub(crate) fn add_internal_transaction(
        &mut self,
        tx: InternalTransaction<P, PK>,
    ) -> Result<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.internal_tx_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity.into());
        }
        self.internal_tx_pool.push(tx);
        Ok(())
    }
    pub(crate) fn next_internal_transactions(&mut self) -> Vec<InternalTransaction<P, PK>> {
        let mut len = self.internal_tx_pool.len();
        if len > Self::TRANSACTIONS_LIMIT {
            len = Self::TRANSACTIONS_LIMIT;
        }
        let mut new_trx = self.internal_tx_pool.split_off(len);
        swap(&mut self.internal_tx_pool, &mut new_trx);
        new_trx
    }
    pub(crate) fn update_lamport_time(&mut self, time: LamportTime) {
        if self.lamport_time < time {
            self.lamport_time = time;
        }
    }
    pub(crate) fn check_event(&self, event: &Event<Data, P, PK, Sig>) -> Result<bool> {
        // FIXME: implement event verification:
        // - self-parant must be the last known event of the creator with height one minus height of the event
        // - all signatures must be verified positively
        Ok(true)
    }
    pub(crate) fn insert_event(&mut self, mut event: Event<Data, P, PK, Sig>) -> Result<bool> {
        let event_hash = event.event_hash()?;
        let self_parent = event.self_parent.clone();
        let other_parent = event.other_parent.clone();
        let (self_parent_event, other_parent_event, self_parent_ft, other_parent_ft) = {
            let store = self.store.read().unwrap();
            (
                store.get_event(&self_parent)?,
                store.get_event(&other_parent)?,
                store.get_flag_table(&self_parent)?,
                store.get_flag_table(&other_parent)?,
            )
        };
        let mut root: bool = false;
        let mut frame: Frame = Frame::default();
        if self_parent_event.frame_number == other_parent_event.frame_number {
            let root_flag_table = strict_merge_flag_table(
                &self_parent_ft,
                &other_parent_ft,
                self_parent_event.frame_number,
            );
            let creator_root_flag_table = {
                let store = self.store.read().unwrap();
                store.derive_creator_flag_table(&root_flag_table)
            };
            let root_majority = {
                let conf = self.conf.read().unwrap();
                conf.peers.root_majority()
            };
            if creator_root_flag_table.len() >= root_majority {
                root = true;
                frame = self_parent_event.frame_number + 1;
            } else {
                root = false;
                frame = self_parent_event.frame_number;
            }
        } else if self_parent_event.frame_number > other_parent_event.frame_number {
            root = false;
            frame = self_parent_event.frame_number;
        } else {
            root = true;
            frame = other_parent_event.frame_number;
        }
        event.frame_number = frame;
        let first_not_finalised_frame = match self.last_finalised_frame {
            Some(x) => x + 1,
            None => 0,
        };
        let mut visibilis_flag_table =
            open_merge_flag_table(&self_parent_ft, &other_parent_ft, first_not_finalised_frame);
        if root {
            visibilis_flag_table.insert(event_hash.clone(), frame);
        }
        {
            let mut store = self.store.write().unwrap();
            store.set_flag_table(&event_hash, &visibilis_flag_table)?;
        }
        let creator_visibilis_flag_table = {
            let store = self.store.read().unwrap();
            store.derive_creator_flag_table(&visibilis_flag_table)
        };
        let peer_size = self.conf.read().unwrap().peers.len();
        if peer_size == creator_visibilis_flag_table.len() {
            let frame_upto = min_frame(&visibilis_flag_table);
            for frame in first_not_finalised_frame..frame_upto {
                // FIXME: need to be implemented
                //self.finalise_frame(frame)
                self.last_finalised_frame = Some(frame);
            }
        }
        self.store.write().unwrap().set_event(event)?;
        Ok(true)
    }
}
