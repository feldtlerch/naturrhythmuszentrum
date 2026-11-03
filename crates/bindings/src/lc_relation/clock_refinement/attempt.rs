// SPDX-License-Identifier: AGPL-3.0-only
//! Exhausted solving retains the old certified clock cover, horizon and complete refined input question.

mod impossible;

use super::{AT, Cover, LcClockRefinementInputs, LcRelationError, check, storage};
use crate::{Budget, LcClockCover, LcHorizon, LcImpossible, LcSharedTimingCover, LcTimingCover};
use nrz_cas::{Rational, limits::BudgetUsage};

/// A complete nonempty refinement with explicit rate/product inclusion and no-earlier horizon evidence.
#[derive(Debug)]
pub struct LcClockRefinementSuccess {
    /// Complete refined measurement family.
    pub(super) cover: Cover,
    /// Horizon in the original coordinate at the same future reference.
    pub(super) horizon: LcHorizon,
    /// Previous and refined positive product supports.
    pub(super) products: [[Rational; 2]; 2],
    /// Nonnegative exact lower bound on a finite horizon difference; infinity stays explicit.
    pub(super) margin: Option<Rational>,
}

/// Solver standing remains separate from the independently checked whole-question inclusion.
#[derive(Debug)]
pub enum LcClockRefinementState {
    /// Complete nonempty refinement with checked physical and horizon monotonicity.
    Refined(Box<LcClockRefinementSuccess>),
    /// A separate all-cycle contradiction proves the complete refined original physical relation empty.
    Impossible(Box<LcImpossible>),
    /// Solving or subsequent checking refused; previous evidence and all new inputs survive.
    Undecided(LcRelationError),
}

/// A refinement attempt never discards prior certified clock and horizon evidence.
#[derive(Debug)]
pub struct LcClockRefinement {
    /// Exact previous certified family.
    previous: Cover,
    /// Original horizon, retained even when the new solve exhausts resources.
    previous_horizon: LcHorizon,
    /// Whole-question inclusion, with both complete original measurement owners.
    inclusion: LcClockRefinementInputs,
    /// Proposed new cycles; an unresolved result retains the full new original question.
    proposed_cycles: Vec<Rational>,
    /// Explicit complete or unresolved state.
    state: LcClockRefinementState,
    /// Before/after exact solving snapshots, also recorded for an exhausted solve.
    solving: Option<[BudgetUsage; 2]>,
    /// Before/after complete attempt snapshots, without refunding failed work.
    usage: [BudgetUsage; 2],
}

impl LcClockCover {
    /// Attempt a refinement after checking whole-question inclusion, retaining old evidence on solving refusal.
    /// # Errors
    /// Refuses mismatched original authorities or preflight exhaustion. New solving failures return an undecided attempt.
    pub fn refine(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        cycles: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        attempt(
            Cover::Clock(self.clone()),
            inclusion,
            previous_horizon,
            cycles,
            budget,
        )
    }
}

impl LcSharedTimingCover {
    /// Attempt a refinement of both time-error layers while retaining prior cover and horizon evidence.
    /// # Errors
    /// Refuses foreign authorities or preflight exhaustion; exhausted solving remains an explicit undecided attempt.
    pub fn refine(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        cycles: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        attempt(
            Cover::SharedTiming(self.clone()),
            inclusion,
            previous_horizon,
            cycles,
            budget,
        )
    }
}

