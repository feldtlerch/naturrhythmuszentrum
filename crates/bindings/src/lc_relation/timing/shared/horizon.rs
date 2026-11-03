// SPDX-License-Identifier: AGPL-3.0-only
//! Both timing layers remain the source of a future actual-time horizon.

use super::{AT, LcRelationError, LcSharedTimingCover, Rational};
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcSharedTimingCover {
    /// Derive a future actual-time horizon with the original shared offset and independent errors.
    ///
    /// Physical initial-phase fibres retain `alpha=beta-a*offset`. After every admitted actual
    /// observation time, both future extremizers occur at rate endpoints. A reported future
    /// reading needs a separate query retaining its correlation with the same clock offset.
    /// # Errors
    /// Refuses exhausted resources while retaining the original two-layer timing owner.
    pub fn forward_horizon(&self, budget: &mut Budget) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Use one explicit future actual-time reference for original-question refinement comparisons.
    /// # Errors
    /// Refuses references before the admitted observation upper bounds and exhausted resources.
    pub fn forward_horizon_from(
        &self,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, Some(reference), budget)
    }
}

/// Physical initial-phase extrema use one common admissible future reference.
fn build(
    cover: &LcSharedTimingCover,
    explicit: Option<&Rational>,
    budget: &mut Budget,
) -> Result<LcHorizon, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let mut reference = Rational::zero();
    if cover.0.measured.is_some() {
        for time in cover.0.effective.timing().time_boxes() {
            budget.work(1, AT)?;
            if time[1].compare(&reference, budget)? == Ordering::Greater {
                reference.clone_from(&time[1]);
            }
        }
    } else {
        for (nominal, error) in cover
            .timing()
            .nominal_times()
            .iter()
            .zip(cover.timing().errors())
        {
            budget.work(1, AT)?;
            let upper = nominal
                .add(&error[1], budget)?
                .add(&cover.timing().offset()[1], budget)?;
            if upper.compare(&reference, budget)? == Ordering::Greater {
                reference = upper;
            }
        }
    }
    if let Some(selected) = explicit {
        selected.check(budget, AT)?;
        if selected.compare(&reference, budget)? == Ordering::Less {
            return Err(LcRelationError::Containment);
        }
        reference.clone_from(selected);
    }
    let lower = cover.phase_at_rate(&cover.rate()[0], budget)?;
    let upper = cover.phase_at_rate(&cover.rate()[1], budget)?;
    let extrema = [
        [lower[0].clone(), cover.rate()[0].clone()],
        [upper[1].clone(), cover.rate()[1].clone()],
    ];
    LcHorizon::shared_timing(
        cover.clone(),
        extrema,
        reference,
        cover.0.effective.pi_bounds().clone(),
        &before,
        budget,
    )
}
