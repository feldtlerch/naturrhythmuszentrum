// SPDX-License-Identifier: AGPL-3.0-only
//! Repeated element measurements retain one calibration and the original phase/rate domain.

mod project;

use super::{
    AT, LcRelationError,
    calibration::{self, Bounds, Calibration},
    storage,
};
use crate::{
    Budget, LcCertainty, LcConsistency, LcDeterminacy, LcFreeDirection, LcInterval, LcPhaseGauge,
    LcQuantity as Q, LcRelation, LcSampleTimes, LcTimingCover, LcValues, LcWitness,
};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Original independent timing question and repeated calibrated measurements of one element.
#[derive(Clone, Debug)]
pub struct LcElementSamples(Rc<Question>);

/// One element's full measurement domain retains every original physical restriction.
#[derive(Debug)]
struct Question {
    /// Original equations, observation inventory and independent timestamp supports.
    timing: LcSampleTimes,
    /// Measured physical element; inductance and capacitance have separate calibration owners.
    element: Q,
    /// Every repeated reading interval and its single shared positive calibration.
    calibration: Calibration,
    /// Physical element support intersected with all original element restrictions.
    values: Bounds,
    /// Conservative complete retained ownership.
    bytes: u64,
}

/// Joint coordinates retain element/calibration and rate/phase correlations.
#[derive(Clone, Debug)]
pub struct LcElementParameters {
    /// Physical positive rate in the recovered domain.
    pub rate: Rational,
    /// Local initial phase in its rate-dependent fibre.
    pub initial_phase: Rational,
    /// Original common integer phase gauge.
    pub gauge: Rational,
    /// Physical value of the measured element.
    pub element: Rational,
    /// The single positive calibration for every element reading.
    pub calibration: Rational,
    /// Positive free voltage amplitude.
    pub voltage: Rational,
}

/// Full original physical tuple with every element reading checked under one calibration.
#[derive(Clone, Debug)]
pub struct LcElementWitness {
    /// Complete original element measurement question.
    element: LcElementSamples,
    /// Physical equations and every original constraint have been checked.
    original: LcWitness,
    /// Identical calibration used for every retained reading.
    calibration: Rational,
    /// Cumulative original and measurement checking receipt.
    usage: [BudgetUsage; 2],
}

/// Complete original phase branches and the retained element/calibration domain.
#[derive(Clone, Debug)]
pub struct LcElementCover(Rc<Cover>);

/// Positive element elimination leaves the original phase/rate question independent.
#[derive(Debug)]
struct Cover {
    /// Complete original measurement owner.
    element: LcElementSamples,
    /// Exact phase/rate and independent-timestamp domain.
    effective: LcTimingCover,
    /// Reconstructed original joint nonemptiness witness.
    witness: LcElementWitness,
    /// Cumulative cover construction receipt.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained ownership.
    bytes: u64,
}

impl LcElementSamples {
    /// Admit repeated positive readings of one element with one positive shared calibration.
    ///
    /// The physical element equals `calibration*reading` for every sample. Original element
    /// restrictions intersect jointly; other simultaneous physical scales refuse this adapter.
    /// Independent timestamp uncertainty and unknown physical initial phase remain supported.
    /// # Errors
    /// Refuses unsupported elements/scales, invalid supports and exhausted resources.
    pub fn new(
        timing: LcSampleTimes,
        element: Q,
        samples: Vec<LcInterval>,
        gain: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        if !matches!(element, Q::Inductance | Q::Capacitance) {
            return Err(LcRelationError::Projection);
        }
        let calibration = Calibration::new(samples, gain, budget)?;
        let mut values = calibration.image.clone();
        for input in timing.relation().inputs() {
            budget.work(1, AT)?;
            if input.quantity() == element {
                let range = calibration::positive_bounds(input.interval(), budget)?;
                calibration::intersect(&mut values, &range, budget)?;
            } else if !matches!(
                input.quantity(),
                Q::Time(_) | Q::Phase(_) | Q::InitialPhase | Q::Rate | Q::AngularFrequency
            ) {
                return Err(LcRelationError::Projection);
            }
        }
        let bytes = values
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
            element,
            calibration,
            values,
            bytes,
        })))
    }

    /// Original independent timestamp owner and complete physical relation.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }

    /// Every original equation and physical input, including repeated element restrictions.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }

    /// Physical quantity measured by the retained reading inventory.
    #[must_use]
    pub fn element(&self) -> Q {
        self.0.element
    }

    /// Every original independent element reading interval.
    #[must_use]
    pub fn samples(&self) -> &[LcInterval] {
        &self.0.calibration.samples
    }

    /// Common reading support before the shared calibration.
    #[must_use]
    pub fn measured(&self) -> &Bounds {
        &self.0.calibration.measured
    }

    /// One positive calibration shared by all readings of this element.
    #[must_use]
    pub fn calibration(&self) -> &Bounds {
        &self.0.calibration.gain
    }

    /// Physical element marginal intersected with every original restriction.
    #[must_use]
    pub fn values(&self) -> &Bounds {
        &self.0.values
    }

    /// Allocation identity of the complete original joint question.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Recover complete original phase branches with all element/calibration fibres retained.
    /// # Errors
    /// Refuses unsupported phase domains, incomplete cycle coverage and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcElementCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Check the full original physical tuple and every reading with the same calibration.
    /// # Errors
    /// Refuses original or measured constraint violations and exhausted resources.
    pub fn check_witness(
        &self,
        values: LcValues,
        calibration: Rational,
        budget: &mut Budget,
    ) -> Result<LcElementWitness, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let value = if self.element() == Q::Inductance {
            &values.inductance
        } else {
            &values.capacitance
        };
        self.0.calibration.check(value, &calibration, budget)?;
        let original = self.relation().check_witness(values, budget)?;
        let bytes = self
            .retained_payload_bytes()
            .saturating_add(original.retained_payload_bytes())
            .saturating_add(storage::rational(&calibration))
            .saturating_add(512);
        storage::retained(bytes, budget)?;
        Ok(LcElementWitness {
            element: self.clone(),
            original,
            calibration,
            usage: [before, budget.usage()],
        })
    }

    /// Conservative complete original and measurement metadata ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcElementWitness {
    /// Original complete element measurement owner.
    #[must_use]
    pub const fn element(&self) -> &LcElementSamples {
        &self.element
    }

    /// Physical tuple checked against the unchanged original relation.
    #[must_use]
    pub const fn original(&self) -> &LcWitness {
        &self.original
    }

    /// Single calibration checked for every element reading.
    #[must_use]
    pub const fn calibration(&self) -> &Rational {
        &self.calibration
    }

    /// Cumulative original and measured witness-checking receipt.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

