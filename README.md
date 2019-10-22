libconsensus-dag
==================
[![Rust: nightly](https://img.shields.io/badge/Rust-nightly-blue.svg)](https://www.rust-lang.org) [![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE) [![Build Status](https://travis-ci.org/Fantom-foundation/libconsensus-dag.svg?branch=master)](https://travis-ci.org/Fantom-foundation/libconsensus-dag)

libconsensus-dag in Rust.

## Example

### Prelude
```rust
use libconsensus_dag::conf::DAGconfig;
use libconsensus::Consensus;
use libconsensus::ConsensusConfiguration;
pub use libconsensus_dag::peer::DAGPeer;
pub use libconsensus_dag::peer::DAGPeerList;
use crate::DAG;
use core::fmt::Display;
use core::fmt::Formatter;
use futures::executor::block_on;
use futures::stream::StreamExt;
use libcommon_rs::peer::Peer;
use libcommon_rs::peer::PeerList;
use libhash_sha3::Hash as EventHash;
use libsignature::Signature as LibSignature;
use libsignature_ed25519_dalek::{PublicKey, SecretKey, Signature};
use serde::{Deserialize, Serialize};
type Id = PublicKey;
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Hash, Copy)]
struct Data {
    byte: i8,
}

impl Display for Data {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let mut formatted = String::new();
        formatted.push_str(&self.byte.to_string());
        write!(f, "{}", formatted)
    }
}

impl From<i8> for Data {
    fn from (i: i8) -> Data {
        Data {
            byte: i,
        }
    }
}
```

### Step-by-step example

#### Prepare public/secret key for nodes
```rust
let kp1 = Signature::<EventHash>::generate_key_pair().unwrap();
let kp2 = Signature::<EventHash>::generate_key_pair().unwrap();
let kp3 = Signature::<EventHash>::generate_key_pair().unwrap();
let kp4 = Signature::<EventHash>::generate_key_pair().unwrap();
let kp5 = Signature::<EventHash>::generate_key_pair().unwrap();
```

#### Prepare peer list
```rust
let mut peer_list = DAGPeerList::<Id, PublicKey>::default();
let peer1 = DAGPeer::<Id, PublicKey>::new(kp1.0.clone(), "127.0.0.1:9001".to_string());
let peer2 = DAGPeer::<Id, PublicKey>::new(kp2.0.clone(), "127.0.0.1:9003".to_string());
let peer3 = DAGPeer::<Id, PublicKey>::new(kp3.0.clone(), "127.0.0.1:9005".to_string());
let peer4 = DAGPeer::<Id, PublicKey>::new(kp4.0.clone(), "127.0.0.1:9007".to_string());
let peer5 = DAGPeer::<Id, PublicKey>::new(kp5.0.clone(), "127.0.0.1:9009".to_string());

peer_list.add(peer1).unwrap();
peer_list.add(peer2).unwrap();
peer_list.add(peer3).unwrap();
peer_list.add(peer4).unwrap();
peer_list.add(peer5).unwrap();
```

#### Create Consensus configuration for every node
```rust
let mut consensus_config1 = DAGconfig::<Id, Data, SecretKey, PublicKey>::new();
consensus_config1.request_addr = "127.0.0.1:9001".to_string();
consensus_config1.reply_addr = "127.0.0.1:9002".to_string();
consensus_config1.transport_type = libtransport::TransportType::TCP;
consensus_config1.store_type = crate::store::StoreType::Sled;
consensus_config1.creator = kp1.0;
consensus_config1.secret_key = kp1.1;
consensus_config1.peers = peer_list.clone();

let mut consensus_config2 = DAGconfig::<Id, Data, SecretKey, PublicKey>::new();
consensus_config2.request_addr = "127.0.0.1:9003".to_string();
consensus_config2.reply_addr = "127.0.0.1:9004".to_string();
consensus_config2.transport_type = libtransport::TransportType::TCP;
consensus_config2.store_type = crate::store::StoreType::Sled;
consensus_config2.creator = kp2.0;
consensus_config2.secret_key = kp2.1;
consensus_config2.peers = peer_list.clone();

let mut consensus_config3 = DAGconfig::<Id, Data, SecretKey, PublicKey>::new();
consensus_config3.request_addr = "127.0.0.1:9005".to_string();
consensus_config3.reply_addr = "127.0.0.1:9006".to_string();
consensus_config3.transport_type = libtransport::TransportType::TCP;
consensus_config3.store_type = crate::store::StoreType::Sled;
consensus_config3.creator = kp3.0;
consensus_config3.secret_key = kp3.1;
consensus_config3.peers = peer_list.clone();

let mut consensus_config4 = DAGconfig::<Id, Data, SecretKey, PublicKey>::new();
consensus_config4.request_addr = "127.0.0.1:9007".to_string();
consensus_config4.reply_addr = "127.0.0.1:9008".to_string();
consensus_config4.transport_type = libtransport::TransportType::TCP;
consensus_config4.store_type = crate::store::StoreType::Sled;
consensus_config4.creator = kp4.0;
consensus_config4.secret_key = kp4.1;
consensus_config4.peers = peer_list.clone();

let mut consensus_config5 = DAGconfig::<Id, Data, SecretKey, PublicKey>::new();
consensus_config5.request_addr = "127.0.0.1:9009".to_string();
consensus_config5.reply_addr = "127.0.0.1:9010".to_string();
consensus_config5.transport_type = libtransport::TransportType::TCP;
consensus_config5.store_type = crate::store::StoreType::Sled;
consensus_config5.creator = kp5.0;
consensus_config5.secret_key = kp5.1;
consensus_config5.peers = peer_list.clone();
```

#### Start every node
```rust
let mut DAG1 =
    DAG::<Id, Data, SecretKey, PublicKey, Signature<EventHash>>::new(consensus_config1)
        .unwrap();
let mut DAG2 =
    DAG::<Id, Data, SecretKey, PublicKey, Signature<EventHash>>::new(consensus_config2)
        .unwrap();
let mut DAG3 =
    DAG::<Id, Data, SecretKey, PublicKey, Signature<EventHash>>::new(consensus_config3)
        .unwrap();
let mut DAG4 =
    DAG::<Id, Data, SecretKey, PublicKey, Signature<EventHash>>::new(consensus_config4)
        .unwrap();
let mut DAG5 =
    DAG::<Id, Data, SecretKey, PublicKey, Signature<EventHash>>::new(consensus_config5)
        .unwrap();
```

#### Send transactions into Consensus
```rust
let data: [Data; 5] = [
    Data { byte: 1 },
    Data { byte: 2 },
    Data { byte: 3 },
    Data { byte: 4 },
    Data { byte: 5 },
];

DAG1.send_transaction(data[0].clone()).unwrap();
DAG2.send_transaction(data[1].clone()).unwrap();
DAG3.send_transaction(data[2].clone()).unwrap();
DAG4.send_transaction(data[3].clone()).unwrap();
DAG5.send_transaction(data[4].clone()).unwrap();
```

#### Receive transaction in Consensus
```rust
block_on(async {
    let data = match DAG1.next().await {
                    Some(d) => d,
                    None => panic!("unexpected None"),
    };
    // process `data` here
});
```

---

## RFCs

https://github.com/Fantom-foundation/fantom-rfcs

# Developer guide

Install the latest version of [Rust](https://www.rust-lang.org). We tend to use nightly versions. [CLI tool for installing Rust](https://rustup.rs).

We use [rust-clippy](https://github.com/rust-lang-nursery/rust-clippy) linters to improve code quality.

There are plenty of [IDEs](https://areweideyet.com) and other [Rust development tools to consider](https://github.com/rust-unofficial/awesome-rust#development-tools).

### CLI instructions

```bash
# Install Rust (nightly)
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly
# Install cargo-make (cross-platform feature-rich reimplementation of Make)
$ cargo install --force cargo-make
# Install rustfmt (Rust formatter)
$ rustup component add rustfmt
# Install clippy (Rust linter)
$ rustup component add clippy
# Clone this repo
$ git clone https://github.com/Fantom-foundation/libconsensus-dag && cd libconsensus-dag
# Run tests
$ cargo test
# Format, build and test
$ cargo make
```
