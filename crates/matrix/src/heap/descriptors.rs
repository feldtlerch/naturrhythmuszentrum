// SPDX-License-Identifier: AGPL-3.0-only
//! Legacy and managed descriptors share one occupied slot table.

use super::{
    HeapError,
    identity::Key,
    records::Record,
    slots::{PreparedSlots, SlotPlan, Slots},
};

/// Every descriptor occupation declares its resolution discipline.
#[derive(Debug)]
pub(crate) enum DescriptorRecord {
    /// Existing exclusive dense-region access.
    Legacy(Record),
    /// A managed channel never resolves to a legacy region.
    Managed(crate::loan::ManagedDescriptor),
}
impl From<Record> for DescriptorRecord {
    fn from(value: Record) -> Self {
        Self::Legacy(value)
    }
}

/// Compatibility facade over the single tagged occupation table.
#[derive(Debug)]
pub(crate) struct Descriptors {
    /// Managed reservations and legacy records occupy the same positions.
    pub(crate) slots: Slots<DescriptorRecord>,
}
impl Descriptors {
    /// Empty descriptor inventory.
    pub(crate) const fn new() -> Self {
        Self {
            slots: Slots::new(),
        }
    }
    /// Prepare a complete legacy occupation change.
    pub(crate) fn change(
        &mut self,
        count: usize,
        keep: Option<Key>,
        retired: &[Key],
        limit: usize,
    ) -> Result<SlotPlan, HeapError> {
        self.slots.change(count, keep, retired, limit)
    }
    /// Install prepared legacy payloads without allocation.
    pub(crate) fn publish(&mut self, prepared: PreparedSlots<Record>) {
        self.slots.publish(prepared);
    }
    /// Resolve only a legacy occupation, refusing managed storage before any numerical access.
    pub(crate) fn get(&self, key: Key) -> Result<&Record, HeapError> {
        match self.slots.get(key)? {
            DescriptorRecord::Legacy(record) => Ok(record),
            DescriptorRecord::Managed(_) => Err(HeapError::OwnershipPending),
        }
    }
    /// Legacy relocation visits only legacy owners.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Key, &Record)> {
        self.slots.iter().filter_map(|(key, record)| match record {
            DescriptorRecord::Legacy(record) => Some((key, record)),
            DescriptorRecord::Managed(_) => None,
        })
    }
    /// Legacy mutation cannot reach a managed channel.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (Key, &mut Record)> {
        self.slots
            .iter_mut()
            .filter_map(|(key, record)| match record {
                DescriptorRecord::Legacy(record) => Some((key, record)),
                DescriptorRecord::Managed(_) => None,
            })
    }
    /// Private corruption controls retain their legacy checked accessor.
    #[cfg(test)]
    pub(crate) fn get_mut(&mut self, key: Key) -> Result<&mut Record, HeapError> {
        match self.slots.get_mut(key)? {
            DescriptorRecord::Legacy(record) => Ok(record),
            DescriptorRecord::Managed(_) => Err(HeapError::OwnershipPending),
        }
    }
}
