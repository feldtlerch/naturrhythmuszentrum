// SPDX-License-Identifier: AGPL-3.0-only
//! Closed input admission checks every premise of the total zero-time feasibility map.

use super::{AT, LcRelation, LcRelationError, LcWitness, storage};
use crate::{Budget, ExactConstant, LcQuantity as Q, LcValues};
use nrz_cas::Rational;
use std::cmp::Ordering;

/// Optional energy support is realized by one positive energy at every rate.
#[derive(Debug)]
pub(super) struct Rectangle {
    /// Positive exact energy satisfying all original energy constraints.
    energy: Rational,
}

impl Rectangle {
    /// Missing supports or any unsupported original constraint refuse the complete adapter.
    pub(super) fn check(
        relation: &LcRelation,
        budget: &mut Budget,
    ) -> Result<(Self, [Rational; 2]), LcRelationError> {
        budget.work(1, AT)?;
        if relation.observations() != 0 {
            return Err(LcRelationError::Projection);
        }
        let mut slots: [Option<[Rational; 2]>; 3] = [None, None, None];
        for input in relation.inputs() {
            budget.work(1, AT)?;
            let slot = match input.quantity() {
                Q::Rate | Q::AngularFrequency => 0,
                Q::InitialPhase => 1,
                Q::Energy => 2,
                _ => return Err(LcRelationError::Projection),
            };
            let lower = input
                .interval()
                .lower()
                .and_then(ExactConstant::as_rational)
                .ok_or(LcRelationError::Projection)?;
            let upper = input
                .interval()
                .upper()
                .and_then(ExactConstant::as_rational)
                .ok_or(LcRelationError::Projection)?;
            let mut next = [lower.clone(), upper.clone()];
            if let Some(previous) = slots[slot].as_ref() {
                if previous[0].compare(&next[0], budget)? == Ordering::Greater {
                    next[0].clone_from(&previous[0]);
                }
                if previous[1].compare(&next[1], budget)? == Ordering::Less {
                    next[1].clone_from(&previous[1]);
                }
            }
            if next[0].compare(&next[1], budget)? == Ordering::Greater {
                return Err(LcRelationError::Containment);
            }
            slots[slot] = Some(next);
        }
        let [rate, phase, energy] = slots;
        let rate = rate.ok_or(LcRelationError::Projection)?;
        let phase = phase.ok_or(LcRelationError::Projection)?;
        if !phase[0].is_zero() || !phase[1].is_zero() {
            return Err(LcRelationError::Projection);
        }
        let energy = energy.map_or_else(Rational::one, |bounds| bounds[1].clone());
        if energy.is_zero() || energy.is_negative() {
            return Err(LcRelationError::Positivity);
        }
        Ok((Self { energy }, rate))
    }

    /// Total positive map satisfying the original LC and energy laws at every admitted rate.
    pub(super) fn witness(
        &self,
        relation: &LcRelation,
        support: &[Rational; 2],
        rate: Rational,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        let above = rate.compare(&support[0], budget)? != Ordering::Less;
        let below = rate.compare(&support[1], budget)? != Ordering::Greater;
        if !above || !below {
            return Err(LcRelationError::Containment);
        }
        let element = ExactConstant::from(Rational::one().div(&rate, budget)?);
        let twice = Rational::from_ratio(2, 1, budget)?;
        let square = ExactConstant::from(self.energy.mul(&twice, budget)?.mul(&rate, budget)?);
        let amplitude = square
            .positive()
            .ok_or(LcRelationError::Positivity)?
            .sqrt(budget)?;
        relation.check_witness(
            LcValues {
                inductance: element.clone(),
                capacitance: element,
                rate: rate.into(),
                initial_phase: Some(ExactConstant::zero()),
                voltage: amplitude.clone(),
                current: amplitude,
                energy: self.energy.clone().into(),
                observations: Vec::new(),
            },
            budget,
        )
    }

    /// Conservative exact map ownership.
    pub(super) fn bytes(&self) -> u64 {
        storage::rational(&self.energy).saturating_add(256)
    }
}
