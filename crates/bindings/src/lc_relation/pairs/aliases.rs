// SPDX-License-Identifier: AGPL-3.0-only
//! Complete nonnegative sampling aliases with a checked primitive timestamp lattice.

mod admit;
mod witness;
pub use witness::LcAliasWitness;

use super::{
    LcCertainty, LcDeterminacy, LcPairParameters, LcPairsCover, LcPhaseGauge, LcQuantity,
    LcRelation, LcRelationError, LcWitness, Rational, storage, support,
};
use crate::{Budget, ExactConstant};
use nrz_cas::{limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Untrusted primitive timestamp proposal, checked against the original exact observations.
#[derive(Clone, Debug)]
pub struct LcAliasLattice {
    /// Positive time quantum dividing all three original timestamps exactly.
    pub quantum: Rational,
    /// Integral coefficients with `u*(t1-t0)/quantum + v*(t2-t0)/quantum = 1`.
    pub bezout: [Rational; 2],
}

/// Complete infinite union over every nonnegative mathematical integer alias index.
#[derive(Clone, Debug)]
pub struct LcAliasCover(Rc<Data>);

/// The bounded representative is a derived coordinate chart, never a physical cutoff.
#[derive(Debug)]
struct Data {
    /// Original unbounded-frequency relation, with no synthesized range constraint.
    relation: LcRelation,
    /// Complete representative strip family after the proved integer-lattice reduction.
    fundamental: LcPairsCover,
    /// Exact checked primitive timestamp quantum and Bezout identity.
    lattice: LcAliasLattice,
    /// Prepared exact full-turn rate step, shared by all member and witness lookups.
    step: ExactConstant,
    /// Original integer timestamp multipliers, retaining every cycle shift.
    ticks: [Rational; 3],
    /// Before/after construction snapshots on the caller's cumulative ledger.
    usage: [BudgetUsage; 2],
    /// Conservative retained ownership, excluding the shared original declaration.
    bytes: u64,
}

impl LcRelation {
    /// Certify a complete infinite alias family for three exact rational timestamps.
    ///
    /// The original frequency must be unbounded. Checked primitive gaps force every
    /// relative cycle tuple onto the one retained lattice. The derived representative
    /// interval and positivity prove the complete index domain `k >= 0`.
    /// # Errors
    /// Refuses unsupported inputs, a false or insufficient integer-lattice certificate,
    /// an unresolved index domain, empty representative family and exhausted resources.
    pub fn alias_cover(
        &self,
        cycles: Vec<Rational>,
        lattice: LcAliasLattice,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcAliasCover, LcRelationError> {
        let before = budget.usage();
        let checked_pi = pi.check(budget)?;
        let reduced = admit::check(self, &cycles, &lattice, &checked_pi, budget)?;
        let fundamental = reduced.relation.pairs_cover(cycles, pi, budget)?;
        let step = support::turn(budget)?.div(&lattice.quantum.clone().into(), budget)?;
        let mut bytes = fundamental
            .retained_payload_bytes()
            .saturating_add(fundamental.relation().retained_payload_bytes())
            .saturating_add(storage::constant(&step))
            .saturating_add(4_096);
        for value in reduced
            .ticks
            .iter()
            .chain(&lattice.bezout)
            .chain(std::iter::once(&lattice.quantum))
        {
            bytes = bytes.saturating_add(storage::rational(value));
        }
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcAliasCover(Rc::new(Data {
            relation: self.clone(),
            fundamental,
            lattice,
            step,
            ticks: reduced.ticks,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcAliasCover {
    /// Original relation with absent frequency bounds preserved as mathematical unboundedness.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Included first alias; there is no finite last index or implicit materialization cutoff.
    #[must_use]
    pub fn first_index(&self) -> Rational {
        Rational::zero()
    }

    /// Complete integer domain has no upper endpoint.
    #[must_use]
    pub const fn last_index(&self) -> Option<&Rational> {
        None
    }

    /// Rate support in the proved fundamental representative, before the exact alias shift.
    #[must_use]
    pub fn base_rate(&self) -> &[Rational; 2] {
        self.0.fundamental.rate()
    }

    /// Original cycles in the fundamental representative; alias shifts are the retained ticks.
    #[must_use]
    pub fn base_cycles(&self) -> &[Rational] {
        self.0.fundamental.cycles()
    }

    /// Every alias index adds this exact integer vector to the original cycle tuple.
    #[must_use]
    pub fn cycle_steps(&self) -> &[Rational; 3] {
        &self.0.ticks
    }

    /// Complete common initial-phase lift domain, independent of the frequency alias index.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.0.fundamental.phase_gauge()
    }

    /// Exact coefficient of pi in the frequency shift per alias index.
    /// # Errors
    /// Refuses exhausted caller arithmetic.
    pub fn rate_step(&self, budget: &mut Budget) -> Result<Rational, LcRelationError> {
        Ok(Rational::from_ratio(2, 1, budget)?.div(&self.0.lattice.quantum, budget)?)
    }

    /// Structural rate/product determination has infinitely many discrete branches.
    /// # Errors
    /// Refuses unsupported quantity projections.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        match self.0.fundamental.determinacy(quantity)? {
            LcDeterminacy::DeterminedFinite => Ok(LcDeterminacy::DeterminedInfinite),
            LcDeterminacy::Underdetermined => Ok(LcDeterminacy::Underdetermined),
            LcDeterminacy::DeterminedInfinite => Err(LcRelationError::Projection),
        }
    }

    /// Complete lattice reduction and a nonempty feasible fibre for every index are checked.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Exact rate endpoints for any member of the complete infinite integer family.
    /// # Errors
    /// Refuses negative/fractional indices or exhausted caller resources.
    pub fn rate_endpoints(
        &self,
        index: &Rational,
        budget: &mut Budget,
    ) -> Result<[ExactConstant; 2], LcRelationError> {
        Self::check_index(index, budget)?;
        let shift = self.0.step.mul(&index.clone().into(), budget)?;
        Ok([
            ExactConstant::from(self.base_rate()[0].clone()).add(&shift, budget)?,
            ExactConstant::from(self.base_rate()[1].clone()).add(&shift, budget)?,
        ])
    }

    /// Check one full original tuple in any alias, including arbitrary positive physical scales.
    /// # Errors
    /// Refuses invalid indices, unsupported parameters or exhausted caller arithmetic.
    pub fn witness(
        &self,
        index: &Rational,
        parameters: &LcPairParameters,
        budget: &mut Budget,
    ) -> Result<LcAliasWitness, LcRelationError> {
        Self::check_index(index, budget)?;
        witness::check(self, index, parameters, budget)
    }

    /// Before/after cumulative exact construction usage.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative retained exact family ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }

    /// Index checks never convert a mathematical integer into a machine-sized loop bound.
    fn check_index(index: &Rational, budget: &mut Budget) -> Result<(), LcRelationError> {
        support::integer(index, budget)?;
        if index.is_negative() {
            return Err(LcRelationError::Cycle);
        }
        Ok(())
    }
}
