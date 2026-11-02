// SPDX-License-Identifier: AGPL-3.0-only
//! Known exact zero starting phase preserves the original recovered branch in inverse time.

use super::{AT, LcPairsCover, LcRelationError};
use crate::{Budget, LcZeroTimes};

impl LcPairsCover {
    /// Project all nonnegative zero times of the complete known-zero-phase pair family.
    ///
    /// Every rate in the recovered interval admits starting phase zero. The original
    /// phase/time strips and scale maps stay in the answer, rather than being redeclared
    /// as an unrelated rate rectangle with a fresh clock or branch.
    /// # Errors
    /// Refuses unknown/nonzero/non-singleton starting phase and exhausted caller resources.
    pub fn zero_times(&self, budget: &mut Budget) -> Result<LcZeroTimes, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if !self.0.inputs.fixed_phase {
            return Err(LcRelationError::Projection);
        }
        let phase = &self
            .0
            .inputs
            .strips
            .last()
            .ok_or(LcRelationError::Projection)?
            .phase;
        if !phase[0].is_zero() || !phase[1].is_zero() {
            return Err(LcRelationError::Projection);
        }
        LcZeroTimes::pairs(self.clone(), &before, budget)
    }
}