impl LcTimingCover {
    /// Attempt independent timing refinement while retaining the previous cover and common-reference horizon.
    /// # Errors
    /// Refuses foreign source authorities or preflight exhaustion; failed solving retains an undecided attempt.
    pub fn refine(
        &self,
        inclusion: &LcClockRefinementInputs,
        previous_horizon: &LcHorizon,
        cycles: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<LcClockRefinement, LcRelationError> {
        attempt(
            Cover::Timing(self.clone()),
            inclusion,
            previous_horizon,
            cycles,
            budget,
        )
    }
}

impl LcClockRefinementSuccess {
    /// Complete refined independent timing family, when applicable.
    #[must_use]
    pub const fn timing_cover(&self) -> Option<&LcTimingCover> {
        self.cover.timing()
    }
    /// Complete refined single-clock family, when applicable.
    #[must_use]
    pub const fn clock_cover(&self) -> Option<&LcClockCover> {
        self.cover.clock()
    }
    /// Complete refined two-layer timing family, when applicable.
    #[must_use]
    pub const fn shared_timing_cover(&self) -> Option<&LcSharedTimingCover> {
        self.cover.shared_timing()
    }
    /// New no-earlier horizon in the same coordinate and at the same future reference.
    #[must_use]
    pub const fn horizon(&self) -> &LcHorizon {
        &self.horizon
    }
    /// Previous and refined product supports with checked inclusion.
    #[must_use]
    pub const fn product_inclusion(&self) -> &[[Rational; 2]; 2] {
        &self.products
    }
    /// Nonnegative finite-difference lower bound; an infinite horizon has no fabricated endpoint.
    #[must_use]
    pub const fn finite_horizon_margin(&self) -> Option<&Rational> {
        self.margin.as_ref()
    }
}

impl LcClockRefinement {
    /// Previous certified independent timing family, retained in every state.
    #[must_use]
    pub const fn previous_timing(&self) -> Option<&LcTimingCover> {
        self.previous.timing()
    }
    /// Previous certified single-clock family, retained in every state.
    #[must_use]
    pub const fn previous_clock(&self) -> Option<&LcClockCover> {
        self.previous.clock()
    }
    /// Previous certified two-layer family, retained in every state.
    #[must_use]
    pub const fn previous_shared_timing(&self) -> Option<&LcSharedTimingCover> {
        self.previous.shared_timing()
    }
    /// Previously certified horizon, independent of the new solver outcome.
    #[must_use]
    pub const fn previous_horizon(&self) -> &LcHorizon {
        &self.previous_horizon
    }
    /// Whole-question implication and the complete new original measurement domain.
    #[must_use]
    pub const fn inclusion(&self) -> &LcClockRefinementInputs {
        &self.inclusion
    }
    /// Caller cycle proposals; they never truncate the unresolved original candidate domain.
    #[must_use]
    pub fn proposed_cycles(&self) -> &[Rational] {
        &self.proposed_cycles
    }
    /// Explicit refined, separately certified impossible or unresolved standing.
    #[must_use]
    pub const fn state(&self) -> &LcClockRefinementState {
        &self.state
    }
    /// A newly certified horizon exists only after a complete successful solve and monotonicity check.
    #[must_use]
    pub const fn refined_horizon(&self) -> Option<&LcHorizon> {
        match &self.state {
            LcClockRefinementState::Refined(success) => Some(&success.horizon),
            LcClockRefinementState::Impossible(_) | LcClockRefinementState::Undecided(_) => None,
        }
    }
    /// Exact before/after solver snapshots, including an exhausted solver's consumed work.
    #[must_use]
    pub const fn solving_usage(&self) -> Option<&[BudgetUsage; 2]> {
        self.solving.as_ref()
    }
    /// All attempt work, including failed solving and checking, remains on the caller ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// Source identity and retained storage precede the recoverable solver attempt.
fn attempt(
    previous: Cover,
    inclusion: &LcClockRefinementInputs,
    horizon: &LcHorizon,
    cycles: Vec<Rational>,
    budget: &mut Budget,
) -> Result<LcClockRefinement, LcRelationError> {
    let before = budget.usage();
    let retained = validate(&previous, inclusion, horizon, budget)?;
    let mut solving = None;
    let result = budget.with_retained_payload(retained, AT, |budget| {
        budget.work(storage::size(cycles.len())?.max(1), AT)?;
        let count = cycles.len() == inclusion.0.refined.relation().observations();
        let original_cycles = cycles.starts_with(previous.cycles());
        if !count || !original_cycles {
            return Err(LcRelationError::CycleCover);
        }
        let cycle_bytes = cycles.iter().fold(256_u64, |sum, cycle| {
            sum.saturating_add(storage::rational(cycle))
        });
        storage::retained(cycle_bytes.saturating_mul(2), budget)?;
        let start = budget.usage();
        let cover = inclusion.0.refined.solve(
            cycles.clone(),
            horizon.mathematical().pi_bounds().clone(),
            budget,
        );
        solving = Some([start, budget.usage()]);
        let success = check::success(&previous, horizon, cover?, budget)?;
        Ok::<_, LcRelationError>(Box::new(success))
    });
    let state = match result {
        Ok(success) => LcClockRefinementState::Refined(success),
        Err(reason) => LcClockRefinementState::Undecided(reason),
    };
    Ok(LcClockRefinement {
        previous,
        previous_horizon: horizon.clone(),
        inclusion: inclusion.clone(),
        proposed_cycles: cycles,
        state,
        solving,
        usage: [before, budget.usage()],
    })
}

/// Equal numeric covers or questions cannot replace the original allocation authority.
fn validate(
    previous: &Cover,
    inclusion: &LcClockRefinementInputs,
    horizon: &LcHorizon,
    budget: &mut Budget,
) -> Result<u64, LcRelationError> {
    budget.work(1, AT)?;
    let original = previous.matches_question(&inclusion.0.previous);
    let same_horizon = previous.matches_horizon(horizon);
    if !original || !same_horizon {
        return Err(LcRelationError::Containment);
    }
    let retained = previous
        .bytes()
        .saturating_add(previous.relation().retained_payload_bytes())
        .saturating_add(inclusion.retained_payload_bytes())
        .saturating_add(inclusion.0.refined.bytes())
        .saturating_add(inclusion.0.refined.relation().retained_payload_bytes())
        .saturating_add(horizon.retained_payload_bytes())
        .saturating_add(8_192);
    storage::retained(retained, budget)?;
    Ok(retained)
}
