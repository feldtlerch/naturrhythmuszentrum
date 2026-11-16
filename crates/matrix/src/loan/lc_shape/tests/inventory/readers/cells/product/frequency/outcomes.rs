// SPDX-License-Identifier: AGPL-3.0-only
//! Original sign obligations distinguish invalid elements from unresolved complete boxes.

use super::{Rational, TestResult, VALUES, check, publish_values, setup};
use crate::{LcFrequencyState, LcFrequencyTarget, LcProductError, LoanError};
use nrz_scalar::Scalar;
use std::cmp::Ordering;

/// Both-negative elements cannot authorize a positive product; zero and touching boxes
/// remain undecided. Changed companions at fresh occupations require fresh admission.
/// ´test:unit:loan-lc-frequency-sign-outcomes´
#[test]
fn loan_lc_frequency_sign_outcomes() -> TestResult {
    signs::<f32>()?;
    signs::<f64>()
}
fn signs<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    for case in 0..11 {
        let mut values =
            VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
        let (left, right) = sign_values::<P>(case);
        values[0][0] = left;
        values[1][0] = right;
        if case == 10 {
            values[4][0] = [-P::ONE, P::ZERO, P::ZERO];
        }
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
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
        let result = product.read_frequency(0, &mut account)?;
        sign_outcome(case, result.state());
        assert_eq!(result.product().is_some(), matches!(case, 5 | 7 | 8 | 10));
        assert_eq!(result.positions(), [0, 1]);
        assert!(std::ptr::eq(
            result.inputs().inventory(),
            &raw const inventory
        ));
        assert_eq!(result.inputs().axes(), axes);
        for (reading, original) in result.inputs().readings().iter().zip(values) {
            assert_eq!(reading.values(), original[0]);
        }
        assert_eq!(account.usage().work - before, 43);
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            [
                15,
                match case {
                    5 => 1,
                    7 | 8 | 10 => 3,
                    _ => 0,
                }
            ]
        );
        assert_eq!(result.inputs().cell(), 0);
        assert_eq!(result.inputs().readings()[0].values(), left);
        super::oracle(&product.read_frequency(11, &mut account)?, [0, 1], 1)?;
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}

fn sign_outcome<P: Scalar>(case: usize, state: LcFrequencyState<P>) {
    match case {
        0 | 2 => assert!(matches!(
            state,
            LcFrequencyState::Invalid(LcFrequencyTarget::Inductance)
        )),
        1 | 6 => assert!(matches!(
            state,
            LcFrequencyState::Invalid(LcFrequencyTarget::Capacitance)
        )),
        3 | 4 => assert!(matches!(
            state,
            LcFrequencyState::Undecided(LcFrequencyTarget::Inductance)
        )),
        9 => assert!(matches!(
            state,
            LcFrequencyState::Undecided(LcFrequencyTarget::Capacitance)
        )),
        5 => assert!(matches!(
            state,
            LcFrequencyState::Undecided(LcFrequencyTarget::Product)
        )),
        _ => assert!(matches!(state, LcFrequencyState::Ready { .. })),
    }
}

fn sign_values<P: Scalar>(case: usize) -> ([P; 3], [P; 3]) {
    let one = P::ONE;
    let two = P::from_u8(2);
    let three_quarters = P::from_u8(3) / P::from_u8(4);
    match case {
        0 => ([-one, P::ZERO, P::ZERO], [one, P::ZERO, P::ZERO]),
        1 => ([one, P::ZERO, P::ZERO], [-one, P::ZERO, P::ZERO]),
        2 => ([-one, P::ZERO, P::ZERO], [-one, P::ZERO, P::ZERO]),
        3 => ([P::ZERO, P::ZERO, P::ZERO], [one, P::ZERO, P::ZERO]),
        4 => ([one, one, P::ZERO], [one, P::ZERO, P::ZERO]),
        5 => (
            [one, three_quarters, P::ZERO],
            [one, three_quarters, P::ZERO],
        ),
        6 => ([one, two, P::ZERO], [-one, P::ZERO, P::ZERO]),
        7 => ([one, P::ZERO, P::ZERO], [two, P::ZERO, P::ZERO]),
        9 => ([one, P::ZERO, P::ZERO], [one, one, P::ZERO]),
        _ => ([one, P::ZERO, P::ZERO], [one, P::ZERO, P::ZERO]),
    }
}

