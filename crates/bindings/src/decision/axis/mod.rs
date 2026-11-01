// SPDX-License-Identifier: AGPL-3.0-only
//! Closed original reactance questions retain exact axis branches and reciprocal guards.

mod declaration;
mod proof;
mod syntax;

use super::{DecisionError, DecisionExpression, storage};
use crate::{BindingSymbols, Budget, Descriptor};
use nrz_cas::{Cas, Handle, decision::CheckedRationalEnclosure, proof::VerifiedWitness};
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

/// The original normalized reactance and its explicit source correspondence.
#[derive(Clone, Debug)]
pub struct AxisPhaseSpec {
    /// Original context, including the source/model assumptions.
    pub context: Handle,
    /// Free Real coordinate x for X divided by its exact positive ohm reference.
    pub coordinate: Handle,
    /// Closed exact original target, independently matched to the physical expression.
    pub target: Handle,
    /// Original context slot asserting coordinate equals target; never inferred from floats.
    pub correspondence: nrz_cas::domain::HypothesisRef,
}

/// Complete predeclared sign question and both dependent recipes.
#[derive(Clone, Debug)]
pub struct AxisPhaseCase(Rc<Data>);

/// One immutable graph authority and the complete original question.
#[derive(Debug)]
struct Data {
    /// Authoritative graph for the canonical case and its checked witnesses.
    graph: Arc<Mutex<Cas>>,
    /// Original physical ratio, including all source declarations and exact unit maps.
    source: DecisionExpression,
    /// Original context, coordinate, target and correspondence slot.
    spec: AxisPhaseSpec,
    /// Canonical negative, singleton-zero and positive partition.
    partition: nrz_cas::decision::CasePartition,
    /// Ordered exact regions and their native associations.
    branches: [[Handle; 2]; 3],
    /// Exact negative and positive half-pi expressions, declared before any attempt.
    phases: [Handle; 2],
    /// Original reciprocal expression whose own NonZero(x) guard is discharged.
    reciprocal: Handle,
    /// Checked original coordinate-to-target hypothesis application.
    correspondence: nrz_cas::proof::ApplicationDraft,
    /// Coherent angle-difference meaning, never a ratio retag.
    angle: Descriptor,
    /// Predeclared normalized-reactance value and both radius symbols.
    symbols: BindingSymbols,
    /// New retained metadata, symbols and original correspondence witness allowance.
    bytes: u64,
}

/// A complete sign decision, including outcomes that produce no angle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisPhaseOutcome {
    /// Strict negative membership gives exactly minus pi/2 radians.
    Negative,
    /// Proved singleton zero gives undefined phase and reciprocal-domain refusal.
    Undefined,
    /// Strict positive membership gives exactly plus pi/2 radians.
    Positive,
    /// The complete box intersects zero without proving singleton zero.
    Undecided,
}

/// Checked mathematical attempt; this archived owner authenticates no heap versions.
#[derive(Clone, Debug)]
pub struct AxisPhaseAttempt {
    /// Complete original question and authority.
    case: AxisPhaseCase,
    /// Independently checked original enclosure, retained even when undecided.
    enclosure: CheckedRationalEnclosure,
    /// Explicit result; no default numerical angle exists.
    outcome: AxisPhaseOutcome,
    /// Certified introduction, absent when no region uniformly contains the box.
    membership: Option<VerifiedWitness>,
    /// Original defined reciprocal equality with exactly its matching guard discharged.
    reciprocal: Option<VerifiedWitness>,
    /// Caller ledger immediately before and after this attempt.
    usage: [nrz_cas::limits::BudgetUsage; 2],
    /// Conservative newly retained certificate and metadata bytes.
    bytes: u64,
}

/// Finite attempt counter shared across refinements of one original question.
#[derive(Debug)]
pub struct AxisPhaseAttempts {
    /// Immutable original question shared by every attempted enclosure.
    case: AxisPhaseCase,
    /// Predeclared maximum number of attempts.
    maximum: u32,
    /// Attempts consumed, including failed original-enclosure checks.
    used: u32,
}

