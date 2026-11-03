// SPDX-License-Identifier: AGPL-3.0-only
//! Positive original energy intervals retain one free voltage coordinate without a measurement.

mod calibrated;
mod prepare;
pub use calibrated::{LcEnergyOnlyCalibrationCover, LcEnergyOnlySamples, LcEnergyOnlyWitness};

use super::{
    AT, LcCertainty, LcConsistency, LcDeterminacy, LcFreeDirection, LcInterval, LcPairParameters,
    LcPhaseGauge, LcQuantity, LcRelation, LcRelationError, LcSampleTimes, LcTimingCover, LcWitness,
    amplitude::scale, calibration, storage,
};
use crate::Budget;
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Exact parameters of the full timing/energy domain, with one unconstrained positive voltage.
#[derive(Clone, Debug)]
pub struct LcEnergyOnlyParameters {
    /// Positive coherent rate in the complete recovered timing support.
    pub rate: Rational,
    /// Physical initial phase in its retained conditional fibre.
    pub initial_phase: Rational,
    /// Common integer phase lift, preserving the original gauge contract.
    pub gauge: Rational,
    /// One conserved energy satisfying every original energy interval.
    pub energy: Rational,
    /// Free positive voltage amplitude; it is not an amplitude measurement.
    pub voltage: Rational,
}

/// Complete exact timing recovery with energy intervals and no amplitude measurement channel.
#[derive(Clone, Debug)]
pub struct LcEnergyOnlyCover(Rc<Data>);

/// Eliminated energy restrictions remain in the original owner and every reconstruction.
#[derive(Debug)]
struct Data {
    /// Complete original timing question, including all original energy declarations.
    timing: LcSampleTimes,
    /// Exact joint timing domain after eliminating only independent positive energy.
    effective: LcTimingCover,
    /// Nonempty intersection of all finite positive original energy supports.
    energy: [Rational; 2],
    /// Full-original-relation nonemptiness witness.
    witness: LcWitness,
    /// Before/after complete checking on the caller's cumulative ledger.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained metadata ownership.
    bytes: u64,
}

