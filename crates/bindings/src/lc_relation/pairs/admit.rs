// SPDX-License-Identifier: AGPL-3.0-only
//! Every supplied quantity participates in the exact finite-cycle recovery question.

use super::{
    AT, Bounds, LcQuantity, LcRelation, LcRelationError, Rational, Strip, storage, support,
};
use crate::Budget;

/// Input supports before exact elimination; absent initial phase retains its integer gauge.
#[derive(Debug)]
pub(super) struct Inputs {
    /// All original observations as lifted rational strips, followed by optional initial phase.
    pub(super) strips: Vec<Strip>,
    /// Explicit supplied finite positive frequency domain; never an inferred search cutoff.
    pub(super) rate: Bounds,
    /// Whether the starting phase fixes the common cycle gauge.
    pub(super) fixed_phase: bool,
}

/// One original timestamp/phase inventory slot, intersecting repeated constraints.
#[derive(Default)]
struct Slot {
    /// Finite exact timestamp support; this adapter requires a singleton after intersection.
    time: Option<Bounds>,
    /// Lifted phase support on the caller's proposed branch.
    phase: Option<Bounds>,
}

impl Inputs {
    /// Admit all constraints or refuse the whole projection without silently dropping any.
    pub(super) fn check(
        relation: &LcRelation,
        cycles: &[Rational],
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        let count = relation.observations();
        if count < 2 || cycles.len() != count {
            return Err(LcRelationError::Observation);
        }
        storage::retained(storage::size(count)?.saturating_mul(8_192), budget)?;
        let mut slots: Vec<_> = (0..count).map(|_| Slot::default()).collect();
        let mut rate = None;
        let mut phase = None;
        let turn = support::turn(budget)?;
        for cycle in cycles {
            support::integer(cycle, budget)?;
        }
        for input in relation.inputs() {
            budget.work(1, AT)?;
            match input.quantity() {
                LcQuantity::Time(i) => support::constrain(
                    &mut slots[i].time,
                    support::bounds(input.interval(), budget)?,
                    budget,
                )?,
                LcQuantity::Phase(i) => {
                    let shift = turn.mul(&cycles[i].clone().into(), budget)?;
                    support::constrain(
                        &mut slots[i].phase,
                        support::lifted(input.interval(), &shift, budget)?,
                        budget,
                    )?;
                }
                LcQuantity::Rate | LcQuantity::AngularFrequency => support::constrain(
                    &mut rate,
                    support::bounds(input.interval(), budget)?,
                    budget,
                )?,
                LcQuantity::InitialPhase => support::constrain(
                    &mut phase,
                    support::bounds(input.interval(), budget)?,
                    budget,
                )?,
                _ => return Err(LcRelationError::Projection),
            }
        }
        let rate = rate.ok_or(LcRelationError::Projection)?;
        if rate[0].is_negative() || rate[0].is_zero() {
            return Err(LcRelationError::Projection);
        }
        let mut strips = Vec::with_capacity(count.saturating_add(1));
        for slot in slots {
            let time = slot.time.ok_or(LcRelationError::Projection)?;
            if time[0] != time[1] {
                return Err(LcRelationError::Projection);
            }
            strips.push(Strip {
                time: time[0].clone(),
                phase: slot.phase.ok_or(LcRelationError::Projection)?,
            });
        }
        let fixed_phase = phase.is_some();
        if let Some(phase) = phase {
            strips.push(Strip {
                time: Rational::zero(),
                phase,
            });
        }
        Ok(Self {
            strips,
            rate,
            fixed_phase,
        })
    }
}
