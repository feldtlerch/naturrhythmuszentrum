// SPDX-License-Identifier: AGPL-3.0-only
//! Repeated amplitude intervals retain one common positive gain and the original LC equations.

mod admit;
mod project;
pub(super) mod scale;
mod witness;

use super::{AT, LcRelationError, storage};
use crate::{
    Budget, LcCertainty, LcConsistency, LcDeterminacy, LcFreeDirection, LcInterval, LcPhaseGauge,
    LcQuantity, LcRelation, LcSampleTimes, LcTimingCover, LcValues, LcWitness,
};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Finite positive rational support in coherent coordinates.
type Bounds = [Rational; 2];

/// Repeated measurements of one autonomous amplitude use one shared gain.
#[derive(Clone, Debug)]
pub struct LcAmplitudeSamples(Rc<Question>);

/// The physical amplitude equals `gain*measured` for every retained sample interval.
#[derive(Debug)]
struct Question {
    /// Original physical relation and independent time domains.
    timing: LcSampleTimes,
    /// Exactly one of the voltage or current amplitude channels.
    channel: LcQuantity,
    /// Every independent repeated amplitude measurement support.
    samples: Vec<LcInterval>,
    /// Intersection of the measured supports; every original support remains retained.
    measured: Bounds,
    /// One positive multiplicative error shared by all repeated measurements.
    gain: Bounds,
    /// Exact positive gain/sample image intersected with every original amplitude restriction.
    physical: Bounds,
    /// Supplied finite positive energy support; absence leaves a continuous element scale.
    energy: Option<Bounds>,
    /// Conservative new exact ownership excluding original shared owners.
    bytes: u64,
}

/// Every continuous physical coordinate used to reconstruct a joint measurement witness.
#[derive(Clone, Debug)]
pub struct LcAmplitudeParameters {
    /// Positive coherent rate in the complete retained branch domain.
    pub rate: Rational,
    /// Local physical initial phase before the common integer gauge.
    pub initial_phase: Rational,
    /// Original common integer phase gauge.
    pub gauge: Rational,
    /// Common measured amplitude belonging to every independent sample interval.
    pub measured: Rational,
    /// Single positive gain used for all amplitude measurements.
    pub gain: Rational,
    /// Positive source-free energy, including its original supplied constraints.
    pub energy: Rational,
}

/// The original physical tuple and its checked single calibration gain.
#[derive(Clone, Debug)]
pub struct LcAmplitudeWitness {
    /// Complete original measurement owner.
    amplitude: LcAmplitudeSamples,
    /// Original equations and every original supplied input have been checked.
    original: LcWitness,
    /// The same gain used by every measured sample.
    gain: Rational,
    /// Cumulative original and measurement checking snapshots.
    usage: [BudgetUsage; 2],
}

/// Complete phase/rate coverage with retained gain, measured amplitude and physical energy.
#[derive(Clone, Debug)]
pub struct LcAmplitudeCover(Rc<Cover>);

/// Phase recovery and positive scale reconstruction are independent exact coordinate maps.
#[derive(Debug)]
struct Cover {
    /// Complete original two-layer amplitude question.
    amplitude: LcAmplitudeSamples,
    /// Complete phase/rate domain with the independent timestamp constraints.
    effective: LcTimingCover,
    /// Original physical nonemptiness plus all repeated amplitude equations.
    witness: LcAmplitudeWitness,
    /// Full cumulative projection snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative retained metadata, including the original owner.
    bytes: u64,
}

impl LcAmplitudeSamples {
    /// Add separate positive amplitude intervals and one common positive gain.
    ///
    /// The closed adapter supports either voltage or current amplitude and optional finite
    /// positive energy intervals. Other simultaneous physical scale constraints refuse explicitly.
    /// Original amplitude restrictions intersect the gain/sample image jointly; the original
    /// measured supports and gain remain retained with their conditional fibre.
    /// # Errors
    /// Refuses unsupported constraints, empty sample intersections and exhausted resources.
    pub fn new(
        timing: LcSampleTimes,
        channel: LcQuantity,
        samples: Vec<LcInterval>,
        gain: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        admit::question(timing, channel, samples, gain, budget)
    }

    /// Original physical timing domain and complete declared relation.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }

    /// Original equations and every supplied physical constraint.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }

    /// Voltage or current amplitude measured by this owner.
    #[must_use]
    pub fn channel(&self) -> LcQuantity {
        self.0.channel
    }

