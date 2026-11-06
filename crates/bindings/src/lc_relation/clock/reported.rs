// SPDX-License-Identifier: AGPL-3.0-only
//! A future reported reading shares the original clock offset instead of adding an independent error box.

use super::{AT, LcClockCover, LcRelationError, Rational};
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcClockCover {
    /// Exact effective reported intercept, retaining its conditional original clock fibre.
    pub(in crate::lc_relation) fn reported_intercept_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Derive a horizon for future reported r with actual time `r+d` under the same shared offset.
    ///
    /// The physical future phase is `alpha+a*(r+d)=beta+a*r`. No independent future clock
    /// error is added. The reference ensures actual time follows zero and every admitted observation.
    /// # Errors
    /// Refuses exhausted resources while retaining the complete original clock owner.
    pub fn forward_reported_horizon(
        &self,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Derive the same reported-coordinate query at an explicit common future reference.
    /// # Errors
    /// Refuses earlier-than-admitted references and exhausted resources.
    pub fn forward_reported_horizon_from(
        &self,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, Some(reference), budget)
    }
}

/// Effective intercept extrema retain the original physical alpha/offset correlation.
fn build(
    cover: &LcClockCover,
    reference: Option<&Rational>,
    budget: &mut Budget,
) -> Result<LcHorizon, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let offset = cover.clock().offset_support();
    let mut actual = Rational::zero();
    for nominal in cover.clock().nominal_times() {
        budget.work(1, AT)?;
        let upper = nominal.add(&offset[1], budget)?;
        if upper.compare(&actual, budget)? == Ordering::Greater {
            actual = upper;
        }
    }
    let mut earliest = actual.sub(&offset[0], budget)?;
    if earliest.is_negative() {
        earliest = Rational::zero();
    }
    let reference = reference.unwrap_or(&earliest);
    reference.check(budget, AT)?;
    if reference.compare(&earliest, budget)? == Ordering::Less {
        return Err(LcRelationError::Containment);
    }
    let lower = cover.0.effective.phase_at_rate(&cover.rate()[0], budget)?;
    let upper = cover.0.effective.phase_at_rate(&cover.rate()[1], budget)?;
    let extrema = [
        [lower[0].clone(), cover.rate()[0].clone()],
        [upper[1].clone(), cover.rate()[1].clone()],
    ];
    LcHorizon::clock_reported(
        cover.clone(),
        extrema,
        reference.clone(),
        cover.0.effective.pi_bounds().clone(),
        &before,
        budget,
    )
}
