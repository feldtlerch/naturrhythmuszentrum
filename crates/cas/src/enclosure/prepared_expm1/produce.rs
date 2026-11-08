// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded proposals; none of these calculations grants checked authority.

use super::{AT, Counts, Proof, Stage, StageKind, count, math};
use crate::budget::Budget;
use crate::{Fault, Rational};
use std::cmp::Ordering;

/// Construct a candidate from factorial recurrences and upper-rounded error bounds.
pub(super) fn candidate(
    scope: Vec<u8>,
    degree: u32,
    bits: u32,
    budget: &mut Budget,
    counts: &mut Counts,
) -> Result<Proof, Fault> {
    math::capacity(degree, budget)?;
    let reserved = super::workspace_bytes(degree, budget).saturating_add(scope.len() as u64);
    budget.with_retained_payload(reserved, AT, |budget| {
        budget.work(u64::from(degree).saturating_mul(2), AT)?;
        count(&mut counts.proposals, "proof proposal")?;
        let mut coefficients = Vec::new();
        let mut ideals = Vec::new();
        coefficients
            .try_reserve_exact(degree as usize)
            .map_err(|_| Fault::allocation(AT))?;
        ideals
            .try_reserve_exact(degree as usize)
            .map_err(|_| Fault::allocation(AT))?;
        let mut ideal = Rational::one();
        for index in 1..=degree {
            budget.work(1, AT)?;
            ideal = ideal.div(&Rational::from_ratio(i64::from(index), 1, budget)?, budget)?;
            count(&mut counts.coefficients, "coefficient")?;
            coefficients.push(math::coefficient(&ideal, bits, budget)?);
            ideals.push(ideal.clone());
        }
        let u = math::two_power(-53, budget)?;
        let two = Rational::from_ratio(2, 1, budget)?;
        let three = Rational::from_ratio(3, 1, budget)?;
        let (stages, magnitude, error) = rounded_stages(&coefficients, &ideals, budget)?;
        let last = degree as usize - 1;
        let omitted = ideals[last].div(
            &Rational::from_ratio(i64::from(degree) + 1, 1, budget)?,
            budget,
        )?;
        let tail_per_alpha = math::two_power(i64::from(degree), budget)?.mul(&omitted, budget)?;
        let absolute_tail = two.mul(&tail_per_alpha, budget)?;
        let relative_tail = three.mul(&tail_per_alpha, budget)?;
        let import_ratio = two.mul(&u, budget)?;
        let alpha_ratio = Rational::one().add(&import_ratio, budget)?;
        let arithmetic = error.add(&u.mul(&magnitude.add(&error, budget)?, budget)?, budget)?;
        let central_ratio = three.mul(&u, budget)?.add(
            &alpha_ratio.mul(&arithmetic.add(&tail_per_alpha, budget)?, budget)?,
            budget,
        )?;
        let needed = central_ratio.div(&Rational::one().sub(&import_ratio, budget)?, budget)?;
        let mut radius_bits = 0;
        for bit in 1..=1074 {
            if math::two_power(-i64::from(bit), budget)?.compare(&needed, budget)? == Ordering::Less
            {
                break;
            }
            radius_bits = bit;
        }
        let factor = Rational::one().add(&u, budget)?.mul(&alpha_ratio, budget)?;
        let growth = factor.sub(&Rational::one(), budget)?;
        let additive = three.add(&import_ratio, budget)?;
        let radius_ratio = factor
            .mul(&alpha_ratio, budget)?
            .mul(&math::two_power(-i64::from(radius_bits), budget)?, budget)?
            .add(&additive.mul(&u, budget)?, budget)?;
        let endpoint_ratio = three.mul(
            &central_ratio
                .add(&radius_ratio, budget)?
                .add(
                    &growth.mul(
                        &Rational::one()
                            .add(&central_ratio, budget)?
                            .add(&radius_ratio, budget)?,
                        budget,
                    )?,
                    budget,
                )?
                .add(&additive.mul(&u, budget)?, budget)?,
            budget,
        )?;
        Ok(Proof {
            version: 1,
            scope,
            degree,
            coefficient_bits: bits,
            coefficients,
            stages,
            absolute_tail,
            relative_tail,
            import_ratio,
            central_ratio,
            radius_bits,
            endpoint_ratio,
            restoration_error: Rational::zero(),
        })
    })
}

