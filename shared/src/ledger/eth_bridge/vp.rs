//! Validity predicate for the Ethereum bridge

use std::collections::{BTreeSet, HashSet};

use eyre::eyre;

use crate::ledger::eth_bridge::storage::{self, wrapped_erc20s};
use crate::ledger::native_vp::{Ctx, NativeVp};
use crate::ledger::storage as ledger_storage;
use crate::ledger::storage::StorageHasher;
use crate::types::address::{Address, InternalAddress};
use crate::types::storage::Key;
use crate::vm::WasmCacheAccess;

/// Validity predicate for the Ethereum bridge
pub struct EthBridge<'ctx, DB, H, CA>
where
    DB: ledger_storage::DB + for<'iter> ledger_storage::DBIter<'iter>,
    H: StorageHasher,
    CA: 'static + WasmCacheAccess,
{
    /// Context to interact with the host structures.
    pub ctx: Ctx<'ctx, DB, H, CA>,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
/// Generic error that may be returned by the validity predicate
pub struct Error(#[from] eyre::Error);

impl<'a, DB, H, CA> NativeVp for EthBridge<'a, DB, H, CA>
where
    DB: 'static + ledger_storage::DB + for<'iter> ledger_storage::DBIter<'iter>,
    H: 'static + StorageHasher,
    CA: 'static + WasmCacheAccess,
{
    type Error = Error;

    const ADDR: InternalAddress = super::INTERNAL_ADDRESS;

    /// Validate that a wasm transaction is permitted to change keys under this
    /// account.
    ///
    /// We permit only the following changes via wasm for the time being:
    /// - a wrapped ERC20's supply key to decrease iff one of its balance keys
    ///   decreased by the same amount
    /// - a wrapped ERC20's balance key to decrease iff another one of its
    ///   balance keys increased by the same amount
    ///
    /// Some other changes to the storage subspace of this account are expected
    /// to happen natively i.e. bypassing this validity predicate. For example,
    /// changes to the `eth_msgs/...` keys. For those cases, we reject here as
    /// no wasm transactions should be able to modify those keys.
    fn validate_tx(
        &self,
        tx_data: &[u8],
        keys_changed: &BTreeSet<Key>,
        verifiers: &BTreeSet<Address>,
    ) -> Result<bool, Self::Error> {
        tracing::debug!(
            tx_data_len = tx_data.len(),
            keys_changed_len = keys_changed.len(),
            verifiers_len = verifiers.len(),
            "Validity predicate triggered",
        );
        validate_tx(tx_data, keys_changed, verifiers)
    }
}

/// Pure function not attached to the [`EthBridge`] struct so that it is easier
/// to test
fn validate_tx(
    _tx_data: &[u8],
    keys_changed: &BTreeSet<Key>,
    _verifiers: &BTreeSet<Address>,
) -> Result<bool, Error> {
    // we aren't concerned with keys that changed outside of our account
    let keys_changed: HashSet<_> = keys_changed
        .into_iter()
        .filter(|key| storage::is_eth_bridge_key(key))
        .collect();
    if keys_changed.is_empty() {
        return Err(Error(eyre!(
            "No keys changed under our account so this validity predicate \
             shouldn't have been triggered"
        )));
    }
    tracing::debug!(
        relevant_keys.len = keys_changed.len(),
        "Found keys changed under our account"
    );

    if keys_changed.len() != 2 {
        tracing::debug!(
            relevant_keys.len = keys_changed.len(),
            "Rejecting transaction as only two keys should have changed"
        );
        return Ok(false);
    }

    let mut keys = HashSet::<_>::default();
    for key in keys_changed.into_iter() {
        let key = match wrapped_erc20s::MultitokenKey::try_from(key) {
            Ok(key) => key,
            Err(error) => {
                tracing::debug!(
                    %key,
                    ?error,
                    "Rejecting transaction as key is not a wrapped ERC20 key"
                );
                return Ok(false);
            }
        };
        keys.insert(key);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;
    use crate::types::ethereum_events;

    const ARBITRARY_OWNER_ADDRESS: &str =
        "atest1d9khqw36x9zyxwfhgfpygv2pgc65gse4gy6rjs34gfzr2v69gy6y23zpggurjv2yx5m52sesu6r4y4";

    /// Return some arbitrary random key belonging to this account
    fn arbitrary_key() -> Key {
        let mut rng = rand::thread_rng();
        let rn = rng.gen::<u64>();
        storage::prefix()
            .push(&format!("arbitrary key segment {}", rn))
            .expect("should always be able to construct this key")
    }

    #[test]
    fn test_error_if_triggered_without_keys_changed() {
        let tx_data = vec![];
        let keys_changed = BTreeSet::new();
        let verifiers = BTreeSet::new();

        let result = validate_tx(&tx_data, &keys_changed, &verifiers);

        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_if_not_two_keys_changed() {
        let tx_data = vec![];
        let verifiers = BTreeSet::new();
        {
            let keys_changed = BTreeSet::from_iter(vec![arbitrary_key()]);

            let result = validate_tx(&tx_data, &keys_changed, &verifiers);

            assert!(matches!(result, Ok(false)));
        }
        {
            let keys_changed = BTreeSet::from_iter(vec![
                arbitrary_key(),
                arbitrary_key(),
                arbitrary_key(),
            ]);

            let result = validate_tx(&tx_data, &keys_changed, &verifiers);

            assert!(matches!(result, Ok(false)));
        }
    }

    #[test]
    fn test_rejects_if_not_two_multitoken_keys_changed() {
        let tx_data = vec![];
        let verifiers = BTreeSet::new();
        {
            let keys_changed =
                BTreeSet::from_iter(vec![arbitrary_key(), arbitrary_key()]);

            let result = validate_tx(&tx_data, &keys_changed, &verifiers);

            assert!(matches!(result, Ok(false)));
        }

        {
            let keys_changed = BTreeSet::from_iter(vec![
                arbitrary_key(),
                wrapped_erc20s::Keys::from(
                    &ethereum_events::testing::DAI_ERC20_ETH_ADDRESS,
                )
                .supply(),
            ]);

            let result = validate_tx(&tx_data, &keys_changed, &verifiers);

            assert!(matches!(result, Ok(false)));
        }

        {
            let keys_changed = BTreeSet::from_iter(vec![
                arbitrary_key(),
                wrapped_erc20s::Keys::from(
                    &ethereum_events::testing::DAI_ERC20_ETH_ADDRESS,
                )
                .balance(
                    &Address::decode(ARBITRARY_OWNER_ADDRESS)
                        .expect("Couldn't set up test"),
                ),
            ]);

            let result = validate_tx(&tx_data, &keys_changed, &verifiers);

            assert!(matches!(result, Ok(false)));
        }
    }
}
