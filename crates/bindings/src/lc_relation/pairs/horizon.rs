// SPDX-License-Identifier: AGPL-3.0-only
//! Future phase extrema follow the retained correlated fibres after every observation.

use super::{AT, LcPairsCover, LcRelationError, Rational};
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcPairsCover {
    /// Derive the eventual horizon from joint phase/rate fibres after the last observation.
    ///
    /// For `t>=max(t_i,0)`, every affine function `phase_i+a*(t-t_i)` is nondecreasing
    /// in `a`. The lower fibre's maximum and the upper fibre's minimum inherit that
    /// monotonicity. Thus the minimum is at `a_min`, the maximum at `a_max`; convexity
    /// realizes every intervening phase. All common integer gauges wrap to the same circle.
    /// # Errors
    /// Refuses exact fibre arithmetic or exhausted resources, retaining the original cover.
    pub fn forward_horizon(&self, budget: &mut Budget) -> Result<LcHorizon, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let mut reference = Rational::zero();
        for strip in &self.0.inputs.strips {
            budget.work(1, AT)?;
            if strip.time.compare(&reference, budget)? == Ordering::Greater {
                reference.clone_from(&strip.time);
            }
        }
        let lower = self.phase_at_rate(&self.0.rate[0], budget)?;
        let upper = self.phase_at_rate(&self.0.rate[1], budget)?;
        let extrema = [
            [lower[0].clone(), self.0.rate[0].clone()],
            [upper[1].clone(), self.0.rate[1].clone()],
        ];
        LcHorizon::pairs(self.clone(), extrema, reference, &before, budget)
    }
}
