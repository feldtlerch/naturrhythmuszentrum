// SPDX-License-Identifier: AGPL-3.0-only
//! Shared inductance calibration coexists with bounded original capacitance and complete timing.

mod forward;
mod project;
pub use forward::{LcCalibratedForwardCover, LcCalibratedForwardWitness};

use super::{
    AT, LcRelationError,
    calibration::{self, Calibration},
    storage,
};
use crate::{
    Budget, LcCertainty, LcInput, LcInterval, LcJointElementsCover, LcJointElementsParameters,
    LcQuantity as Q, LcRelation, LcSampleTimes, LcWitness,
};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Complete original simultaneous element question with one common inductance calibration.
#[derive(Clone, Debug)]
pub struct LcCalibratedElements(Rc<Question>);

/// Every original physical and measured input survives the redundant auxiliary restriction.
#[derive(Debug)]
struct Question {
    /// Original independent timing owner and both original element supports.
    timing: LcSampleTimes,
    /// Every inductance reading and its single positive common calibration.
    calibration: Calibration,
    /// Original inductance intersected with the complete calibrated measurement image.
    inductance: [Rational; 2],
    /// Conservative retained complete original and calibration metadata.
    bytes: u64,
}

/// Conditional original element coordinates keep their single calibration explicit.
#[derive(Clone, Debug)]
pub struct LcCalibratedElementsParameters {
    /// Joint timing and element point, including conditional capacitance and free positive voltage.
    pub physical: LcJointElementsParameters,
    /// One positive gain shared by every original inductance reading.
    pub calibration: Rational,
}

/// Complete original timing/element image with a retained conditional calibration fibre.
#[derive(Clone, Debug)]
pub struct LcCalibratedElementsCover(Rc<Cover>);

/// The derived physical strip is exact only together with its original calibration witness.
#[derive(Debug)]
struct Cover {
    /// Complete original joint question, not the derived physical relation.
    question: LcCalibratedElements,
    /// Original-implied physical intersection and independently complete timing proof.
    effective: LcJointElementsCover,
    /// Full original feasibility, including every retained measurement under one gain.
    witness: LcCalibratedElementsWitness,
    /// Cumulative construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained payload.
    bytes: u64,
}

/// Every original equation and inductance reading is checked at one joint point.
#[derive(Clone, Debug)]
pub struct LcCalibratedElementsWitness {
    /// Exact complete original joint allocation.
    question: LcCalibratedElements,
    /// Full original physical tuple, with unchanged source authority.
    original: LcWitness,
    /// Single common multiplicative calibration checked for all readings.
    calibration: Rational,
    /// Cumulative physical and calibration checking snapshots.
    usage: [BudgetUsage; 2],
}

