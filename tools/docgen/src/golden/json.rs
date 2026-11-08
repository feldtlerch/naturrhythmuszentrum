// SPDX-License-Identifier: AGPL-3.0-only
//! JSON decoding that refuses duplicate keys instead of overwriting evidence.

use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::{Cause, Error, Limits, invalid, preflight};

/// Audit the boundary before the allocating decoder, without global test state.
#[derive(Default)]
pub(super) struct Audit {
    /// Number of attempts to construct a JSON tree after preflight.
    pub(super) tree_decodes: usize,
    /// Exact inventory of decoded rational string bytes.
    pub(super) inventory: preflight::Inventory,
}

/// A recursively duplicate-free JSON value.
struct Unique(Value);

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

/// Decode only the JSON alternatives used by this exact schema.
struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("duplicate-free exact JSON")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Unique, E> {
        Ok(Unique(Value::String(value.to_owned())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Unique, E> {
        Ok(Unique(Value::from(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Unique, E> {
        Ok(Unique(Value::from(value)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Unique, A::Error> {
        let mut values = Vec::new();
        while let Some(Unique(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(Unique(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate member"));
            }
            let Unique(value) = map.next_value()?;
            values.insert(key, value);
        }
        Ok(Unique(Value::Object(values)))
    }
}

/// Decode the complete document with serde's bounded recursion.
pub(super) fn parse(bytes: &[u8], limits: Limits, audit: &mut Audit) -> Result<Value, Error> {
    audit.inventory = preflight::check(bytes, limits)?;
    audit.tree_decodes += 1;
    serde_json::from_slice::<Unique>(bytes)
        .map(|unique| unique.0)
        .map_err(|_| invalid(Cause::Json, "$"))
}
