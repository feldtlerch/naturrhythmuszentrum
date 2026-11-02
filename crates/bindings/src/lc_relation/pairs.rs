// SPDX-License-Identifier: AGPL-3.0-only
//! Correlated phase-pair projections, complete aliases and positive scale eliminations.

mod admit;
mod aliases;
pub use aliases::{LcAliasCover, LcAliasLattice, LcAliasWitness};
mod horizon;
mod project;
mod scale;
mod support;
mod witness;

use super::{
    AT, LcCertainty, LcDeterminacy, LcInterval, LcQuantity, LcRelation, LcRelationError, LcWitness,
    storage,
};
use crate::Budget;
use nrz_cas::{
    Rational,
    limits::BudgetUsage,
    pi_bounds::{CheckedPiBounds, PiBoundCertificate},
};
use std::rc::Rc;

/// Closed rational support; positivity remains part of the original relation.
type Bounds = [Rational; 2];

/// One retained inequality pair `lower <= alpha + rate*time <= upper`.
#[derive(Debug)]
struct Strip {
    /// Exact timestamp from the original common epoch.
    time: Rational,
    /// Original phase interval after its checked integer lift.
    phase: Bounds,
}

/// An unconstrained continuous physical direction, distinct from measurement uncertainty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcFreeDirection {
    /// Positive element ratio varies while the measured LC product is preserved.
    ElementScale,
    /// Positive amplitudes and energy vary together without changing the phase clock.
    AmplitudeScale,
}

/// Joint feasibility of all original constraints, separate from projection determinacy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcConsistency {
    /// The original relation has a checked full-tuple witness.
    Feasible,
    /// Supplied elements, amplitudes, energy and starting phase satisfy every observation.
    Overdetermined,
}

/// Complete common-lift freedom retained when the starting phase was not supplied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcPhaseGauge {
    /// The supplied starting-phase interval is represented by the proved original lifts.
    Fixed,
    /// Add `2*pi*g` to initial phase and `g` to every cycle, for every integer `g`.
    AllIntegers,
}

/// Exact parameters for the constructive full-relation feasibility map.
#[derive(Clone, Debug)]
pub struct LcPairParameters {
    /// Positive rate inside the complete recovered rate support.
    pub rate: Rational,
    /// Local lifted starting phase inside the correlated fibre at this rate.
    pub initial_phase: Rational,
    /// Positive impedance parameter, used when original inputs leave the element scale free.
    pub ratio: Rational,
    /// Positive voltage parameter, used when original inputs leave amplitude scale free.
    pub voltage: Rational,
    /// Common integer lift; zero is required when starting phase is supplied.
    pub gauge: Rational,
}

/// Complete rational rate/product projection with original correlations and feasibility.
#[derive(Clone, Debug)]
pub struct LcPairsCover(Rc<Data>);

/// Immutable exact evidence is independent of numerical endpoint materialization.
#[derive(Debug)]
struct Data {
    /// Complete original relation, including every supplied constraint.
    relation: LcRelation,
    /// Retained strip intersection; it is never replaced by independent marginals.
    inputs: admit::Inputs,
    /// Original proposed lifts, proved complete up to the declared common gauge.
    cycles: Vec<Rational>,
    /// Complete nonempty projected rate support.
    rate: Bounds,
    /// Independently checked exact pi evidence excluding all other relative lifts.
    pi: CheckedPiBounds,
    /// A complete exact tuple checked against every original equation and input.
    witness: LcWitness,
    /// Checking snapshots on the caller's cumulative ledger.
    usage: [BudgetUsage; 2],
    /// Conservative new exact ownership, excluding the shared original declaration.
    bytes: u64,
}

impl LcRelation {
    /// Recover rate and LC product from exact times and finite lifted phase intervals.
    ///
    /// The supplied positive rational frequency range is mandatory in this closed adapter.
    /// Every other cycle branch is excluded by a checked strict full-turn gap. Unknown
    /// starting phase retains all common integer lifts and its correlated feasible fibre.
    /// # Errors
    /// Refuses unsupported constraints, uncertain timestamps, empty intersections,
    /// unproved complete cycle coverage and exhausted cumulative resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcPairsCover, LcRelationError> {
        let before = budget.usage();
        let inputs = admit::Inputs::check(self, &cycles, budget)?;
        let pi = pi.check(budget)?;
        project::branches(
            &inputs.strips,
            &inputs.rate,
            inputs.fixed_phase,
            &pi,
            budget,
        )?;
        let rate = project::rate(&inputs.strips, inputs.rate.clone(), budget)?;
        let phase = project::phase(&inputs.strips, &rate[0], budget)?;
        let parameters = LcPairParameters {
            rate: rate[0].clone(),
            initial_phase: phase[0].clone(),
            ratio: Rational::one(),
            voltage: Rational::one(),
            gauge: Rational::zero(),
        };
        let witness = witness::check(self, &inputs, &cycles, &rate, &parameters, budget)?;
        let mut bytes = pi
            .retained_bytes()
            .saturating_add(witness.retained_payload_bytes())
            .saturating_add(2_048);
        for strip in &inputs.strips {
            bytes = bytes.saturating_add(storage::rational(&strip.time));
            for value in &strip.phase {
                bytes = bytes.saturating_add(storage::rational(value));
            }
        }
        for value in cycles.iter().chain(&rate).chain(&inputs.rate) {
            bytes = bytes.saturating_add(storage::rational(value));
        }
        bytes = bytes.saturating_add(inputs.scale.bytes());
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcPairsCover(Rc::new(Data {
            relation: self.clone(),
            inputs,
            cycles,
            rate,
            pi,
            witness,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcPairsCover {
    /// Original simultaneous relation retained by every derived answer.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Exact complete rate support, preserving the original phase correlations separately.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        &self.0.rate
    }

    /// Every original observation retains its exact base-cycle association.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        &self.0.cycles
    }

    /// Complete gauge domain, never a truncated list of arbitrary initial-phase lifts.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        if self.0.inputs.fixed_phase {
            LcPhaseGauge::Fixed
        } else {
            LcPhaseGauge::AllIntegers
        }
    }

