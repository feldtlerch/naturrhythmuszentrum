// SPDX-License-Identifier: AGPL-3.0-only
//! An explicit bounded clock offset remains shared by every original timestamp.

mod admit;
mod horizon;
mod project;
mod reported;
mod witness;

use super::{
    AT, LcInterval, LcPairsCover, LcRelation, LcRelationError, LcWitness, calibration, storage,
};
use crate::{Budget, Descriptor, LcQuantity};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// A joint clock question adds `t_i=nominal_i+offset` to the complete original relation.
#[derive(Clone, Debug)]
pub struct LcSharedClock(Rc<Question>);

/// Exact shared offset support; independent timestamp boxes never replace this owner.
#[derive(Debug)]
struct Question {
    /// Complete original physical constraints and their common epoch.
    relation: LcRelation,
    /// Reported timestamps, each in coherent seconds relative to the declared epoch.
    nominal: Vec<Rational>,
    /// One bounded rational offset, shared by all timestamps.
    offset: [Rational; 2],
    /// Exact intersection of the declared offset with every original timestamp restriction.
    support: [Rational; 2],
    /// Checked positive original rate support used for the exact clock-coordinate elimination.
    rate: [Rational; 2],
    /// Original physical initial-phase support; absence retains all common integer gauges.
    initial: Option<[Rational; 2]>,
    /// Coherent duration difference meaning of the offset.
    descriptor: Descriptor,
    /// Conservative new joint-question metadata ownership.
    bytes: u64,
}

/// A full original-equation witness also checked against the shared clock equation.
#[derive(Clone, Debug)]
pub struct LcClockWitness {
    /// Identical original joint question, not just its marginal timestamp boxes.
    clock: LcSharedClock,
    /// Exact physical tuple satisfying every original relation input and equation.
    original: LcWitness,
    /// The single checked offset used in every timestamp.
    offset: Rational,
    /// Cumulative checking snapshots.
    usage: [BudgetUsage; 2],
}

/// Known-zero shorthand parameters for the shared-clock feasible family.
#[derive(Clone, Debug)]
pub struct LcClockParameters {
    /// Positive recovered rate.
    pub rate: Rational,
    /// Single offset in the retained correlated rate/offset fibre.
    pub offset: Rational,
    /// Positive free impedance scale when original scales do not fix it.
    pub ratio: Rational,
    /// Positive free voltage scale when original scales do not fix it.
    pub voltage: Rational,
}

/// Original physical phase and clock offset remain separate correlated coordinates.
#[derive(Clone, Debug)]
pub struct LcClockPhaseParameters {
    /// Positive physical rate in the recovered domain.
    pub rate: Rational,
    /// Local physical initial phase before the common integer gauge.
    pub initial_phase: Rational,
    /// Common integer lift of physical initial phase and every observation cycle.
    pub gauge: Rational,
    /// Single original clock offset shared by every timestamp.
    pub offset: Rational,
    /// Positive physical impedance scale when the original inputs leave it free.
    pub ratio: Rational,
    /// Positive voltage amplitude when the original inputs leave it free.
    pub voltage: Rational,
}

/// Complete bounded phase-pair recovery with the shared clock constraint retained.
#[derive(Clone, Debug)]
pub struct LcClockCover(Rc<Cover>);

/// The effective phase coordinate is eliminated back to one offset before physical publication.
#[derive(Debug)]
struct Cover {
    /// Original joint clock question.
    clock: LcSharedClock,
    /// Complete strip family in the effective coordinate `beta=alpha+a*offset`.
    effective: LcPairsCover,
    /// Exact nonemptiness for every original equation and the shared offset.
    witness: LcClockWitness,
    /// Cumulative construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative retained exact cover metadata.
    bytes: u64,
}