impl LcCalibratedElements {
    /// Retain bounded original L/C supports with independent readings and one shared L calibration.
    /// # Errors
    /// Refuses missing/nonpositive/unbounded original elements, invalid calibration or exhausted Work.
    pub fn new(
        timing: LcSampleTimes,
        samples: Vec<LcInterval>,
        gain: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let calibration = Calibration::new(samples, gain, budget)?;
        let mut inductance = calibration.image.clone();
        let mut found = [false; 2];
        for input in timing.relation().inputs() {
            budget.work(1, AT)?;
            match input.quantity() {
                Q::Inductance => {
                    calibration::intersect(
                        &mut inductance,
                        &calibration::positive_bounds(input.interval(), budget)?,
                        budget,
                    )?;
                    found[0] = true;
                }
                Q::Capacitance => {
                    calibration::positive_bounds(input.interval(), budget)?;
                    found[1] = true;
                }
                Q::Time(_) | Q::Phase(_) | Q::InitialPhase | Q::Rate | Q::AngularFrequency => {}
                _ => return Err(LcRelationError::Projection),
            }
        }
        if found != [true, true] {
            return Err(LcRelationError::Projection);
        }
        let bytes = inductance
            .iter()
            .fold(calibration.bytes.saturating_add(1_024), |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(timing.retained_payload_bytes())
            .saturating_add(timing.relation().retained_payload_bytes());
        storage::retained(bytes, budget)?;
        Ok(Self(Rc::new(Question {
            timing,
            calibration,
            inductance,
            bytes,
        })))
    }

    /// Complete original physical relation and every independent timing declaration.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }

    /// Original physical equations and inputs, before any auxiliary implied strip.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }

    /// All original independent inductance readings.
    #[must_use]
    pub fn samples(&self) -> &[LcInterval] {
        &self.0.calibration.samples
    }

    /// Original shared positive calibration support.
    #[must_use]
    pub fn calibration(&self) -> &[Rational; 2] {
        &self.0.calibration.gain
    }

    /// Calibrated inductance support intersected with every original L declaration.
    #[must_use]
    pub fn inductance(&self) -> &[Rational; 2] {
        &self.0.inductance
    }

    /// Complete original joint-question allocation authority.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Retain complete original rate/cycle coverage and both conditional element/calibration fibres.
    /// # Errors
    /// Refuses unsupported product roots, incomplete timing covers and exhausted resources.
    pub fn cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcCalibratedElementsCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Exact calibration fibre within the calibrated L support; the cover checks timing/C compatibility.
    /// # Errors
    /// Refuses inductance outside that support and exhausted resources.
    pub fn calibration_at_inductance(
        &self,
        value: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        calibration::contains(self.inductance(), value, budget)?;
        self.0.calibration.gain_at_value(value, budget)
    }

    /// Full original physical and measurement substitution with one common gain.
    /// # Errors
    /// Refuses physical equations, original input membership, crossed calibration and exhausted Work.
    pub fn check_witness(
        &self,
        values: crate::LcValues,
        gain: Rational,
        budget: &mut Budget,
    ) -> Result<LcCalibratedElementsWitness, LcRelationError> {
        let before = budget.usage();
        self.0
            .calibration
            .check(&values.inductance, &gain, budget)?;
        let original = self.relation().check_witness(values, budget)?;
        self.retain_witness(original, gain, &before, budget)
    }

    /// Reuse only an unchanged sealed physical tuple and recheck the complete original inputs.
    fn restore_physical(
        &self,
        physical: &LcWitness,
        gain: Rational,
        budget: &mut Budget,
    ) -> Result<LcCalibratedElementsWitness, LcRelationError> {
        let before = budget.usage();
        self.0
            .calibration
            .check(&physical.values().inductance, &gain, budget)?;
        let original = physical.recheck_inputs_for(self.relation(), budget)?;
        self.retain_witness(original, gain, &before, budget)
    }

    /// Both the physical equations and shared readings are already checked on this ledger.
    fn retain_witness(
        &self,
        original: LcWitness,
        gain: Rational,
        before: &BudgetUsage,
        budget: &mut Budget,
    ) -> Result<LcCalibratedElementsWitness, LcRelationError> {
        budget.work(1, AT)?;
        storage::retained(
            self.0
                .bytes
                .saturating_add(original.retained_payload_bytes())
                .saturating_add(storage::rational(&gain))
                .saturating_add(512),
            budget,
        )?;
        Ok(LcCalibratedElementsWitness {
            question: self.clone(),
            original,
            calibration: gain,
            usage: [*before, budget.usage()],
        })
    }

    /// Conservative complete retained original and calibration ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcCalibratedElementsCover {
    /// Complete original joint measurement question.
    #[must_use]
    pub fn question(&self) -> &LcCalibratedElements {
        &self.0.question
    }

    /// Complete original physical relation, without substituting an auxiliary allocation.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.question().relation()
    }

    /// Exact complete rate projection after original timing, both elements and shared calibration.
    #[must_use]
    pub fn rate(&self) -> &[Rational; 2] {
        self.0.effective.rate()
    }

    /// Complete retained original cycle representative.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// Complete original branch and simultaneous scale coverage is certified.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Original fixed or all-integer common phase-gauge contract.
    #[must_use]
    pub fn phase_gauge(&self) -> crate::LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Conditional physical phase at the selected rate.
    /// # Errors
    /// Refuses outside rates and exhausted Work.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Original C fibre retains the implied L calibration image and `a²LC=1` jointly.
    /// # Errors
    /// Refuses outside rates and exhausted Work.
    pub fn capacitance_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.effective.capacitance_at_rate(rate, budget)
    }

    /// Reconstruct every original physical equation and every reading at one common calibration.
    /// # Errors
    /// Refuses crossed conditional parameters and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcCalibratedElementsParameters,
        budget: &mut Budget,
    ) -> Result<LcCalibratedElementsWitness, LcRelationError> {
        budget.with_retained_payload(self.0.bytes, AT, |budget| {
            let physical = self.0.effective.witness(&parameters.physical, budget)?;
            self.question()
                .restore_physical(&physical, parameters.calibration.clone(), budget)
        })
    }

    /// Sealed original joint nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcCalibratedElementsWitness {
        &self.0.witness
    }

    /// Complete construction remains on the caller's cumulative ledger.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative complete original, auxiliary and witness ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcCalibratedElementsWitness {
    /// Exact complete original measurement allocation.
    #[must_use]
    pub const fn question(&self) -> &LcCalibratedElements {
        &self.question
    }

    /// Original physical equations and every original input were checked.
    #[must_use]
    pub const fn original(&self) -> &LcWitness {
        &self.original
    }

    /// Single positive common calibration used for every original reading.
    #[must_use]
    pub const fn calibration(&self) -> &Rational {
        &self.calibration
    }

    /// Full original checking snapshots on the cumulative ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}
