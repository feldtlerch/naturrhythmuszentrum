// SPDX-License-Identifier: AGPL-3.0-only
//! Zero-energy rectangle queries retain original inputs and distinct undefined outcomes.

mod domain;

use super::{
    AT, LcCertainty, LcDeterminacy, LcInterval, LcQuantity, LcRelation, LcRelationError, LcWitness,
    storage,
};
use crate::Budget;
use domain::Domain;
use nrz_cas::{Rational, limits::BudgetUsage};
use std::rc::Rc;

/// Defined scalar support or a specific physical undefinedness at rest.
#[derive(Clone, Debug)]
pub enum LcQuiescentOutcome {
    /// Complete exact scalar image of the retained positive element rectangle.
    Defined(LcInterval),
    /// There is no physical oscillation phase at zero energy.
    UndefinedPhase,
    /// Both amplitudes vanish, so their ratio has no scalar value.
    UndefinedRatio,
}

/// Complete observation-free zero-energy relation over independent positive elements.
#[derive(Clone, Debug)]
pub struct LcQuiescentCover(Rc<Data>);

/// Every scalar query and witness shares the same original simultaneous declaration.
#[derive(Debug)]
struct Data {
    /// Original repeated constraints, never replaced by their intersections.
    relation: LcRelation,
    /// Exact element rectangle and monotone scalar images.
    domain: Domain,
    /// Original-equation nonemptiness witness at one rectangle corner.
    feasible: LcWitness,
    /// Caller ledger before and after complete construction.
    usage: [BudgetUsage; 2],
    /// Conservative new retained payload, excluding the shared relation.
    bytes: u64,
}

/// One query result keeps its physical quantity and complete original source.
#[derive(Clone, Debug)]
pub struct LcQuiescentQuery {
    /// Joint source of this scalar projection or undefined outcome.
    cover: LcQuiescentCover,
    /// Original coherent quantity, distinguishing rate and angular frequency.
    quantity: LcQuantity,
    /// A missing physical value is never encoded as a zero interval.
    outcome: LcQuiescentOutcome,
}

impl LcRelation {
    /// Admit an observation-free positive L/C rectangle with exactly zero energy.
    ///
    /// All original L/C/energy declarations must have finite rational endpoints;
    /// their repeated intersections are retained beside every original input.
    /// Element lower bounds are positive and the intersected energy is exactly zero.
    /// Product square-root extrema must normalize to rationals in the closed checker.
    /// # Errors
    /// Refuses other supplied quantities, observations, missing or incompatible bounds,
    /// unsupported roots and exhausted caller resources, without claiming impossibility.
    pub fn quiescent_cover(
        &self,
        budget: &mut Budget,
    ) -> Result<LcQuiescentCover, LcRelationError> {
        let before = budget.usage();
        let domain = Domain::check(self, budget)?;
        let values = domain.values(&domain.inductance[0], &domain.capacitance[0], budget)?;
        let feasible = self.check_witness(values, budget)?;
        let bytes = domain
            .bytes()
            .saturating_add(feasible.retained_payload_bytes());
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcQuiescentCover(Rc::new(Data {
            relation: self.clone(),
            domain,
            feasible,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcQuiescentCover {
    /// Query a complete scalar image or the specific undefined phase/ratio outcome.
    /// # Errors
    /// Refuses indexed observation queries and exhausted caller resources.
    pub fn query(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcQuiescentQuery, LcRelationError> {
        budget.work(1, AT)?;
        let retained = self
            .0
            .bytes
            .saturating_add(self.relation().retained_payload_bytes());
        let outcome = budget
            .with_retained_payload(retained, AT, |budget| self.0.domain.query(quantity, budget))?;
        let bytes = match &outcome {
            LcQuiescentOutcome::Defined(interval) => interval.bytes(),
            LcQuiescentOutcome::UndefinedPhase | LcQuiescentOutcome::UndefinedRatio => 0,
        };
        storage::retained(retained.saturating_add(bytes).saturating_add(256), budget)?;
        Ok(LcQuiescentQuery {
            cover: self.clone(),
            quantity,
            outcome,
        })
    }

    /// Reconstruct a full rest tuple and recheck every original equation and declaration.
    /// # Errors
    /// Refuses out-of-rectangle parameters, unsupported exact roots or exhausted resources.
    pub fn witness(
        &self,
        inductance: &Rational,
        capacitance: &Rational,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        let retained = self
            .0
            .bytes
            .saturating_add(self.relation().retained_payload_bytes());
        budget.with_retained_payload(retained, AT, |budget| {
            let values = self.0.domain.values(inductance, capacitance, budget)?;
            self.relation().check_witness(values, budget)
        })
    }

    /// Complete original constraints and epoch, with shared allocation identity.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Checked full-original-relation nonemptiness witness.
    #[must_use]
    pub fn feasible(&self) -> &LcWitness {
        &self.0.feasible
    }

    /// Construction receipt on the caller's cumulative ledger.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative retained ownership excluding the original shared relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcQuiescentQuery {
    /// Original joint owner, including full-equation nonemptiness evidence.
    #[must_use]
    pub const fn cover(&self) -> &LcQuiescentCover {
        &self.cover
    }

    /// Coherent physical quantity requested by this query.
    #[must_use]
    pub const fn quantity(&self) -> LcQuantity {
        self.quantity
    }

    /// Distinct scalar, undefined-phase and undefined-ratio outcomes.
    #[must_use]
    pub const fn outcome(&self) -> &LcQuiescentOutcome {
        &self.outcome
    }

    /// Defined scalar support only; undefined physical quantities have no interval.
    #[must_use]
    pub const fn interval(&self) -> Option<&LcInterval> {
        match &self.outcome {
            LcQuiescentOutcome::Defined(interval) => Some(interval),
            LcQuiescentOutcome::UndefinedPhase | LcQuiescentOutcome::UndefinedRatio => None,
        }
    }

    /// Undefined physical quantities carry no scalar determinacy classification.
    #[must_use]
    pub const fn determinacy(&self) -> Option<LcDeterminacy> {
        match &self.outcome {
            LcQuiescentOutcome::Defined(_) => Some(LcDeterminacy::DeterminedFinite),
            LcQuiescentOutcome::UndefinedPhase | LcQuiescentOutcome::UndefinedRatio => None,
        }
    }

    /// The scalar image or physical undefinedness is checked for the complete admitted domain.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }
}
