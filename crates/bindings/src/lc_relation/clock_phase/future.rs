// SPDX-License-Identifier: AGPL-3.0-only
//! Narrow original physical phase strips have an equivalent clock projection with retained phase fibres.

use super::{AT, LcClockPhaseCover, LcRelationError, Rational, admit, storage};
use crate::{
    Budget, LcCalibratedFuture, LcClockPhaseParameters, LcClockPhaseWitness, LcHorizon,
    LcSharedClockPhaseParameters,
};

impl LcClockPhaseCover {
    /// Derive a reported-clock horizon while retaining original clock/phase calibration fibres.
    ///
    /// This closed adapter requires narrower original physical phase strips, whose retained
    /// effective clock domain is a complete physical projection of the original joint question.
    /// # Errors
    /// Refuses the broad shifted-phase path, unsupported domains and exhausted resources.
    pub fn forward_reported_horizon(
        &self,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Derive the same original reported-clock query at an explicit common future reference.
    /// # Errors
    /// Refuses broad shifted-phase domains, references before the supported future domain and exhausted Work.
    pub fn forward_reported_horizon_from(
        &self,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, Some(reference), budget)
    }
}

/// Exact physical projection and full original extremizers jointly retain the future query's authority.
fn build(
    cover: &LcClockPhaseCover,
    reference: Option<&Rational>,
    budget: &mut Budget,
) -> Result<LcHorizon, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    if cover.0.measured.is_none() {
        return Err(LcRelationError::Projection);
    }
    storage::retained(
        cover
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(16_384),
        budget,
    )?;
    let auxiliary = if let Some(reference) = reference {
        cover
            .0
            .effective
            .forward_reported_horizon_from(reference, budget)?
    } else {
        cover.0.effective.forward_reported_horizon(budget)?
    };
    let mut witnesses = Vec::with_capacity(2);
    for (side, expected) in auxiliary.extremal_parameters().iter().enumerate() {
        let original = cover.feasibility_witness();
        let retained = crate::lc_relation::calibrated_future::matches_extremizer(
            original.original().original(),
            original.original().offset(),
            expected,
            budget,
        )?;
        if retained {
            witnesses.push(original.clone());
        } else {
            witnesses.push(extremizer(cover, side, expected, budget)?);
        }
    }
    let source = LcCalibratedFuture::phase(cover.clone(), witnesses, auxiliary, budget)?;
    LcHorizon::calibrated(source, &before, budget)
}

/// The physical intercept selects one original clock offset and one conditional phase calibration.
fn extremizer(
    cover: &LcClockPhaseCover,
    side: usize,
    expected: &[Rational; 2],
    budget: &mut Budget,
) -> Result<LcClockPhaseWitness, LcRelationError> {
    budget.work(1, AT)?;
    let [intercept, rate] = expected;
    let mut alpha = cover.phase_at_rate(rate, budget)?;
    let clock = cover.0.effective.clock().offset_support();
    let allowed = [
        intercept.sub(&rate.mul(&clock[1], budget)?, budget)?,
        intercept.sub(&rate.mul(&clock[0], budget)?, budget)?,
    ];
    admit::intersect(&mut alpha, &allowed, budget)?;
    let alpha = alpha[side].clone();
    let clock_offset = intercept.sub(&alpha, budget)?.div(rate, budget)?;
    let measured = cover
        .0
        .measured
        .as_ref()
        .ok_or(LcRelationError::Projection)?
        .phase_at_rate(rate, budget)?;
    let mut phase_offset = [
        intercept.sub(&measured[1], budget)?,
        intercept.sub(&measured[0], budget)?,
    ];
    admit::intersect(&mut phase_offset, cover.phase().offset(), budget)?;
    let parameters = LcSharedClockPhaseParameters {
        physical: LcClockPhaseParameters {
            rate: rate.clone(),
            initial_phase: alpha,
            offset: clock_offset,
            gauge: Rational::zero(),
            ratio: Rational::one(),
            voltage: Rational::one(),
        },
        offset: phase_offset[0].clone(),
    };
    cover.witness(&parameters, budget)
}
