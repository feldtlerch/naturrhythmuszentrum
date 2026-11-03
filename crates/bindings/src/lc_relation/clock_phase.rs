// SPDX-License-Identifier: AGPL-3.0-only
//! Independent phase intervals and one systematic phase offset retain a common physical question.

use super::phase::admit;
mod project;
mod witness;

use super::{AT, LcRelationError, storage};
use crate::{
    Budget, LcCertainty, LcClockCover, LcClockPhaseParameters, LcClockWitness, LcConsistency,
    LcDeterminacy, LcFreeDirection, LcInterval, LcPhaseGauge, LcQuantity, LcRelation,
    LcSharedClock,
};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Exact finite support in coherent rational coordinates.
type Bounds = [Rational; 2];

/// Physical observation phases equal separate measured phases plus one common bounded offset.
#[derive(Clone, Debug)]
pub struct LcSharedClockPhase(Rc<Question>);

/// Both phase layers remain associated with the original shared-clock domain.
#[derive(Debug)]
struct Question {
    /// Original physical relation and one shared clock offset for every observation time.
    clock: LcSharedClock,
    /// Each measured phase representative's own deterministic interval.
    samples: Vec<LcInterval>,
    /// One additive phase offset, in radians, shared by every observation.
    offset: Bounds,
    /// Original physical initial-phase intersection; absence retains all common gauges.
    initial: Option<Bounds>,
    /// Conservative new measurement-question metadata.
    bytes: u64,
}

/// Full physical parameters include a shared offset separately from local initial phase.
#[derive(Clone, Debug)]
pub struct LcSharedClockPhaseParameters {
    /// Physical local initial phase, rate, positive scales and original common integer gauge.
    pub physical: LcClockPhaseParameters,
    /// One additive offset used by every measured phase.
    pub offset: Rational,
}

/// A physical tuple checked against both phase layers and all original relation constraints.
#[derive(Clone, Debug)]
pub struct LcClockPhaseWitness {
    /// Identical original two-layer phase question.
    phase: LcSharedClockPhase,
    /// Checked physical tuple with actual phases and actual initial phase.
    original: LcClockWitness,
    /// The same checked phase offset used for every observation.
    offset: Rational,
    /// Cumulative original and systematic-error checking snapshots.
    usage: [BudgetUsage; 2],
}

/// Complete rate and phase/offset fibres retain all original phase and clock constraints.
#[derive(Clone, Debug)]
pub struct LcClockPhaseCover(Rc<Cover>);

/// `beta=alpha-offset` is a coordinate change with a retained original offset fibre.
#[derive(Debug)]
struct Cover {
    /// Original physical clock and two-layer phase question.
    phase: LcSharedClockPhase,
    /// Complete effective rate/initial-phase domain, retaining the original shared clock offset.
    effective: LcClockCover,
    /// Full physical and systematic-offset nonemptiness witness.
    witness: LcClockPhaseWitness,
    /// Cumulative full projection Work snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative total retained exact metadata.
    bytes: u64,
}

impl LcSharedClockPhase {
    /// Declare independent phase intervals and a single shared additive phase error.
    ///
    /// Each original physical phase input must contain its full sample-plus-offset image.
    /// Original initial-phase constraints are retained jointly through `beta=alpha-offset`.
    /// # Errors
    /// Refuses unsupported intervals, omitted original restrictions and exhausted resources.
    pub fn new(
        clock: LcSharedClock,
        samples: Vec<LcInterval>,
        offset: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        storage::retained(storage::size(samples.len())?.saturating_mul(2_048), budget)?;
        let offset = admit::bounds(offset, budget)?;
        let initial = admit::question(clock.relation(), &samples, &offset, budget)?;
        let bytes = offset
            .iter()
            .chain(initial.iter().flatten())
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            });
        let bytes = samples
            .iter()
            .fold(bytes, |sum, interval| sum.saturating_add(interval.bytes()));
        storage::retained(
            bytes
                .saturating_add(clock.retained_payload_bytes())
                .saturating_add(clock.relation().retained_payload_bytes()),
            budget,
        )?;
        Ok(Self(Rc::new(Question {
            clock,
            samples,
            offset,
            initial,
            bytes,
        })))
    }

    /// Original shared-clock owner, nominal timestamps and complete physical relation.
    #[must_use]
    pub fn clock(&self) -> &LcSharedClock {
        &self.0.clock
    }

    /// Every original physical constraint and named common epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.clock().relation()
    }

    /// Per-sample phase measurement supports, before adding the common offset.
    #[must_use]
    pub fn samples(&self) -> &[LcInterval] {
        &self.0.samples
    }

    /// Single retained additive phase-error domain.
    #[must_use]
    pub fn offset(&self) -> &Bounds {
        &self.0.offset
    }

    /// Identity of the complete original phase measurement question.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Solve the exact correlated phase/rate/offset domain over a complete cycle cover.
    /// # Errors
    /// Refuses unsupported scales, incomplete cycle coverage and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcClockPhaseCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Check every original physical equation and each measured phase after subtracting one offset.
    /// # Errors
    /// Refuses original or systematic-error membership violations and exhausted resources.
    pub fn check_witness(
        &self,
        values: crate::LcValues,
        clock_offset: Rational,
        offset: Rational,
        budget: &mut Budget,
    ) -> Result<LcClockPhaseWitness, LcRelationError> {
        witness::check(self, values, clock_offset, offset, budget)
    }

    /// Conservative new exact measurement metadata, excluding shared original owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcClockPhaseWitness {
    /// Original two-layer phase question.
    #[must_use]
    pub const fn phase(&self) -> &LcSharedClockPhase {
        &self.phase
    }

    /// Physical tuple after every original equation and input check.
    #[must_use]
    pub const fn original(&self) -> &LcClockWitness {
        &self.original
    }

    /// Single offset used for all phase samples.
    #[must_use]
    pub const fn offset(&self) -> &Rational {
        &self.offset
    }

    /// Cumulative physical and measurement checking snapshots.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

