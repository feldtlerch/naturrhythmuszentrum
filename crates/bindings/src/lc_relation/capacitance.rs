// SPDX-License-Identifier: AGPL-3.0-only
//! One positive capacitor interval retains its joint rate/capacitance domain and phase fibres.

mod admit;

use super::{
    AT, LcCertainty, LcDeterminacy, LcFreeDirection, LcInterval, LcPairParameters, LcPairsCover,
    LcQuantity, LcRelation, LcRelationError, LcWitness, storage,
};
use crate::Budget;
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::{cmp::Ordering, rc::Rc};

/// Exact parameters of the retained joint domain; voltage supplies the remaining amplitude scale.
#[derive(Clone, Debug)]
pub struct LcCapacitanceParameters {
    /// Positive rate in the recovered support.
    pub rate: Rational,
    /// Starting phase in the retained rate-dependent fibre.
    pub initial_phase: Rational,
    /// Positive capacitance in its original interval, independent of the rate parameter.
    pub capacitance: Rational,
    /// Positive free voltage magnitude.
    pub voltage: Rational,
    /// Original common integer phase lift.
    pub gauge: Rational,
}

/// Complete bounded pair family with a positive rational capacitor interval.
#[derive(Clone, Debug)]
pub struct LcCapacitanceCover(Rc<Data>);

/// The original capacitor constraint is retained alongside its checked existential elimination.
#[derive(Debug)]
struct Data {
    /// Every original supplied constraint, including all capacitor intervals.
    relation: LcRelation,
    /// Complete phase/rate projection after eliminating only the positive capacitor scale.
    pairs: LcPairsCover,
    /// Joint capacitor parameter support; element marginals do not replace this domain.
    capacitance: [Rational; 2],
    /// A complete original-equation tuple including the supplied capacitor interval.
    witness: LcWitness,
    /// Cumulative original checking snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative new exact ownership, excluding the shared original relation.
    bytes: u64,
}

impl LcRelation {
    /// Admit one positive capacitor interval on an otherwise phase/rate-only bounded pair family.
    ///
    /// Repeated capacitor declarations intersect. All other physical scales refuse this
    /// closed adapter, so existentially eliminating capacitance cannot change the rate set.
    /// The answer keeps the original full relation and the joint `(a,C)` parameter domain.
    /// # Errors
    /// Refuses unsupported scales, nonpositive/empty capacitor support, unproved branches
    /// and exhausted caller resources, without publishing a narrowed answer.
    pub fn capacitance_pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcCapacitanceCover, LcRelationError> {
        let before = budget.usage();
        let (phase_relation, capacitance) = admit::inputs(self, budget)?;
        let pairs = phase_relation.pairs_cover(cycles, pi, budget)?;
        let rate = pairs.rate()[0].clone();
        let phase = pairs.phase_at_rate(&rate, budget)?;
        let parameters = LcCapacitanceParameters {
            rate,
            initial_phase: phase[0].clone(),
            capacitance: capacitance[0].clone(),
            voltage: Rational::one(),
            gauge: Rational::zero(),
        };
        let witness = witness(self, &pairs, &capacitance, &parameters, budget)?;
        let bytes = capacitance
            .iter()
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
            .saturating_add(pairs.retained_payload_bytes())
            .saturating_add(phase_relation.retained_payload_bytes())
            .saturating_add(witness.retained_payload_bytes());
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcCapacitanceCover(Rc::new(Data {
            relation: self.clone(),
            pairs,
            capacitance,
            witness,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcCapacitanceCover {
    /// Full original relation, including the capacitor input and original common clock.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Complete rate support; its feasible phase fibre remains separately queryable.
    #[must_use]
    pub fn rate(&self) -> &[Rational; 2] {
        self.0.pairs.rate()
    }

    /// Original intersected positive capacitor support in the joint parameter domain.
    #[must_use]
    pub fn capacitance(&self) -> &[Rational; 2] {
        &self.0.capacitance
    }

    /// Exact original base-cycle tuple, with no additional alias cutoff.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.0.pairs.cycles()
    }

    /// Retained initial-phase gauge, unchanged by the capacitor uncertainty.
    #[must_use]
    pub fn phase_gauge(&self) -> super::LcPhaseGauge {
        self.0.pairs.phase_gauge()
    }

    /// Original complete tuple establishing nonemptiness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcWitness {
        &self.0.witness
    }

    /// Whole-family coverage and the total positive map establish certified nonemptiness.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Finite physical uncertainty is distinct from the unconstrained amplitude scale.
    /// # Errors
    /// Refuses quantities outside the admitted projection surface.
    pub const fn determinacy(
        &self,
        quantity: LcQuantity,
    ) -> Result<LcDeterminacy, LcRelationError> {
        Ok(match quantity {
            LcQuantity::Rate
            | LcQuantity::AngularFrequency
            | LcQuantity::Product
            | LcQuantity::NaturalTime
            | LcQuantity::Inductance
            | LcQuantity::Capacitance
            | LcQuantity::AmplitudeRatio => LcDeterminacy::DeterminedFinite,
            LcQuantity::VoltageAmplitude | LcQuantity::CurrentAmplitude | LcQuantity::Energy => {
                LcDeterminacy::Underdetermined
            }
            LcQuantity::Time(_) | LcQuantity::Phase(_) | LcQuantity::InitialPhase => {
                return Err(LcRelationError::Projection);
            }
        })
    }

    /// Name the remaining amplitude freedom, without hiding it in finite element marginals.
    /// # Errors
    /// Refuses unsupported projection quantities.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        Ok(match self.determinacy(quantity)? {
            LcDeterminacy::Underdetermined => &[LcFreeDirection::AmplitudeScale],
            LcDeterminacy::DeterminedFinite => &[],
            LcDeterminacy::DeterminedInfinite | LcDeterminacy::Impossible => {
                return Err(LcRelationError::Projection);
            }
        })
    }

