// SPDX-License-Identifier: AGPL-3.0-only
//! Positive rectangle extrema and zero-energy reconstruction use the caller's exact ledger.

use super::{AT, LcQuantity, LcQuiescentOutcome, LcRelation, LcRelationError, storage};
use crate::lc_relation::calibration::{self, Bounds};
use crate::{Budget, ExactConstant, LcInterval, LcObservationValue, LcValues};
use nrz_cas::Rational;

/// Repeated original declarations define one independent positive element rectangle.
#[derive(Debug)]
pub(super) struct Domain {
    /// Intersected inductance support.
    pub(super) inductance: Bounds,
    /// Intersected capacitance support.
    pub(super) capacitance: Bounds,
    /// Optional single original timestamp, independent of the positive elements.
    pub(super) time: Option<Bounds>,
    /// Complete positive product image.
    product: Bounds,
    /// Complete natural-time image, with supported exact rational extrema.
    natural_time: Bounds,
    /// Reciprocal natural-time image.
    rate: Bounds,
}

impl Domain {
    /// Inspect all originals, refusing any constraint that would alter the rectangle domain.
    pub(super) fn check(
        relation: &LcRelation,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        storage::retained(
            relation.retained_payload_bytes().saturating_add(4_096),
            budget,
        )?;
        if relation.observations() > 1 {
            return Err(LcRelationError::Projection);
        }
        let mut slots: [Option<Bounds>; 4] = [None, None, None, None];
        for input in relation.inputs() {
            budget.work(1, AT)?;
            let slot = match input.quantity() {
                LcQuantity::Inductance => 0,
                LcQuantity::Capacitance => 1,
                LcQuantity::Energy => 2,
                LcQuantity::Time(_) => 3,
                _ => return Err(LcRelationError::Projection),
            };
            let next = bounds(input.interval(), budget)?;
            if let Some(previous) = &mut slots[slot] {
                calibration::intersect(previous, &next, budget)?;
            } else {
                slots[slot] = Some(next);
            }
        }
        let [inductance, capacitance, energy, time] = slots;
        if time.is_some() != (relation.observations() == 1) {
            return Err(LcRelationError::Projection);
        }
        let inductance = inductance.ok_or(LcRelationError::Projection)?;
        let capacitance = capacitance.ok_or(LcRelationError::Projection)?;
        let energy = energy.ok_or(LcRelationError::Projection)?;
        calibration::positive(&inductance[0], budget)?;
        calibration::positive(&capacitance[0], budget)?;
        if !energy[0].is_zero() || !energy[1].is_zero() {
            return Err(LcRelationError::Projection);
        }
        let product = [
            inductance[0].mul(&capacitance[0], budget)?,
            inductance[1].mul(&capacitance[1], budget)?,
        ];
        let natural_time = [
            rational_root(&product[0], budget)?,
            rational_root(&product[1], budget)?,
        ];
        let rate = [
            Rational::one().div(&natural_time[1], budget)?,
            Rational::one().div(&natural_time[0], budget)?,
        ];
        Ok(Self {
            inductance,
            capacitance,
            time,
            product,
            natural_time,
            rate,
        })
    }

    /// Positive multiplication, square root and reciprocal give the exact scalar images.
    pub(super) fn query(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcQuiescentOutcome, LcRelationError> {
        budget.work(1, AT)?;
        let range = match quantity {
            LcQuantity::InitialPhase => return Ok(LcQuiescentOutcome::UndefinedPhase),
            LcQuantity::AmplitudeRatio => return Ok(LcQuiescentOutcome::UndefinedRatio),
            LcQuantity::Phase(index) => {
                if index != 0 || self.time.is_none() {
                    return Err(LcRelationError::Observation);
                }
                return Ok(LcQuiescentOutcome::UndefinedPhase);
            }
            LcQuantity::Time(index) => {
                if index != 0 {
                    return Err(LcRelationError::Observation);
                }
                self.time.as_ref().ok_or(LcRelationError::Observation)?
            }
            LcQuantity::Inductance => &self.inductance,
            LcQuantity::Capacitance => &self.capacitance,
            LcQuantity::Product => &self.product,
            LcQuantity::NaturalTime => &self.natural_time,
            LcQuantity::Rate | LcQuantity::AngularFrequency => &self.rate,
            LcQuantity::Energy | LcQuantity::VoltageAmplitude | LcQuantity::CurrentAmplitude => {
                return Ok(LcQuiescentOutcome::Defined(LcInterval::exact(
                    ExactConstant::zero(),
                    budget,
                )?));
            }
        };
        Ok(LcQuiescentOutcome::Defined(calibration::interval(
            range, budget,
        )?))
    }

    /// Independent element parameters reconstruct one full tuple, with no arbitrary phase clock.
    pub(super) fn values(
        &self,
        inductance: &Rational,
        capacitance: &Rational,
        time: Option<&Rational>,
        budget: &mut Budget,
    ) -> Result<LcValues, LcRelationError> {
        calibration::contains(&self.inductance, inductance, budget)?;
        calibration::contains(&self.capacitance, capacitance, budget)?;
        let natural_time = root(&inductance.mul(capacitance, budget)?, budget)?;
        let rate = ExactConstant::one().div(&natural_time, budget)?;
        let observations = match (&self.time, time) {
            (Some(bounds), Some(time)) => {
                if !calibration::interval(bounds, budget)?.contains(&time.clone().into(), budget)? {
                    return Err(LcRelationError::Containment);
                }
                vec![LcObservationValue {
                    time: time.clone().into(),
                    phase: None,
                    cycle: None,
                }]
            }
            (None, None) => Vec::new(),
            _ => return Err(LcRelationError::Observation),
        };
        Ok(LcValues {
            inductance: inductance.clone().into(),
            capacitance: capacitance.clone().into(),
            rate,
            initial_phase: None,
            voltage: ExactConstant::zero(),
            current: ExactConstant::zero(),
            energy: ExactConstant::zero(),
            observations,
        })
    }

    /// Account for all exact derived endpoints and the new owner record.
    pub(super) fn bytes(&self) -> u64 {
        self.inductance
            .iter()
            .chain(&self.capacitance)
            .chain(&self.product)
            .chain(&self.natural_time)
            .chain(&self.rate)
            .chain(self.time.iter().flatten())
            .fold(1_024_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
    }
}

/// Energy declarations may straddle zero; their exact original intersection must be zero.
fn bounds(interval: &LcInterval, budget: &mut Budget) -> Result<Bounds, LcRelationError> {
    budget.work(1, AT)?;
    let lower = interval
        .lower()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let upper = interval
        .upper()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    lower.check(budget, AT)?;
    upper.check(budget, AT)?;
    Ok([lower.clone(), upper.clone()])
}

/// The restricted scalar cover requires rational extrema; it never approximates an exact root.
fn rational_root(value: &Rational, budget: &mut Budget) -> Result<Rational, LcRelationError> {
    root(value, budget)?
        .as_rational()
        .cloned()
        .ok_or(LcRelationError::Projection)
}

/// A witness may retain an exact positive radical in the existing closed arithmetic fragment.
fn root(value: &Rational, budget: &mut Budget) -> Result<ExactConstant, LcRelationError> {
    Ok(ExactConstant::from(value.clone())
        .positive()
        .ok_or(LcRelationError::Positivity)?
        .sqrt(budget)?)
}
