// SPDX-License-Identifier: AGPL-3.0-only
//! Reported intercept elimination preserves the shared phase-offset fibre in both original paths.

use super::{AT, LcClockPhaseCover, LcRelationError, Rational, admit, storage};
use crate::{
    Budget, LcCalibratedFuture, LcClockPhaseParameters, LcClockPhaseWitness, LcHorizon,
    LcSharedClockPhaseParameters,
};

impl LcClockPhaseCover {
    /// Derive a reported-clock horizon while retaining original clock/phase calibration fibres.
    ///
    /// Broad shifted-phase domains restore the physical reported intercept with its conditional
    /// phase-offset fibre; narrower original strips already retain the equivalent physical image.
    /// # Errors
    /// Refuses unsupported original domains and exhausted resources.
    pub fn forward_reported_horizon(
        &self,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Derive the same original reported-clock query at an explicit common future reference.
    /// # Errors
    /// Refuses references before the supported future domain and exhausted Work.
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
    let extrema = physical_image(cover, auxiliary.extremal_parameters(), budget)?;
    let mut witnesses = Vec::with_capacity(2);
    for (side, expected) in extrema.iter().enumerate() {
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
    let source = LcCalibratedFuture::phase(cover.clone(), witnesses, extrema, auxiliary, budget)?;
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
    let measured = if let Some(measured) = &cover.0.measured {
        measured.phase_at_rate(rate, budget)?
    } else {
        cover.0.effective.reported_intercept_at_rate(rate, budget)?
    };
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

/// At each rate the physical image is `(G+S) intersect (A+aD)`, with every offset fibre retained.
fn physical_image(
    cover: &LcClockPhaseCover,
    auxiliary: &[[Rational; 2]; 2],
    budget: &mut Budget,
) -> Result<[[Rational; 2]; 2], LcRelationError> {
    budget.work(1, AT)?;
    let mut extrema = auxiliary.clone();
    if cover.0.measured.is_some() {
        return Ok(extrema);
    }
    let phase = cover.phase();
    for (side, [intercept, rate]) in extrema.iter_mut().enumerate() {
        budget.work(1, AT)?;
        *intercept = intercept.add(&phase.offset()[side], budget)?;
        if let Some(initial) = &phase.0.initial {
            let boundary = initial[side].add(
                &rate.mul(&phase.clock().offset_support()[side], budget)?,
                budget,
            )?;
            let order = boundary.compare(intercept, budget)?;
            let lower = side == 0 && order == std::cmp::Ordering::Greater;
            let upper = side == 1 && order == std::cmp::Ordering::Less;
            if lower || upper {
                *intercept = boundary;
            }
        }
    }
    Ok(extrema)
}
