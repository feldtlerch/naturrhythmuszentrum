// SPDX-License-Identifier: AGPL-3.0-only
//! Both timing layers remain the source of a future actual-time horizon.

use super::{AT, LcRelationError, LcSharedTimingCover, Rational};
use crate::{Budget, LcHorizon};
use std::cmp::Ordering;

impl LcSharedTimingCover {
    /// Derive a future actual-time horizon with the original shared offset and independent errors.
    ///
    /// Physical initial phase is exactly zero in this adapter. After every admitted actual
    /// observation time, the width is `(a_max-a_min)*t`. A reported future reading would
    /// need a different query retaining its correlation with the same clock offset.
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
        let extrema = [
            [Rational::zero(), self.rate()[0].clone()],
            [Rational::zero(), self.rate()[1].clone()],
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
