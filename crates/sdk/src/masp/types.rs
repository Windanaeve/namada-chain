//! The public types for using the MASP tooling
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, RwLock};

use borsh_ext::BorshSerializeExt;
use masp_primitives::asset_type::AssetType;
use masp_primitives::convert::AllowedConversion;
use masp_primitives::memo::MemoBytes;
use masp_primitives::merkle_tree::MerklePath;
use masp_primitives::sapling::{
    Diversifier, Node, Note, Nullifier, ViewingKey,
};
use masp_primitives::transaction::builder::{Builder, MapBuilder};
use masp_primitives::transaction::components::sapling::builder::SaplingMetadata;
use masp_primitives::transaction::components::{I128Sum, ValueSum};
use masp_primitives::transaction::{
    builder, Authorization, Authorized, Transaction, Unauthorized,
};
use masp_primitives::zip32::{ExtendedFullViewingKey, ExtendedSpendingKey};
use masp_proofs::bellman::groth16::PreparedVerifyingKey;
use masp_proofs::bls12_381::Bls12;
use namada_core::address::Address;
use namada_core::borsh::{BorshDeserialize, BorshSerialize};
use namada_core::collections::HashMap;
use namada_core::dec::Dec;
use namada_core::hash::Hash;
use namada_core::storage::{BlockHeight, Epoch};
use namada_core::uint::Uint;
use namada_macros::BorshDeserializer;
#[cfg(feature = "migrations")]
use namada_migrations::*;
use namada_token as token;
use namada_tx::{IndexedTx, TxCommitments};
use thiserror::Error;

use crate::error::Error;
use crate::masp::{ShieldedContext, ShieldedUtils};

/// Type alias for convenience and profit
pub type IndexedNoteData = BTreeMap<IndexedTx, Transaction>;

/// Type alias for the entries of [`IndexedNoteData`] iterators
pub type IndexedNoteEntry = (IndexedTx, Transaction);

/// Represents the amount used of different conversions
pub type Conversions =
    BTreeMap<AssetType, (AllowedConversion, MerklePath<Node>, i128)>;

/// Represents the changes that were made to a list of transparent accounts
pub type TransferDelta = HashMap<Address, MaspChange>;

/// a masp amount
pub type MaspAmount = ValueSum<(Option<Epoch>, Address), token::Change>;

/// Represents the changes that were made to a list of shielded accounts
pub type TransactionDelta = HashMap<ViewingKey, I128Sum>;

/// A return type for gen_shielded_transfer
#[derive(Error, Debug)]
pub enum TransferErr {
    /// Build error for masp errors
    #[error("{0}")]
    Build(#[from] builder::Error<std::convert::Infallible>),
    /// errors
    #[error("{0}")]
    General(#[from] Error),
}

/// Represents an authorization where the Sapling bundle is authorized and the
/// transparent bundle is unauthorized.
pub struct PartialAuthorized;

impl Authorization for PartialAuthorized {
    type SaplingAuth = <Authorized as Authorization>::SaplingAuth;
    type TransparentAuth = <Unauthorized as Authorization>::TransparentAuth;
}

/// Shielded transfer
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshDeserializer)]
pub struct ShieldedTransfer {
    /// Shielded transfer builder
    pub builder: Builder<(), ExtendedFullViewingKey, ()>,
    /// MASP transaction
    pub masp_tx: Transaction,
    /// Metadata
    pub metadata: SaplingMetadata,
    /// Epoch in which the transaction was created
    pub epoch: Epoch,
}

/// Shielded pool data for a token
#[allow(missing_docs)]
#[derive(Debug, BorshSerialize, BorshDeserialize, BorshDeserializer)]
pub struct MaspTokenRewardData {
    pub name: String,
    pub address: Address,
    pub max_reward_rate: Dec,
    pub kp_gain: Dec,
    pub kd_gain: Dec,
    pub locked_amount_target: Uint,
}

/// The MASP transaction(s) found in a Namada tx.
#[derive(Debug, Clone)]
pub(crate) struct ExtractedMaspTxs(pub Vec<(TxCommitments, Transaction)>);

/// MASP verifying keys
pub struct PVKs {
    /// spend verifying key
    pub spend_vk: PreparedVerifyingKey<Bls12>,
    /// convert verifying key
    pub convert_vk: PreparedVerifyingKey<Bls12>,
    /// output verifying key
    pub output_vk: PreparedVerifyingKey<Bls12>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone)]
/// The possible sync states of the shielded context
pub enum ContextSyncStatus {
    /// The context contains only data that has been confirmed by the protocol
    Confirmed,
    /// The context contains that that has not yet been confirmed by the
    /// protocol and could end up being invalid
    Speculative,
}