impl LcSampleTimes {
    /// Recover exact independent timing while retaining original energy intervals and free voltage.
    ///
    /// For positive `a,E,V`, reconstruction uses `C=2E/V²`, `L=V²/(2Ea²)` and
    /// `I=2Ea/V`. Every original input and physical equation is checked again.
    /// # Errors
    /// Refuses missing/nonpositive energy, other physical scale declarations, unsupported
    /// timing domains, incomplete cycle coverage and exhausted caller resources.
    pub fn energy_only_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcEnergyOnlyCover, LcRelationError> {
        let before = budget.usage();
        let (energy, effective) = prepare::domain(self, cycles, pi, budget)?;
        let witness = restore(
            self,
            effective.feasibility_witness(),
            &energy[0],
            &Rational::one(),
            budget,
        )?;
        let bytes = energy
            .iter()
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(self.retained_payload_bytes())
            .saturating_add(self.relation().retained_payload_bytes())
            .saturating_add(effective.retained_payload_bytes())
            .saturating_add(witness.retained_payload_bytes());
        storage::retained(bytes, budget)?;
        Ok(LcEnergyOnlyCover(Rc::new(Data {
            timing: self.clone(),
            effective,
            energy,
            witness,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcEnergyOnlyCover {
    /// Complete original independent timing question, with all energy declarations retained.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }

    /// Complete original physical relation and its epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }

    /// Exact positive intersection of all original energy intervals.
    #[must_use]
    pub fn energy(&self) -> &[Rational; 2] {
        &self.0.energy
    }

    /// Complete rate projection with its timing and initial-phase fibres retained.
    #[must_use]
    pub fn rate(&self) -> &[Rational; 2] {
        self.0.effective.rate()
    }

    /// Complete original cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// Original common integer gauge, unchanged by energy elimination.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Joint nonemptiness and complete timing branch coverage are both checked.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// A full original physical tuple certifies nonemptiness.
    #[must_use]
    pub const fn consistency(&self) -> LcConsistency {
        LcConsistency::Feasible
    }

    /// Energy and rate-derived quantities are finite; both element and amplitude scales remain free.
    /// # Errors
    /// Refuses unsupported phase or observation scalar queries.
    pub const fn determinacy(
        &self,
        quantity: LcQuantity,
    ) -> Result<LcDeterminacy, LcRelationError> {
        match quantity {
            LcQuantity::Energy
            | LcQuantity::Rate
            | LcQuantity::AngularFrequency
            | LcQuantity::Product
            | LcQuantity::NaturalTime => Ok(LcDeterminacy::DeterminedFinite),
            LcQuantity::Inductance
            | LcQuantity::Capacitance
            | LcQuantity::AmplitudeRatio
            | LcQuantity::VoltageAmplitude
            | LcQuantity::CurrentAmplitude => Ok(LcDeterminacy::Underdetermined),
            _ => Err(LcRelationError::Projection),
        }
    }

    /// The free voltage coordinate varies element and amplitude scales together at fixed energy.
    /// # Errors
    /// Refuses unsupported quantities instead of fabricating a scalar classification.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        if self.determinacy(quantity)? == LcDeterminacy::Underdetermined {
            Ok(&[LcFreeDirection::ElementScale])
        } else {
            Ok(&[])
        }
    }

    /// Complete exact marginals retain the original joint source through this owner.
    ///
    /// Unbounded positive scales use zero as an unattained outer lower bound and no upper bound.
    /// # Errors
    /// Refuses unsupported quantities and exhausted caller resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        budget.work(1, AT)?;
        if quantity == LcQuantity::Energy {
            return calibration::interval(self.energy(), budget);
        }
        if self.determinacy(quantity)? == LcDeterminacy::Underdetermined {
            return LcInterval::new(Some(Rational::zero().into()), None, budget);
        }
        self.0.effective.bounds(quantity, budget)
    }

    /// Full initial-phase fibre remains conditional on the selected physical rate.
    /// # Errors
    /// Refuses out-of-domain rates and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Reconstruct all scales from one energy/voltage pair and recheck the complete original question.
    /// # Errors
    /// Refuses nonpositive free voltage, crossed parameters, original violations and exhausted Work.
    pub fn witness(
        &self,
        parameters: &LcEnergyOnlyParameters,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        let values = self.candidate(parameters, budget)?;
        self.relation().check_witness(values, budget)
    }

    /// Restore an unsealed candidate for a retaining owner to check against its complete originals.
    /// This avoids publishing and checking an intermediate physical tuple on an auxiliary relation.
    pub(super) fn candidate(
        &self,
        parameters: &LcEnergyOnlyParameters,
        budget: &mut Budget,
    ) -> Result<crate::LcValues, LcRelationError> {
        budget.work(1, AT)?;
        calibration::contains(self.energy(), &parameters.energy, budget)?;
        calibration::positive(&parameters.voltage, budget)?;
        let phase = LcPairParameters {
            rate: parameters.rate.clone(),
            initial_phase: parameters.initial_phase.clone(),
            gauge: parameters.gauge.clone(),
            ratio: Rational::one(),
            voltage: Rational::one(),
        };
        let point = self.0.effective.witness(&phase, budget)?;
        restore_values(&point, &parameters.energy, &parameters.voltage, budget)
    }

    /// Full original tuple, never the auxiliary timing witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcWitness {
        &self.0.witness
    }

    /// Cumulative receipt for timing recovery, energy elimination and original reconstruction.
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

/// The positive monomial map retains the original phase/time tuple while restoring every scale.
fn restore(
    timing: &LcSampleTimes,
    point: &LcWitness,
    energy: &Rational,
    voltage: &Rational,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    let values = restore_values(point, energy, voltage, budget)?;
    timing.relation().check_witness(values, budget)
}

/// The scale map produces a candidate only; its caller must recheck the final original relation.
fn restore_values(
    point: &LcWitness,
    energy: &Rational,
    voltage: &Rational,
    budget: &mut Budget,
) -> Result<crate::LcValues, LcRelationError> {
    budget.work(1, AT)?;
    calibration::positive(energy, budget)?;
    calibration::positive(voltage, budget)?;
    let mut values = point.values().clone();
    let rate = values
        .rate
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let channel = LcQuantity::VoltageAmplitude;
    values.inductance = scale::coordinate(
        channel,
        LcQuantity::Inductance,
        rate,
        voltage,
        energy,
        budget,
    )?
    .into();
    values.capacitance = scale::coordinate(
        channel,
        LcQuantity::Capacitance,
        rate,
        voltage,
        energy,
        budget,
    )?
    .into();
    values.current = scale::coordinate(
        channel,
        LcQuantity::CurrentAmplitude,
        rate,
        voltage,
        energy,
        budget,
    )?
    .into();
    values.voltage = voltage.clone().into();
    values.energy = energy.clone().into();
    Ok(values)
}
