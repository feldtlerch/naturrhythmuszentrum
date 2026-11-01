// SPDX-License-Identifier: AGPL-3.0-only
//! Exact represented affine copies require every original coordinate in the same import.

use super::{LiveBindingError, ReadingRegion, admission::AT};
use crate::{CompanionAdapter, HeapError, ReadingAdapter, ReadingCoverage};
use nrz_bindings::{
    Budget,
    readings::{
        CoordinateFamily, RepresentationKind,
        algebra::{Cas, Rational, limits::BudgetKind, readings::AffineMap},
    },
};
use nrz_scalar::Scalar;

/// Check exact binary centers against the original affine equation before any heap publication.
pub(super) fn check<P: Scalar>(
    graph: &Cas,
    coverage: &ReadingCoverage<usize>,
    companions: &[CompanionAdapter<usize>],
    regions: &[ReadingRegion<P>],
    target: &ReadingAdapter<usize>,
    budget: &mut Budget,
) -> Result<(), LiveBindingError> {
    if !identity(target) {
        return Err(HeapError::MissingCoverage.into());
    }
    for companion in companions {
        for actual in companion.targets() {
            super::super::adapters::reading_work(actual.reading(), budget)?;
            if actual.reading() == target.reading() {
                return Err(HeapError::MissingCoverage.into());
            }
        }
    }
    let map = target
        .reading()
        .family_map(graph, budget)
        .map_err(|error| match error {
            nrz_bindings::readings::ReadingError::Algebra(fault) => LiveBindingError::Exact(fault),
            _ => LiveBindingError::Heap(HeapError::MissingCoverage),
        })?;
    let retained =
        map.coefficients()
            .iter()
            .chain(map.offsets())
            .try_fold(0_u64, |sum, value| {
                sum.checked_add(bytes(value)?)
                    .ok_or(HeapError::ExtentOverflow)
            })?;
    budget.check(BudgetKind::StagedPayloadBytes, retained, AT)?;
    budget.with_retained_payload(retained, AT, |budget| {
        let family = target.reading().family();
        let mut expected = map.offsets()[0].clone();
        for (coordinate, coefficient) in map.coefficients().iter().enumerate() {
            let source = original(coverage, regions, family, coordinate, budget)?;
            let held = bytes(&expected)?;
            expected = budget.with_retained_payload(held, AT, |budget| {
                let value = rational(source, budget)?;
                budget.with_retained_payload(bytes(&value)?, AT, |budget| {
                    let product = coefficient.mul(&value, budget)?;
                    budget.with_retained_payload(bytes(&product)?, AT, |budget| {
                        Ok::<_, LiveBindingError>(expected.add(&product, budget)?)
                    })
                })
            })?;
        }
        let actual = budget.with_retained_payload(bytes(&expected)?, AT, |budget| {
            rational(value(target, regions)?, budget)
        })?;
        if expected != actual {
            return Err(LiveBindingError::UncoveredCopy);
        }
        Ok(())
    })
}

/// Scalar identity is the only interpretation this represented-copy check executes.
fn identity(adapter: &ReadingAdapter<usize>) -> bool {
    let map: &AffineMap = adapter.reading().map();
    adapter.representation().kind() == RepresentationKind::Defined
        && adapter.entries().len() == 1
        && map.coefficients() == [Rational::one()]
        && map.offsets() == [Rational::zero()]
}

/// Missing coordinates cannot disappear through a zero map coefficient; duplicate views must agree.
fn original<P: Scalar>(
    coverage: &ReadingCoverage<usize>,
    regions: &[ReadingRegion<P>],
    family: &CoordinateFamily,
    coordinate: usize,
    budget: &mut Budget,
) -> Result<P, LiveBindingError> {
    let mut found = None;
    for candidate in coverage.readings() {
        super::super::adapters::reading_work(candidate.reading(), budget)?;
        let origin = candidate.reading().family();
        if origin.node() != family.root() || !identity(candidate) {
            continue;
        }
        if origin.node() != origin.root()
            || origin.context() != family.context()
            || origin.original_entries() != family.original_entries()
        {
            return Err(HeapError::MissingCoverage.into());
        }
        let [entry] = candidate.reading().selection().indices() else {
            return Err(HeapError::MissingCoverage.into());
        };
        let index = u64::from(entry.column) * u64::from(origin.shape()[0]) + u64::from(entry.row);
        if usize::try_from(index).map_err(|_| HeapError::ExtentOverflow)? != coordinate {
            continue;
        }
        let actual = value(candidate, regions)?;
        if found.is_some_and(|previous| previous != actual) {
            return Err(LiveBindingError::UncoveredCopy);
        }
        found = Some(actual);
    }
    found.ok_or_else(|| HeapError::MissingCoverage.into())
}

/// Resolve exactly the one declared selected storage ticket.
fn value<P: Scalar>(
    adapter: &ReadingAdapter<usize>,
    regions: &[ReadingRegion<P>],
) -> Result<P, HeapError> {
    let [entry] = adapter.entries() else {
        return Err(HeapError::MissingCoverage);
    };
    let index = usize::try_from(entry.index).map_err(|_| HeapError::ExtentOverflow)?;
    regions
        .get(entry.region)
        .and_then(|region| region.values.get(index))
        .copied()
        .ok_or(HeapError::InvalidLayout)
}

/// Decode finite IEEE data as integers; no floating value becomes an exact-checker premise.
fn rational<P: Scalar>(value: P, budget: &mut Budget) -> Result<Rational, LiveBindingError> {
    budget.work(32, AT)?;
    let bits = value.to_f64().to_bits();
    let encoded = (bits >> 52) & 0x7ff;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let (magnitude, exponent) = if encoded == 0 {
        (fraction, -1074)
    } else {
        (
            fraction | (1 << 52),
            i64::try_from(encoded).map_err(|_| HeapError::ExtentOverflow)? - 1075,
        )
    };
    if magnitude == 0 {
        return Ok(Rational::zero());
    }
    let shift = magnitude.trailing_zeros();
    let exponent = exponent + i64::from(shift);
    let magnitude = i64::try_from(magnitude >> shift).map_err(|_| HeapError::ExtentOverflow)?;
    let signed = if bits >> 63 == 0 {
        magnitude
    } else {
        -magnitude
    };
    let coefficient = Rational::from_ratio(signed, 1, budget)?;
    let power = budget.with_retained_payload(bytes(&coefficient)?, AT, |budget| {
        Rational::from_ratio(2, 1, budget)?.pow(exponent, budget)
    })?;
    let retained = bytes(&coefficient)?
        .checked_add(bytes(&power)?)
        .ok_or(HeapError::ExtentOverflow)?;
    budget.with_retained_payload(retained, AT, |budget| Ok(coefficient.mul(&power, budget)?))
}

/// Reserve shared coefficient containers and rounded arbitrary-precision limbs together.
fn bytes(value: &Rational) -> Result<u64, HeapError> {
    let (numerator, denominator) = value.component_bits();
    numerator
        .div_ceil(64)
        .checked_add(denominator.div_ceil(64))
        .and_then(|limbs| limbs.checked_mul(16))
        .and_then(|limbs| limbs.checked_add(u64::try_from(size_of::<Rational>()).ok()?))
        .ok_or(HeapError::ExtentOverflow)
}
