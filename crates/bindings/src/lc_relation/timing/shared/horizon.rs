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
        let before = budget.usage();
        budget.work(1, AT)?;
        let mut reference = Rational::zero();
        for (nominal, error) in self
            .timing()
            .nominal_times()
            .iter()
            .zip(self.timing().errors())
        {
            budget.work(1, AT)?;
            let upper = nominal
                .add(&error[1], budget)?
                .add(&self.timing().offset()[1], budget)?;
            if upper.compare(&reference, budget)? == Ordering::Greater {
                reference = upper;
            }
        }
        let lower = self.phase_at_rate(&self.rate()[0], budget)?;
        let upper = self.phase_at_rate(&self.rate()[1], budget)?;
        let extrema = [
            [lower[0].clone(), self.rate()[0].clone()],
            [upper[1].clone(), self.rate()[1].clone()],
        ];
        LcHorizon::shared_timing(
            self.clone(),
            extrema,
            reference,
            self.0.effective.pi_bounds().clone(),
            &before,
            budget,
        )
    }
}
