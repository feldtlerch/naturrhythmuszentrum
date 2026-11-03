// SPDX-License-Identifier: AGPL-3.0-only
//! Conserved energy readings retain their own shared calibration beside amplitude calibration.

mod project;

use super::{
    AT, LcRelationError,
    calibration::{self, Bounds, Calibration},
    storage,
};
use crate::{
    Budget, LcAmplitudeCover, LcAmplitudeParameters, LcAmplitudeSamples, LcAmplitudeWitness,
    LcCertainty, LcConsistency, LcDeterminacy, LcInterval, LcPhaseGauge, LcQuantity, LcRelation,
    LcValues,
};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Original amplitude question and every energy reading share one conserved physical energy.
#[derive(Clone, Debug)]
pub struct LcEnergySamples(Rc<Question>);

/// Complete original measurement domain, before elimination of either calibration.
#[derive(Debug)]
struct Question {
    /// Original physical equations, observations and amplitude calibration.
    amplitude: LcAmplitudeSamples,
    /// Independent energy reading intervals and one positive common calibration.
    calibration: Calibration,
    /// Physical energy support after intersecting all original energy restrictions.
    energy: Bounds,
    /// Conservative complete ownership, including shared original owners.
    bytes: u64,
}

/// Original physical and amplitude equations checked with one shared energy calibration.
#[derive(Clone, Debug)]
pub struct LcEnergyWitness {
    /// Complete original energy measurement owner.
    energy: LcEnergySamples,
    /// Original amplitude owner, physical relation and amplitude gain.
    original: LcAmplitudeWitness,
    /// Identical positive calibration for all energy readings.
    calibration: Rational,
    /// Cumulative original and energy measurement checks.
    usage: [BudgetUsage; 2],
}

/// Complete phase branches and the joint energy/amplitude scale domain.
#[derive(Clone, Debug)]
pub struct LcEnergyCover(Rc<Cover>);

/// The auxiliary energy marginal is valid only with the original calibration fibres retained.
#[derive(Debug)]
struct Cover {
    /// Complete original joint question.
    energy: LcEnergySamples,
    /// Exact elimination of energy readings and calibration into physical energy support.
    effective: LcAmplitudeCover,
    /// Rechecked original joint nonemptiness evidence.
    witness: LcEnergyWitness,
    /// Full cumulative projection receipt.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained ownership.
    bytes: u64,
}

impl LcEnergySamples {
    /// Retain positive per-sample energy intervals with one shared multiplicative calibration.
    ///
    /// Physical energy is `calibration*reading` for every sample. Existing original physical
    /// energy restrictions are intersected jointly. The amplitude owner's supported domain applies.
    /// # Errors
    /// Refuses nonpositive/nonrational supports, empty intersections and exhausted resources.
    pub fn new(
        amplitude: LcAmplitudeSamples,
        samples: Vec<LcInterval>,
        gain: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let calibration = Calibration::new(samples, gain, budget)?;
        let mut energy = calibration.image.clone();
        if let Some(original) = amplitude.energy() {
            calibration::intersect(&mut energy, original, budget)?;
        }
        let bytes = energy
            .iter()
            .fold(calibration.bytes, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(amplitude.retained_payload_bytes())
            .saturating_add(amplitude.timing().retained_payload_bytes())
            .saturating_add(amplitude.relation().retained_payload_bytes())
            .saturating_add(1_024);
        storage::retained(bytes, budget)?;
        Ok(Self(Rc::new(Question {
            amplitude,
            calibration,
            energy,
            bytes,
        })))
    }

    /// Original amplitude measurement question, with its own shared gain unchanged.
    #[must_use]
    pub fn amplitude(&self) -> &LcAmplitudeSamples {
        &self.0.amplitude
    }

    /// Original physical relation and every original declaration.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.amplitude().relation()
    }

    /// Every original independent energy reading interval.
    #[must_use]
    pub fn samples(&self) -> &[LcInterval] {
        &self.0.calibration.samples
    }

    /// Common energy reading support before calibration.
    #[must_use]
    pub fn measured(&self) -> &Bounds {
        &self.0.calibration.measured
    }

    /// One energy calibration shared by all energy readings.
    #[must_use]
    pub fn calibration(&self) -> &Bounds {
        &self.0.calibration.gain
    }

