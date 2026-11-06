// SPDX-License-Identifier: AGPL-3.0-only
//! Exact substitution checks every original equation, supplied range and cycle index.

mod check;

use super::{AT, LcQuantity, LcRelation, LcRelationError, storage};
use crate::{Budget, ExactConstant};
use nrz_cas::{Rational, limits::BudgetUsage};

/// Distinct domains of the autonomous relation, independent of any projection verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcBranch {
    /// Positive energy and strictly positive voltage/current amplitudes.
    Oscillating,
    /// Zero energy and zero amplitudes; natural frequency remains defined.
    Quiescent,
}

/// A proposed exact phase/time value, including its original integer lift.
#[derive(Clone, Debug)]
pub struct LcObservationValue {
    /// Duration from the declaration's common epoch.
    pub time: ExactConstant,
    /// A chosen phase representative; absent exactly on the quiescent branch.
    pub phase: Option<ExactConstant>,
    /// Exact integer n in theta0 + a t = theta + 2 pi n; absent when phase is undefined.
    pub cycle: Option<Rational>,
}

/// A complete candidate, never evidence until checked against the original relation.
#[derive(Clone, Debug)]
pub struct LcValues {
    /// Inductance in henries.
    pub inductance: ExactConstant,
    /// Capacitance in farads.
    pub capacitance: ExactConstant,
    /// Coherent rate in inverse seconds, constrained by a squared L C = 1.
    pub rate: ExactConstant,
    /// Initial angle representative in radians, absent for zero energy.
    pub initial_phase: Option<ExactConstant>,
    /// Nonnegative voltage amplitude in volts.
    pub voltage: ExactConstant,
    /// Nonnegative current amplitude in amperes.
    pub current: ExactConstant,
    /// Source-free energy in joules.
    pub energy: ExactConstant,
    /// Complete ordered observation values, including unsupplied coordinates.
    pub observations: Vec<LcObservationValue>,
}

/// A sealed exact feasible tuple for one immutable original declaration.
///
/// This is a nonemptiness witness only. It asserts neither that an outer box is
/// filled nor that the projection is determined, and authenticates no live heap value.
#[derive(Clone, Debug)]
pub struct LcWitness {
    /// The original relation identity and every supplied constraint remain retained.
    relation: LcRelation,
    /// The entire exact candidate after independent substitution checking.
    values: LcValues,
    /// Checked positive or zero-energy domain.
    branch: LcBranch,
    /// Exact product, natural time and optional positive amplitude ratio.
    derived: Derived,
    /// Work snapshots for this attempt on the caller's cumulative ledger.
    usage: [BudgetUsage; 2],
    /// New conservative exact record ownership, excluding the shared relation.
    bytes: u64,
}

/// Derived coordinates remain exact eliminations of the same full checked tuple.
#[derive(Clone, Debug)]
struct Derived {
    /// L C in seconds squared.
    product: ExactConstant,
    /// 1/a, positive because the original rate is checked positive.
    natural_time: ExactConstant,
    /// V/I only on the strictly positive amplitude branch.
    ratio: Option<ExactConstant>,
}

impl LcRelation {
    /// Check nonemptiness by exact substitution into all original equations and inputs.
    ///
    /// A failed candidate establishes no global impossibility. Unproved comparisons,
    /// equal-looking floats and missing cycle assignments never become evidence.
    /// # Errors
    /// Refuses domain, equation, integer, inventory and input-containment failures,
    /// unsupported closed exact arithmetic, or exhausted cumulative resources.
    pub fn check_witness(
        &self,
        values: LcValues,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if values.observations.len() != self.observations() {
            return Err(LcRelationError::Observation);
        }
        let bytes = values.bytes(budget)?;
        storage::retained(bytes, budget)?;
        let retained = self.retained_payload_bytes().saturating_add(bytes);
        let (branch, derived) = budget
            .with_retained_payload(retained, AT, |budget| check::original(&values, budget))?;
        let bytes = bytes.saturating_add(derived.bytes());
        storage::retained(bytes, budget)?;
        let mut witness = LcWitness {
            relation: self.clone(),
            values,
            branch,
            derived,
            usage: [before, budget.usage()],
            bytes,
        };
        let retained = self.retained_payload_bytes().saturating_add(bytes);
        budget.with_retained_payload(retained, AT, |budget| witness.check_inputs(budget))?;
        witness.usage[1] = budget.usage();
        Ok(witness)
    }
}

