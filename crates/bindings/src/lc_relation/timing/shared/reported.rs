// SPDX-License-Identifier: AGPL-3.0-only
//! A future reported reading retains the same shared offset beside both original time-error layers.

use super::{AT, Bounds, LcRelationError, LcSharedTimingCover, Rational};
use crate::lc_relation::calibration;
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcSharedTimingCover {
    /// Derive a horizon for future reported r with actual time `r+d` under the same shared offset.
    ///
    /// Original independent sample errors remain retained. This future query adds no new
    /// independent error: its physical phase is `alpha+a*(r+d)=beta+a*r`.
    /// # Errors
    /// Refuses exhausted resources while retaining both original timing layers.
    pub fn forward_reported_horizon(
        &self,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Derive the same reported-coordinate query at one explicit common future reference.
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

/// Physical and effective strip fibres determine the exact reported intercept at a rate.
fn intercept(
    cover: &LcSharedTimingCover,
    rate: &Rational,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    if let Some(measured) = &cover.0.measured {
        let mut beta = measured.phase_at_rate(rate, budget)?;
        let alpha = cover.0.effective.phase_at_rate(rate, budget)?;
        let offset = cover.timing().offset_support();
        let image = [
            alpha[0].add(&rate.mul(&offset[0], budget)?, budget)?,
            alpha[1].add(&rate.mul(&offset[1], budget)?, budget)?,
        ];
        calibration::intersect(&mut beta, &image, budget)?;
        Ok(beta)
    } else {
        cover.0.effective.phase_at_rate(rate, budget)
    }
}

/// After the safe reported reference every active future affine face is nondecreasing in rate.
fn build(
    cover: &LcSharedTimingCover,
    reference: Option<&Rational>,
    budget: &mut Budget,
) -> Result<LcHorizon, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let times = if cover.0.measured.is_some() {
        cover.0.effective.timing().time_boxes()
    } else {
        cover.timing().time_boxes()
    };
    let mut actual = Rational::zero();
    for time in times {
        budget.work(1, AT)?;
        if time[1].compare(&actual, budget)? == Ordering::Greater {
            actual.clone_from(&time[1]);
        }
    }
    let mut earliest = actual.sub(&cover.timing().offset_support()[0], budget)?;
    if earliest.is_negative() {
        earliest = Rational::zero();
    }
    let reference = reference.unwrap_or(&earliest);
    reference.check(budget, AT)?;
    if reference.compare(&earliest, budget)? == Ordering::Less {
        return Err(LcRelationError::Containment);
    }
    let lower = intercept(cover, &cover.rate()[0], budget)?;
    let upper = intercept(cover, &cover.rate()[1], budget)?;
    let extrema = [
        [lower[0].clone(), cover.rate()[0].clone()],
        [upper[1].clone(), cover.rate()[1].clone()],
    ];
    LcHorizon::shared_timing_reported(
        cover.clone(),
        extrema,
        reference.clone(),
        cover.0.effective.pi_bounds().clone(),
        &before,
        budget,
    )
}
