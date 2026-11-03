// SPDX-License-Identifier: AGPL-3.0-only
//! Certified emptiness is supplied separately for the complete refined original physical relation.

use super::{
    AT, Cover, LcClockRefinement, LcClockRefinementInputs, LcClockRefinementState, LcRelationError,
    storage, validate,
};
use crate::{Budget, LcClockCover, LcHorizon, LcImpossible, LcSharedTimingCover, LcTimingCover};

impl LcClockCover {
    /// Retain a separately checked whole-original-domain contradiction as an impossible refinement.
    ///
    /// The complete original physical relation being empty excludes its shared-clock measurement
    /// subset. Solver failure and a contradiction for only one proposed cycle branch do not suffice.
    /// # Errors
    /// Refuses foreign source/certificate authorities and exhausted cumulative resources.
    pub fn refine_impossible(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        contradiction: &LcImpossible,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        complete(
            Cover::Clock(self.clone()),
            inclusion,
            previous_horizon,
            contradiction,
            budget,
        )
    }
}

impl LcSharedTimingCover {
    /// Retain whole-original-relation emptiness with both timing layers and all previous evidence.
    /// # Errors
    /// Refuses foreign source/certificate authorities and exhausted cumulative resources.
    pub fn refine_impossible(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        contradiction: &LcImpossible,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        complete(
            Cover::SharedTiming(self.clone()),
            inclusion,
            previous_horizon,
            contradiction,
            budget,
        )
    }
}

impl LcTimingCover {
    /// Retain whole-original-relation emptiness for independent timing without discarding prior evidence.
    /// # Errors
    /// Refuses foreign source/certificate authorities and exhausted cumulative resources.
    pub fn refine_impossible(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        contradiction: &LcImpossible,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        complete(
            Cover::Timing(self.clone()),
            inclusion,
            previous_horizon,
            contradiction,
            budget,
        )
    }
}

/// Inclusion and original allocation identity connect a whole-domain certificate to the refined question.
fn complete(
    previous: Cover,
    inclusion: &LcClockRefinementInputs,
    horizon: &LcHorizon,
    contradiction: &LcImpossible,
    budget: &mut Budget,
) -> Result<LcClockRefinement, LcRelationError> {
    let before = budget.usage();
    let retained = validate(&previous, inclusion, horizon, budget)?;
    budget.work(1, AT)?;
    if !contradiction
        .relation()
        .shares_allocation(inclusion.0.refined.relation())
    {
        return Err(LcRelationError::Containment);
    }
    storage::retained(
        retained
            .saturating_add(contradiction.retained_payload_bytes())
            .saturating_add(1_024),
        budget,
    )?;
    budget.work(1, AT)?;
    let state = LcClockRefinementState::Impossible(Box::new(contradiction.clone()));
    Ok(LcClockRefinement {
        previous,
        previous_horizon: horizon.clone(),
        inclusion: inclusion.clone(),
        proposed_cycles: Vec::new(),
        state,
        solving: None,
        usage: [before, budget.usage()],
    })
}
