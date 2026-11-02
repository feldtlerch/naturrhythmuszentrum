// SPDX-License-Identifier: AGPL-3.0-only
//! Monotone pair refinements keep prior cover and horizon on success, emptiness or exhaustion.

use super::{AT, Bounds, LcPairsCover, LcRelationError, Rational, storage, support};
use crate::{Budget, LcHorizon, LcHorizonValue, LcImpossible, LcInputRefinement};
use nrz_cas::limits::BudgetUsage;

/// A nonempty checked refinement, with explicit rate/product inclusion and horizon order.
#[derive(Debug)]
pub struct LcRefinementSuccess {
    /// Complete new joint relation and all new feasible fibres.
    cover: LcPairsCover,
    /// New eventual horizon, proved no earlier than the retained previous horizon.
    horizon: LcHorizon,
    /// Previous and refined product supports, both exact monotone images.
    products: [Bounds; 2],
    /// Checked nonnegative lower bound on the finite horizon difference; absent for infinity.
    horizon_margin: Option<Rational>,
}

impl LcRefinementSuccess {
    /// Complete refined nonempty relation; every original constraint is retained.
    #[must_use]
    pub const fn cover(&self) -> &LcPairsCover {
        &self.cover
    }

    /// Certified no-earlier eventual horizon on the original clock.
    #[must_use]
    pub const fn horizon(&self) -> &LcHorizon {
        &self.horizon
    }

    /// Original and refined product intervals, with checked inclusion.
    #[must_use]
    pub const fn product_inclusion(&self) -> &[Bounds; 2] {
        &self.products
    }

    /// Nonnegative finite-difference lower bound; infinity uses its explicit exact value.
    #[must_use]
    pub const fn finite_horizon_margin(&self) -> Option<&Rational> {
        self.horizon_margin.as_ref()
    }
}

/// Solver standing is separate from the already checked input-family inclusion.
#[derive(Debug)]
pub enum LcRefinementState {
    /// New rate/product support is contained and its horizon cannot move earlier.
    Refined(Box<LcRefinementSuccess>),
    /// Original all-alias contradiction proves the new set empty; it has no physical horizon.
    Impossible(Box<LcImpossible>),
    /// New solving or checking is unresolved; the complete previous information remains valid.
    Undecided(LcRelationError),
}

/// A refinement attempt always carries the original certified cover and horizon.
#[derive(Debug)]
pub struct LcPairRefinement {
    /// Immutable previous complete feasible family.
    previous: LcPairsCover,
    /// Previously proved horizon, retained even after an exhausted attempt.
    previous_horizon: LcHorizon,
    /// Independent exact implication establishing that prior information remains sound.
    inclusion: LcInputRefinement,
    /// Proposed new lifts, never a restriction of unresolved new observation branches.
    proposed_cycles: Vec<Rational>,
    /// Explicit nonempty, empty or unresolved outcome.
    state: LcRefinementState,
    /// Before/after attempt snapshots, without refunding a failed solve.
    usage: [BudgetUsage; 2],
}

impl LcPairsCover {
    /// Attempt a refinement after independently proving input-family inclusion.
    ///
    /// Exhausted solving returns an undecided attempt retaining the prior complete cover,
    /// horizon and implication certificate. An invalid source identity is a refusal.
    /// # Errors
    /// Refuses mismatched original authorities or preflight exhaustion. Invalid branch proposals
    /// remain unresolved attempts with the complete original candidate domain.
    pub fn refine(
        &self,
        inclusion: &LcInputRefinement,
        previous_horizon: &LcHorizon,
        cycles: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<LcPairRefinement, LcRelationError> {
        let before = budget.usage();
        let retained = validate(self, inclusion, previous_horizon, budget)?;
        let result = budget.with_retained_payload(retained, AT, |budget| {
            budget.work(storage::size(cycles.len())?.max(1), AT)?;
            if cycles.len() != inclusion.refined().observations()
                || !cycles.starts_with(self.cycles())
            {
                return Err(LcRelationError::CycleCover);
            }
            let cycle_bytes = cycles.iter().fold(256_u64, |sum, cycle| {
                sum.saturating_add(storage::rational(cycle))
            });
            storage::retained(cycle_bytes.saturating_mul(2), budget)?;
            let cover = inclusion.refined().pairs_cover_checked(
                cycles.clone(),
                self.pi_bounds().clone(),
                budget,
            )?;
            let checked = success(self, previous_horizon, cover, budget)?;
            Ok::<_, LcRelationError>(Box::new(checked))
        });
        let state = match result {
            Ok(checked) => LcRefinementState::Refined(checked),
            Err(reason) => LcRefinementState::Undecided(reason),
        };
        Ok(LcPairRefinement {
            previous: self.clone(),
            previous_horizon: previous_horizon.clone(),
            inclusion: inclusion.clone(),
            proposed_cycles: cycles,
            state,
            usage: [before, budget.usage()],
        })
    }

    /// Attach independently checked all-alias emptiness to an included new input family.
    /// # Errors
    /// Refuses a foreign previous horizon or emptiness certificate and exhausted resources.
    pub fn refine_impossible(
        &self,
        inclusion: &LcInputRefinement,
        previous_horizon: &LcHorizon,
        impossible: LcImpossible,
        budget: &mut Budget,
    ) -> Result<LcPairRefinement, LcRelationError> {
        let before = budget.usage();
        let retained = validate(self, inclusion, previous_horizon, budget)?;
        if !impossible.relation().shares_allocation(inclusion.refined()) {
            return Err(LcRelationError::Containment);
        }
        storage::retained(
            retained
                .saturating_add(impossible.retained_payload_bytes())
                .saturating_add(2_048),
            budget,
        )?;
        Ok(LcPairRefinement {
            previous: self.clone(),
            previous_horizon: previous_horizon.clone(),
            inclusion: inclusion.clone(),
            proposed_cycles: Vec::new(),
            state: LcRefinementState::Impossible(Box::new(impossible)),
            usage: [before, budget.usage()],
        })
    }
}

impl LcPairRefinement {
    /// Previous certified complete family, retained in every outcome.
    #[must_use]
    pub const fn previous(&self) -> &LcPairsCover {
        &self.previous
    }