impl LcWitness {
    /// Reuse the unchanged sealed equations while checking every input of the target declaration.
    pub(super) fn recheck_inputs_for(
        &self,
        relation: &LcRelation,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let epoch_bytes = self.relation.epoch().len().max(relation.epoch().len());
        budget.work(storage::size(epoch_bytes)?, AT)?;
        if self.relation.epoch() != relation.epoch() {
            return Err(LcRelationError::Epoch);
        }
        if self.values.observations.len() != relation.observations() {
            return Err(LcRelationError::Observation);
        }
        let retained = self
            .relation
            .retained_payload_bytes()
            .saturating_add(relation.retained_payload_bytes())
            .saturating_add(self.bytes.saturating_mul(2));
        storage::retained(retained, budget)?;
        budget.with_retained_payload(retained, AT, |budget| {
            let mut witness = self.clone();
            witness.relation = relation.clone();
            witness.check_inputs(budget)?;
            witness.usage = [before, budget.usage()];
            Ok(witness)
        })
    }

    /// Check every supplied constraint, including redundant and derived quantities.
    fn check_inputs(&self, budget: &mut Budget) -> Result<(), LcRelationError> {
        for input in self.relation.inputs() {
            budget.work(1, AT)?;
            let value = self
                .coordinate(input.quantity())
                .ok_or(LcRelationError::QuiescentPhase)?;
            if !input.interval().contains(value, budget)? {
                return Err(LcRelationError::Containment);
            }
        }
        Ok(())
    }

    /// The immutable relation against which all equations and input constraints were checked.
    #[must_use]
    pub const fn relation(&self) -> &LcRelation {
        &self.relation
    }

    /// The full exact feasible tuple, including original cycle lifts.
    #[must_use]
    pub const fn values(&self) -> &LcValues {
        &self.values
    }

    /// Whether physical oscillation phase and amplitude ratio are defined.
    #[must_use]
    pub const fn branch(&self) -> LcBranch {
        self.branch
    }

    /// A coordinate of this witness, not a bound on every feasible tuple.
    ///
    /// Returns `None` for an absent observation or an undefined quiescent phase/ratio.
    #[must_use]
    pub fn coordinate(&self, quantity: LcQuantity) -> Option<&ExactConstant> {
        match quantity {
            LcQuantity::Inductance => Some(&self.values.inductance),
            LcQuantity::Capacitance => Some(&self.values.capacitance),
            LcQuantity::InitialPhase => self.values.initial_phase.as_ref(),
            LcQuantity::Time(i) => self.values.observations.get(i).map(|value| &value.time),
            LcQuantity::Phase(i) => self.values.observations.get(i)?.phase.as_ref(),
            LcQuantity::VoltageAmplitude => Some(&self.values.voltage),
            LcQuantity::CurrentAmplitude => Some(&self.values.current),
            LcQuantity::Energy => Some(&self.values.energy),
            LcQuantity::Product => Some(&self.derived.product),
            LcQuantity::NaturalTime => Some(&self.derived.natural_time),
            LcQuantity::AmplitudeRatio => self.derived.ratio.as_ref(),
            LcQuantity::AngularFrequency | LcQuantity::Rate => Some(&self.values.rate),
        }
    }

    /// Exact checking work snapshots; unsuccessful attempts also spend the caller ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }

    /// Conservative exact ownership excluding the retained shared relation allocation.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

impl LcValues {
    /// Bound the complete candidate, including unused coordinates and cycle proposals.
    fn bytes(&self, budget: &mut Budget) -> Result<u64, LcRelationError> {
        let mut bytes = storage::size(self.observations.capacity())?.saturating_mul(512);
        for value in [
            &self.inductance,
            &self.capacitance,
            &self.rate,
            &self.voltage,
            &self.current,
            &self.energy,
        ]
        .into_iter()
        .chain(&self.initial_phase)
        {
            budget.work(1, AT)?;
            bytes = bytes.saturating_add(storage::constant(value));
        }
        for observation in &self.observations {
            budget.work(1, AT)?;
            bytes = bytes.saturating_add(storage::constant(&observation.time));
            if let Some(value) = &observation.phase {
                bytes = bytes.saturating_add(storage::constant(value));
            }
            if let Some(value) = &observation.cycle {
                bytes = bytes.saturating_add(storage::rational(value));
            }
        }
        Ok(bytes.saturating_add(1_024))
    }
}

impl Derived {
    /// Derived exact coordinates are owned by the result, never by a numerical side channel.
    fn bytes(&self) -> u64 {
        [&self.product, &self.natural_time]
            .into_iter()
            .chain(&self.ratio)
            .fold(128_u64, |bytes, value| {
                bytes.saturating_add(storage::constant(value))
            })
    }
}
