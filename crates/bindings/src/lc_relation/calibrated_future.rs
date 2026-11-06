// SPDX-License-Identifier: AGPL-3.0-only
//! Calibrated future questions retain their original joint fibres and full original extremizers.

use super::{AT, LcRelationError, storage};
use crate::{
    Budget, LcClockPhaseCover, LcClockPhaseWitness, LcHorizon, LcRelation,
    LcSharedMeasurementCover, LcSharedMeasurementWitness, LcWitness,
};
use nrz_cas::{Rational, lattice::PiWidthHorizon};
use std::{cmp::Ordering, rc::Rc};

/// Original calibrated source and checked physical extremizers for a reported-clock horizon.
#[derive(Clone, Debug)]
pub struct LcCalibratedFuture(Rc<Data>);

/// Exact elimination retains the complete original calibration fibres beside its auxiliary horizon.
#[derive(Debug)]
struct Data {
    /// Complete original phase or phase/amplitude source, including checked extremizers.
    original: Original,
    /// Complete auxiliary clock projection before any shared phase-offset restoration.
    auxiliary: LcHorizon,
    /// Checked physical reported-intercept/rate extremizers after restoring the shared phase fibre.
    extrema: [[Rational; 2]; 2],
    /// Exact physical future width and its finite/infinite/undecided threshold.
    mathematical: PiWidthHorizon,
    /// Conservative total retained ownership.
    bytes: u64,
}

/// Both variants retain their own original measurement question rather than a marginal substitute.
#[derive(Debug)]
enum Original {
    /// Shared clock and phase offsets with the complete original physical phase domain.
    Phase {
        /// Complete original phase-calibrated family.
        cover: LcClockPhaseCover,
        /// Lower and upper full-original physical extremizers.
        witnesses: Vec<LcClockPhaseWitness>,
    },
    /// The same joint clock/phase domain with original amplitude readings and one shared gain.
    Amplitude {
        /// Complete original amplitude, phase and clock family.
        cover: LcSharedMeasurementCover,
        /// Lower and upper physical extremizers with every original calibration.
        witnesses: Vec<LcSharedMeasurementWitness>,
    },
}

impl LcCalibratedFuture {
    /// Retain the original phase calibration and recheck correspondence of both physical extremizers.
    pub(super) fn phase(
        cover: LcClockPhaseCover,
        witnesses: Vec<LcClockPhaseWitness>,
        extrema: [[Rational; 2]; 2],
        auxiliary: LcHorizon,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        if witnesses.len() != 2 {
            return Err(LcRelationError::Observation);
        }
        let mut bytes = cover.retained_payload_bytes();
        for (point, expected) in witnesses.iter().zip(&extrema) {
            budget.work(1, AT)?;
            if !point.phase().shares_allocation(cover.phase()) {
                return Err(LcRelationError::Containment);
            }
            extremizer(
                point.original().original(),
                point.original().offset(),
                expected,
                budget,
            )?;
            bytes = bytes
                .saturating_add(point.original().original().retained_payload_bytes())
                .saturating_add(storage::rational(point.offset()))
                .saturating_add(storage::rational(point.original().offset()));
        }
        Self::finish(
            Original::Phase { cover, witnesses },
            auxiliary,
            extrema,
            bytes,
            budget,
        )
    }

    /// Retain every amplitude reading and gain beside the original physical future extremizers.
    pub(super) fn amplitude(
        cover: LcSharedMeasurementCover,
        witnesses: Vec<LcSharedMeasurementWitness>,
        auxiliary: LcHorizon,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        if witnesses.len() != 2 {
            return Err(LcRelationError::Observation);
        }
        let mut bytes = cover.retained_payload_bytes();
        for (point, expected) in witnesses.iter().zip(auxiliary.extremal_parameters()) {
            budget.work(1, AT)?;
            if !point.measurements().shares_allocation(cover.measurements()) {
                return Err(LcRelationError::Containment);
            }
            extremizer(
                point.original().original().original(),
                point.original().original().offset(),
                expected,
                budget,
            )?;
            bytes = bytes
                .saturating_add(
                    point
                        .original()
                        .original()
                        .original()
                        .retained_payload_bytes(),
                )
                .saturating_add(storage::rational(point.gain()))
                .saturating_add(storage::rational(point.original().offset()))
                .saturating_add(storage::rational(point.original().original().offset()));
        }
        let extrema = auxiliary.extremal_parameters().clone();
        Self::finish(
            Original::Amplitude { cover, witnesses },
            auxiliary,
            extrema,
            bytes,
            budget,
        )
    }