impl AxisPhaseCase {
    /// Original question and its retained source/model premises.
    #[must_use]
    pub fn specification(&self) -> &AxisPhaseSpec {
        &self.0.spec
    }
    /// Exact original physical expression, before any numerical sign test.
    #[must_use]
    pub fn source(&self) -> &DecisionExpression {
        &self.0.source
    }
    /// Exact angle-difference output meaning in coherent radians.
    #[must_use]
    pub fn angle(&self) -> &Descriptor {
        &self.0.angle
    }
    /// Normalized scalar input symbols, declared together with both continuations.
    #[must_use]
    pub fn symbols(&self) -> &BindingSymbols {
        &self.0.symbols
    }
    /// Original reciprocal syntax; its guard is never replaced by a fresh question.
    #[must_use]
    pub fn reciprocal(&self) -> &Handle {
        &self.0.reciprocal
    }
    /// Exact negative and positive half-pi branches, respectively.
    #[must_use]
    pub fn phases(&self) -> &[Handle; 2] {
        &self.0.phases
    }
    /// Newly retained declaration allowance, excluding shared physical source owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
    /// Create a finite execution identity; all attempts still consume the caller's ledger.
    #[must_use]
    pub fn attempts(&self, maximum: u32) -> AxisPhaseAttempts {
        AxisPhaseAttempts {
            case: self.clone(),
            maximum,
            used: 0,
        }
    }
    /// Authenticate an input against this exact original declaration, including both radii.
    /// # Errors
    /// Refuses foreign value or companion symbols and exhausted caller resources.
    pub fn authenticate(
        &self,
        symbols: &BindingSymbols,
        budget: &mut Budget,
    ) -> Result<(), DecisionError> {
        budget.work(3, storage::AT)?;
        let expected = self.symbols();
        if symbols.value() != expected.value()
            || symbols.bound() != expected.bound()
            || symbols.error() != expected.error()
        {
            return Err(DecisionError::Association);
        }
        Ok(())
    }
    /// Whether both references name the same original declaration, not merely equal targets.
    #[must_use]
    pub fn same_question(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl AxisPhaseAttempt {
    /// Original exact case, assumptions and numerical input association.
    #[must_use]
    pub const fn case(&self) -> &AxisPhaseCase {
        &self.case
    }
    /// Complete checked original box; historical inspection grants no current live authority.
    #[must_use]
    pub const fn enclosure(&self) -> &CheckedRationalEnclosure {
        &self.enclosure
    }
    /// Explicit decided, undefined or undecided result.
    #[must_use]
    pub const fn outcome(&self) -> AxisPhaseOutcome {
        self.outcome
    }
    /// Exact selected phase; undefined and undecided deliberately have no expression.
    #[must_use]
    pub fn phase(&self) -> Option<&Handle> {
        match self.outcome {
            AxisPhaseOutcome::Negative => Some(&self.case.phases()[0]),
            AxisPhaseOutcome::Positive => Some(&self.case.phases()[1]),
            AxisPhaseOutcome::Undefined | AxisPhaseOutcome::Undecided => None,
        }
    }
    /// Original certified introduction with every used source assumption retained.
    #[must_use]
    pub const fn membership(&self) -> Option<&VerifiedWitness> {
        self.membership.as_ref()
    }
    /// Discharged original reciprocal statement; numerical division still needs its own guard.
    #[must_use]
    pub const fn reciprocal(&self) -> Option<&VerifiedWitness> {
        self.reciprocal.as_ref()
    }
    /// Cumulative usage on the caller's actual ledger.
    #[must_use]
    pub const fn usage(&self) -> [nrz_cas::limits::BudgetUsage; 2] {
        self.usage
    }
    /// Newly retained metadata and witness allowance, excluding the shared case.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

impl AxisPhaseAttempts {
    /// Immutable original question across every attempt.
    #[must_use]
    pub const fn case(&self) -> &AxisPhaseCase {
        &self.case
    }
    /// Consumed attempts; refusal does not restore this counter.
    #[must_use]
    pub const fn used(&self) -> u32 {
        self.used
    }
    /// Predeclared finite ceiling.
    #[must_use]
    pub const fn maximum(&self) -> u32 {
        self.maximum
    }
}
