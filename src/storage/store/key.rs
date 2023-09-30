use derive_more::{Display, From};
use thiserror::Error;

use super::StorePrefix;

/// A Zarr abstract store key.
///
/// See <https://zarr-specs.readthedocs.io/en/latest/v3/core/v3.0.html#abstract-store-interface>.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub struct StoreKey(String);

/// An invalid store key.
#[derive(Debug, From, Error)]
#[error("invalid store key {0}")]
pub struct StoreKeyError(String);

/// A list of [`StoreKey`].
pub type StoreKeys = Vec<StoreKey>;

impl StoreKey {
    /// Create a new Zarr abstract store key from `key`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreKeyError`] if `key` is not valid according to [`StoreKey::validate()`].
    pub fn new(key: &str) -> Result<StoreKey, StoreKeyError> {
        if StoreKey::validate(key) {
            Ok(StoreKey(key.to_string()))
        } else {
            Err(StoreKeyError(key.to_string()))
        }
    }

    /// Create a new Zarr abstract store key from `key` without validation.
    ///
    /// # Safety
    ///
    /// `key` is not validated, so this can result in an invalid store key.
    #[must_use]
    pub unsafe fn new_unchecked(key: String) -> StoreKey {
        StoreKey(key)
    }

    /// Extracts a string slice of the underlying Key [String].
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validates a key according to the following rules from the specification:
    /// - a key is a Unicode string, where the final character is not a / character.
    #[must_use]
    pub fn validate(key: &str) -> bool {
        !key.ends_with('/')
    }

    /// Returns true if the key has prefix `prefix`.
    #[must_use]
    pub fn has_prefix(&self, prefix: &StorePrefix) -> bool {
        self.0.starts_with(prefix.as_str())
    }

    /// Convert to a [`StoreKey`].
    #[must_use]
    pub fn to_prefix(&self) -> StorePrefix {
        StorePrefix::new(&(self.0.clone() + "/")).unwrap_or_else(|_| StorePrefix::root())
    }

    /// Returns the parent of this key, or [`None`] this key is the root key and it has no parent.
    #[must_use]
    pub fn parent(&self) -> Option<StorePrefix> {
        let key_split: Vec<_> = self.as_str().split('/').collect();
        if key_split.len() > 1 {
            let parent = key_split[0..key_split.len() - 1].join("/").to_string() + "/";
            Some(unsafe { StorePrefix::new_unchecked(&parent) })
        } else {
            None
        }
    }
}

impl TryFrom<&str> for StoreKey {
    type Error = StoreKeyError;

    fn try_from(key: &str) -> Result<Self, Self::Error> {
        StoreKey::new(key)
    }
}

impl From<&StorePrefix> for StoreKey {
    fn from(prefix: &StorePrefix) -> StoreKey {
        let prefix = prefix.as_str();
        let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
        let prefix = prefix.strip_prefix('/').unwrap_or(prefix);
        unsafe { StoreKey::new_unchecked(prefix.to_string()) }
    }
}
