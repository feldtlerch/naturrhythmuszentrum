// SPDX-License-Identifier: AGPL-3.0-only
//! Rational LC products preserve complete tuple authority and explicit repeated-role choices.

mod boundaries;
mod refusals;
use super::{SELECTED, TestResult, VALUES, publish, publish_values, setup};
use crate::loan::application::tests::binary::oracle::Rational;
use crate::{LcInputProduct, LcProductError, LoanError};
use nrz_scalar::Scalar;
use std::cmp::Ordering;

/// Exact endpoint products use literal source numerators, never production enclosures.
/// Both original L positions, separate strided layouts, empty/sparse/full masks and source
/// holes are observed through protected outside pins and fresh occupations in both formats.
/// ´test:unit:loan-lc-input-product-rational-images´
#[test]
fn loan_lc_input_product_rational_images() -> TestResult {
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
                    let result = product.read_cell(11, &mut account)?;
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
        let result = product.read_cell(cell, account);
        if demanded && present {
            let result = result?;
            assert_eq!(result.positions(), positions);
            assert_eq!(result.inputs().cell(), cell);
            assert_eq!(
                result.numerical_error_scope(),
                crate::NumericalErrorScope::LocalOperation
            );
            super::oracle(result.inputs(), instance)?;
            oracle(result.values(), positions, instance)?;
            assert_eq!(
                [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                [15, 1]
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
fn oracle<P: Scalar>(values: [P; 3], positions: [usize; 2], instance: usize) -> TestResult {
    let [left, right] = positions.map(|position| VALUES[position][instance]);
    let [center, total, error] = values.map(|value| Rational::binary(value.to_f64()));
    let (center, total, error) = (center?, total?, error?);
    let nominal = Rational::fraction(i64::from(left[0]) * i64::from(right[0]), 1024)?;
    assert_eq!(center.compare(&nominal), Ordering::Equal);
    let lower = Rational::fraction(
        i64::from(left[0] - left[1]) * i64::from(right[0] - right[1]),
        1024,
    )?;
    let upper = Rational::fraction(
        i64::from(left[0] + left[1]) * i64::from(right[0] + right[1]),
        1024,
    )?;
    assert_ne!(center.sub(&total).compare(&lower), Ordering::Greater);
    assert_ne!(center.add(&total).compare(&upper), Ordering::Less);
    // Outward scalar operations allow at most 1/65536 beyond the exact maximum radius.
    let ceiling = upper.sub(&nominal).add(&Rational::fraction(1, 65536)?);
    assert_ne!(total.compare(&ceiling), Ordering::Greater);
    assert_ne!(
        error.compare(&Rational::fraction(1, 65536)?),
        Ordering::Greater
    );
    Ok(())
}

/// Exact binary rationals bound tiny, large and inexact nominal products independently.
/// Finite-image refusal retains tuple attribution and permits a later complete-cell retry.
/// These numeric-domain controls deliberately do not assert original-support satisfaction.
/// ´test:unit:loan-lc-input-product-extreme-images´
#[test]
fn loan_lc_input_product_extreme_images() -> TestResult {
    extremes::<f32>()?;
    extremes::<f64>()
}
fn extremes<P: Scalar + 'static>() -> TestResult {
    let pairs = [
        [P::min_positive_normal(), P::ONE / P::from_u8(2)],
        [P::max_finite() / P::from_u8(16), P::from_u8(2)],
        [P::ONE.next_up(), P::ONE.next_up()],
        [P::max_finite(), P::from_u8(2)],
    ];
    for (index, pair) in pairs.into_iter().enumerate() {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
        let mut values =
            VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
        for (role, center) in pair.into_iter().enumerate() {
            values[role][0] = [center, P::ZERO, P::ZERO];
        }
        let members = publish_values(
            &inventory,
            &axes,
            &mut heap,
            &mut account,
            &masks,
            1,
            &values,
        )?;
        let references = members.references();
        let product = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?
            .into_product([0, 1], &mut account)?;
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let injection = crate::loan::fault::Injection::after(0);
        let before = account.usage().work;
        let result = product.read_cell(3, &mut account);
        if index == 3 {
            assert!(matches!(
                result,
                Err(LcProductError::Kernel(crate::KernelError::BinaryDomain {
                    function: crate::BinaryElementwise::Product,
                    entry: 3,
                    reason: crate::DomainReason::UnboundedImage
                }))
            ));
        } else {
            let result = result?;
            exact_nominal(pair, result.values())?;
            nearest_center::<P>(index, result.values()[0])?;
        }
        assert_eq!(account.usage().work - before, 37);
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            [15, 1]
        );
        oracle(product.read_cell(11, &mut account)?.values(), [0, 1], 1)?;
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}
fn exact_nominal<P: Scalar>(pair: [P; 2], values: [P; 3]) -> TestResult {
    let exact = Rational::binary(pair[0].to_f64())?.image(
        &Rational::binary(pair[1].to_f64())?,
        crate::BinaryElementwise::Product,
    )?;
    let center = Rational::binary(values[0].to_f64())?;
    // A loose but independent 1/65536 relative ceiling bounds accumulated outward slack.
    let ceiling = exact.image(
        &Rational::fraction(1, 65536)?,
        crate::BinaryElementwise::Product,
    )?;
    for value in [values[1], values[2]] {
        let radius = Rational::binary(value.to_f64())?;
        assert_ne!(center.sub(&radius).compare(&exact), Ordering::Greater);
        assert_ne!(center.add(&radius).compare(&exact), Ordering::Less);
        assert_ne!(radius.compare(&ceiling), Ordering::Greater);
    }
    Ok(())
}

fn nearest_center<P: Scalar>(index: usize, value: P) -> TestResult {
    let expected = match index {
        0 => Rational::binary(P::min_positive_normal().to_f64())?.image(
            &Rational::fraction(1, 2)?,
            crate::BinaryElementwise::Product,
        )?,
        1 => Rational::binary(P::max_finite().to_f64())?.image(
            &Rational::fraction(1, 8)?,
            crate::BinaryElementwise::Product,
        )?,
        _ => {
            let denominator = if P::PRECISION == nrz_scalar::Precision::F32 {
                4_194_304
            } else {
                2_251_799_813_685_248
            };
            Rational::fraction(1, 1)?.add(&Rational::fraction(1, denominator)?)
        }
    };
    assert_eq!(
        Rational::binary(value.to_f64())?.compare(&expected),
        Ordering::Equal
    );
    Ok(())
}
