// SPDX-License-Identifier: AGPL-3.0-only
//! Positive energy elimination is exact only with the original calibration and conditional fibres retained.

use super::{
    AT, Cover, LcEnergyOnlyCalibrationCover, LcEnergyOnlyParameters, LcEnergyOnlySamples,
    LcEnergyOnlyWitness, LcInterval, LcRelation, LcRelationError, LcSampleTimes, calibration,
    storage,
};
use crate::{
    Budget, LcCertainty, LcConsistency, LcDeterminacy, LcFreeDirection, LcInput, LcPhaseGauge,
    LcQuantity,
};
use calibration::Bounds;
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Every physical energy in the exact image has a nonempty original calibration fibre.
pub(super) fn cover(
    energy: &LcEnergyOnlySamples,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcEnergyOnlyCalibrationCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(energy.retained_payload_bytes().saturating_mul(2), budget)?;
    let mut inputs = Vec::new();
    for input in energy.relation().inputs() {
        budget.work(1, AT)?;
        if input.quantity() != LcQuantity::Energy {
            inputs.push(input.clone());
        }
    }
    inputs.push(LcInput::new(
        LcQuantity::Energy,
        &LcQuantity::Energy.descriptor()?,
        calibration::interval(energy.energy(), budget)?,
        budget,
    )?);
    let relation = LcRelation::new(
        energy.relation().epoch().into(),
        energy.relation().observations(),
        inputs,
        budget,
    )?;
    let timing = LcSampleTimes::new(relation, energy.timing().nominal_times().to_vec(), budget)?;
    let effective = timing.energy_only_cover(cycles, pi, budget)?;
    let point = effective.feasibility_witness();
    let value = point
        .values()
        .energy
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let gain = energy.0.calibration.gain_at_value(value, budget)?[0].clone();
    let witness = energy.check_witness(point.values().clone(), gain, budget)?;
    let bytes = energy
        .retained_payload_bytes()
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.calibration()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcEnergyOnlyCalibrationCover(Rc::new(Cover {
        energy: energy.clone(),
        effective,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

impl LcEnergyOnlyCalibrationCover {
    /// Complete original energy reading and independent timing question.
    #[must_use]
    pub fn energy(&self) -> &LcEnergyOnlySamples {
        &self.0.energy
    }

    /// Every original physical input and the common epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.energy().relation()
    }

    /// Complete physical rate marginal with the original timing/phase fibres retained.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Complete original observation cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// Original common integer gauge, unchanged by energy elimination.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Complete branch coverage and full-original nonemptiness are checked.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// A full original physical and measurement witness establishes feasibility.
    #[must_use]
    pub const fn consistency(&self) -> LcConsistency {
        LcConsistency::Feasible
    }

    /// Finite energy/rate images and explicitly free element/amplitude scales.
    /// # Errors
    /// Refuses unsupported scalar quantities.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        self.0.effective.determinacy(quantity)
    }

    /// Free voltage varies element and amplitude scales jointly at each positive energy.
    /// # Errors
    /// Refuses unsupported scalar quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        self.0.effective.free_directions(quantity)
    }

    /// Complete physical images retain their joint original calibration source through this owner.
    /// # Errors
    /// Refuses unsupported quantities and exhausted resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        self.0.effective.bounds(quantity, budget)
    }

    /// Complete original initial-phase fibre at a selected physical rate.
    /// # Errors
    /// Refuses out-of-domain rates and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Conditional shared calibration support at one selected physical energy.
    /// # Errors
    /// Refuses energies outside the original question and exhausted resources.
    pub fn calibration_at_energy(
        &self,
        energy: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        calibration::contains(self.energy().energy(), energy, budget)?;
        self.energy().0.calibration.gain_at_value(energy, budget)
    }

    /// Common reading fibre at one calibration, including every original physical energy bound.
    /// # Errors
    /// Refuses out-of-domain calibration, empty conditional fibres and exhausted resources.
    pub fn reading_at_calibration(
        &self,
        gain: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        calibration::contains(self.energy().calibration(), gain, budget)?;
        let energy = self.energy().energy();
        let mut reading = [energy[0].div(gain, budget)?, energy[1].div(gain, budget)?];
        calibration::intersect(&mut reading, self.energy().measured(), budget)?;
        Ok(reading)
    }

    /// Reconstruct all physical scales from energy/free voltage and recheck every original reading.
    /// # Errors
    /// Refuses crossed calibration/energy parameters, original violations and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcEnergyOnlyParameters,
        gain: Rational,
        budget: &mut Budget,
    ) -> Result<LcEnergyOnlyWitness, LcRelationError> {
        budget.work(1, AT)?;
        let values = self.0.effective.candidate(parameters, budget)?;
        self.energy().check_witness(values, gain, budget)
    }

    /// Full original physical and energy-calibration nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcEnergyOnlyWitness {
        &self.0.witness
    }

    /// Before/after complete projection snapshots on the caller ledger.
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
