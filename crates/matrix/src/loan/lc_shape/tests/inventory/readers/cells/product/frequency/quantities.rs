// SPDX-License-Identifier: AGPL-3.0-only
//! Independent literal pi brackets and rational products check shared local quantities.

mod boundaries;
mod extremes;
use super::{Rational, TestResult, VALUES, publish, publish_values, setup};
use crate::{LcFrequencyPrelude, LcFrequencyState, LcQuantityCell, PiConstant};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Finite, PiLimits, Precision, Scalar};
use std::cmp::Ordering;

/// Both formats reuse one checked pi owner and factors across independently admitted tiles,
/// repeated selected roles, sparse cells and changed input companions without per-cell preparation.
/// ´test:unit:loan-lc-quantity-rational-images´
#[test]
fn loan_lc_quantity_rational_images() -> TestResult {
    images::<f32>()?;
    images::<f64>()
}
fn images<P: Scalar + 'static>() -> TestResult {
    let mut budget = Budget::new(Limits::default())?;
    let context = ArithmeticContext::new()?;
    let pi = pi::<P>(&context, &mut budget)?;
    let before = budget.usage().work;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    assert_eq!(budget.usage().work - before, 5);
    assert!(prelude.pi().same_region(&pi));
    let prepared = budget.usage();
    for shift in [0, 1] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
        let mut members = publish(&inventory, &axes, &mut heap, &mut account, &masks, shift)?;
        let mut outside = Vec::new();
        for tile in &members.tiles {
            outside.push(tile.retain(&mut heap, &mut account)?);
        }
        members.tiles = outside;
        assert!(matches!(
            heap.residence(&mut account, 100_000)?,
            crate::Residence::Pending { .. }
        ));
        let references = members.references();
        for positions in [[0, 1], [4, 1]] {
            let cells = inventory
                .readers(&heap, &references, &mut account)?
                .into_cells(&mut account)?
                .into_product(positions, &mut account)?;
            let injection = crate::loan::fault::Injection::after(0);
            observe(&cells, &prelude, &mut account, positions, shift)?;
            let result = cells.read_quantities(0, &prelude, &mut account)?;
            assert!(std::ptr::eq(
                result.frequency().inputs().inventory(),
                &raw const inventory
            ));
            assert_eq!(result.frequency().inputs().axes(), axes);
            assert_eq!(crate::loan::fault::Injection::attempts(), 0);
            drop(injection);
        }
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    assert_eq!(budget.usage(), prepared);
    Ok(())
}
fn observe<P: Scalar + 'static>(
    cells: &crate::LcInputProduct<'_, P, 5>,
    prelude: &LcFrequencyPrelude<P>,
    account: &mut crate::ExecutionAccount,
    positions: [usize; 2],
    shift: usize,
) -> TestResult {
    for (cell, instance) in [(0, 0), (3, 0), (11, 1)] {
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let before = account.usage().work;
        let result = cells.read_quantities(cell, prelude, account);
        assert_eq!(account.usage().work - before, 46);
        if instance == 1 && shift == 0 {
            assert!(matches!(
                result,
                Err(crate::LcProductError::Loan(
                    crate::LoanError::CoverageMismatch
                ))
            ));
            assert_eq!(
                [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                [0, 0]
            );
        } else {
            let result = result?;
            assert!(std::ptr::eq(result.prelude(), std::ptr::from_ref(prelude)));
            assert_eq!(result.frequency().inputs().cell(), cell);
            assert_eq!(result.frequency().positions(), positions);
            super::super::super::oracle(result.frequency().inputs(), instance)?;
            oracle(&result, positions, instance)?;
            assert_eq!(
                [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                [15, 5]
            );
        }
    }
    Ok(())
}
fn oracle<P: Scalar + 'static>(
    result: &LcQuantityCell<'_, '_, P, 5>,
    positions: [usize; 2],
    instance: usize,
) -> TestResult {
    super::oracle(result.frequency(), positions, instance)?;
    let values = result.values().ok_or("ready quantities")?;
    let LcFrequencyState::Ready { tau, rate } = result.frequency().state() else {
        return Err("ready chain".into());
    };
    assert_eq!(values.omega, rate);
    assert_eq!(
        result.numerical_error_scope(),
        crate::NumericalErrorScope::LocalOperation
    );
    let [left, right] = positions.map(|position| VALUES[position][instance]);
    let low = Rational::fraction(
        i64::from(left[0] - left[1]) * i64::from(right[0] - right[1]),
        1024,
    )?;
    let high = Rational::fraction(
        i64::from(left[0] + left[1]) * i64::from(right[0] + right[1]),
        1024,
    )?;
    // Independent decimal rational bounds on pi; no production constant fields form this oracle.
    let turn_low = Rational::fraction(6_283_185_307_179_586_476, 1_000_000_000_000_000_000)?;
    let turn_high = Rational::fraction(6_283_185_307_179_586_478, 1_000_000_000_000_000_000)?;
    let low = mul(&low, &mul(&turn_low, &turn_low)?)?;
    let high = mul(&high, &mul(&turn_high, &turn_high)?)?;
    super::check(values.period, &low, &high, false)?;
    super::check(values.frequency, &low, &high, true)?;
    let [turn, inverse] = result.prelude().factors();
    local(values.period, tau[0], turn[0])?;
    local(values.frequency, rate[0], inverse[0])?;
    for (value, numerator, denominator) in [
        (values.period[1], 2, 1),
        (values.frequency[1], 1, 4),
        (values.period[2], 1, 4096),
        (values.frequency[2], 1, 4096),
    ] {
        assert_ne!(
            Rational::binary(value.to_f64())?.compare(&Rational::fraction(numerator, denominator)?),
            Ordering::Greater
        );
    }
    Ok(())
}
fn mul(left: &Rational, right: &Rational) -> TestResult<Rational> {
    left.image(right, crate::BinaryElementwise::Product)
}
fn local<P: Scalar>(values: [P; 3], left: P, right: P) -> TestResult {
    let exact = mul(
        &Rational::binary(left.to_f64())?,
        &Rational::binary(right.to_f64())?,
    )?;
    let center = Rational::binary(values[0].to_f64())?;
    let radius = Rational::binary(values[2].to_f64())?;
    assert_ne!(center.sub(&radius).compare(&exact), Ordering::Greater);
    assert_ne!(center.add(&radius).compare(&exact), Ordering::Less);
    Ok(())
}
/// Shared bounded pi fixture for quantity and sign-observation tests.
pub(super) fn pi<P: Scalar + 'static>(
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> TestResult<PiConstant<P>> {
    let (tolerance, limits) = match P::PRECISION {
        Precision::F32 => (
            "0.0000003",
            PiLimits {
                work: 668,
                steps: 5,
                staged_bytes: 240,
                output_bytes: 64,
            },
        ),
        Precision::F64 => (
            "0.0000000000000005",
            PiLimits {
                work: 2054,
                steps: 11,
                staged_bytes: 256,
                output_bytes: 80,
            },
        ),
    };
    Ok(PiConstant::new(
        Finite::new(P::parse_nearest(tolerance).ok_or("tolerance")?)?,
        limits,
        context,
        budget,
    )?)
}
