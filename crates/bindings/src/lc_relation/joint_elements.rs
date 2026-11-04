// SPDX-License-Identifier: AGPL-3.0-only
//! A bounded simultaneous L/C domain retains the original timing and its conditional physical fibre.

mod prepare;
use super::{AT, LcRelationError, calibration, storage};
use crate::{
    Budget, LcCertainty, LcPairParameters, LcPhaseGauge, LcRelation, LcSampleTimes, LcTimingCover,
    LcWitness,
};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Complete bounded original element domain over exact independent timing recovery.
#[derive(Clone, Debug)]
pub struct LcJointElementsCover(Rc<Data>);

/// Every feasible rate keeps its simultaneous positive L/C fibre, never a product of marginal images.
#[derive(Debug)]
struct Data {
    /// Complete original timing question, including every physical element declaration.
    timing: LcSampleTimes,
    /// Equivalent timing projection with independently implied positive rate bounds.
    effective: LcTimingCover,
    /// Original finite positive inductance and capacitance intersections, in that order.
    elements: [[Rational; 2]; 2],
    /// Original nonempty physical tuple after restoring the conditional element coordinates.
    witness: LcWitness,
    /// Complete original construction receipt on the cumulative caller ledger.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained metadata.
    bytes: u64,
}

/// One physical point selects capacitance conditionally on rate and keeps voltage free and positive.
#[derive(Clone, Debug)]
pub struct LcJointElementsParameters {
    /// Rate in the complete original timing/element projection.
    pub rate: Rational,
    /// Local physical initial phase at the selected rate.
    pub initial_phase: Rational,
    /// Original common integer gauge, subject to the original fixed-phase contract.
    pub gauge: Rational,
    /// Capacitance in the retained simultaneous original element fibre.
    pub capacitance: Rational,
    /// Free positive voltage, without an amplitude or energy measurement.
    pub voltage: Rational,
}

impl LcSampleTimes {
    /// Recover one complete bounded simultaneous element domain with conditional physical reconstruction.
    ///
    /// The rate image of positive original L/C boxes is exact; this closed adapter requires
    /// rational normalized square roots of both product endpoints. Other physical scales remain free.
    /// # Errors
    /// Refuses missing/unbounded/nonpositive element supports, other scale declarations,
    /// unsupported roots/timing/cycles and exhausted cumulative resources.
    pub fn joint_elements_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcJointElementsCover, LcRelationError> {
        let before = budget.usage();
        let (elements, effective) = prepare::domain(self, cycles, pi, budget)?;
        let point = effective.feasibility_witness();
        let rate = point
            .values()
            .rate
            .as_rational()
            .ok_or(LcRelationError::Projection)?;
        let fibre = capacitance(&elements, rate, budget)?;
        let witness = restore(self.relation(), point, &fibre[0], &Rational::one(), budget)?;
        let bytes = elements
            .iter()
            .flatten()
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(self.retained_payload_bytes())
            .saturating_add(self.relation().retained_payload_bytes())
            .saturating_add(effective.retained_payload_bytes())
            .saturating_add(witness.retained_payload_bytes());
        storage::retained(bytes, budget)?;
        Ok(LcJointElementsCover(Rc::new(Data {
            timing: self.clone(),
            effective,
            elements,
            witness,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcJointElementsCover {
    /// Complete original timing question, retaining every supplied L/C constraint.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }
    /// Complete original physical relation with its unchanged epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }
    /// Complete exact rate image of the joint original timing/element domain.
    #[must_use]
    pub fn rate(&self) -> &[Rational; 2] {
        self.0.effective.rate()
    }
    /// Original finite L/C supports, before conditional intersection at the selected rate.
    #[must_use]
    pub fn original_elements(&self) -> &[[Rational; 2]; 2] {
        &self.0.elements
    }
    /// Complete original integer cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }
    /// Original common integer gauge, unchanged by positive element elimination.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }
    /// Independent complete branch coverage and a full original tuple establish joint feasibility.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }
    /// Original initial phase remains conditional on the selected joint rate.
    /// # Errors
    /// Refuses rates outside the original projection and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }
    /// Exact fibre `C_original intersect [1/(a² L_max),1/(a² L_min)]`.
    /// # Errors
    /// Refuses rates outside the complete joint support and exhausted resources.
    pub fn capacitance_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        calibration::contains(self.rate(), rate, budget)?;
        capacitance(&self.0.elements, rate, budget)
    }
    /// Reconstruct one full simultaneous original tuple with free positive voltage.
    /// # Errors
    /// Refuses crossed element/phase/rate parameters, nonpositive voltage and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcJointElementsParameters,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        let fibre = self.capacitance_at_rate(&parameters.rate, budget)?;
        calibration::contains(&fibre, &parameters.capacitance, budget)?;
        calibration::positive(&parameters.voltage, budget)?;
        let point = self.0.effective.witness(
            &LcPairParameters {
                rate: parameters.rate.clone(),
                initial_phase: parameters.initial_phase.clone(),
                gauge: parameters.gauge.clone(),
                ratio: Rational::one(),
                voltage: Rational::one(),
            },
            budget,
        )?;
        restore(
            self.relation(),
            &point,
            &parameters.capacitance,
            &parameters.voltage,
            budget,
        )
    }
    /// Original checked physical tuple, never the auxiliary timing witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcWitness {
        &self.0.witness
    }
    /// Cumulative before/after construction snapshots.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }
    /// Conservative complete retained source/proof ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// Exact positive inversion is the original simultaneous product constraint at one rate.
fn capacitance(
    elements: &[[Rational; 2]; 2],
    rate: &Rational,
    budget: &mut Budget,
) -> Result<[Rational; 2], LcRelationError> {
    budget.work(1, AT)?;
    calibration::positive(rate, budget)?;
    let product = Rational::one().div(&rate.pow(2, budget)?, budget)?;
    let mut result = [
        product.div(&elements[0][1], budget)?,
        product.div(&elements[0][0], budget)?,
    ];
    calibration::intersect(&mut result, &elements[1], budget)?;
    Ok(result)
}

/// The positive scale map preserves the original timing tuple and rechecks every original equation.
fn restore(
    relation: &LcRelation,
    point: &LcWitness,
    capacitance: &Rational,
    voltage: &Rational,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    budget.work(1, AT)?;
    calibration::positive(capacitance, budget)?;
    calibration::positive(voltage, budget)?;
    let mut values = point.values().clone();
    let rate = values
        .rate
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let inductance =
        Rational::one().div(&rate.pow(2, budget)?.mul(capacitance, budget)?, budget)?;
    let current = voltage.mul(&rate.mul(capacitance, budget)?, budget)?;
    let energy = capacitance
        .mul(&voltage.pow(2, budget)?, budget)?
        .div(&Rational::from_ratio(2, 1, budget)?, budget)?;
    values.inductance = inductance.into();
    values.capacitance = capacitance.clone().into();
    values.voltage = voltage.clone().into();
    values.current = current.into();
    values.energy = energy.into();
    relation.check_witness(values, budget)
}
