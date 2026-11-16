// SPDX-License-Identifier: AGPL-3.0-only
//! Squared rational comparisons verify the positive chain without production root oracles.

mod boundaries;
mod observation;
mod outcomes;
mod quantities;
mod requested;
use super::{Rational, SELECTED, TestResult, VALUES, publish, publish_values, setup};
use crate::{LcInputProduct, LcProductError, LcRequestedCell, LcRequestedState, LoanError};
use nrz_scalar::Scalar;
use std::cmp::Ordering;

/// Exact endpoint products use literal source numerators, never production enclosures.
/// Both original L positions, separate strided layouts, empty/sparse/full masks and source
/// holes are observed through protected outside pins and fresh occupations in both formats.
/// ´test:unit:loan-lc-frequency-rational-images´
#[test]
fn loan_lc_frequency_rational_images() -> TestResult {
    images::<f32>()?;
    images::<f64>()
}
fn images<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    for mode in 0..3 {
        for shift in [0, 1] {
            let masks = std::array::from_fn(|index| match mode {
                0 => vec![],
                1 => SELECTED[index].to_vec(),
                _ => (0..12).collect(),
            });
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
                let readers = inventory.readers(&heap, &references, &mut account)?;
                let injection = crate::loan::fault::Injection::after(0);
                let product = readers
                    .into_cells(&mut account)?
                    .into_product(positions, &mut account)?;
                observe(&product, &mut account, mode, shift, positions)?;
                let full = mode == 2;
                let complete_source = shift == 1;
                if full && complete_source {
                    let result = product.read_requested(
                        11,
                        crate::LcQuantityDemand::Omega,
                        None,
                        &mut account,
                    )?;
                    assert!(std::ptr::eq(
                        result.inputs().inventory(),
                        &raw const inventory
                    ));
                    assert_eq!(result.inputs().axes(), axes);
                }
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                drop(injection);
            }
            drop(references);
            members.finish(&mut heap, &mut account)?;
        }
    }
    Ok(())
}
fn observe<P: Scalar + 'static>(
    product: &LcInputProduct<'_, P, 5>,
    account: &mut crate::ExecutionAccount,
    mode: usize,
    shift: usize,
    positions: [usize; 2],
) -> TestResult {
    for (cell, instance) in [0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1].into_iter().enumerate() {
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let full = mode == 2;
        let sparse = mode == 1;
        let intersection = [0, 3, 11].contains(&cell);
        let demanded = full || (sparse && intersection);
        let present = instance == 0 || shift == 1;
        let result = product.read_requested(cell, crate::LcQuantityDemand::Omega, None, account);
        if demanded && present {
            let result = result?;
            assert_eq!(result.positions(), positions);
            assert_eq!(result.inputs().cell(), cell);
            assert_eq!(
                result.numerical_error_scope(),
                crate::NumericalErrorScope::LocalOperation
            );
            super::super::oracle(result.inputs(), instance)?;
            oracle(&result, positions, instance)?;
            assert_eq!(
                [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                [15, 3]
            );
        } else {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::CoverageMismatch))
            ));
            assert_eq!(
                [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                [0; 2]
            );
        }
    }
    Ok(())
}
fn oracle<P: Scalar>(
    result: &LcRequestedCell<'_, '_, P, 5>,
    positions: [usize; 2],
    instance: usize,
) -> TestResult {
    let [left, right] = positions.map(|position| VALUES[position][instance]);
    let low = Rational::fraction(
        i64::from(left[0] - left[1]) * i64::from(right[0] - right[1]),
        1024,
    )?;
    let high = Rational::fraction(
        i64::from(left[0] + left[1]) * i64::from(right[0] + right[1]),
        1024,
    )?;
    let nominal = Rational::fraction(i64::from(left[0]) * i64::from(right[0]), 1024)?;
    let LcRequestedState::Ready {
        tau,
        rate: Some(rate),
        ..
    } = result.state()
    else {
        return Err("positive chain expected".into());
    };
    check(tau, &low, &high, false)?;
    check(rate, &low, &high, true)?;
    check([tau[0], tau[2], tau[2]], &nominal, &nominal, false)?;
    // Reciprocal local error targets its supplied root center, not the whole expression.
    let root_center = Rational::binary(tau[0].to_f64())?;
    let reciprocal =
        Rational::fraction(1, 1)?.image(&root_center, crate::BinaryElementwise::Quotient)?;
    let rate_center = Rational::binary(rate[0].to_f64())?;
    let error = Rational::binary(rate[2].to_f64())?;
    assert_ne!(
        rate_center.sub(&error).compare(&reciprocal),
        Ordering::Greater
    );
    assert_ne!(rate_center.add(&error).compare(&reciprocal), Ordering::Less);
    for (value, denominator) in [(tau[1], 8), (rate[1], 2), (tau[2], 65536), (rate[2], 65536)] {
        assert_ne!(
            Rational::binary(value.to_f64())?.compare(&Rational::fraction(1, denominator)?),
            Ordering::Greater
        );
    }
    Ok(())
}
fn check<P: Scalar>(values: [P; 3], low: &Rational, high: &Rational, inverse: bool) -> TestResult {
    let center = Rational::binary(values[0].to_f64())?;
    let radius = Rational::binary(values[1].to_f64())?;
    let lower = center.sub(&radius);
    let upper = center.add(&radius);
    assert_eq!(lower.compare(&Rational::fraction(0, 1)?), Ordering::Greater);
    let lower_squared = lower.image(&lower, crate::BinaryElementwise::Product)?;
    let upper_squared = upper.image(&upper, crate::BinaryElementwise::Product)?;
    if inverse {
        let one = Rational::fraction(1, 1)?;
        assert_ne!(
            lower_squared
                .image(high, crate::BinaryElementwise::Product)?
                .compare(&one),
            Ordering::Greater
        );
        assert_ne!(
            upper_squared
                .image(low, crate::BinaryElementwise::Product)?
                .compare(&one),
            Ordering::Less
        );
    } else {
        assert_ne!(lower_squared.compare(low), Ordering::Greater);
        assert_ne!(upper_squared.compare(high), Ordering::Less);
    }
    Ok(())
}