/// A MASP specific amount delta.
#[derive(BorshSerialize, BorshDeserialize, BorshDeserializer, Debug, Clone)]
pub struct MaspChange {
    /// the token address
    pub asset: Address,
    /// the change in the token
    pub change: token::Change,
}

#[derive(Debug, Default)]
/// Data returned by successfully scanning a tx
///
/// This is append-only data that will be sent
/// to a [`TaskManager`] to be applied to the
/// shielded context.
pub(super) struct ScannedData {
    pub div_map: HashMap<usize, Diversifier>,
    pub memo_map: HashMap<usize, MemoBytes>,
    pub note_map: HashMap<usize, Note>,
    pub nf_map: HashMap<Nullifier, usize>,
    pub pos_map: HashMap<ViewingKey, BTreeSet<usize>>,
    pub vk_map: HashMap<usize, ViewingKey>,
    pub decrypted_note_cache: DecryptedDataCache,
}

impl ScannedData {
    /// Append `self` to a [`ShieldedContext`]
    pub(super) fn apply_to<U: ShieldedUtils>(
        mut self,
        ctx: &mut ShieldedContext<U>,
    ) {
        for (k, v) in self.note_map.drain(..) {
            ctx.note_map.insert(k, v);
        }
        for (k, v) in self.nf_map.drain(..) {
            ctx.nf_map.insert(k, v);
        }
        for (k, v) in self.pos_map.drain(..) {
            let map = ctx.pos_map.entry(k).or_default();
            for ix in v {
                map.insert(ix);
            }
        }
        for (k, v) in self.div_map.drain(..) {
            ctx.div_map.insert(k, v);
        }
        for (k, v) in self.vk_map.drain(..) {
            ctx.vk_map.insert(k, v);
        }
        for (k, v) in self.memo_map.drain(..) {
            ctx.memo_map.insert(k, v);
        }
        // NB: the `decrypted_note_cache` is not carried over
        // from `self` because it is assumed they are pointing
        // to the same underlying `Arc`
        debug_assert_eq!(
            Arc::as_ptr(&ctx.decrypted_note_cache.inner),
            Arc::as_ptr(&self.decrypted_note_cache.inner),
        );
    }

    /// Merge to different instances of `Self`.
    pub(super) fn merge(&mut self, mut other: Self) {
        for (k, v) in other.note_map.drain(..) {
            self.note_map.insert(k, v);
        }
        for (k, v) in other.nf_map.drain(..) {
            self.nf_map.insert(k, v);
        }
        for (k, v) in other.pos_map.drain(..) {
            let map = self.pos_map.entry(k).or_default();
            for ix in v {
                map.insert(ix);
            }
        }
        for (k, v) in other.div_map.drain(..) {
            self.div_map.insert(k, v);
        }
        for (k, v) in other.vk_map.drain(..) {
            self.vk_map.insert(k, v);
        }
        for (k, v) in other.memo_map.drain(..) {
            self.memo_map.insert(k, v);
        }
        // NB: the `decrypted_note_cache` is not carried over
        // from `other` because it is assumed they are pointing
        // to the same underlying `Arc`
        debug_assert_eq!(
            Arc::as_ptr(&other.decrypted_note_cache.inner),
            Arc::as_ptr(&self.decrypted_note_cache.inner),
        );
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
/// Data extracted from a successfully decrypted MASP note
///
/// These will be cached until the trial-decryption phase
/// of shielded-sync has finished. Then they will be
/// re-scanned as part of nullifying spent notes (which
/// is not parallelizable).
pub struct DecryptedData {
    /// The actual transaction
    pub tx: Transaction,
    /// balance changes from the tx
    pub delta: TransactionDelta,
}

/// A cache of decrypted txs that have not yet been
/// updated to the shielded ctx. Necessary in case
/// scanning gets interrupted.
#[derive(Debug, Clone, Default)]
#[allow(clippy::type_complexity)]
pub struct DecryptedDataCache {
    inner: Arc<RwLock<HashMap<Hash, (IndexedTx, ViewingKey, DecryptedData)>>>,
}

impl BorshSerialize for DecryptedDataCache {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let locked = self.inner.read().unwrap();
        locked.serialize(writer)
    }
}

impl BorshDeserialize for DecryptedDataCache {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let inner = BorshDeserialize::deserialize_reader(reader)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(inner)),
        })
    }
}

impl DecryptedDataCache {
    /// Add an entry to the cache
    pub fn insert(
        &self,
        indexed_tx: IndexedTx,
        viewing_key: ViewingKey,
        decrypted_data: DecryptedData,
    ) {
        let mut locked = self.inner.write().unwrap();
        let key = Hash::sha256_borsh(&(&indexed_tx, &viewing_key));
        let value = (indexed_tx, viewing_key, decrypted_data);
        locked.insert(key, value);
    }

