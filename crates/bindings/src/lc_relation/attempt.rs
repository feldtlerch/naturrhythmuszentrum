// SPDX-License-Identifier: AGPL-3.0-only
//! Unresolved branch attempts retain the complete original candidate domain and pending evidence.

use super::{AT, LcCertainty, LcDeterminacy, LcPairsCover, LcRelation, LcRelationError, storage};
use crate::Budget;
use nrz_cas::{
    FaultKind, Rational, lattice::IntegerTupleDomain, limits::BudgetUsage,
    pi_bounds::PiBoundCertificate,
};
use std::rc::Rc;

/// Unfinished proof obligation, distinct from malformed inputs and certified impossibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcBranchObligation {
    /// Other integer lifts have not been excluded by a complete exact coverage proof.
    CycleCoverage,
    /// Joint feasibility or emptiness of the original constraints remains unresolved.
    JointFeasibility,
    /// The attempt exhausted resources while retaining its complete original candidate domain.
    Resources,
}

/// A checked pair cover or a typed unresolved result; no failed proposal silently disappears.
#[derive(Clone, Debug)]
pub enum LcPairAttempt {
    /// Complete branch coverage and original nonemptiness have checked evidence.
    Certified(LcPairsCover),
    /// All original constraints and integer candidates remain while the named obligation is open.
    Unresolved(LcUnresolvedBranches),
}

/// An unpruned integer cover with original constraints, never an assertion of feasible branches.
#[derive(Clone, Debug)]
pub struct LcUnresolvedBranches(Rc<Data>);

/// Immutable candidate ownership survives exact solving and numerical-resource refusals.
#[derive(Debug)]
struct Data {
    /// Every original declared input and the same clock epoch.
    relation: LcRelation,
    /// All mathematical integer tuples; original equations still restrict their feasibility.
    domain: IntegerTupleDomain,
    /// Failed proposal, retained as a proposal rather than a replacement domain.
    proposal: Vec<Rational>,
    /// Explicit unfinished obligation.
    obligation: LcBranchObligation,
    /// Exact refusal that prevented stronger evidence.
    reason: LcRelationError,
    /// Cumulative before/after attempt counters, including failed work.
    usage: [BudgetUsage; 2],
    /// Conservative newly retained candidate metadata.
    bytes: u64,
}

impl LcRelation {
    /// Attempt bounded pair recovery while preserving complete candidates on unresolved evidence.
    ///
    /// Original phase equations require an integer per observation, so `Z^d` is always
    /// a safe unpruned candidate cover. Original rate, phase and scale constraints remain
    /// authoritative; membership in this index domain alone is not physical feasibility.
    /// # Errors
    /// Refuses malformed tuple shapes/indices, false pi evidence, unsupported inputs or
    /// preflight exhaustion before a complete candidate owner can be reserved.
    pub fn attempt_pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcPairAttempt, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if self.observations() < 2 {
            return Err(LcRelationError::Observation);
        }
        let domain = IntegerTupleDomain::unrestricted(self.observations(), budget)?;
        if !domain.contains(&cycles, budget)? {
            return Err(LcRelationError::Cycle);
        }
        let bytes = cycles.iter().fold(2_048_u64, |sum, cycle| {
            sum.saturating_add(storage::rational(cycle))
        });
        let retained = bytes.saturating_add(self.retained_payload_bytes());
        storage::retained(retained, budget)?;
        let checked_pi = pi.check(budget)?;
        let result = budget.with_retained_payload(retained, AT, |budget| {
            self.pairs_cover_checked(cycles.clone(), checked_pi, budget)
        });
        match result {
            Ok(cover) => Ok(LcPairAttempt::Certified(cover)),
            Err(reason) => {
                let obligation = match &reason {
                    LcRelationError::CycleCover => LcBranchObligation::CycleCoverage,
                    LcRelationError::Containment => LcBranchObligation::JointFeasibility,
                    LcRelationError::Algebra(fault)
                        if matches!(fault.kind, FaultKind::Exhausted { .. }) =>
                    {
                        LcBranchObligation::Resources
                    }
                    _ => return Err(reason),
                };
                Ok(LcPairAttempt::Unresolved(LcUnresolvedBranches(Rc::new(
                    Data {
                        relation: self.clone(),
                        domain,
                        proposal: cycles,
                        obligation,
                        reason,
                        usage: [before, budget.usage()],
                        bytes,
                    },
                ))))
            }
        }
    }
}

impl LcUnresolvedBranches {
    /// Original complete relation; all declared bounds and physical meanings remain present.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Complete unpruned integer tuple domain, with no hidden magnitude cutoff.
    #[must_use]
    pub fn domain(&self) -> &IntegerTupleDomain {
        &self.0.domain
    }

    /// Original unproved cycle proposal; it never restricts the candidate domain.
    #[must_use]
    pub fn proposal(&self) -> &[Rational] {
        &self.0.proposal
    }

    /// Named unfinished coverage, feasibility or resource obligation.
    #[must_use]
    pub fn obligation(&self) -> LcBranchObligation {
        self.0.obligation
    }

    /// Exact reason for withholding a stronger verdict.
    #[must_use]
    pub fn reason(&self) -> &LcRelationError {
        &self.0.reason
    }

    /// Neither nonemptiness nor impossibility is inferred from this outer candidate domain.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Undecided
    }

    /// Structural determination is not asserted without its whole-family evidence.
    #[must_use]
    pub const fn determinacy(&self) -> Option<LcDeterminacy> {
        None
    }

    /// All attempt work, including the failed exact solve.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new metadata ownership, excluding the original relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
