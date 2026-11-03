// SPDX-License-Identifier: AGPL-3.0-only
//! Future actual time retains the original physical initial phase and shared clock offset.

use super::{AT, LcClockCover, LcRelationError, Rational};
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcClockCover {
    /// Derive a horizon in future actual time, after zero and every nominal-plus-offset upper bound.
    ///
    /// This is not a query at a future reported clock reading. Physical initial-phase fibres
    /// retain `alpha=beta-a*offset`; after the reference all active future faces are
    /// nondecreasing in rate. Their extrema therefore occur at the recovered rate endpoints.
    /// # Errors
    /// Refuses exhausted exact arithmetic while retaining the complete original clock owner.
    pub fn forward_horizon(&self, budget: &mut Budget) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Use one explicit future actual-time reference for comparisons of original clock owners.
    /// # Errors
    /// Refuses an earlier-than-admitted reference or exhausted resources.
    pub fn forward_horizon_from(
        &self,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, Some(reference), budget)
    }
}

/// Exact physical phase fibres, not effective beta marginals, determine actual-time extrema.
fn build(
    cover: &LcClockCover,
    reference: Option<&Rational>,
    budget: &mut Budget,
) -> Result<LcHorizon, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let mut earliest = Rational::zero();
    for nominal in cover.clock().nominal_times() {
        budget.work(1, AT)?;
        let upper = nominal.add(&cover.clock().offset_support()[1], budget)?;
        if upper.compare(&earliest, budget)? == Ordering::Greater {
            earliest = upper;
        }
    }
    let reference = reference.unwrap_or(&earliest);
    reference.check(budget, AT)?;
    if reference.compare(&earliest, budget)? == Ordering::Less {
        return Err(LcRelationError::Containment);
    }
    let lower = cover.phase_at_rate(&cover.rate()[0], budget)?;
    let upper = cover.phase_at_rate(&cover.rate()[1], budget)?;
    let extrema = [
        [lower[0].clone(), cover.rate()[0].clone()],
        [upper[1].clone(), cover.rate()[1].clone()],
    ];
    LcHorizon::clock(
        cover.clone(),
        extrema,
        reference.clone(),
        cover.0.effective.pi_bounds().clone(),
        &before,
        budget,
    )
}
