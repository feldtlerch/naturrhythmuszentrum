// SPDX-License-Identifier: AGPL-3.0-only
//! An explicit bounded clock offset remains shared by every original timestamp.

mod admit;
mod project;
mod witness;

use super::{AT, LcInterval, LcPairsCover, LcRelation, LcRelationError, LcWitness, storage};
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
    /// Checked positive original rate support used for the exact clock-coordinate elimination.
    rate: [Rational; 2],
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

/// Parameters of the shared-clock feasible family; actual initial phase is exactly zero.
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

/// Complete bounded phase-pair recovery with the shared clock constraint retained.
#[derive(Clone, Debug)]
pub struct LcClockCover(Rc<Cover>);

/// The effective phase coordinate is eliminated back to one offset before physical publication.
#[derive(Debug)]
struct Cover {
    /// Original joint clock question.
    clock: LcSharedClock,
    /// Complete strip family in the effective coordinate `beta=a*offset`.
    effective: LcPairsCover,
    /// Exact nonemptiness for every original equation and the shared offset.
    witness: LcClockWitness,
    /// Cumulative construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative retained exact cover metadata.
    bytes: u64,
}

impl LcSharedClock {
    /// Admit one explicitly bounded shared offset with exact known zero starting phase.
    ///
    /// Every supplied timestamp interval must contain its full nominal-plus-offset image.
    /// Other timestamp restrictions refuse this closed adapter instead of being omitted.
    /// Positive finite rational rate support and at least two nominal timestamps are required.
    /// # Errors
    /// Refuses unsupported clocks, missing zero phase/rate premises and exhausted resources.
    pub fn new(
        relation: LcRelation,
        nominal: Vec<Rational>,
        offset: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        let offset = admit::bounds(offset, budget)?;
        let rate = admit::question(&relation, &nominal, &offset, budget)?;
        let bytes = nominal
            .iter()
            .chain(&offset)
            .chain(&rate)
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
            rate,
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
            return Ok(&[]);
        }
        self.0.effective.free_directions(quantity)
    }

    /// Exact complete physical marginals; actual initial phase remains zero.
    /// # Errors
    /// Refuses unsupported phase/time marginals or exhausted exact arithmetic.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        budget.work(1, AT)?;
        if quantity == LcQuantity::InitialPhase {
            return LcInterval::new(
                Some(Rational::zero().into()),
                Some(Rational::zero().into()),
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
        Ok([phase[0].div(rate, budget)?, phase[1].div(rate, budget)?])
    }

    /// Realize an admissible shared offset and rate with every original physical scale constraint.
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
