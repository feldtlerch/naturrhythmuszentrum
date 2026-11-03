// SPDX-License-Identifier: AGPL-3.0-only
//! Actual-time future extrema retain the original independent timestamp owner.

use super::{AT, LcRelationError, LcTimingCover, Rational};
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcTimingCover {
    /// Derive a horizon after every original timestamp upper endpoint and time zero.
    ///
    /// Future time is actual coherent time from the original epoch. Each affine face has
    /// nonnegative future rate slope after this reference, so rate endpoints attain both
    /// extrema. Common integer gauges wrap to the same circle and remain with the source.
    /// # Errors
    /// Refuses exhausted exact arithmetic; the original complete timing cover survives.
    pub fn forward_horizon(&self, budget: &mut Budget) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Use an explicit common future actual-time reference for refinement comparisons.
    /// # Errors
    /// Refuses a reference before zero or any original timestamp upper endpoint and exhausted Work.
    pub fn forward_horizon_from(
        &self,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, Some(reference), budget)
    }

    /// Prepared exact circle evidence is reused by derived timing owners.
    #[must_use]
    pub(in crate::lc_relation) fn pi_bounds(&self) -> &nrz_cas::pi_bounds::CheckedPiBounds {
        self.0.effective.pi_bounds()
    }
}

/// Original existential time inequalities give exact phase extrema at the two rate endpoints.
fn build(
    cover: &LcTimingCover,
    reference: Option<&Rational>,
    budget: &mut Budget,
) -> Result<LcHorizon, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let mut earliest = Rational::zero();
    for time in cover.timing().time_boxes() {
        budget.work(1, AT)?;
        if time[1].compare(&earliest, budget)? == Ordering::Greater {
            earliest.clone_from(&time[1]);
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
    LcHorizon::timing(
        cover.clone(),
        extrema,
        reference.clone(),
        cover.pi_bounds().clone(),
        &before,
        budget,
    )
}
