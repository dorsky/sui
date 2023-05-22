// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    str::FromStr,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SequenceNumber, SuiAddress},
    digests::TransactionDigest,
    effects::{TransactionEffects, TransactionEffectsAPI},
    event::Event,
    executable_transaction::VerifiedExecutableTransaction,
    gas::GasCostSummary,
    object::{Object, Owner},
    storage::ObjectStore,
    transaction::{TransactionData, TransactionDataAPI},
    Identifier,
};

use crate::types_ex::{NodeStreamPayload, NodeStreamPerEpochTopic, NodeStreamTopic};

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum TxInfoNodeStreamTopic {
    String,
    PackagePublish,
    ObjectChangeLight,
    ObjectChangeRaw,
    MoveCall,
    Transaction,
    Effects,
    GasCostSummary,
    // TODO:
    // CoinBalanceChange
    // Epoch
    // Checkpoint
}

impl Display for TxInfoNodeStreamTopic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TxInfoNodeStreamTopic::String => write!(f, "string"),
            TxInfoNodeStreamTopic::PackagePublish => write!(f, "package_publish"),
            TxInfoNodeStreamTopic::ObjectChangeLight => write!(f, "object_change_light"),
            TxInfoNodeStreamTopic::ObjectChangeRaw => write!(f, "object_change_raw"),
            TxInfoNodeStreamTopic::MoveCall => write!(f, "move_call"),
            TxInfoNodeStreamTopic::Transaction => write!(f, "transaction"),
            TxInfoNodeStreamTopic::GasCostSummary => write!(f, "gas_cost_summary"),
            TxInfoNodeStreamTopic::Effects => write!(f, "effects"),
        }
    }
}

impl FromStr for TxInfoNodeStreamTopic {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "string" => Ok(TxInfoNodeStreamTopic::String),
            "package_publish" => Ok(TxInfoNodeStreamTopic::PackagePublish),
            "object_change_light" => Ok(TxInfoNodeStreamTopic::ObjectChangeLight),
            "object_change_raw" => Ok(TxInfoNodeStreamTopic::ObjectChangeRaw),
            "move_call" => Ok(TxInfoNodeStreamTopic::MoveCall),
            "transaction" => Ok(TxInfoNodeStreamTopic::Transaction),
            "effects" => Ok(TxInfoNodeStreamTopic::Effects),
            "gas_cost_summary" => Ok(TxInfoNodeStreamTopic::GasCostSummary),
            _ => Err(anyhow::anyhow!("Invalid topic")),
        }
    }
}

impl NodeStreamPerEpochTopic<TxInfoData, TxInfoMetadata> for TxInfoNodeStreamTopic {
    type FromBytesError = anyhow::Error;
    type ToBytesError = anyhow::Error;

    fn topic_for_epoch(&self, epoch: u64) -> NodeStreamTopic {
        NodeStreamTopic::new(format!("{}-{}", epoch, self))
    }

    fn payload_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<NodeStreamPayload<TxInfoData, TxInfoMetadata>, Self::FromBytesError> {
        bcs::from_bytes(bytes).map_err(|e| e.into())
    }