/// Missing early or unused late roles remain unavailable; no placeholder reaches Product.
/// Foreign ledgers and capacity refusals preserve the complete view for other cells.
/// ´test:unit:loan-lc-frequency-availability-refusals´
#[test]
fn loan_lc_frequency_availability_refusals() -> TestResult {
    availability::<f32>()?;
    availability::<f64>()
}
fn availability<P: Scalar + 'static>() -> TestResult {
    for missing in [0, 4, 5] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let mut masks = std::array::from_fn(|_| vec![0, 3, 11]);
        if missing < 5 {
            masks[missing] = vec![3, 11];
        }
        let members = super::publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
        let references = members.references();
        let product = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?
            .into_product([0, 1], &mut account)?;
        let mut other = crate::ExecutionAccount::new(nrz_bindings::Budget::new(
            nrz_bindings::Limits::default(),
        )?)?;
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let before = [account.usage().work, other.usage().work];
        assert!(matches!(
            product.read_frequency(3, &mut other),
            Err(LcProductError::Loan(LoanError::ForeignAccount))
        ));
        assert_eq!([account.usage().work, other.usage().work], before);
        let injection = crate::loan::fault::Injection::after(0);
        let result = product.read_frequency(if missing == 5 { 12 } else { 0 }, &mut account);
        if missing == 5 {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::LayoutMismatch))
            ));
        } else {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::CoverageMismatch))
            ));
        }
        assert_eq!(account.usage().work - before[0], 43);
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            [0; 2]
        );
        super::oracle(&product.read_frequency(3, &mut account)?, [0, 1], 0)?;
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            [15, 3]
        );
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}

/// Native finite refusal and unresolved underflow stay distinct from positive nonsquare,
/// tiny and large images. Squared inverse endpoints use exact binary rationals.
/// ´test:unit:loan-lc-frequency-extreme-images´
#[test]
fn loan_lc_frequency_extreme_images() -> TestResult {
    extremes::<f32>()?;
    extremes::<f64>()
}
fn extremes<P: Scalar + 'static>() -> TestResult {
    let pairs = [
        [P::ONE, P::from_u8(2)],
        [P::min_positive_normal(), P::ONE / P::from_u8(2)],
        [P::max_finite() / P::from_u8(16), P::from_u8(2)],
        [P::max_finite(), P::from_u8(2)],
        [P::min_positive_normal(), P::min_positive_normal()],
        [-P::max_finite(), P::from_u8(2)],
        [P::from_u8(2), -P::max_finite()],
        [P::max_finite(), P::from_u8(2)],
    ];
    for (case, pair) in pairs.into_iter().enumerate() {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
        let mut values =
            VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
        for (role, center) in pair.into_iter().enumerate() {
            values[role][0] = [center, P::ZERO, P::ZERO];
        }
        if case == 7 {
            values[0][0][1] = P::max_finite();
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
        let result = product.read_frequency(3, &mut account);
        extreme_outcome(case, pair, result)?;
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            [
                15,
                match case {
                    0..3 => 3,
                    3 | 4 => 1,
                    _ => 0,
                }
            ]
        );
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        super::oracle(&product.read_frequency(11, &mut account)?, [0, 1], 1)?;
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}
fn extreme_outcome<P: Scalar>(
    case: usize,
    pair: [P; 2],
    result: Result<crate::LcFrequencyCell<'_, P, 5>, LcProductError>,
) -> TestResult {
    if case == 3 {
        assert!(matches!(
            result,
            Err(LcProductError::Kernel(crate::KernelError::BinaryDomain {
                entry: 3,
                reason: crate::DomainReason::UnboundedImage,
                ..
            }))
        ));
        return Ok(());
    }
    let result = result?;
    assert_eq!(result.inputs().cell(), 3);
    assert_eq!(result.positions(), [0, 1]);
    assert_eq!(result.product().is_some(), case < 5);
    for (role, center) in pair.into_iter().enumerate() {
        assert_eq!(result.inputs().readings()[role].values()[0], center);
    }
    match case {
        4 => assert!(matches!(
            result.state(),
            LcFrequencyState::Undecided(LcFrequencyTarget::Product)
        )),
        5 => assert!(matches!(
            result.state(),
            LcFrequencyState::Invalid(LcFrequencyTarget::Inductance)
        )),
        6 => assert!(matches!(
            result.state(),
            LcFrequencyState::Invalid(LcFrequencyTarget::Capacitance)
        )),
        7 => assert!(matches!(
            result.state(),
            LcFrequencyState::Undecided(LcFrequencyTarget::Inductance)
        )),
        _ => extreme_oracle(pair, result.state())?,
    }
    Ok(())
}
fn extreme_oracle<P: Scalar>(pair: [P; 2], state: LcFrequencyState<P>) -> TestResult {
    let target = Rational::binary(pair[0].to_f64())?.image(
        &Rational::binary(pair[1].to_f64())?,
        crate::BinaryElementwise::Product,
    )?;
    let LcFrequencyState::Ready { tau, rate } = state else {
        return Err("positive extreme chain".into());
    };
    check(tau, &target, &target, false)?;
    check(rate, &target, &target, true)?;
    for values in [tau, rate] {
        let center = Rational::binary(values[0].to_f64())?;
        let ceiling = center.image(
            &Rational::fraction(1, 4096)?,
            crate::BinaryElementwise::Product,
        )?;
        for value in [values[1], values[2]] {
            assert_ne!(
                Rational::binary(value.to_f64())?.compare(&ceiling),
                Ordering::Greater
            );
        }
    }
    Ok(())
}