impl LcSharedClock {
    /// Admit one explicitly bounded shared offset with optional original initial-phase support.
    ///
    /// Every supplied timestamp interval intersects the shared offset through `t_i=nominal_i+d`.
    /// The original declared offset remains beside its exact surviving intersection.
    /// Positive finite rational rate support and at least two nominal timestamps are required.
    /// # Errors
    /// Refuses unsupported clocks, missing positive rate premises and exhausted resources.
    pub fn new(
        relation: LcRelation,
        nominal: Vec<Rational>,
        offset: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        let offset = admit::bounds(offset, budget)?;
        let premises = admit::question(&relation, &nominal, &offset, budget)?;
        let rate = premises.rate;
        let initial = premises.initial;
        let support = premises.offset;
        let bytes = nominal
            .iter()
            .chain(&offset)
            .chain(&support)
            .chain(&rate)
            .chain(initial.iter().flatten())
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            });
        storage::retained(
            bytes.saturating_add(relation.retained_payload_bytes()),
            budget,
        )?;
        Ok(Self(Rc::new(Question {
            relation,
            nominal,
            offset,
            support,
            rate,
            initial,
            descriptor: LcQuantity::NaturalTime.descriptor()?,
            bytes,
        })))
    }

    /// Original complete relation whose timestamp boxes are further coupled by this question.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Every original nominal timestamp in observation order.
    #[must_use]
    pub fn nominal_times(&self) -> &[Rational] {
        &self.0.nominal
    }

    /// Single original shared offset support, never independent per-observation intervals.
    #[must_use]
    pub fn offset(&self) -> &[Rational; 2] {
        &self.0.offset
    }

    /// Exact surviving shared offset after every original timestamp restriction.
    ///
    /// The separately retained `offset` is the original declaration. This support is one common
    /// interval, never a set of independent per-observation offsets.
    #[must_use]
    pub fn offset_support(&self) -> &[Rational; 2] {
        &self.0.support
    }

    /// Every original physical initial-phase restriction, intersected without losing its clock correlation.
    #[must_use]
    pub fn initial_phase_support(&self) -> Option<&[Rational; 2]> {
        self.0.initial.as_ref()
    }

    /// Coherent seconds with duration difference kind.
    #[must_use]
    pub fn offset_descriptor(&self) -> &Descriptor {
        &self.0.descriptor
    }

    /// Identity of the original joint question, including the timestamp coupling.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Recover the complete rate/offset fibres while retaining the original physical constraints.
    /// # Errors
    /// Refuses unsupported scale constraints, incomplete cycle proposals and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcClockCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Reuse prepared constant evidence while retaining the same complete original clock question.
    pub(in crate::lc_relation) fn pairs_cover_checked(
        &self,
        cycles: Vec<Rational>,
        pi: nrz_cas::pi_bounds::CheckedPiBounds,
        budget: &mut Budget,
    ) -> Result<LcClockCover, LcRelationError> {
        project::checked(self, cycles, pi, budget)
    }

    /// Check every original equation and one shared offset against an arbitrary physical tuple.
    /// # Errors
    /// Refuses independent timestamp substitutions, offset-domain violations and resource exhaustion.
    pub fn check_witness(
        &self,
        values: crate::LcValues,
        offset: Rational,
        budget: &mut Budget,
    ) -> Result<LcClockWitness, LcRelationError> {
        witness::check(self, values, offset, budget)
    }

    /// Conservative new joint-question metadata, excluding the original relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcClockWitness {
    /// Original joint timestamp coupling retained with the witness.
    #[must_use]
    pub const fn clock(&self) -> &LcSharedClock {
        &self.clock
    }

    /// Full original physical tuple and equation evidence.
    #[must_use]
    pub const fn original(&self) -> &LcWitness {
        &self.original
    }

    /// Single exact clock offset used in every timestamp.
    #[must_use]
    pub const fn offset(&self) -> &Rational {
        &self.offset
    }

    /// All original and shared-clock checking work.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

impl LcClockCover {
    /// Whether two handles retain the exact same certified cover allocation.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Complete original joint question, with its unchanged physical epoch.
    #[must_use]
    pub fn clock(&self) -> &LcSharedClock {
        &self.0.clock
    }