    fn payload_to_bytes(
        &self,
        payload: &NodeStreamPayload<TxInfoData, TxInfoMetadata>,
    ) -> Result<Vec<u8>, Self::ToBytesError> {
        bcs::to_bytes(payload).map_err(|e| e.into())
    }
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum ObjectChangeStatus {
    Created(ObjectRef, Owner),
    Mutated(ObjectRef, Owner),
    Deleted(ObjectRef),
    Wrapped(ObjectRef),
    Unwrapped(ObjectRef, Owner),
    UnwrappedThenDeleted(ObjectRef),
    LoadedChildObject(ObjectID, SequenceNumber),
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum TxInfoData {
    String(String),
    PackagePublish(Object),
    ObjectChangeLight(ObjectChangeStatus),
    ObjectChangeRaw(ObjectChangeStatus, Option<Object>),
    MoveCall(ObjectID, Identifier, Identifier),
    Transaction(TransactionData),
    Effects(TransactionEffects),
    GasCostSummary(GasCostSummary),
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct TxInfoMetadata {
    pub message_process_timestamp_ms: u64,
    pub checkpoint_id: u64,
    pub sender: SuiAddress,
    pub tx_digest: TransactionDigest,
}

pub fn from_post_exec(
    message_process_timestamp_ms: u64,
    checkpoint_id: u64,
    sender: &SuiAddress,
    tx_digest: &TransactionDigest,
    cert: &VerifiedExecutableTransaction,
    effects: &TransactionEffects,
    loaded_child_objects: &BTreeMap<ObjectID, SequenceNumber>,
    store: &dyn ObjectStore,
) -> Vec<(
    NodeStreamPayload<TxInfoData, TxInfoMetadata>,
    TxInfoNodeStreamTopic,
)> {
    TxInfoData::from_post_exec(cert, effects.clone(), loaded_child_objects, store)
        .into_iter()
        .map(|data| {
            let topic = data.to_topic();
            (
                NodeStreamPayload {
                    metdata: TxInfoMetadata {
                        message_process_timestamp_ms,
                        checkpoint_id,
                        sender: *sender,
                        tx_digest: *tx_digest,
                    },
                    data,
                },
                topic,
            )
        })
        .collect()
}

impl TxInfoData {
    pub fn from_post_exec(
        cert: &VerifiedExecutableTransaction,
        effects: TransactionEffects,
        loaded_child_objects: &BTreeMap<ObjectID, SequenceNumber>,
        store: &dyn ObjectStore,
    ) -> Vec<Self> {
        let mut result = vec![];
        // Objects
        result.extend(
            effects
                .created()
                .iter()
                .map(|q| Self::ObjectChangeLight(ObjectChangeStatus::Created(q.0, q.1))),
        );
        result.extend(
            effects
                .mutated()
                .iter()
                .map(|q| Self::ObjectChangeLight(ObjectChangeStatus::Mutated(q.0, q.1))),
        );
        result.extend(
            effects
                .deleted()
                .iter()
                .map(|q| Self::ObjectChangeLight(ObjectChangeStatus::Deleted(*q))),
        );
        result.extend(
            effects
                .wrapped()
                .iter()
                .map(|q| Self::ObjectChangeLight(ObjectChangeStatus::Wrapped(*q))),
        );
        result.extend(
            effects
                .unwrapped()
                .iter()
                .map(|q| Self::ObjectChangeLight(ObjectChangeStatus::Unwrapped(q.0, q.1))),
        );
        result.extend(
            effects
                .unwrapped_then_deleted()
                .iter()
                .map(|q| Self::ObjectChangeLight(ObjectChangeStatus::UnwrappedThenDeleted(*q))),
        );
        result.extend(
            loaded_child_objects.iter().map(|q| {
                Self::ObjectChangeLight(ObjectChangeStatus::LoadedChildObject(*q.0, *q.1))
            }),
        );

        // Get the objects
        let mut packages = vec![];
        let mut objects = result
            .iter()
            .filter_map(|q| {
                if let Self::ObjectChangeLight(change) = q {
                    Some(match change {
                        ObjectChangeStatus::Created(r, _)
                        | ObjectChangeStatus::Mutated(r, _)
                        | ObjectChangeStatus::Deleted(r)
                        | ObjectChangeStatus::Wrapped(r)
                        | ObjectChangeStatus::Unwrapped(r, _)
                        | ObjectChangeStatus::UnwrappedThenDeleted(r) => {
                            Self::ObjectChangeRaw(change.clone(), {
                                let obj = store
                                    .get_object_by_key(&r.0, r.1)
                                    .expect("DB read should not fail");
                                if let Some(o) = obj.clone() {
                                    if o.is_package() {
                                        packages.push(Self::PackagePublish(o));
                                    }
                                }
                                obj
                            })
                        }
                        ObjectChangeStatus::LoadedChildObject(id, seq) => Self::ObjectChangeRaw(
                            change.clone(),
                            store
                                .get_object_by_key(id, *seq)
                                .expect("DB read should not fail"),
                        ),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        result.append(&mut packages);
        result.append(&mut objects);

        // Gas
        result.push(Self::GasCostSummary(effects.gas_cost_summary().clone()));

        // Move calls
        result.extend(cert.intent_message().value.move_calls().iter().map(
            |(package, module, function)| {
                Self::MoveCall(
                    **package,
                    module.to_owned().to_owned(),
                    function.to_owned().to_owned(),
                )
            },
        ));

        // Effects
        result.push(Self::Effects(effects));

        // Record this TX
        result.push(Self::Transaction(cert.intent_message().value.clone()));
        result
    }

    pub fn to_topic(&self) -> TxInfoNodeStreamTopic {
        match self {
            Self::String(_) => TxInfoNodeStreamTopic::String,
            Self::PackagePublish(_) => TxInfoNodeStreamTopic::PackagePublish,
            Self::ObjectChangeLight(_) => TxInfoNodeStreamTopic::ObjectChangeLight,
            Self::ObjectChangeRaw(_, _) => TxInfoNodeStreamTopic::ObjectChangeRaw,
            Self::MoveCall(_, _, _) => TxInfoNodeStreamTopic::MoveCall,
            Self::Transaction(_) => TxInfoNodeStreamTopic::Transaction,
            Self::GasCostSummary(_) => TxInfoNodeStreamTopic::GasCostSummary,
            Self::Effects(_) => TxInfoNodeStreamTopic::Effects,
        }
    }
}