impl LcElementCover {
    /// Complete original element and timing measurement question.
    #[must_use]
    pub fn element(&self) -> &LcElementSamples {
        &self.0.element
    }

    /// Original full physical relation and observation inventory.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.element().relation()
    }

    /// Complete recovered positive rate support.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Original complete cycle association, without auxiliary observations.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// All original common integer initial-phase gauges remain retained.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Complete original branch coverage and nonemptiness are checked.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// A joint original physical and measurement witness establishes feasibility.
    #[must_use]
    pub const fn consistency(&self) -> LcConsistency {
        LcConsistency::Feasible
    }

    /// Element and ratio queries are finite; voltage/current/energy retain a free amplitude scale.
    /// # Errors
    /// Refuses quantities outside the admitted scalar projection surface.
    pub const fn determinacy(&self, quantity: Q) -> Result<LcDeterminacy, LcRelationError> {
        match quantity {
            Q::Rate
            | Q::AngularFrequency
            | Q::Product
            | Q::NaturalTime
            | Q::Inductance
            | Q::Capacitance
            | Q::AmplitudeRatio => Ok(LcDeterminacy::DeterminedFinite),
            Q::VoltageAmplitude | Q::CurrentAmplitude | Q::Energy => {
                Ok(LcDeterminacy::Underdetermined)
            }
            Q::Time(_) | Q::Phase(_) | Q::InitialPhase => Err(LcRelationError::Projection),
        }
    }

    /// The remaining physical freedom is amplitude scale, never element/calibration independence.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn free_directions(
        &self,
        quantity: Q,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        if self.determinacy(quantity)? == LcDeterminacy::Underdetermined {
            Ok(&[LcFreeDirection::AmplitudeScale])
        } else {
            Ok(&[])
        }
    }

    /// Exact original phase fibre at a chosen physical rate.
    /// # Errors
    /// Refuses out-of-domain rates and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Conditional calibration support at one physical element value.
    /// # Errors
    /// Refuses values outside the joint original domain and exhausted resources.
    pub fn calibration_at_element(
        &self,
        value: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        calibration::contains(self.element().values(), value, budget)?;
        self.element().0.calibration.gain_at_value(value, budget)
    }

    /// Exact physical marginals retain the original joint element/calibration/rate domain.
    /// # Errors
    /// Refuses unsupported quantities and exhausted resources.
    pub fn bounds(&self, quantity: Q, budget: &mut Budget) -> Result<LcInterval, LcRelationError> {
        project::bounds(self, quantity, budget)
    }

    /// Reconstruct all original equations from correlated phase and calibration coordinates.
    /// # Errors
    /// Refuses crossed marginals, original violations and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcElementParameters,
        budget: &mut Budget,
    ) -> Result<LcElementWitness, LcRelationError> {
        project::witness(self.element(), &self.0.effective, parameters, budget)
    }

    /// Reconstructed original nonemptiness evidence.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcElementWitness {
        &self.0.witness
    }

    /// Cumulative complete cover construction snapshots.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative complete retained metadata ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