    /// The equivalent auxiliary proof is retained privately with every original source and witness.
    fn finish(
        original: Original,
        auxiliary: LcHorizon,
        extrema: [[Rational; 2]; 2],
        bytes: u64,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let mut identical = true;
        for (physical, effective) in extrema
            .iter()
            .flatten()
            .zip(auxiliary.extremal_parameters().iter().flatten())
        {
            identical &= physical.compare(effective, budget)? == Ordering::Equal;
        }
        let mathematical = if identical {
            auxiliary.mathematical().clone()
        } else {
            let width = [
                extrema[1][0].sub(&extrema[0][0], budget)?,
                extrema[1][1].sub(&extrema[0][1], budget)?,
            ];
            PiWidthHorizon::check(
                width,
                auxiliary.mathematical().reference().clone(),
                auxiliary.mathematical().pi_bounds().clone(),
                budget,
            )?
        };
        let bytes = extrema
            .iter()
            .flatten()
            .fold(bytes, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(mathematical.retained_payload_bytes());
        let bytes = bytes
            .saturating_add(auxiliary.retained_payload_bytes())
            .saturating_add(auxiliary.source_payload_bytes())
            .saturating_add(2_048);
        storage::retained(bytes, budget)?;
        Ok(Self(Rc::new(Data {
            original,
            auxiliary,
            extrema,
            mathematical,
            bytes,
        })))
    }

    /// Complete original physical relation and every original input.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        match &self.0.original {
            Original::Phase { cover, .. } => cover.relation(),
            Original::Amplitude { cover, .. } => cover.relation(),
        }
    }

    /// Original clock/phase cover, when no amplitude calibration is attached.
    #[must_use]
    pub fn clock_phase_cover(&self) -> Option<&LcClockPhaseCover> {
        match &self.0.original {
            Original::Phase { cover, .. } => Some(cover),
            Original::Amplitude { .. } => None,
        }
    }

    /// Original clock/phase/amplitude cover, with all calibration fibres retained.
    #[must_use]
    pub fn shared_measurement_cover(&self) -> Option<&LcSharedMeasurementCover> {
        match &self.0.original {
            Original::Amplitude { cover, .. } => Some(cover),
            Original::Phase { .. } => None,
        }
    }

    /// Full original phase witnesses for the lower and upper future extremizers, in that order.
    #[must_use]
    pub fn phase_witnesses(&self) -> Option<&[LcClockPhaseWitness]> {
        match &self.0.original {
            Original::Phase { witnesses, .. } => Some(witnesses),
            Original::Amplitude { .. } => None,
        }
    }

    /// Full original amplitude/phase witnesses for both future extremizers, in endpoint order.
    #[must_use]
    pub fn amplitude_witnesses(&self) -> Option<&[LcSharedMeasurementWitness]> {
        match &self.0.original {
            Original::Amplitude { witnesses, .. } => Some(witnesses),
            Original::Phase { .. } => None,
        }
    }

    /// Conservative complete original and auxiliary proof ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }

    /// Exact physical image after restoring any original shared phase offset.
    pub(super) fn extrema(&self) -> &[[Rational; 2]; 2] {
        &self.0.extrema
    }

    /// Checked threshold for the physical image, reusing unchanged prepared evidence.
    pub(super) fn mathematical(&self) -> &PiWidthHorizon {
        &self.0.mathematical
    }

    /// Only the retaining physical horizon constructor can reuse the equivalent auxiliary proof.
    pub(super) fn auxiliary(&self) -> &LcHorizon {
        &self.0.auxiliary
    }
}

/// The original full witness attains the exact reported intercept and rate used in the horizon.
fn extremizer(
    point: &LcWitness,
    clock: &Rational,
    expected: &[Rational; 2],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if matches_extremizer(point, clock, expected, budget)? {
        Ok(())
    } else {
        Err(LcRelationError::Containment)
    }
}

/// A sealed immutable original tuple can be reused after an exact endpoint identity check.
/// This is a lookup of existing physical evidence, never a new witness inferred from marginals.
pub(super) fn matches_extremizer(
    point: &LcWitness,
    clock: &Rational,
    expected: &[Rational; 2],
    budget: &mut Budget,
) -> Result<bool, LcRelationError> {
    budget.work(1, AT)?;
    let values = point.values();
    let Some(rate) = values.rate.as_rational() else {
        return Ok(false);
    };
    let Some(alpha) = values
        .initial_phase
        .as_ref()
        .and_then(crate::ExactConstant::as_rational)
    else {
        return Ok(false);
    };
    let intercept = alpha.add(&rate.mul(clock, budget)?, budget)?;
    let same_rate = rate.compare(&expected[1], budget)? == Ordering::Equal;
    let same_intercept = intercept.compare(&expected[0], budget)? == Ordering::Equal;
    Ok(same_rate && same_intercept)
}