    /// Both complete branch coverage and original nonemptiness have checked evidence.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Independent structural verdict for the requested physical projection.
    /// # Errors
    /// Refuses quantities whose projected set is not supplied by this closed adapter.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        match quantity {
            LcQuantity::Rate
            | LcQuantity::AngularFrequency
            | LcQuantity::Product
            | LcQuantity::NaturalTime => Ok(LcDeterminacy::DeterminedFinite),
            LcQuantity::Inductance
            | LcQuantity::Capacitance
            | LcQuantity::AmplitudeRatio
            | LcQuantity::VoltageAmplitude
            | LcQuantity::CurrentAmplitude
            | LcQuantity::Energy => Ok(if self.0.inputs.scale.bounded(quantity) {
                LcDeterminacy::DeterminedFinite
            } else {
                LcDeterminacy::Underdetermined
            }),
            _ => Err(LcRelationError::Projection),
        }
    }

    /// Named continuous freedoms for each supported query, separate from finite uncertainty.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        match quantity {
            LcQuantity::Rate
            | LcQuantity::AngularFrequency
            | LcQuantity::Product
            | LcQuantity::NaturalTime => Ok(&[]),
            LcQuantity::Inductance
            | LcQuantity::Capacitance
            | LcQuantity::AmplitudeRatio
            | LcQuantity::VoltageAmplitude
            | LcQuantity::CurrentAmplitude
            | LcQuantity::Energy => Ok(self.0.inputs.scale.freedom(quantity)),
            _ => Err(LcRelationError::Projection),
        }
    }

    /// Certified outer marginal; all values remain associated with this same joint relation.
    ///
    /// A zero lower bound on an unbounded scale marginal is an outer endpoint; the original
    /// positive domain still excludes zero. Missing upper bounds remain mathematical absence.
    /// # Errors
    /// Refuses unsupported quantities and exhausted caller arithmetic.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        let bounds = match quantity {
            LcQuantity::Rate | LcQuantity::AngularFrequency => self.0.rate.clone(),
            LcQuantity::Product => support::inverse_power(&self.0.rate, 2, budget)?,
            LcQuantity::NaturalTime => support::inverse_power(&self.0.rate, 1, budget)?,
            LcQuantity::Inductance
            | LcQuantity::Capacitance
            | LcQuantity::AmplitudeRatio
            | LcQuantity::VoltageAmplitude
            | LcQuantity::CurrentAmplitude
            | LcQuantity::Energy => {
                if let Some(bounds) = self.0.inputs.scale.bounds(quantity, &self.0.rate, budget)? {
                    bounds
                } else {
                    return LcInterval::new(Some(Rational::zero().into()), None, budget);
                }
            }
            _ => return Err(LcRelationError::Projection),
        };
        let [lower, upper] = bounds;
        LcInterval::new(Some(lower.into()), Some(upper.into()), budget)
    }

    /// Joint full-input consistency, backed by the retained original-equation witness.
    #[must_use]
    pub fn consistency(&self) -> LcConsistency {
        let full_scales = self.0.inputs.scale.full_tuple;
        let supplied_phase = self.0.inputs.fixed_phase;
        if full_scales && supplied_phase {
            LcConsistency::Overdetermined
        } else {
            LcConsistency::Feasible
        }
    }

    /// Full correlated local starting-phase fibre at a specified recovered rate.
    /// # Errors
    /// Refuses rates outside the projected support and exhausted caller resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        if !support::contains(&self.0.rate, rate, budget)? {
            return Err(LcRelationError::Containment);
        }
        project::phase(&self.0.inputs.strips, rate, budget)
    }

    /// Check an arbitrary member of the retained free-scale and common-lift parameter map.
    /// # Errors
    /// Refuses parameters outside the complete joint constraints or caller resource exhaustion.
    pub fn witness(
        &self,
        parameters: &LcPairParameters,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        witness::check(
            &self.0.relation,
            &self.0.inputs,
            &self.0.cycles,
            &self.0.rate,
            parameters,
            budget,
        )
    }

    /// Original exact tuple establishing nonemptiness, retained with the coverage certificate.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcWitness {
        &self.0.witness
    }

    /// Independent exact constant evidence for complete branch exclusion.
    #[must_use]
    pub fn pi_bounds(&self) -> &CheckedPiBounds {
        &self.0.pi
    }

    /// Before/after exact checking counters on the original cumulative caller ledger.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new exact evidence storage, excluding the shared original declaration.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
