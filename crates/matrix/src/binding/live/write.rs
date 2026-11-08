// SPDX-License-Identifier: AGPL-3.0-only
//! Prepare numerical writes and their complete invalidation closure before an atomic commit.

use super::{LiveBindingError, ReadingBinding, admission::charge};
use crate::heap::{
    identity::Key,
    records::{Location, Meaning},
};
use crate::storage::{Format, Region};
use crate::{Heap, HeapError, ValueVersion};
use nrz_bindings::Budget;
use nrz_scalar::Scalar;

/// One private whole-region replacement and its injective selected write set.
struct Replacement<P: Scalar> {
    /// Surviving numerical occupation.
    key: Key,
    /// Complete previous allocation retired at commit.
    old: Region,
    /// Fully validated private replacement payload.
    values: Vec<P>,
    /// Injective exact entry write set.
    indices: Vec<usize>,
    /// Checked successor base revision for ordinary numerical evidence.
    version: ValueVersion,
}

impl Heap {
    /// Replace selected stored coordinates and invalidate every affected reading and proof.
    ///
    /// This writes the representation's ordered storage entries, not the physical
    /// map's output coordinates. A None representation has no writable entries.
    /// # Errors
    /// Refuses stale bindings, incomplete/nonfinite payloads, exhausted entry versions,
    /// failed dependency queries and resource limits without changing values or readers.
    pub fn replace_reading_entries<P: Scalar + 'static>(
        &mut self,
        binding: &ReadingBinding<P>,
        values: &[P],
        budget: &mut Budget,
    ) -> Result<(), LiveBindingError> {
        self.validate_reading(binding)?;
        if values.len() != binding.adapter().entries().len() || values.is_empty() {
            return Err(HeapError::LengthMismatch.into());
        }
        if binding.reading().family().node() != binding.reading().family().root() {
            if binding.adapter().representation().kind()
                != nrz_bindings::readings::RepresentationKind::ComplexBlock
            {
                return Err(HeapError::MissingCoverage.into());
            }
            super::admission::block(values)?;
        }
        replace(
            self,
            &binding.group,
            binding.adapter().entries(),
            values,
            budget,
        )
    }
}

/// The same checked transaction updates companion parameters without inventing a value reading.
pub(super) fn replace<P: Scalar + 'static>(
    heap: &mut Heap,
    group: &super::Group,
    entries: &[crate::EntryTicket<usize>],
    values: &[P],
    budget: &mut Budget,
) -> Result<(), LiveBindingError> {
    replace_with_policy(
        heap,
        group,
        entries,
        values,
        budget,
        crate::heap::resources::ResourcePolicy::compatibility(),
    )
}

/// The full numerical change is admitted before any affected base is exported.
pub(super) fn replace_with_policy<P: Scalar + 'static>(
    heap: &mut Heap,
    group: &super::Group,
    entries: &[crate::EntryTicket<usize>],
    values: &[P],
    budget: &mut Budget,
    policy: crate::heap::resources::ResourcePolicy,
) -> Result<(), LiveBindingError> {
    charge::<P>(values.len(), budget)?;
    let (permit, count) = resource_permit::<P>(heap, group, entries, policy)?;
    let replacements = prepare(heap, group, entries, values, budget, count)?;
    let writes = replacements
        .iter()
        .map(|item| (item.key, item.indices.clone()))
        .collect::<Vec<_>>();
    let invalidated = super::invalidation::prepare(heap, &writes, budget)?;
    let payloads = replacements.iter().map(|item| item.values.as_slice());
    let mut locations = crate::binding::buffer(count)?;
    #[cfg(test)]
    crate::loan::fault::entered_write_buffer(locations.capacity() * size_of::<Region>());
    locations.extend(replacements.iter().map(|item| item.old));
    let storage = heap.prepare_storage_iter(payloads, &locations, &permit)?;
    locate::<P>(
        &mut locations,
        &replacements,
        storage.keys(),
        storage.epoch(),
    )?;
    heap.storage.publish(storage);
    advance_entries(heap, &replacements);
    commit(heap, &replacements, &locations);
    super::invalidation::publish(invalidated);
    Ok(())
}

/// Enumerate distinct whole-base replacements from authenticated entry metadata only.
fn resource_permit<P: Scalar + 'static>(
    heap: &Heap,
    group: &super::Group,
    entries: &[crate::EntryTicket<usize>],
    policy: crate::heap::resources::ResourcePolicy,
) -> Result<(crate::heap::resources::ResourcePermit, usize), HeapError> {
    let mut regions = regions(heap, group, entries)?;
    let count = regions.clone().count();
    let mut plan =
        heap.storage_plan::<P>(regions.clone().map(|region| region.length), regions.clone())?;
    let copies = regions.try_fold(0_usize, |bytes, region| {
        crate::heap::resources::add(
            bytes,
            region
                .length
                .checked_mul(size_of::<P>())
                .ok_or(HeapError::Capacity)?,
        )
    })?;
    // One exact Replacement backing owns raw-copy headers. One Region backing is reused
    // from retirement through destination publication; no payload-reference vector is built.
    let headers = count
        .checked_mul(size_of::<Replacement<P>>() + size_of::<Region>())
        .ok_or(HeapError::Capacity)?;
    plan.scratch =
        crate::heap::resources::add(plan.scratch, crate::heap::resources::add(copies, headers)?)?;
    Ok((heap.preflight_resources(plan.bytes_only(), policy)?, count))
}

