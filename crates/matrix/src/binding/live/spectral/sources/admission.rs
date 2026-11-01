// SPDX-License-Identifier: AGPL-3.0-only
//! Exact source ancestry and actual selected owners are authenticated independently of centers.

use super::{AT, Error, Heap, Input, LcRootJacobian, LcSourceBinding, LcSpectrum};
use crate::{CompanionRef, HeapError, JointLimits, ReadingBinding, ReciprocalDerivativeEvaluation};
use nrz_bindings::{
    Budget,
    readings::{ReadingSourceMap, algebra::limits::BudgetKind},
};
use nrz_scalar::Scalar;
use std::rc::Rc;

/// The exact maps target the original physical inputs and the same evaluated derivative.
pub(super) fn targets<P: Scalar + 'static>(
    spectrum: &LcSpectrum<P>,
    derivative: &ReciprocalDerivativeEvaluation<P>,
    map: &ReadingSourceMap,
    budget: &mut Budget,
) -> Result<(), Error> {
    if spectrum.extension().is_some() || derivative.inputs().len() != 2 {
        return Err(Error::Identity);
    }
    for ((input, actual), expected) in spectrum
        .inputs()
        .iter()
        .zip(derivative.inputs())
        .zip(map.targets())
    {
        budget.work(1, AT)?;
        let selected = Rc::ptr_eq(&input.group, &actual.group) && input.index == actual.index;
        if !selected || input.reading() != expected {
            return Err(Error::Identity);
        }
    }
    if map.maps().iter().any(|map| map.outputs() != 1) {
        return Err(Error::Identity);
    }
    Ok(())
}

/// Source identities bind through the actual joint targets, not through equal numeric values.
pub(super) fn sources<P: Scalar + 'static>(
    heap: &Heap,
    spectrum: &LcSpectrum<P>,
    map: &ReadingSourceMap,
    sources: &[ReadingBinding<P>],
    companion: CompanionRef<P>,
    budget: &mut Budget,
) -> Result<Vec<usize>, Error> {
    if sources.len() != map.sources().len() {
        return Err(Error::Identity);
    }
    let view = heap.statistical(companion)?;
    if view.model().sources().len() != sources.len() {
        return Err(Error::Identity);
    }
    let entries = sources
        .len()
        .checked_mul(sources.len())
        .ok_or(HeapError::ExtentOverflow)?;
    let limits = JointLimits::new(sources.len(), entries, entries);
    budget.work(
        u64::try_from(entries).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    budget.check(
        BudgetKind::MatrixEntries,
        u64::try_from(entries).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    let bytes = sources
        .len()
        .checked_mul(size_of::<usize>())
        .ok_or(HeapError::ExtentOverflow)?;
    budget.check(
        BudgetKind::StagedPayloadBytes,
        u64::try_from(bytes).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    let mut columns = crate::binding::buffer(sources.len())?;
    for (source, expected) in sources.iter().zip(map.sources()) {
        budget.work(1, AT)?;
        heap.validate_reading(source)?;
        if source.reading() != expected.reading() {
            return Err(Error::Identity);
        }
        linked(spectrum, source)?;
        let column = column(heap, source, companion, budget)?;
        if columns.contains(&column) {
            return Err(Error::Identity);
        }
        columns.push(column);
    }
    view.model().require_blocks(&columns, &columns, limits)?;
    Ok(columns)
}

/// Exact family identity alone cannot authorize an independently imported equal-center source.
fn linked<P: Scalar>(spectrum: &LcSpectrum<P>, source: &ReadingBinding<P>) -> Result<(), Error> {
    for target in spectrum.inputs() {
        if target.reading().family().root() != source.reading().family().root() {
            continue;
        }
        let direct = Rc::ptr_eq(&target.group, &source.group);
        let copied = target
            .group
            .antecedents
            .iter()
            .any(|ancestor| Rc::ptr_eq(&ancestor.group, &source.group));
        if !direct && !copied {
            return Err(Error::Identity);
        }
    }
    Ok(())
}

/// A scalar original source may be an overlapping component of either homogeneous joint target.
fn column<P: Scalar + 'static>(
    heap: &Heap,
    source: &ReadingBinding<P>,
    companion: CompanionRef<P>,
    budget: &mut Budget,
) -> Result<usize, Error> {
    let [entry] = source.adapter().entries() else {
        return Err(Error::Identity);
    };
    let record = heap
        .joint
        .records
        .get(companion.index)
        .ok_or(Error::Identity)?;
    let mut found = None;
    for (side, target) in record.targets.iter().enumerate() {
        let selected = heap.symbols.selection(target.key).ok_or(Error::Identity)?;
        if !Rc::ptr_eq(&selected.group, &source.group) {
            continue;
        }
        let adapter = selected
            .group
            .coverage
            .readings()
            .get(selected.index)
            .ok_or(Error::Identity)?;
        let region = adapter.entries().first().ok_or(Error::Identity)?.region;
        for position in 0..target.layout.extent().entries() {
            budget.work(1, AT)?;
            let address = target.layout.flat_index(position)?;
            if entry.region != region
                || u64::try_from(address).map_err(|_| HeapError::ExtentOverflow)? != entry.index
            {
                continue;
            }
            let column = *record.selections[side]
                .get(position)
                .ok_or(Error::Identity)?;
            if found.is_some_and(|prior| prior != column) {
                return Err(Error::Identity);
            }
            found = Some(column);
        }
    }
    found.ok_or(Error::Identity)
}

/// Separate new owner allowance retains the old native derivative and exact source map owners.
pub(super) fn storage<P: Scalar + 'static>(
    spectrum: &LcSpectrum<P>,
    derivative: &ReciprocalDerivativeEvaluation<P>,
    exact: &LcRootJacobian,
    map: &ReadingSourceMap,
    count: usize,
    budget: &Budget,
) -> Result<u64, Error> {
    let metadata = size_of::<Input<P>>() + size_of::<LcSourceBinding<P>>() + 2 * size_of::<usize>();
    let selections = count
        .checked_mul(size_of::<ReadingBinding<P>>() + size_of::<usize>())
        .ok_or(HeapError::ExtentOverflow)?;
    let mut bytes = u64::try_from(
        metadata
            .checked_add(selections)
            .ok_or(HeapError::ExtentOverflow)?,
    )
    .map_err(|_| HeapError::ExtentOverflow)?;
    for owner in [
        spectrum.retained_payload_bytes(),
        derivative.native_payload_bytes()?,
        exact.retained_payload_bytes(),
        map.retained_payload_bytes(),
    ] {
        bytes = bytes.checked_add(owner).ok_or(HeapError::ExtentOverflow)?;
    }
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    Ok(bytes)
}
