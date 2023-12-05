// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use ethers::{
    abi::RawLog,
    contract::{abigen, EthLogDecode},
    types::Log,
};

// TODO: write a macro to handle variants

// TODO: Dummy placeholder, will be replaced by actual abis
pub enum EthBridgeEvent {
    ExampleContract(ExampleContractEvents),
}

abigen!(
    ExampleContract,
    "abi/example.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

impl EthBridgeEvent {
    pub fn try_from_eth_log(log: &Log) -> Option<EthBridgeEvent> {
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        };
        if let Ok(decoded) = ExampleContractEvents::decode_log(&raw_log) {
            return Some(EthBridgeEvent::ExampleContract(decoded));
        }

        // TODO: try other variants
        None
    }
}
