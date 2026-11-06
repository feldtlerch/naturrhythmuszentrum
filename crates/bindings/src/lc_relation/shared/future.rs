// SPDX-License-Identifier: AGPL-3.0-only
//! Reported future phase retains every original amplitude reading and its conditional gain.

use super::{AT, LcRelationError, LcSharedMeasurementCover, Rational, admit, storage, witness};
use crate::{Budget, LcCalibratedFuture, LcHorizon};

impl LcSharedMeasurementCover {
    /// Derive a reported-clock horizon with original phase and amplitude calibration fibres.
    ///
    /// The retained phase owner restores either admitted original phase-calibration path.
    /// Original physical scales and all readings are rechecked at both future extremizers.
    /// # Errors
    /// Refuses unsupported phase domains and exhausted cumulative resources.
    pub fn forward_reported_horizon(
        &self,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, None, budget)
    }

    /// Preserve a common reported future reference through calibrated measurement refinement.
    /// # Errors
    /// Refuses unsupported phase domains, an early reference or exhausted resources.
    pub fn forward_reported_horizon_from(
        &self,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        build(self, Some(reference), budget)
    }
}

/// Positive scale elimination is surjective onto the retained phase domain, with original gain fibres.
fn build(
    cover: &LcSharedMeasurementCover,
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
    let phase = auxiliary
        .calibrated_future()
        .and_then(LcCalibratedFuture::phase_witnesses)
        .ok_or(LcRelationError::Projection)?;
    let amplitude = cover.measurements();
    let energy = amplitude
        .energy()
        .map_or_else(Rational::one, |range| range[0].clone());
    let physical = &amplitude.physical_amplitude()[0];
    let gain = admit::gain_at_amplitude(amplitude, physical, budget)?;
    let measured = physical.div(&gain[0], budget)?;
    let mut witnesses = Vec::with_capacity(2);
    for (point, expected) in phase.iter().zip(auxiliary.extremal_parameters()) {
        let original = cover.feasibility_witness();
        let retained = crate::lc_relation::calibrated_future::matches_extremizer(
            original.original().original().original(),
            original.original().original().offset(),
            expected,
            budget,
        )?;
        if retained {
            witnesses.push(original.clone());
        } else {
            witnesses.push(witness::from_point(
                amplitude, point, &measured, &gain[0], &energy, budget,
            )?);
        }
    }
    let source = LcCalibratedFuture::amplitude(cover.clone(), witnesses, auxiliary, budget)?;
    LcHorizon::calibrated(source, &before, budget)
}
