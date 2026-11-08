// SPDX-License-Identifier: AGPL-3.0-only
//! Reprepare one complete reservation with statistical regions appended after value keys.

use super::StatisticsError;
use crate::binding::schema::{BindingSchema, intern};
use crate::heap::records::{Location, Meaning, Record};
use crate::storage::{Format, Region};
use crate::{Bound, Candidate, Extent, OperationError};
use nrz_bindings::Descriptor;
use nrz_scalar::Scalar;

/// One independently shaped statistical region awaiting the atomic transaction commit.
#[derive(Debug)]
pub(crate) struct RegionData<P: Scalar> {
    /// Parameter-specific logical shape.
    pub(crate) extent: Extent,
    /// Homogeneous schema; paired matrices retain per-entry descriptors in their owner.
    pub(crate) quantity: Option<Descriptor>,
    /// Complete actual symbolic targets for every parameter entry.
    pub(crate) declaration: nrz_bindings::ParameterSymbol,
    /// Private finite numerical payload.
    pub(crate) values: crate::private::Buffer<P>,
}

/// Preserve every reserved value key while preparing all numerical storage in one change.
pub(crate) fn append<P: Scalar + 'static, Q: Scalar + 'static>(
    bound: &mut Bound<'_, P, Q>,
    candidate: &Candidate<Q>,
    regions: &[RegionData<Q>],
) -> Result<usize, OperationError> {
    let ordinary = ordinary(candidate);
    let prefix = ordinary.clone().count();
    let payloads = ordinary.chain(regions.iter().map(|region| region.values.as_slice()));
    let count = prefix
        .checked_add(regions.len())
        .ok_or(crate::HeapError::Capacity)?;
    let mut elements = [0_usize; 2];
    elements[Format::of::<Q>().index()] = payloads.clone().try_fold(0_usize, |sum, payload| {
        sum.checked_add(payload.len())
            .ok_or(crate::HeapError::Capacity)
    })?;
    bound.heap.preflight_elements(elements, [0; 2])?;
    let plan = bound
        .heap
        .change_descriptors(count, None, &[], &bound.resource_permit)?;
    if plan.keys().get(..prefix) != Some(bound.reservations.records.keys()) {
        return Err(StatisticsError::Contract.into());
    }
    for (&key, region) in plan.keys().iter().skip(prefix).zip(regions) {
        bound
            .reservations
            .covers
            .push(bound.heap.symbols.prepare_parameter(
                key,
                region.declaration.clone(),
                bound.heap.limits.descriptors,
            )?);
    }
    // The original route's permit includes every parameter channel and both reserved slab sets.
    // Publication consumes that admission without substituting compatibility ceilings.
    let storage = bound.heap.storage.prepare_iter(
        payloads,
        &[],
        bound.heap.limits,
        &bound.resource_permit,
    )?;
    if storage.keys().get(..prefix) != Some(bound.reservations.storage.keys()) {
        return Err(StatisticsError::Contract.into());
    }
    let mut records = std::mem::take(&mut bound.reservations.records.values);
    records
        .try_reserve_exact(regions.len())
        .map_err(|_| StatisticsError::ResourceLimit)?;
    if records.capacity() > count {
        return Err(StatisticsError::ResourceLimit.into());
    }
    for (index, region) in regions.iter().enumerate() {
        let schema = intern(
            &bound.heap.meanings,
            &mut bound.reservations.schemas,
            BindingSchema {
                extent: region.extent,
                quantity: region.quantity.clone(),
            },
            bound.heap.limits.schemas,
        )?;
        let arena = *storage
            .keys()
            .get(prefix + index)
            .ok_or(StatisticsError::Contract)?;
        records.push(Record {
            schema,
            layout: crate::Layout::contiguous(region.extent)?,
            location: Location::Owned(Region {
                arena,
                epoch: storage.epoch(),
                format: Format::of::<Q>(),
                offset: 0,
                length: region.extent.entries(),
            }),
            meaning: Meaning::Statistical,
        });
    }
    crate::binding::schema::reserve(bound.heap, bound.reservations.schemas.len())?;
    bound.reservations.records = plan.bind(records)?;
    bound.reservations.storage = storage;
    Ok(prefix)
}

/// Numerical region order remains exactly the existing deterministic transaction order.
fn ordinary<P: Scalar>(candidate: &Candidate<P>) -> impl Iterator<Item = &[P]> + Clone {
    candidate.outputs.iter().flat_map(|output| {
        std::iter::once(output.centers.as_slice())
            .chain(output.bounds.as_deref())
            .chain(output.evaluation.as_deref())
            .chain(output.contributions.iter().flatten().map(Vec::as_slice))
    })
}

/// All fallible allocation is finished before a standalone cross-block publication.
pub(crate) struct Standalone {
    /// Complete private numerical covers.
    pub(crate) storage: crate::storage::PreparedStorage,
    /// Newly interned statistical schemas.
    pub(crate) schemas: Vec<BindingSchema>,
    /// Complete private descriptor occupations.
    pub(crate) records: crate::heap::slots::PreparedSlots<Record>,
    /// Complete parameter covers, prepared before allocating their floats.
    pub(crate) covers: Vec<crate::binding::symbols::RegionCover>,
}
/// Prepare a separately requested statistical block without changing visible state.
pub(crate) fn standalone<P: Scalar + 'static>(
    heap: &mut crate::Heap,
    regions: &[RegionData<P>],
    permit: &crate::heap::resources::ResourcePermit,
) -> Result<Standalone, OperationError> {
    let payloads = regions.iter().map(|region| region.values.as_slice());
    let plan = heap.change_descriptors(regions.len(), None, &[], permit)?;
    let mut covers = Vec::new();
    for (&key, region) in plan.keys().iter().zip(regions) {
        covers.push(heap.symbols.prepare_parameter(
            key,
            region.declaration.clone(),
            heap.limits.descriptors,
        )?);
    }
    let storage = heap.prepare_storage_iter(payloads, &[], permit)?;
    let mut schemas = Vec::new();
    let mut records = crate::binding::buffer(regions.len())?;
    for (region, &arena) in regions.iter().zip(storage.keys()) {
        let schema = intern(
            &heap.meanings,
            &mut schemas,
            BindingSchema {
                extent: region.extent,
                quantity: region.quantity.clone(),
            },
            heap.limits.schemas,
        )?;
        records.push(Record {
            schema,
            layout: crate::Layout::contiguous(region.extent)?,
            location: Location::Owned(Region {
                arena,
                epoch: storage.epoch(),
                format: Format::of::<P>(),
                offset: 0,
                length: region.extent.entries(),
            }),
            meaning: Meaning::Statistical,
        });
    }
    crate::binding::schema::reserve(heap, schemas.len())?;
    Ok(Standalone {
        storage,
        schemas,
        records: plan.bind(records)?,
        covers,
    })
}
