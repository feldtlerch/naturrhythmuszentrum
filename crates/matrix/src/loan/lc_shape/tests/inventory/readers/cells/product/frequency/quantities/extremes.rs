// SPDX-License-Identifier: AGPL-3.0-only
//! Tiny and large Ready quantities retain exact-rational enclosure and radius evidence.

use super::{Rational, TestResult, VALUES, local, mul, pi, publish_values, setup};
use crate::{LcFrequencyPrelude, LcFrequencyState, LcQuantityCell};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::cmp::Ordering;

/// Subnormal products and products near the upper finite range yield bounded quantities
/// in both formats, using literal pi brackets independent of the prepared factors.
/// ´test:unit:loan-lc-quantity-extreme-images´
#[test]
fn loan_lc_quantity_extreme_images() -> TestResult {
    extremes::<f32>()?;
    extremes::<f64>()
}
fn extremes<P: Scalar + 'static>() -> TestResult {
    let mut budget = Budget::new(Limits::default())?;
    let context = ArithmeticContext::new()?;
    let pi = pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    let prepared = budget.usage();
    for pair in [
        [P::min_positive_normal(), P::ONE / P::from_u8(2)],
        [P::max_finite() / P::from_u8(16), P::from_u8(2)],
    ] {
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
        let result = product.read_quantities(3, &prelude, &mut account)?;
        oracle(pair, &result)?;
        assert_eq!(result.frequency().inputs().cell(), 3);
        assert_eq!(result.frequency().positions(), [0, 1]);
        for (reading, original) in result.frequency().inputs().readings().iter().zip(values) {
            assert_eq!(reading.values(), original[0]);
        }
        assert_eq!(account.usage().work - before, 46);
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            [15, 5]
        );
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        super::oracle(
            &product.read_quantities(11, &prelude, &mut account)?,
            [0, 1],
            1,
        )?;
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    assert_eq!(budget.usage(), prepared);
    Ok(())
}
fn oracle<P: Scalar + 'static>(pair: [P; 2], result: &LcQuantityCell<'_, '_, P, 5>) -> TestResult {
    let target = mul(
        &Rational::binary(pair[0].to_f64())?,
        &Rational::binary(pair[1].to_f64())?,
    )?;
    let LcFrequencyState::Ready { tau, rate } = result.frequency().state() else {
        return Err("positive extreme quantities".into());
    };
    let values = result.values().ok_or("ready quantities")?;
    assert_eq!(values.omega, rate);
    super::super::check(tau, &target, &target, false)?;
    super::super::check(values.omega, &target, &target, true)?;
    let turn_low = Rational::fraction(6_283_185_307_179_586_476, 1_000_000_000_000_000_000)?;
    let turn_high = Rational::fraction(6_283_185_307_179_586_478, 1_000_000_000_000_000_000)?;
    let low = mul(&target, &mul(&turn_low, &turn_low)?)?;
    let high = mul(&target, &mul(&turn_high, &turn_high)?)?;
    super::super::check(values.period, &low, &high, false)?;
    super::super::check(values.frequency, &low, &high, true)?;
    let [turn, inverse] = result.prelude().factors();
    local(values.period, tau[0], turn[0])?;
    local(values.frequency, rate[0], inverse[0])?;
    for value in [values.omega, values.frequency, values.period] {
        let ceiling = mul(
            &Rational::binary(value[0].to_f64())?,
            &Rational::fraction(1, 4096)?,
        )?;
        for radius in [value[1], value[2]] {
            assert_ne!(
                Rational::binary(radius.to_f64())?.compare(&ceiling),
                Ordering::Greater
            );
        }
    }
    Ok(())
}
