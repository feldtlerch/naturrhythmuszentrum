// SPDX-License-Identifier: AGPL-3.0-only
//! Exact coordinate scales and original numerical error survive unary composition.

use super::{TestResult, bounded, channels, covered, fixture};
use crate::kernel_oracle::Rational;
use nrz_bindings::{Budget, Descriptor, ExponentPolicy, Limits, Role};
use nrz_matrix::{Heap, HeapLimits};

/// Exact scales compose while independent endpoint truths track inherited error.
/// ´test:integration:quantity-powers-measure-companions´
/// ´claim:matrix:quantity-powers-measure-companions´ Squaring and rooting noncoherent measures preserve exact scales and both original uncertainty channels without implicit coordinate conversion.
#[test]
fn quantity_powers_measure_companions() -> TestResult {
    let (context, [length, _, _]) = fixture(ExponentPolicy::Rational)?;
    let scaled = Descriptor::new(
        length.kind().clone(),
        crate::conversion_testing::measure(1, 1000, 0, 1)?,
        Role::Difference,
    )?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(&context, &mut heap, &scaled, 3.0_f64, 0.25, 0.125)?;
    let square =
        context.integer_power(&mut heap, &input, 2, &mut Budget::new(Limits::default())?)?;
    let mut exact_budget = Budget::new(Limits::default())?;
    assert_eq!(
        square.descriptor().measure().rational_scale()?,
        &nrz_bindings::readings::algebra::Rational::from_ratio(1, 1_000_000, &mut exact_budget)?
    );
    let [center, total, error] = channels(&heap, &square)?;
    assert_eq!(center, 9.0);
    for (denominator, endpoints, radius) in [(4, [11, 13], total), (8, [23, 25], error)] {
        for numerator in endpoints {
            let input = Rational::new(numerator, denominator)?;
            assert!(input.multiply(input)?.covered(center, radius)?);
        }
    }
    let root = context.square_root(&mut heap, &square, &mut Budget::new(Limits::default())?)?;
    assert_eq!(
        root.descriptor().measure().scale(),
        scaled.measure().scale()
    );
    let [center, total, error] = channels(&heap, &root)?;
    for (denominator, endpoints, radius) in [(4, [11, 13], total), (8, [23, 25], error)] {
        for numerator in endpoints {
            assert!(Rational::new(numerator, denominator)?.covered(center, radius)?);
        }
    }
    assert!(total >= error);
    Ok(())
}

/// Original source replacement invalidates powers, roots and converted descendants.
/// ´test:integration:quantity-powers-live-evidence´
/// ´claim:matrix:quantity-powers-live-evidence´ Unary operation records retain original descriptors and versions through compaction and descendants; raw access refuses after source replacement.
#[test]
fn quantity_powers_live_evidence() -> TestResult {
    let (context, [length, _, _]) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(&context, &mut heap, &length, 3.0_f64, 0.25, 0.125)?;
    let square =
        context.integer_power(&mut heap, &input, 2, &mut Budget::new(Limits::default())?)?;
    let record = square.operation_record(&heap)?.id();
    let root = context.square_root(&mut heap, &square, &mut Budget::new(Limits::default())?)?;
    let converted = context.convert(
        &mut heap,
        &root,
        &length,
        &mut Budget::new(Limits::default())?,
    )?;
    heap.compact()?;
    channels(&heap, &converted)?;
    let view = input.view(&heap)?;
    let version = view.stamp().version();
    let handle = heap
        .companion(view.bound().ok_or("original total")?)?
        .target();
    heap.replace_represented(handle, version, &[3.0])?;
    for result in [&square, &root, &converted] {
        let Err(fault) = result.view(&heap) else {
            panic!("changed unary original source must invalidate every descendant");
        };
        assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
        let Err(fault) = heap.value_by_symbol(result.symbol(), result.binding()) else {
            panic!("raw unary source access must refuse stale provenance");
        };
        assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    }
    assert_eq!(
        heap.historical_record(record)?
            .operation()
            .quantity_integer_exponent(),
        Some(2)
    );
    Ok(())
}

/// Negative bases preserve parity and reciprocal signs in either native precision.
/// ´test:integration:quantity-powers-negative-base-oracles´
/// ´claim:matrix:quantity-powers-negative-base-oracles´ Integer powers of negative bases enclose signed rational targets in binary32 and binary64 without imposing the square-root positivity domain.
#[test]
fn quantity_powers_negative_base_oracles() -> TestResult {
    negative::<f32>()?;
    negative::<f64>()
}

/// Compare each signed represented-center target to an independent integer fraction.
fn negative<P: nrz_scalar::Scalar + 'static>() -> TestResult {
    let (context, _) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(
        &context,
        &mut heap,
        &Descriptor::ratio(),
        P::ZERO - P::from_u8(3),
        P::ZERO,
        P::ZERO,
    )?;
    for (exponent, numerator, denominator) in
        [(-2, 1, 9), (-1, -1, 3), (0, 1, 1), (2, 9, 1), (3, -27, 1)]
    {
        let result = context.integer_power(
            &mut heap,
            &input,
            exponent,
            &mut Budget::new(Limits::default())?,
        )?;
        let [center, total, error] = channels(&heap, &result)?;
        let exact = Rational::new(numerator, denominator)?;
        assert!(covered(exact, center, total)?);
        assert!(covered(exact, center, error)?);
    }
    Ok(())
}
