// SPDX-License-Identifier: AGPL-3.0-only
//! Calibrated refinements preserve original source authority and the same reported future reference.

use super::{
    Cover, LcClockRefinement, LcClockRefinementInputs, LcClockRefinementSuccess, LcRelationError,
    attempt,
};
use crate::{Budget, LcClockPhaseCover, LcHorizon, LcSharedMeasurementCover};
use nrz_cas::Rational;

impl LcClockPhaseCover {
    /// Attempt whole-original calibration refinement, retaining old evidence on failed solving.
    /// # Errors
    /// Refuses foreign source or horizon authority and preflight exhaustion.
    pub fn refine(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        cycles: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        attempt(
            Cover::Phase(self.clone()),
            inclusion,
            previous_horizon,
            cycles,
            budget,
        )
    }
}
impl LcClockRefinementSuccess {
    /// Complete original calibrated cover after successful refinement, when applicable.
    #[must_use]
    pub const fn clock_phase_cover(&self) -> Option<&LcClockPhaseCover> {
        self.cover.phase()
    }
}
impl LcClockRefinement {
    /// Exact previous calibrated cover retained in every attempt state, when applicable.
    #[must_use]
    pub const fn previous_clock_phase(&self) -> Option<&LcClockPhaseCover> {
        self.previous.phase()
    }
}

impl LcSharedMeasurementCover {
    /// Attempt whole-original calibration refinement, retaining old evidence on failed solving.
    /// # Errors
    /// Refuses foreign source or horizon authority and preflight exhaustion.
    pub fn refine(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        cycles: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        attempt(
            Cover::Amplitude(self.clone()),
            inclusion,
            previous_horizon,
            cycles,
            budget,
        )
    }
}
impl LcClockRefinementSuccess {
    /// Complete original calibrated cover after successful refinement, when applicable.
    #[must_use]
    pub const fn shared_measurement_cover(&self) -> Option<&LcSharedMeasurementCover> {
        self.cover.amplitude()
    }
}
impl LcClockRefinement {
    /// Exact previous calibrated cover retained in every attempt state, when applicable.
    #[must_use]
    pub const fn previous_shared_measurement(&self) -> Option<&LcSharedMeasurementCover> {
        self.previous.amplitude()
    }
}