    /// Check if the cache already contains an entry for a given IndexedTx and
    /// viewing key.
    pub fn contains(
        &self,
        indexed_tx: &IndexedTx,
        viewing_key: &ViewingKey,
    ) -> bool {
        let key = Hash::sha256_borsh(&(&indexed_tx, &viewing_key));
        let locked = self.inner.read().unwrap();
        locked.contains_key(&key)
    }

    /// Return an iterator over the cache that consumes it.
    pub fn drain(
        &self,
    ) -> impl Iterator<Item = (IndexedTx, ViewingKey, DecryptedData)> {
        let mut locked = self.inner.write().unwrap();
        std::mem::take(&mut *locked).into_values()
    }
}

/// A cache of fetched indexed transactions.
///
/// An invariant that shielded-sync maintains is that
/// this cache either contains all transactions from
/// a given height, or none.
#[derive(Debug, Default, Clone)]
pub struct Unscanned {
    pub(super) txs: Arc<Mutex<IndexedNoteData>>,
}

impl BorshSerialize for Unscanned {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let locked = self.txs.lock().unwrap();
        let bytes = locked.serialize_to_vec();
        writer.write(&bytes).map(|_| ())
    }
}

impl BorshDeserialize for Unscanned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let unscanned = IndexedNoteData::deserialize_reader(reader)?;
        Ok(Self {
            txs: Arc::new(Mutex::new(unscanned)),
        })
    }
}

impl Unscanned {
    /// Append elements to the cache from an iterator.
    pub fn extend<I>(&self, items: I)
    where
        I: IntoIterator<Item = IndexedNoteEntry>,
    {
        let mut locked = self.txs.lock().unwrap();
        locked.extend(items);
    }

    /// Add a single entry to the cache.
    pub fn insert(&self, (k, v): IndexedNoteEntry) {
        let mut locked = self.txs.lock().unwrap();
        locked.insert(k, v);
    }

    /// Check if this cache has already been populated for a given
    /// block height.
    pub fn contains_height(&self, height: u64) -> bool {
        let locked = self.txs.lock().unwrap();
        locked.keys().any(|k| k.height.0 == height)
    }

    /// We remove all indices from blocks that have been entirely scanned.
    /// If a block is only partially scanned, we leave all the events in the
    /// cache.
    pub fn scanned(&self, ix: &IndexedTx) {
        let mut locked = self.txs.lock().unwrap();
        locked.retain(|i, _| i.height >= ix.height);
    }

    /// Gets the latest block height present in the cache
    pub fn latest_height(&self) -> BlockHeight {
        let txs = self.txs.lock().unwrap();
        txs.keys()
            .max_by_key(|ix| ix.height)
            .map(|ix| ix.height)
            .unwrap_or_default()
    }

    /// Gets the first block height present in the cache
    pub fn first_height(&self) -> BlockHeight {
        let txs = self.txs.lock().unwrap();
        txs.keys()
            .min_by_key(|ix| ix.height)
            .map(|ix| ix.height)
            .unwrap_or_default()
    }

    /// Remove the first entry from the cache and return it.
    pub fn pop_first(&self) -> Option<IndexedNoteEntry> {
        let mut locked = self.txs.lock().unwrap();
        locked.pop_first()
    }
}

impl IntoIterator for Unscanned {
    type IntoIter = <IndexedNoteData as IntoIterator>::IntoIter;
    type Item = IndexedNoteEntry;

    fn into_iter(self) -> Self::IntoIter {
        let txs = {
            let mut txs: IndexedNoteData = Default::default();
            let mut locked = self.txs.lock().unwrap();
            std::mem::swap(&mut txs, &mut locked);
            txs
        };
        txs.into_iter()
    }
}

/// Freeze a Builder into the format necessary for inclusion in a Tx. This is
/// the format used by hardware wallets to validate a MASP Transaction.
pub(super) struct WalletMap;

impl<P1>
    masp_primitives::transaction::components::sapling::builder::MapBuilder<
        P1,
        ExtendedSpendingKey,
        (),
        ExtendedFullViewingKey,
    > for WalletMap
{
    fn map_params(&self, _s: P1) {}

    fn map_key(&self, s: ExtendedSpendingKey) -> ExtendedFullViewingKey {
        (&s).into()
    }
}

impl<P1, N1>
    MapBuilder<P1, ExtendedSpendingKey, N1, (), ExtendedFullViewingKey, ()>
    for WalletMap
{
    fn map_notifier(&self, _s: N1) {}
}