impl LcClockPhaseCover {
    /// Complete original two-layer phase owner.
    #[must_use]
    pub fn phase(&self) -> &LcSharedClockPhase {
        &self.0.phase
    }

    /// Original physical relation, without changed initial-phase coordinates.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.phase().relation()
    }

    /// Complete projected rate domain with original phase and offset fibres retained.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Every original integer cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.effective.cycles()
    }

    /// Unknown actual initial phase preserves the complete common integer gauge.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Complete branch coverage and an original joint witness have checked evidence.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Physical scale determinacy is independent of the bounded phase-error width.
    /// # Errors
    /// Refuses quantities outside the retained physical projection adapter.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        if quantity == LcQuantity::InitialPhase {
            return Err(LcRelationError::Projection);
        }
        self.0.effective.determinacy(quantity)
    }

    /// Original physical scale freedoms remain visible beside the measurement fibres.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        if quantity == LcQuantity::InitialPhase {
            return Err(LcRelationError::Projection);
        }
        self.0.effective.free_directions(quantity)
    }

    /// Phase restoration leaves the original rate and physical scale marginals unchanged.
    /// # Errors
    /// Refuses unsupported quantities and exhausted resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        budget.work(1, AT)?;
        if quantity == LcQuantity::InitialPhase {
            return Err(LcRelationError::Projection);
        }
        self.0.effective.bounds(quantity, budget)
    }

    /// Full physical consistency is backed by the restored original witness.
    #[must_use]
    pub const fn consistency(&self) -> LcConsistency {
        LcConsistency::Feasible
    }

    /// Exact local physical initial-phase fibre at a retained rate.
    /// # Errors
    /// Refuses rates outside the domain and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        let beta = self.0.effective.phase_at_rate(rate, budget)?;
        let mut physical = [
            beta[0].add(&self.phase().offset()[0], budget)?,
            beta[1].add(&self.phase().offset()[1], budget)?,
        ];
        if let Some(initial) = &self.phase().0.initial {
            admit::intersect(&mut physical, initial, budget)?;
        }
        Ok(physical)
    }

    /// Exact offset fibre at a retained rate and local physical initial phase.
    /// # Errors
    /// Refuses crossed marginals that violate the retained joint domain and exhausted Work.
    pub fn offset_at_phase(
        &self,
        rate: &Rational,
        initial_phase: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        let physical = self.phase_at_rate(rate, budget)?;
        admit::contains(&physical, initial_phase, budget)?;
        let beta = self.0.effective.phase_at_rate(rate, budget)?;
        let mut offset = [
            initial_phase.sub(&beta[1], budget)?,
            initial_phase.sub(&beta[0], budget)?,
        ];
        admit::intersect(&mut offset, self.phase().offset(), budget)?;
        Ok(offset)
    }

    /// Conditional clock-offset fibre with both physical initial phase and phase calibration fixed.
    /// # Errors
    /// Refuses crossed shared-error marginals and exhausted resources.
    pub fn clock_offset_at_phases(
        &self,
        rate: &Rational,
        initial_phase: &Rational,
        phase_offset: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        admit::contains(
            &self.offset_at_phase(rate, initial_phase, budget)?,
            phase_offset,
            budget,
        )?;
        let effective_phase = initial_phase.sub(phase_offset, budget)?;
        self.0
            .effective
            .offset_at_phase(rate, &effective_phase, budget)
    }

    /// Restore the same offset in actual initial phase and every physical observation phase.
    /// # Errors
    /// Refuses parameters outside the joint domain and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcSharedClockPhaseParameters,
        budget: &mut Budget,
    ) -> Result<LcClockPhaseWitness, LcRelationError> {
        witness::parameters(self.phase(), &self.0.effective, parameters, budget)
    }

    /// Original physical and measurement-equation nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcClockPhaseWitness {
        &self.0.witness
    }

    /// Cumulative projection snapshots include restoration and all original checks.
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