    /// Original physical relation, accompanied by the additional shared-clock owner.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.0.clock.relation()
    }

    /// Complete exact recovered rate support.
    #[must_use]
    pub fn rate(&self) -> &[Rational; 2] {
        self.0.effective.rate()
    }

    /// Original observation cycles only; auxiliary strip indices are not physical observations.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        &self.0.effective.cycles()[..self.relation().observations()]
    }

    /// Exact original joint nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcClockWitness {
        &self.0.witness
    }

    /// Complete original-domain coverage and nonemptiness are both established.
    #[must_use]
    pub const fn certainty(&self) -> super::LcCertainty {
        super::LcCertainty::Certified
    }

    /// Same physical determinacy as the complete effective-coordinate projection.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn determinacy(
        &self,
        quantity: LcQuantity,
    ) -> Result<super::LcDeterminacy, LcRelationError> {
        if quantity == LcQuantity::InitialPhase {
            self.fixed_initial()?;
            return Ok(super::LcDeterminacy::DeterminedFinite);
        }
        self.0.effective.determinacy(quantity)
    }

    /// Named original physical freedoms survive the shared-clock elimination.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [super::LcFreeDirection], LcRelationError> {
        if quantity == LcQuantity::InitialPhase {
            self.fixed_initial()?;
            return Ok(&[]);
        }
        self.0.effective.free_directions(quantity)
    }

    /// Exact complete physical marginals; the scalar initial-phase query requires a fixed physical phase.
    /// # Errors
    /// Refuses unsupported phase/time marginals or exhausted exact arithmetic.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        budget.work(1, AT)?;
        if quantity == LcQuantity::InitialPhase {
            let initial = self.fixed_initial()?;
            return LcInterval::new(
                Some(initial.clone().into()),
                Some(initial.clone().into()),
                budget,
            );
        }
        self.0.effective.bounds(quantity, budget)
    }

    /// Complete correlated offset fibre at an admissible rate.
    /// # Errors
    /// Refuses out-of-domain rates or exhausted exact arithmetic.
    pub fn offset_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        let phase = self.0.effective.phase_at_rate(rate, budget)?;
        let mut offset = self.clock().offset_support().clone();
        if let Some([lower, upper]) = self.clock().initial_phase_support() {
            let image = [
                phase[0].sub(upper, budget)?.div(rate, budget)?,
                phase[1].sub(lower, budget)?.div(rate, budget)?,
            ];
            calibration::intersect(&mut offset, &image, budget)?;
        }
        Ok(offset)
    }

    /// Complete common integer gauge of the original physical initial phase.
    #[must_use]
    pub fn phase_gauge(&self) -> super::LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Local physical initial-phase fibre at a rate, before the retained common integer gauge.
    /// # Errors
    /// Refuses out-of-domain rates and exhausted exact arithmetic.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        let beta = self.0.effective.phase_at_rate(rate, budget)?;
        let mut phase = [
            beta[0].sub(
                &rate.mul(&self.clock().offset_support()[1], budget)?,
                budget,
            )?,
            beta[1].sub(
                &rate.mul(&self.clock().offset_support()[0], budget)?,
                budget,
            )?,
        ];
        if let Some(initial) = self.clock().initial_phase_support() {
            calibration::intersect(&mut phase, initial, budget)?;
        }
        Ok(phase)
    }

    /// Clock-offset fibre at a selected rate and local physical initial phase.
    /// # Errors
    /// Refuses crossed phase/rate/offset marginals and exhausted resources.
    pub fn offset_at_phase(
        &self,
        rate: &Rational,
        phase: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        budget.work(1, AT)?;
        phase.check(budget, AT)?;
        let mut selected = [phase.clone(), phase.clone()];
        if let Some(initial) = self.clock().initial_phase_support() {
            calibration::intersect(&mut selected, initial, budget)?;
        }
        let beta = self.0.effective.phase_at_rate(rate, budget)?;
        let mut offset = self.clock().offset_support().clone();
        let image = [
            beta[0].sub(phase, budget)?.div(rate, budget)?,
            beta[1].sub(phase, budget)?.div(rate, budget)?,
        ];
        calibration::intersect(&mut offset, &image, budget)?;
        Ok(offset)
    }

    /// Reconstruct physical initial phase and clock offset together, with every original gauge.
    /// # Errors
    /// Refuses crossed joint coordinates, original violations and exhausted resources.
    pub fn witness_with_phase(
        &self,
        parameters: &LcClockPhaseParameters,
        budget: &mut Budget,
    ) -> Result<LcClockWitness, LcRelationError> {
        witness::with_phase(self.clock(), &self.0.effective, parameters, budget)
    }

    /// A scalar phase marginal is supplied only for an exact original physical phase.
    fn fixed_initial(&self) -> Result<&Rational, LcRelationError> {
        let initial = self
            .clock()
            .initial_phase_support()
            .ok_or(LcRelationError::Projection)?;
        if initial[0] != initial[1] {
            return Err(LcRelationError::Projection);
        }
        Ok(&initial[0])
    }

    /// Realize a known-zero physical initial phase with every original physical scale constraint.
    /// # Errors
    /// Refuses parameters outside the joint fibres or exhausted original-equation checking.
    pub fn witness(
        &self,
        parameters: &LcClockParameters,
        budget: &mut Budget,
    ) -> Result<LcClockWitness, LcRelationError> {
        witness::parameters(&self.0.clock, &self.0.effective, parameters, budget)
    }

    /// Cumulative construction snapshots, excluding later queries.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative complete exact metadata, excluding the original physical relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
