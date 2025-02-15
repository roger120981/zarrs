use derive_more::Display;
use serde::{Deserialize, Serialize};

use super::AdditionalFields;

/// Zarr group metadata (storage specification v3).
///
/// See <https://zarr-specs.readthedocs.io/en/latest/v3/core/v3.0.html#group-metadata>.
///
/// An example `JSON` document for an explicit Zarr V3 group:
/// ```json
/// {
///     "zarr_format": 3,
///     "node_type": "group",
///     "attributes": {
///         "spam": "ham",
///         "eggs": 42,
///     }
/// }
#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, Display)]
#[display("{}", serde_json::to_string(self).unwrap_or_default())]
pub struct GroupMetadataV3 {
    /// An integer defining the version of the storage specification to which the group adheres. Must be `3`.
    pub zarr_format: monostate::MustBe!(3u64),
    /// A string defining the type of hierarchy node element, must be `group` here.
    pub node_type: monostate::MustBe!("group"),
    /// Optional user metadata.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub attributes: serde_json::Map<String, serde_json::Value>,
    /// Additional fields.
    #[serde(flatten)]
    pub additional_fields: AdditionalFields,
}

impl Default for GroupMetadataV3 {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupMetadataV3 {
    /// Create Zarr V3 group metadata.
    #[must_use]
    pub fn new() -> Self {
        Self {
            zarr_format: monostate::MustBe!(3u64),
            node_type: monostate::MustBe!("group"),
            attributes: serde_json::Map::new(),
            additional_fields: AdditionalFields::default(),
        }
    }

    /// Set the user attributes.
    #[must_use]
    pub fn with_attributes(
        mut self,
        attributes: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        self.attributes = attributes;
        self
    }

    /// Set the additional fields.
    #[must_use]
    pub fn with_additional_fields(mut self, additional_fields: AdditionalFields) -> Self {
        self.additional_fields = additional_fields;
        self
    }
}
