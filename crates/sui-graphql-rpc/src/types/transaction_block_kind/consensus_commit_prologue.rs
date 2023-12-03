// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context_data::db_data_provider::PgManager,
    types::{date_time::DateTime, epoch::Epoch},
};
use async_graphql::*;
use fastcrypto::encoding::{Base58, Encoding};
use sui_types::{
    digests::ConsensusCommitDigest,
    messages_checkpoint::CheckpointTimestamp,
    messages_consensus::{
        ConsensusCommitPrologue as NativeConsensusCommitPrologueTransactionV1,
        ConsensusCommitPrologueV2 as NativeConsensusCommitPrologueTransactionV2,
    },
};

/// Unlike other transaction kinds, which wrap a native transaction kind, this transaction lists out
/// all its fields. This is because there are multiple versions of this transaction type in the
/// protocol, and this type merges both fields, with fields that only appear in one version being
/// optional.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ConsensusCommitPrologueTransaction {
    epoch: u64,
    round: u64,
    commit_timestamp_ms: CheckpointTimestamp,
    consensus_commit_digest: Option<ConsensusCommitDigest>,
}

#[Object]
impl ConsensusCommitPrologueTransaction {
    /// Epoch of the commit prologue transaction.
    async fn epoch(&self, ctx: &Context<'_>) -> Result<Epoch> {
        ctx.data_unchecked::<PgManager>()
            .fetch_epoch_strict(self.epoch)
            .await
            .extend()
    }

    /// Consensus round of the commit.
    async fn round(&self) -> u64 {
        self.round
    }

    /// Unix timestamp from consensus.
    async fn commit_timestamp(&self) -> Option<DateTime> {
        DateTime::from_ms(self.commit_timestamp_ms as i64)
    }

    /// Digest of consensus output, encoded as a Base58 string (only available from V2 of the
    /// transaction).
    async fn consensus_commit_digest(&self) -> Option<String> {
        self.consensus_commit_digest
            .map(|digest| Base58::encode(digest.inner()))
    }
}

impl From<NativeConsensusCommitPrologueTransactionV1> for ConsensusCommitPrologueTransaction {
    fn from(ccp: NativeConsensusCommitPrologueTransactionV1) -> Self {
        Self {
            epoch: ccp.epoch,
            round: ccp.round,
            commit_timestamp_ms: ccp.commit_timestamp_ms,
            consensus_commit_digest: None,
        }
    }
}

impl From<NativeConsensusCommitPrologueTransactionV2> for ConsensusCommitPrologueTransaction {
    fn from(ccp: NativeConsensusCommitPrologueTransactionV2) -> Self {
        Self {
            epoch: ccp.epoch,
            round: ccp.round,
            commit_timestamp_ms: ccp.commit_timestamp_ms,
            consensus_commit_digest: Some(ccp.consensus_commit_digest),
        }
    }
}