    /// Previously proved horizon; an empty new set does not replace it with an earlier value.
    #[must_use]
    pub const fn previous_horizon(&self) -> &LcHorizon {
        &self.previous_horizon
    }

    /// Checked implication and the whole original refined candidate domain.
    #[must_use]
    pub const fn inclusion(&self) -> &LcInputRefinement {
        &self.inclusion
    }

    /// Caller branch proposals; unresolved branches retain the full original input question.
    #[must_use]
    pub fn proposed_cycles(&self) -> &[Rational] {
        &self.proposed_cycles
    }

    /// Explicit nonempty, empty or exhausted/unresolved state.
    #[must_use]
    pub const fn state(&self) -> &LcRefinementState {
        &self.state
    }

    /// Only a nonempty successful refinement has a newly certified physical horizon.
    #[must_use]
    pub const fn refined_horizon(&self) -> Option<&LcHorizon> {
        match &self.state {
            LcRefinementState::Refined(success) => Some(&success.horizon),
            LcRefinementState::Impossible(_) | LcRefinementState::Undecided(_) => None,
        }
    }

    /// Attempt work includes failed solving and never rolls the caller ledger back.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// Original authority matching and conservative fixed attempt storage precede any new solve.
fn validate(
    previous: &LcPairsCover,
    inclusion: &LcInputRefinement,
    horizon: &LcHorizon,
    budget: &mut Budget,
) -> Result<u64, LcRelationError> {
    budget.work(1, AT)?;
    let original = previous.relation().shares_allocation(inclusion.previous());
    let same_horizon = horizon
        .pairs_cover()
        .is_some_and(|cover| previous.shares_allocation(cover));
    if !original || !same_horizon {
        return Err(LcRelationError::Containment);
    }
    let retained = previous
        .retained_payload_bytes()
        .saturating_add(previous.relation().retained_payload_bytes())
        .saturating_add(inclusion.retained_payload_bytes())
        .saturating_add(inclusion.refined().retained_payload_bytes())
        .saturating_add(horizon.retained_payload_bytes())
        .saturating_add(8_192);
    storage::retained(retained, budget)?;
    Ok(retained)
}

/// Exact independent set checks accompany the input implication before publication.
fn success(
    previous: &LcPairsCover,
    old_horizon: &LcHorizon,
    cover: LcPairsCover,
    budget: &mut Budget,
) -> Result<LcRefinementSuccess, LcRelationError> {
    for endpoint in cover.rate() {
        if !support::contains(previous.rate(), endpoint, budget)? {
            return Err(LcRelationError::Containment);
        }
    }
    let products = [
        support::inverse_power(previous.rate(), 2, budget)?,
        support::inverse_power(cover.rate(), 2, budget)?,
    ];
    for endpoint in &products[1] {
        if !support::contains(&products[0], endpoint, budget)? {
            return Err(LcRelationError::Containment);
        }
    }
    let horizon = cover.forward_horizon(budget)?;
    let horizon_margin = horizon_order(old_horizon, &horizon, budget)?;
    let bytes = products
        .iter()
        .flatten()
        .chain(horizon_margin.iter())
        .fold(2_048_u64, |sum, value| {
            sum.saturating_add(storage::rational(value))
        })
        .saturating_add(cover.retained_payload_bytes())
        .saturating_add(horizon.retained_payload_bytes());
    storage::retained(bytes, budget)?;
    Ok(LcRefinementSuccess {
        cover,
        horizon,
        products,
        horizon_margin,
    })
}

/// Pi-affine difference bounds are checked explicitly; infinity never becomes a numeric endpoint.
fn horizon_order(
    previous: &LcHorizon,
    refined: &LcHorizon,
    budget: &mut Budget,
) -> Result<Option<Rational>, LcRelationError> {
    budget.work(1, AT)?;
    match (previous.value(), refined.value()) {
        (LcHorizonValue::Finite(old), LcHorizonValue::Finite(new)) => {
            let (old_offset, old_pi) = old.coefficients();
            let (new_offset, new_pi) = new.coefficients();
            let offset = new_offset.sub(old_offset, budget)?;
            let coefficient = new_pi.sub(old_pi, budget)?;
            let pi = previous.mathematical().pi_bounds().contract();
            let bound = if coefficient.is_negative() {
                &pi.upper
            } else {
                &pi.lower
            };
            let margin = offset.add(&coefficient.mul(bound, budget)?, budget)?;
            if margin.is_negative() {
                return Err(LcRelationError::UnprovedOrder);
            }
            Ok(Some(margin))
        }
        (LcHorizonValue::Finite(_) | LcHorizonValue::Infinite, LcHorizonValue::Infinite) => {
            Ok(None)
        }
        (LcHorizonValue::Infinite | LcHorizonValue::Undecided, _)
        | (_, LcHorizonValue::Undecided) => Err(LcRelationError::UnprovedOrder),
    }
}