    /// Exact physical energy marginal, retaining conditional calibration fibres in this owner.
    #[must_use]
    pub fn energy(&self) -> &Bounds {
        &self.0.energy
    }

    /// Allocation identity of the complete original energy question.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Complete original phase branches and physical scale projections.
    /// # Errors
    /// Refuses unsupported phase domains, incomplete cycle coverage and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcEnergyCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Check every original equation and every sample with its quantity's single shared gain.
    /// # Errors
    /// Refuses physical or measurement violations and exhausted resources.
    pub fn check_witness(
        &self,
        values: LcValues,
        amplitude_gain: Rational,
        calibration: Rational,
        budget: &mut Budget,
    ) -> Result<LcEnergyWitness, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        self.0
            .calibration
            .check(&values.energy, &calibration, budget)?;
        let original = self
            .amplitude()
            .check_witness(values, amplitude_gain, budget)?;
        let bytes = self
            .retained_payload_bytes()
            .saturating_add(original.original().retained_payload_bytes())
            .saturating_add(storage::rational(original.gain()))
            .saturating_add(storage::rational(&calibration))
            .saturating_add(512);
        storage::retained(bytes, budget)?;
        Ok(LcEnergyWitness {
            energy: self.clone(),
            original,
            calibration,
            usage: [before, budget.usage()],
        })
    }

    /// Conservative complete ownership including the original amplitude/timing relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcEnergyWitness {
    /// Complete original energy measurement owner.
    #[must_use]
    pub const fn energy(&self) -> &LcEnergySamples {
        &self.energy
    }

    /// Checked original amplitude and physical tuple, retaining original allocation identities.
    #[must_use]
    pub const fn original(&self) -> &LcAmplitudeWitness {
        &self.original
    }

    /// Identical calibration used to check every energy reading.
    #[must_use]
    pub const fn calibration(&self) -> &Rational {
        &self.calibration
    }

    /// Cumulative original and measurement-checking snapshots.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

impl LcEnergyCover {
    /// Original joint energy and amplitude question.
    #[must_use]
    pub fn energy(&self) -> &LcEnergySamples {
        &self.0.energy
    }

    /// Original physical equations and observations.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.energy().relation()
    }

    /// Complete physical rate marginal with its phase fibre retained.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Complete original cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// All common integer gauges remain those of the original phase domain.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Complete branch coverage and original nonemptiness have been checked.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// A full original physical and measurement witness establishes feasibility.
    #[must_use]
    pub const fn consistency(&self) -> LcConsistency {
        LcConsistency::Feasible
    }

    /// Structural finite determinacy of the admitted physical scale queries.
    /// # Errors
    /// Refuses quantities outside the closed amplitude projection adapter.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        self.0.effective.determinacy(quantity)
    }

    /// Exact physical marginals retain this joint owner and both calibration fibres.
    /// # Errors
    /// Refuses unsupported quantities and exhausted resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        self.0.effective.bounds(quantity, budget)
    }

    /// Exact initial-phase fibre at a retained physical rate.
    /// # Errors
    /// Refuses rates outside the complete domain and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Retained energy/calibration correlation at a selected physical energy.
    /// # Errors
    /// Refuses energies outside the original joint domain and exhausted resources.
    pub fn calibration_at_energy(
        &self,
        energy: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        calibration::contains(self.energy().energy(), energy, budget)?;
        self.energy().0.calibration.gain_at_value(energy, budget)
    }

    /// Reconstruct a full tuple using one physical energy and both retained shared gains.
    /// # Errors
    /// Refuses crossed marginals, original violations and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcAmplitudeParameters,
        calibration: Rational,
        budget: &mut Budget,
    ) -> Result<LcEnergyWitness, LcRelationError> {
        budget.work(1, AT)?;
        self.energy().0.calibration.check(
            &parameters.energy.clone().into(),
            &calibration,
            budget,
        )?;
        let point = self.0.effective.witness(parameters, budget)?;
        self.energy().check_witness(
            point.original().values().clone(),
            point.gain().clone(),
            calibration,
            budget,
        )
    }

    /// Checked original joint nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcEnergyWitness {
        &self.0.witness
    }

    /// Cumulative complete projection snapshots.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative complete retained ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