/// Construct all separate rounded stages with conservative exact proof-grid bounds.
fn rounded_stages(
    coefficients: &[super::Coefficient],
    ideals: &[Rational],
    budget: &mut Budget,
) -> Result<(Vec<Stage>, Rational, Rational), Fault> {
    let degree = ideals.len();
    let u = math::two_power(-53, budget)?;
    let eta = math::two_power(-1075, budget)?;
    let two = Rational::from_ratio(2, 1, budget)?;
    budget.work((2 * degree - 1) as u64, AT)?;
    let mut stages = Vec::new();
    stages
        .try_reserve_exact(2 * degree - 1)
        .map_err(|_| Fault::allocation(AT))?;
    let last = degree - 1;
    let mut magnitude = ideals[last].clone();
    let mut error = math::upper(
        &math::abs(
            &coefficients[last].exact(budget)?.sub(&magnitude, budget)?,
            budget,
        )?,
        budget,
    )?;
    for index in (0..last).rev() {
        budget.work(1, AT)?;
        let product_magnitude = two.mul(&magnitude, budget)?;
        let propagated = two.mul(&error, budget)?;
        let product_error = math::upper(
            &propagated
                .add(
                    &u.mul(&product_magnitude.add(&propagated, budget)?, budget)?,
                    budget,
                )?
                .add(&eta, budget)?,
            budget,
        )?;
        stages.push(Stage {
            kind: StageKind::Multiply,
            magnitude: product_magnitude.clone(),
            error: product_error.clone(),
        });
        let coefficient = coefficients[index].exact(budget)?;
        let delta = math::abs(&coefficient.sub(&ideals[index], budget)?, budget)?;
        let sum_rounding = u
            .mul(
                &product_magnitude
                    .add(&product_error, budget)?
                    .add(&coefficient, budget)?,
                budget,
            )?
            .add(&eta, budget)?;
        error = math::upper(
            &product_error
                .add(&delta, budget)?
                .add(&sum_rounding, budget)?,
            budget,
        )?;
        magnitude = product_magnitude.add(&ideals[index], budget)?;
        stages.push(Stage {
            kind: StageKind::Add,
            magnitude: magnitude.clone(),
            error: error.clone(),
        });
    }
    let final_magnitude = two.mul(&magnitude, budget)?;
    let final_propagated = two.mul(&error, budget)?;
    let final_error = math::upper(
        &final_propagated
            .add(
                &u.mul(&final_magnitude.add(&final_propagated, budget)?, budget)?,
                budget,
            )?
            .add(&eta, budget)?,
        budget,
    )?;
    stages.push(Stage {
        kind: StageKind::FinalMultiply,
        magnitude: final_magnitude,
        error: final_error,
    });
    Ok((stages, magnitude, error))
}

#[cfg(test)]
mod tests {
    use super::{Counts, candidate};
    use crate::budget::{Budget, BudgetKind, Limits};
    use crate::{Exhaustion, FaultKind};

    /// Two degree-one vector slots cost two Work before any proposal allocation.
    /// # Panics
    /// Panics if the independently specified entry tariff or construction boundary changes.
    /// ´claim:cas:prepared-expm1-entry-tariff´
    /// ´test:unit:prepared-expm1-entry-tariff´
    #[test]
    fn prepared_expm1_entry_tariff() -> Result<(), crate::Fault> {
        for (work, needed, proposals) in [(1, 2, 0), (2, 3, 1)] {
            let mut budget = Budget::new(Limits {
                work,
                integer_exponent: 4096,
                ..Limits::default()
            })?;
            let mut counts = Counts::default();
            let error =
                candidate(Vec::new(), 1, 53, &mut budget, &mut counts).expect_err("bounded entry");
            assert_eq!(
                error.kind,
                FaultKind::Exhausted {
                    reason: Exhaustion::Limit,
                    budget: BudgetKind::Work,
                    limit: work,
                    needed,
                }
            );
            assert_eq!(counts.proposals, proposals);
            assert_eq!(budget.usage().work, if proposals == 0 { 0 } else { 2 });
        }
        Ok(())
    }
}