/// Authenticate every entry, then enumerate distinct bases without a pre-admission vector.
fn regions<'a>(
    heap: &'a Heap,
    group: &'a super::Group,
    entries: &'a [crate::EntryTicket<usize>],
) -> Result<impl Iterator<Item = Region> + Clone + 'a, HeapError> {
    for entry in entries {
        let ticket = group
            .tickets
            .get(entry.region)
            .ok_or(HeapError::InvalidLayout)?;
        if !matches!(
            heap.descriptors.get(ticket.key)?.location,
            Location::Owned(_)
        ) {
            return Err(HeapError::InvalidLayout);
        }
    }
    Ok(entries
        .iter()
        .enumerate()
        .filter_map(move |(position, entry)| {
            let ticket = group.tickets.get(entry.region)?;
            let repeated = entries[..position].iter().any(|earlier| {
                group
                    .tickets
                    .get(earlier.region)
                    .is_some_and(|previous| previous.key == ticket.key)
            });
            if repeated {
                return None;
            }
            match heap.descriptors.get(ticket.key).ok()?.location {
                Location::Owned(region) => Some(region),
                Location::Alias => None,
            }
        }))
}

/// Copy only affected bases; reject every invalid element before any publication.
fn prepare<P: Scalar + 'static>(
    heap: &Heap,
    group: &super::Group,
    entries: &[crate::EntryTicket<usize>],
    values: &[P],
    budget: &mut Budget,
    count: usize,
) -> Result<Vec<Replacement<P>>, LiveBindingError> {
    let mut replacements: Vec<Replacement<P>> = crate::binding::buffer(count)?;
    #[cfg(test)]
    crate::loan::fault::entered_write_buffer(replacements.capacity() * size_of::<Replacement<P>>());
    for (entry, &value) in entries.iter().zip(values) {
        let index = usize::try_from(entry.index).map_err(|_| HeapError::InvalidLayout)?;
        if !value.is_finite() {
            return Err(HeapError::NonFinite { index }.into());
        }
        let ticket = group
            .tickets
            .get(entry.region)
            .ok_or(HeapError::InvalidLayout)?;
        let position =
            if let Some(position) = replacements.iter().position(|item| item.key == ticket.key) {
                position
            } else {
                replacements.push(base::<P>(heap, ticket, budget)?);
                replacements.len() - 1
            };
        let replacement = replacements
            .get_mut(position)
            .ok_or(HeapError::InvalidLayout)?;
        if replacement.indices.contains(&index) {
            return Err(HeapError::OutputOverlap.into());
        }
        *replacement
            .values
            .get_mut(index)
            .ok_or(HeapError::InvalidLayout)? = value;
        replacement.indices.push(index);
        heap.symbols
            .cover(ticket.key)?
            .versions
            .get(index)
            .ok_or(HeapError::InvalidLayout)?
            .get()
            .checked_add(1)
            .ok_or(HeapError::CounterExhausted)?;
    }
    Ok(replacements)
}

/// Whole-base numeric revisions invalidate ordinary numerical evidence independently.
fn base<P: Scalar + 'static>(
    heap: &Heap,
    ticket: &super::RegionTicket,
    budget: &mut Budget,
) -> Result<Replacement<P>, LiveBindingError> {
    let handle = crate::Handle::<crate::Value, P>::new(ticket.scope, ticket.key);
    let view = heap.value(handle)?;
    charge::<P>(view.region().extent().entries(), budget)?;
    let record = heap.record(handle)?;
    if matches!(
        record.meaning,
        Meaning::Value { bound: Some(_), .. }
            | Meaning::Value {
                evaluation: Some(_),
                ..
            }
            | Meaning::Value {
                contributions: Some(_),
                ..
            }
    ) {
        return Err(HeapError::MissingCoverage.into());
    }
    let Location::Owned(old) = record.location else {
        return Err(HeapError::InvalidLayout.into());
    };
    let version = ValueVersion(
        view.stamp()
            .version()
            .0
            .checked_add(1)
            .ok_or(HeapError::CounterExhausted)?,
    );
    Ok(Replacement {
        key: ticket.key,
        old,
        values: view.region().export()?,
        indices: Vec::new(),
        version,
    })
}

/// Every selected entry version was checked for overflow in preparation.
fn advance_entries<P: Scalar>(heap: &Heap, replacements: &[Replacement<P>]) {
    for item in replacements {
        if let Ok(cover) = heap.symbols.cover(item.key) {
            for &index in &item.indices {
                if let Some(version) = cover.versions.get(index) {
                    version.set(version.get() + 1);
                }
            }
        }
    }
}

/// Match all allocated region keys before entering the allocation-free commit.
fn locate<P: Scalar + 'static>(
    locations: &mut [Region],
    replacements: &[Replacement<P>],
    arenas: &[Key],
    epoch: u64,
) -> Result<(), HeapError> {
    if replacements.len() != arenas.len() || locations.len() != arenas.len() {
        return Err(HeapError::LengthMismatch);
    }
    for ((location, item), &arena) in locations.iter_mut().zip(replacements).zip(arenas) {
        *location = Region {
            arena,
            epoch,
            format: Format::of::<P>(),
            offset: 0,
            length: item.values.len(),
        };
    }
    Ok(())
}

/// All mutable records were resolved under the same uninterrupted exclusive heap lease.
fn commit<P: Scalar>(heap: &mut Heap, replacements: &[Replacement<P>], locations: &[Region]) {
    for (key, record) in heap.descriptors.iter_mut() {
        if let Some(index) = replacements.iter().position(|item| item.key == key) {
            record.location = Location::Owned(locations[index]);
            if let Meaning::Value { stamp, .. } = &mut record.meaning {
                stamp.version = replacements[index].version;
            }
        }
    }
}
