// SPDX-License-Identifier: AGPL-3.0-only
//! Measurement emptiness requires the identical full refined clock/calibration authority.

use super::{AT, Cover, LcClockRefinement, LcClockRefinementInputs, LcClockRefinementState};
use super::{LcRelationError, storage, validate};
use crate::{
    Budget, LcClockPhaseCover, LcHorizon, LcMeasurementImpossible, LcSharedMeasurementCover,
};

impl LcClockPhaseCover {
    /// Retain a complete joint measurement contradiction beside the previous cover and horizon.
    /// # Errors
    /// Refuses foreign question/certificate authorities and exhausted cumulative resources.
    pub fn refine_measurement_impossible(
        &self,
        inclusion: &LcClockRefinementInputs,
        horizon: &LcHorizon,
        contradiction: &LcMeasurementImpossible,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        complete(
            Cover::Phase(self.clone()),
            inclusion,
            horizon,
            contradiction,
            budget,
        )
    }
}

impl LcSharedMeasurementCover {
    /// Retain measurement-only emptiness for the full original clock/phase/amplitude question.
    /// # Errors
    /// Refuses foreign question/certificate authorities and exhausted cumulative resources.
    pub fn refine_measurement_impossible(
        &self,
        inclusion: &LcClockRefinementInputs,
        horizon: &LcHorizon,
        contradiction: &LcMeasurementImpossible,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        complete(
            Cover::Amplitude(self.clone()),
            inclusion,
            horizon,
            contradiction,
            budget,
        )
    }
}

/// Equal physical inputs do not establish identity of the original measurement declarations.
fn complete(
    previous: Cover,
    inclusion: &LcClockRefinementInputs,
    horizon: &LcHorizon,
    contradiction: &LcMeasurementImpossible,
    budget: &mut Budget,
) -> Result<LcClockRefinement, LcRelationError> {
    let before = budget.usage();
    let retained = validate(&previous, inclusion, horizon, budget)?;
    budget.work(1, AT)?;
    let phase = contradiction
        .clock_phase()
        .zip(inclusion.0.refined.phase())
        .is_some_and(|(certificate, question)| certificate.shares_allocation(question));
    let amplitude = contradiction
        .shared_measurements()
        .zip(inclusion.0.refined.amplitude())
        .is_some_and(|(certificate, question)| certificate.shares_allocation(question));
    if !phase && !amplitude {
        return Err(LcRelationError::Containment);
    }
    storage::retained(
        retained
            .saturating_add(contradiction.retained_payload_bytes())
            .saturating_add(1_024),
        budget,
    )?;
    budget.work(1, AT)?;
    Ok(LcClockRefinement {
        previous,
        previous_horizon: horizon.clone(),
        inclusion: inclusion.clone(),
        proposed_cycles: Vec::new(),
        state: LcClockRefinementState::MeasurementImpossible(Box::new(contradiction.clone())),
        solving: None,
        usage: [before, budget.usage()],
    })
}
