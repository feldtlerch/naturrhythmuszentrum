// SPDX-License-Identifier: AGPL-3.0-only
//! Exact uncertain timestamps retain the joint phase/rate domain and restore original times.

mod horizon;
mod prepare;
mod restore;

use super::{AT, Bounds, LcRelationError, LcSampleTimes, Rational, storage};
use crate::{
    Budget, ExactConstant, LcCertainty, LcConsistency, LcDeterminacy, LcFreeDirection, LcInterval,
    LcPairParameters, LcPairsCover, LcPhaseGauge, LcQuantity, LcRelation, LcWitness,
};
use nrz_cas::{limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Complete exact rate/initial-phase fibres for independently uncertain sample timestamps.
#[derive(Clone, Debug)]
pub struct LcTimingCover(Rc<Cover>);

/// Auxiliary strips encode the original existential timestamp inequalities exactly.
#[derive(Debug)]
struct Cover {
    /// Original per-sample timing supports and all physical constraints.
    timing: LcSampleTimes,
    /// Joint phase/rate domain, never replaced by independent coordinate marginals.
    effective: LcPairsCover,
    /// Original lifted phase intersections in observation order.
    phases: Vec<Bounds>,
    /// Original complete cycle proposal, before auxiliary strip duplication.
    cycles: Vec<Rational>,
    /// Prepared original full-turn shifts, shared by admission and every restoration.
    shifts: Vec<ExactConstant>,
    /// One reconstructed physical tuple checked against every original equation and box.
    witness: LcWitness,
    /// Caller cumulative Work snapshots for the whole exact projection.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained metadata ownership.
    bytes: u64,
}

impl LcSampleTimes {
    /// Eliminate time boxes jointly with rate and initial phase on a complete cycle cover.
    ///
    /// For positive rate, each sample requires `p_min-a*t_max <= alpha <= p_max-a*t_min`.
    /// Redundant finite faces encode those inequalities for the existing joint strip solver.
    /// Every original feasible tuple maps into those strips with lifts relative to the proposal,
    /// so their complete cycle exclusion proof also covers the original question.
    /// # Errors
    /// Refuses unsupported inputs, infeasible strips, unproved cycle coverage and exhausted Work.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcTimingCover, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let prepared = prepare::question(self, &cycles, budget)?;
        let effective = prepared.relation.pairs_cover(prepared.cycles, pi, budget)?;
        let rate = effective.rate()[0].clone();
        let phase = effective.phase_at_rate(&rate, budget)?;
        let parameters = LcPairParameters {
            rate,
            initial_phase: phase[0].clone(),
            ratio: Rational::one(),
            voltage: Rational::one(),
            gauge: Rational::zero(),
        };
        let witness = restore::from_point(
            self,
            effective.feasibility_witness(),
            &prepared.phases,
            &cycles,
            &prepared.shifts,
            &parameters,
            budget,
        )?;
        let bytes = cycles
            .iter()
            .chain(prepared.phases.iter().flatten())
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(self.retained_payload_bytes())
            .saturating_add(self.relation().retained_payload_bytes())
            .saturating_add(effective.retained_payload_bytes())
            .saturating_add(effective.relation().retained_payload_bytes())
            .saturating_add(witness.retained_payload_bytes());
        let bytes = prepared.shifts.iter().fold(bytes, |sum, shift| {
            sum.saturating_add(storage::constant(shift))
        });
        storage::retained(bytes, budget)?;
        Ok(LcTimingCover(Rc::new(Cover {
            timing: self.clone(),
            effective,
            phases: prepared.phases,
            cycles,
            shifts: prepared.shifts,
            witness,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcTimingCover {
    /// Original complete independent timing question.
    #[must_use]
    pub fn timing(&self) -> &LcSampleTimes {
        &self.0.timing
    }

    /// Original physical relation and named common epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.0.timing.relation()
    }

    /// Exact complete projected rate interval; every rate has an original witness extension.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Original cycle associations, without the auxiliary strip inventory.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        &self.0.cycles
    }

    /// The original initial-phase constraint determines the complete common integer gauge.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Complete cycle coverage and original nonemptiness both have checked evidence.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Structural projection verdict, independent of finite timing uncertainty width.
    /// # Errors
    /// Refuses physical quantities outside the retained scale projection adapter.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        self.0.effective.determinacy(quantity)
    }

    /// Unconstrained physical directions remain distinct from bounded time uncertainty.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        self.0.effective.free_directions(quantity)
    }

    /// Exact physical marginal, associated with the retained original timing domain.
    /// # Errors
    /// Refuses unsupported quantities and exhausted arithmetic resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        self.0.effective.bounds(quantity, budget)
    }

    /// Full-input consistency is backed by a reconstructed original physical witness.
    #[must_use]
    pub fn consistency(&self) -> LcConsistency {
        self.0.effective.consistency()
    }

    /// Complete correlated initial-phase fibre at the supplied positive rate.
    /// # Errors
    /// Refuses rates outside the exact projected support and exhausted Work.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        self.0.effective.phase_at_rate(rate, budget)
    }

    /// Restore each time from its box intersected with the lifted phase/rate interval.
    ///
    /// The reconstructed tuple rechecks every original equation, interval and cycle.
    /// # Errors
    /// Refuses parameters outside the joint domain and exhausted caller resources.
    pub fn witness(
        &self,
        parameters: &LcPairParameters,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        restore::witness(
            self.timing(),
            &self.0.effective,
            &self.0.phases,
            &self.0.cycles,
            &self.0.shifts,
            parameters,
            budget,
        )
    }

    /// Original-equation nonemptiness, never the auxiliary strip tuple.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcWitness {
        &self.0.witness
    }

    /// Cumulative snapshots include complete cycle checking and original reconstruction.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative retained metadata including original and auxiliary owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