    /// Every separately declared repeated amplitude interval.
    #[must_use]
    pub fn samples(&self) -> &[LcInterval] {
        &self.0.samples
    }

    /// Exact common measured amplitude support before the shared gain.
    #[must_use]
    pub fn measured(&self) -> &Bounds {
        &self.0.measured
    }

    /// Single positive gain support retained by the joint question.
    #[must_use]
    pub fn gain(&self) -> &Bounds {
        &self.0.gain
    }

    /// Physical amplitude support after all original restrictions, with the gain/sample domain retained.
    #[must_use]
    pub fn physical_amplitude(&self) -> &Bounds {
        &self.0.physical
    }

    /// Supplied finite positive energy support; absence leaves energy free and positive.
    #[must_use]
    pub fn energy(&self) -> Option<&Bounds> {
        self.0.energy.as_ref()
    }

    /// Allocation identity of the whole measurement question.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Recover complete phase/rate branches and all remaining physical scale fibres.
    /// # Errors
    /// Refuses unsupported phase domains, incomplete cycle coverage and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcAmplitudeCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Check every original equation and every amplitude interval using the same gain.
    /// # Errors
    /// Refuses physical or measurement violations and exhausted resources.
    pub fn check_witness(
        &self,
        values: LcValues,
        gain: Rational,
        budget: &mut Budget,
    ) -> Result<LcAmplitudeWitness, LcRelationError> {
        witness::check(self, values, gain, budget)
    }

    /// Conservative new exact ownership excluding the shared original relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcAmplitudeWitness {
    /// Identical original amplitude measurement owner.
    #[must_use]
    pub const fn amplitude(&self) -> &LcAmplitudeSamples {
        &self.amplitude
    }

    /// Physical tuple checked against every original equation and supplied range.
    #[must_use]
    pub const fn original(&self) -> &LcWitness {
        &self.original
    }

    /// Single gain checked for all repeated amplitude measurements.
    #[must_use]
    pub const fn gain(&self) -> &Rational {
        &self.gain
    }

    /// Cumulative physical and measurement checking snapshots.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

impl LcAmplitudeCover {
    /// Complete measurement question retained by every projection.
    #[must_use]
    pub fn amplitude(&self) -> &LcAmplitudeSamples {
        &self.0.amplitude
    }

    /// Original physical relation.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.amplitude().relation()
    }

    /// Complete projected rate support with its phase fibre retained.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Complete original cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// Unknown physical initial phase retains every common integer gauge.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Complete original branch coverage and a joint nonemptiness witness are checked.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Structural boundedness distinguishes measured amplitude from a free element scale.
    /// # Errors
    /// Refuses quantities outside the closed physical projection adapter.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        if scale::bounded(self.amplitude(), quantity)? {
            Ok(LcDeterminacy::DeterminedFinite)
        } else {
            Ok(LcDeterminacy::Underdetermined)
        }
    }

    /// Without supplied energy, the remaining physical scales retain an element-scale freedom.
    /// # Errors
    /// Refuses unsupported physical quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        if scale::bounded(self.amplitude(), quantity)? {
            Ok(&[])
        } else {
            Ok(&[LcFreeDirection::ElementScale])
        }
    }

    /// Exact monotone physical marginals retain the common rate, gain, sample and energy domain.
    /// # Errors
    /// Refuses unsupported quantities and exhausted resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        scale::bounds(self, quantity, budget)
    }

    /// The checked original tuple proves feasibility for the retained measurement question.
    #[must_use]
    pub const fn consistency(&self) -> LcConsistency {
        LcConsistency::Feasible
    }

    /// Exact initial-phase fibre at a retained rate.
    /// # Errors
    /// Refuses rates outside the complete domain and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// The gain fibre at a physical amplitude keeps the gain/sample correlation.
    /// # Errors
    /// Refuses physical amplitudes outside the joint domain and exhausted resources.
    pub fn gain_at_amplitude(
        &self,
        value: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        admit::gain_at_amplitude(self.amplitude(), value, budget)
    }

    /// Reconstruct every original physical coordinate from the retained joint parameters.
    /// # Errors
    /// Refuses crossed marginals, original violations and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcAmplitudeParameters,
        budget: &mut Budget,
    ) -> Result<LcAmplitudeWitness, LcRelationError> {
        witness::parameters(self, parameters, budget)
    }

    /// Joint physical and measurement nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcAmplitudeWitness {
        &self.0.witness
    }

    /// Cumulative projection snapshots including every original check.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative complete retained exact ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
