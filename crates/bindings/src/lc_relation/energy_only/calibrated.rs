// SPDX-License-Identifier: AGPL-3.0-only
//! Original energy readings share one calibration while voltage remains a free positive coordinate.

mod project;

use super::{
    AT, LcEnergyOnlyCover, LcEnergyOnlyParameters, LcInterval, LcRelation, LcRelationError,
    LcSampleTimes, LcWitness, calibration, storage,
};
use crate::{Budget, LcQuantity, LcValues};
use calibration::{Bounds, Calibration};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Original independent timing and energy readings with one shared positive energy calibration.
#[derive(Clone, Debug)]
pub struct LcEnergyOnlySamples(Rc<Question>);

/// The physical relation and each original reading remain authoritative after elimination.
#[derive(Debug)]
struct Question {
    /// Complete original physical and independent timing question.
    timing: LcSampleTimes,
    /// Every energy reading and the single declared calibration.
    calibration: Calibration,
    /// Positive physical energy image intersected with every original energy declaration.
    energy: Bounds,
    /// Conservative complete ownership including shared original sources.
    bytes: u64,
}

/// Exact full original physical tuple and one calibration checked against every energy reading.
#[derive(Clone, Debug)]
pub struct LcEnergyOnlyWitness {
    /// Original complete calibrated energy question.
    energy: LcEnergyOnlySamples,
    /// Full physical tuple rechecked against the original relation.
    original: LcWitness,
    /// Identical calibration for every independent energy reading.
    calibration: Rational,
    /// Caller counters before and after all original checks.
    usage: [BudgetUsage; 2],
}

/// Complete original timing/energy domain with retained calibration fibres and free voltage.
#[derive(Clone, Debug)]
pub struct LcEnergyOnlyCalibrationCover(Rc<Cover>);

/// Energy elimination retains its inverse fibre beside the complete timing and physical map.
#[derive(Debug)]
struct Cover {
    /// Original joint question, never replaced by marginal energy boxes.
    energy: LcEnergyOnlySamples,
    /// Exact physical energy elimination with one free positive voltage coordinate.
    effective: LcEnergyOnlyCover,
    /// Full-original-equation and shared-calibration nonemptiness evidence.
    witness: LcEnergyOnlyWitness,
    /// Caller counters around complete projection and reconstruction.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained payload.
    bytes: u64,
}

impl LcEnergyOnlySamples {
    /// Retain positive energy readings and one common multiplicative calibration without an amplitude channel.
    ///
    /// Physical energy equals `calibration*reading` for every sample. Original physical
    /// energy constraints intersect that image; all original declarations remain retained.
    /// # Errors
    /// Refuses nonpositive/nonrational reading supports, incompatible energy bounds and exhausted resources.
    pub fn new(
        timing: LcSampleTimes,
        samples: Vec<LcInterval>,
        gain: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let calibration = Calibration::new(samples, gain, budget)?;
        let mut energy = calibration.image.clone();
        for input in timing.relation().inputs() {
            budget.work(1, AT)?;
            if input.quantity() == LcQuantity::Energy {
                calibration::clip_interval(&mut energy, input.interval(), budget)?;
            }
        }
        let bytes = energy
            .iter()
            .fold(calibration.bytes, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(timing.retained_payload_bytes())
            .saturating_add(timing.relation().retained_payload_bytes())
            .saturating_add(1_024);
        storage::retained(bytes, budget)?;
        Ok(Self(Rc::new(Question {
            timing,
            calibration,
            energy,
            bytes,
        })))
    }

    /// Complete original physical and independent timing question.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }

    /// Original physical relation, epoch and every repeated input.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }

    /// Every original independent energy reading interval.
    #[must_use]
    pub fn samples(&self) -> &[LcInterval] {
        &self.0.calibration.samples
    }

    /// Common reading support before multiplication by the shared calibration.
    #[must_use]
    pub fn measured(&self) -> &Bounds {
        &self.0.calibration.measured
    }

    /// One declared calibration shared by every original energy reading.
    #[must_use]
    pub fn calibration(&self) -> &Bounds {
        &self.0.calibration.gain
    }

    /// Complete physical energy image with all original energy constraints retained.
    #[must_use]
    pub fn energy(&self) -> &Bounds {
        &self.0.energy
    }

    /// Identity of the complete original calibrated question allocation.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Recover complete timing branches while retaining the energy calibration and free voltage.
    /// # Errors
    /// Refuses unsupported timing/scale constraints, incomplete cycle coverage and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcEnergyOnlyCalibrationCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Recheck every original physical equation and energy reading with one common calibration.
    /// # Errors
    /// Refuses physical or measurement violations and exhausted caller resources.
    pub fn check_witness(
        &self,
        values: LcValues,
        calibration: Rational,
        budget: &mut Budget,
    ) -> Result<LcEnergyOnlyWitness, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        self.0
            .calibration
            .check(&values.energy, &calibration, budget)?;
        let original = self.relation().check_witness(values, budget)?;
        let bytes = self
            .retained_payload_bytes()
            .saturating_add(original.retained_payload_bytes())
            .saturating_add(storage::rational(&calibration))
            .saturating_add(512);
        storage::retained(bytes, budget)?;
        Ok(LcEnergyOnlyWitness {
            energy: self.clone(),
            original,
            calibration,
            usage: [before, budget.usage()],
        })
    }

    /// Conservative complete ownership including shared original physical/timing sources.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcEnergyOnlyWitness {
    /// Complete original calibrated energy question.
    #[must_use]
    pub const fn energy(&self) -> &LcEnergyOnlySamples {
        &self.energy
    }

    /// Full original physical tuple, without an invented amplitude measurement.
    #[must_use]
    pub const fn original(&self) -> &LcWitness {
        &self.original
    }

    /// Common calibration checked against every retained energy reading.
    #[must_use]
    pub const fn calibration(&self) -> &Rational {
        &self.calibration
    }

    /// Cumulative original-equation and shared-calibration checking snapshots.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}
