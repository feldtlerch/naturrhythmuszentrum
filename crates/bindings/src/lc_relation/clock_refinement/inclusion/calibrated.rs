// SPDX-License-Identifier: AGPL-3.0-only
//! Every original calibration declaration participates in the independent inclusion proof.

use super::{
    AT, LcClockRefinementInputs, LcRelationError, Question, contained, declarations, finish,
};
use crate::{Budget, LcInterval, LcSharedClockPhase, LcSharedMeasurements};

impl LcSharedClockPhase {
    /// Check original physical inputs, nominal clocks and both original calibration supports.
    /// # Errors
    /// Refuses any weakened or changed original declaration and exhausted resources.
    pub fn refinement_inputs(
        &self,
        refined: &Self,
        budget: &mut Budget,
    ) -> Result<LcClockRefinementInputs, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let original = self
            .relation()
            .refinement_inputs(refined.relation(), budget)?;
        phase(self, refined, budget)?;
        finish(
            Question::Phase(self.clone()),
            Question::Phase(refined.clone()),
            original,
            &before,
            budget,
        )
    }
}

impl LcSharedMeasurements {
    /// Check all original physical, clock, phase and amplitude-reading declarations jointly.
    /// # Errors
    /// Refuses changed channels, removed readings, weakened supports and exhausted resources.
    pub fn refinement_inputs(
        &self,
        refined: &Self,
        budget: &mut Budget,
    ) -> Result<LcClockRefinementInputs, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if self.channel() != refined.channel() {
            return Err(LcRelationError::Containment);
        }
        let original = self
            .relation()
            .refinement_inputs(refined.relation(), budget)?;
        phase(self.phase(), refined.phase(), budget)?;
        samples(self.samples(), refined.samples(), budget)?;
        contained(self.gain(), refined.gain(), budget)?;
        finish(
            Question::Amplitude(self.clone()),
            Question::Amplitude(refined.clone()),
            original,
            &before,
            budget,
        )
    }
}

/// Declared nominal coordinates retain their identity before either shared offset is narrowed.
fn phase(
    previous: &LcSharedClockPhase,
    next: &LcSharedClockPhase,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    declarations(
        previous.clock().nominal_times(),
        next.clock().nominal_times(),
        previous.clock().offset(),
        next.clock().offset(),
        budget,
    )?;
    contained(previous.offset(), next.offset(), budget)?;
    samples(previous.samples(), next.samples(), budget)
}

/// Prefix identity preserves every old reading; each original interval must contain its replacement.
fn samples(
    previous: &[LcInterval],
    next: &[LcInterval],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    if previous.len() > next.len() {
        return Err(LcRelationError::Observation);
    }
    for (old, new) in previous.iter().zip(next) {
        budget.work(1, AT)?;
        let lower = old.contains(new.lower().ok_or(LcRelationError::Projection)?, budget)?;
        let upper = old.contains(new.upper().ok_or(LcRelationError::Projection)?, budget)?;
        if !lower || !upper {
            return Err(LcRelationError::Containment);
        }
    }
    Ok(())
}

impl LcClockRefinementInputs {
    /// Complete previous original calibrated question, when applicable.
    #[must_use]
    pub fn previous_clock_phase(&self) -> Option<&LcSharedClockPhase> {
        self.0.previous.phase()
    }
}

impl LcClockRefinementInputs {
    /// Complete refined original calibrated question, when applicable.
    #[must_use]
    pub fn refined_clock_phase(&self) -> Option<&LcSharedClockPhase> {
        self.0.refined.phase()
    }
}

impl LcClockRefinementInputs {
    /// Complete previous original calibrated question, when applicable.
    #[must_use]
    pub fn previous_shared_measurement(&self) -> Option<&LcSharedMeasurements> {
        self.0.previous.amplitude()
    }
}

impl LcClockRefinementInputs {
    /// Complete refined original calibrated question, when applicable.
    #[must_use]
    pub fn refined_shared_measurement(&self) -> Option<&LcSharedMeasurements> {
        self.0.refined.amplitude()
    }
}
