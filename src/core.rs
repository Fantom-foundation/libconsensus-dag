use crate::conf::DAGconfig;
use crate::errors::{Error, Result};
use crate::lamport_time::LamportTime;
use crate::peer::Frame;
use crate::store::DAGstore;
use crate::store_sled::SledStore;
use crate::transactions::InternalTransaction;
use libcommon_rs::data::DataType;
use libcommon_rs::peer::PeerId;
use libconsensus::errors::Error::AtMaxVecCapacity;
use libconsensus::errors::Result as BaseResult;
use libsignature::PublicKey;
use libsignature::SecretKey;
use std::sync::Arc;
use std::sync::RwLock;

pub(crate) struct DAGcore<P, Data, SK, PK>
where
    Data: DataType,
    P: PeerId,
    SK: SecretKey,
    PK: PublicKey,
{
    pub(crate) conf: Arc<RwLock<DAGconfig<P, Data, SK, PK>>>,
    pub(crate) store: Arc<RwLock<dyn DAGstore<Data, P, PK>>>,
    tx_pool: Vec<Data>,
    internal_tx_pool: Vec<InternalTransaction<P, PK>>,
    lamport_time: LamportTime,
    current_frame: Frame,
    last_finalised_frame: Option<Frame>,
    //    sync_request_transport: Box<dyn Transport<P, SyncReq<P>, Error, DAGPeerList<P>> + 'a>,
    //    sync_reply_transport: Box<dyn Transport<P, SyncReply<P>, Error, DAGPeerList<P>> + 'a>,
}

impl<P, Data, SK, PK> DAGcore<P, Data, SK, PK>
where
    P: PeerId,
    Data: DataType,
    SK: SecretKey,
    PK: PublicKey,
{
    pub(crate) fn new(conf: DAGconfig<P, Data, SK, PK>) -> DAGcore<P, Data, SK, PK> {
        let store_type = conf.store_type.clone();
        let store = {
            match store_type {
                crate::store::StoreType::Unknown => panic!("unknown DAG store"),
                crate::store::StoreType::Sled => {
                    // FIXME: we should use a configurable parameter for store location instead of "./sled_store"
                    <SledStore as DAGstore<Data, P, PK>>::new("./sled_store").unwrap()
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
        self.lamport_time.clone()
    }
    pub(crate) fn add_transaction(&mut self, data: Data) -> BaseResult<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.tx_pool.len() == std::usize::MAX {
            return Err(AtMaxVecCapacity);
        }
        self.tx_pool.push(data);
        Ok(())
    }
    pub(crate) fn add_internal_transaction(
        &mut self,
        tx: InternalTransaction<P, PK>,
    ) -> Result<()> {
        // Vec::push() panics when number of elements overflows `usize`
        if self.internal_tx_pool.len() == std::usize::MAX {
            return Err(Error::Base(AtMaxVecCapacity));
        }
        self.internal_tx_pool.push(tx);
        Ok(())
    }
    pub(crate) fn update_lamport_time(&mut self, time: LamportTime) {
        if self.lamport_time < time {
            self.lamport_time = time;
        }
    }
}