    /// Correlated starting-phase fibre; it is never replaced by a phase marginal.
    /// # Errors
    /// Refuses a rate outside the original support and exhausted exact arithmetic.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.pairs.phase_at_rate(rate, budget)
    }

    /// Exact complete marginals, all retaining this same joint rate/capacitor source.
    /// # Errors
    /// Refuses unsupported quantities and exhausted caller resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        budget.work(1, AT)?;
        let bounds = match quantity {
            LcQuantity::Capacitance => self.0.capacitance.clone(),
            LcQuantity::Inductance | LcQuantity::AmplitudeRatio => {
                let power = if quantity == LcQuantity::Inductance {
                    2
                } else {
                    1
                };
                [
                    Rational::one().div(
                        &self.rate()[1]
                            .pow(power, budget)?
                            .mul(&self.0.capacitance[1], budget)?,
                        budget,
                    )?,
                    Rational::one().div(
                        &self.rate()[0]
                            .pow(power, budget)?
                            .mul(&self.0.capacitance[0], budget)?,
                        budget,
                    )?,
                ]
            }
            _ => return self.0.pairs.bounds(quantity, budget),
        };
        let [lower, upper] = bounds;
        LcInterval::new(Some(lower.into()), Some(upper.into()), budget)
    }

    /// Realize any admissible joint rate/capacitance parameter, retaining every original input.
    /// # Errors
    /// Refuses parameters outside the joint domain or exhausted original-equation checking.
    pub fn witness(
        &self,
        parameters: &LcCapacitanceParameters,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        witness(
            &self.0.relation,
            &self.0.pairs,
            &self.0.capacitance,
            parameters,
            budget,
        )
    }

    /// Original construction snapshots, excluding later numerical materialization.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new exact metadata ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// The complete positive map uses `R=1/(a*C)`, then checks the full original input family.
fn witness(
    relation: &LcRelation,
    pairs: &LcPairsCover,
    capacitance: &[Rational; 2],
    parameters: &LcCapacitanceParameters,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    let above = parameters.capacitance.compare(&capacitance[0], budget)? != Ordering::Less;
    let below = parameters.capacitance.compare(&capacitance[1], budget)? != Ordering::Greater;
    if !above || !below {
        return Err(LcRelationError::Containment);
    }
    let ratio = Rational::one().div(
        &parameters.rate.mul(&parameters.capacitance, budget)?,
        budget,
    )?;
    let point = pairs.witness(
        &LcPairParameters {
            rate: parameters.rate.clone(),
            initial_phase: parameters.initial_phase.clone(),
            ratio,
            voltage: parameters.voltage.clone(),
            gauge: parameters.gauge.clone(),
        },
        budget,
    )?;
    relation.check_witness(point.values().clone(), budget)
}
