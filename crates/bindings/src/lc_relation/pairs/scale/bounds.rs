// SPDX-License-Identifier: AGPL-3.0-only
//! Monotone positive eliminations keep the same recovered rate parameter in every marginal.

use super::{Bounds, LcQuantity, LcRelationError, Rational, Scale, root, support};
use crate::Budget;

impl Scale {
    /// Every finite rational marginal is a monotone image of the same recovered rate support.
    pub(in super::super) fn bounds(
        &self,
        quantity: LcQuantity,
        rate: &Bounds,
        budget: &mut Budget,
    ) -> Result<Option<Bounds>, LcRelationError> {
        let fixed = match quantity {
            LcQuantity::Inductance => &self.inductance,
            LcQuantity::Capacitance => &self.capacitance,
            LcQuantity::AmplitudeRatio => &self.ratio,
            LcQuantity::VoltageAmplitude => &self.voltage,
            LcQuantity::CurrentAmplitude => &self.current,
            LcQuantity::Energy => &self.energy,
            _ => return Err(LcRelationError::Projection),
        };
        if let Some(value) = fixed {
            return Ok(Some([value.clone(), value.clone()]));
        }
        if !self.bounded(quantity) {
            return Ok(None);
        }
        let result = match quantity {
            LcQuantity::Inductance | LcQuantity::Capacitance | LcQuantity::AmplitudeRatio => {
                self.element(quantity, rate, budget)?
            }
            LcQuantity::VoltageAmplitude | LcQuantity::CurrentAmplitude => {
                self.amplitude(quantity, rate, budget)?
            }
            LcQuantity::Energy => self.energy_range(rate, budget)?,
            _ => return Err(LcRelationError::Projection),
        };
        Ok(Some(result))
    }

    /// Positive element and ratio identities share one rate parameter rather than new marginals.
    fn element(
        &self,
        quantity: LcQuantity,
        rate: &Bounds,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        match quantity {
            LcQuantity::Inductance => {
                if let Some(capacitance) = &self.capacitance {
                    divide(
                        &support::inverse_power(rate, 2, budget)?,
                        capacitance,
                        budget,
                    )
                } else {
                    multiply(
                        &support::inverse_power(rate, 1, budget)?,
                        self.ratio.as_ref().ok_or(LcRelationError::Projection)?,
                        budget,
                    )
                }
            }
            LcQuantity::Capacitance => {
                if let Some(inductance) = &self.inductance {
                    divide(
                        &support::inverse_power(rate, 2, budget)?,
                        inductance,
                        budget,
                    )
                } else {
                    divide(
                        &support::inverse_power(rate, 1, budget)?,
                        self.ratio.as_ref().ok_or(LcRelationError::Projection)?,
                        budget,
                    )
                }
            }
            LcQuantity::AmplitudeRatio => {
                if let Some(inductance) = &self.inductance {
                    multiply(rate, inductance, budget)
                } else {
                    divide(
                        &support::inverse_power(rate, 1, budget)?,
                        self.capacitance
                            .as_ref()
                            .ok_or(LcRelationError::Projection)?,
                        budget,
                    )
                }
            }
            _ => Err(LcRelationError::Projection),
        }
    }

    /// Energy from a supplied amplitude retains the same element dependence.
    fn energy_range(&self, rate: &Bounds, budget: &mut Budget) -> Result<Bounds, LcRelationError> {
        let (quantity, amplitude) = if let Some(voltage) = &self.voltage {
            (LcQuantity::Capacitance, voltage)
        } else {
            (
                LcQuantity::Inductance,
                self.current.as_ref().ok_or(LcRelationError::Projection)?,
            )
        };
        let element = self
            .bounds(quantity, rate, budget)?
            .ok_or(LcRelationError::Projection)?;
        let factor = amplitude
            .pow(2, budget)?
            .div(&Rational::from_ratio(2, 1, budget)?, budget)?;
        multiply(&element, &factor, budget)
    }

    /// Energy fixes amplitude only after the same positive element support is retained.
    fn amplitude(
        &self,
        quantity: LcQuantity,
        rate: &Bounds,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        let ratio = self
            .bounds(LcQuantity::AmplitudeRatio, rate, budget)?
            .ok_or(LcRelationError::Projection)?;
        if quantity == LcQuantity::VoltageAmplitude {
            if let Some(current) = &self.current {
                return multiply(&ratio, current, budget);
            }
        } else if let Some(voltage) = &self.voltage {
            return multiply(&support::inverse_power(&ratio, 1, budget)?, voltage, budget);
        }
        let element = if quantity == LcQuantity::VoltageAmplitude {
            LcQuantity::Capacitance
        } else {
            LcQuantity::Inductance
        };
        let element = self
            .bounds(element, rate, budget)?
            .ok_or(LcRelationError::Projection)?;
        let twice = self
            .energy
            .as_ref()
            .ok_or(LcRelationError::Projection)?
            .mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
        let squared = multiply(
            &support::inverse_power(&element, 1, budget)?,
            &twice,
            budget,
        )?;
        Ok([
            support::rational(&root(squared[0].clone(), budget)?)?,
            support::rational(&root(squared[1].clone(), budget)?)?,
        ])
    }
}

/// Positive scalar multiplication preserves exact endpoint order.
fn multiply(
    range: &Bounds,
    factor: &Rational,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    Ok([range[0].mul(factor, budget)?, range[1].mul(factor, budget)?])
}

/// Positive scalar division preserves exact endpoint order.
fn divide(
    range: &Bounds,
    factor: &Rational,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    Ok([range[0].div(factor, budget)?, range[1].div(factor, budget)?])
}
