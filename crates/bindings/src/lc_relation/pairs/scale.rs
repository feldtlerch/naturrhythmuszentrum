// SPDX-License-Identifier: AGPL-3.0-only
//! Positive exact scale constraints derive element, amplitude and energy projections.

mod bounds;
mod values;

use super::{
    AT, Bounds, LcFreeDirection, LcPairParameters, LcQuantity, LcRelationError, Rational, support,
};
use crate::{Budget, ExactConstant, LcInput};

/// Every supplied exact positive physical scale remains part of the same relation.
#[derive(Debug, Default)]
pub(super) struct Scale {
    /// Supplied or exactly eliminated inductance.
    inductance: Option<Rational>,
    /// Supplied or exactly eliminated capacitance.
    capacitance: Option<Rational>,
    /// Supplied or exactly eliminated impedance ratio.
    ratio: Option<Rational>,
    /// Supplied positive voltage magnitude.
    voltage: Option<Rational>,
    /// Supplied positive current magnitude.
    current: Option<Rational>,
    /// Supplied positive energy.
    energy: Option<Rational>,
    /// Every original element, amplitude and energy coordinate was supplied before elimination.
    pub(super) full_tuple: bool,
}

impl Scale {
    /// Every extra physical constraint is admitted exactly or refused, never omitted.
    pub(super) fn admit(
        &mut self,
        input: &LcInput,
        budget: &mut Budget,
    ) -> Result<(), LcRelationError> {
        let value = support::bounds(input.interval(), budget)?;
        if value[0] != value[1] {
            return Err(LcRelationError::Projection);
        }
        let slot = match input.quantity() {
            LcQuantity::Inductance => &mut self.inductance,
            LcQuantity::Capacitance => &mut self.capacitance,
            LcQuantity::AmplitudeRatio => &mut self.ratio,
            LcQuantity::VoltageAmplitude => &mut self.voltage,
            LcQuantity::CurrentAmplitude => &mut self.current,
            LcQuantity::Energy => &mut self.energy,
            _ => return Err(LcRelationError::Projection),
        };
        constrain(slot, value[0].clone(), budget)
    }

    /// Positive energy/amplitude eliminations retain every original equality and premise.
    pub(super) fn derive(
        &mut self,
        budget: &mut Budget,
    ) -> Result<Option<Rational>, LcRelationError> {
        self.full_tuple = [
            &self.inductance,
            &self.capacitance,
            &self.voltage,
            &self.current,
            &self.energy,
        ]
        .into_iter()
        .all(Option::is_some);
        if let Some(energy) = &self.energy {
            let twice = energy.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
            if let Some(voltage) = &self.voltage {
                constrain(
                    &mut self.capacitance,
                    twice.div(&voltage.pow(2, budget)?, budget)?,
                    budget,
                )?;
            }
            if let Some(current) = &self.current {
                constrain(
                    &mut self.inductance,
                    twice.div(&current.pow(2, budget)?, budget)?,
                    budget,
                )?;
            }
        }
        if let (Some(voltage), Some(current)) = (&self.voltage, &self.current) {
            constrain(&mut self.ratio, voltage.div(current, budget)?, budget)?;
        }
        let mut rate = None;
        if let (Some(inductance), Some(capacitance)) = (&self.inductance, &self.capacitance) {
            let product = inductance.mul(capacitance, budget)?;
            let root = root(product, budget)?;
            let rational = support::rational(&root)?;
            constrain(&mut rate, Rational::one().div(&rational, budget)?, budget)?;
        }
        if let (Some(ratio), Some(inductance)) = (&self.ratio, &self.inductance) {
            constrain(&mut rate, ratio.div(inductance, budget)?, budget)?;
        }
        if let (Some(ratio), Some(capacitance)) = (&self.ratio, &self.capacitance) {
            constrain(
                &mut rate,
                Rational::one().div(&ratio.mul(capacitance, budget)?, budget)?,
                budget,
            )?;
        }
        Ok(rate)
    }

    /// A scale relation fixes the impedance direction exactly, leaving only declared rate spread.
    const fn element_fixed(&self) -> bool {
        self.inductance.is_some() || self.capacitance.is_some() || self.ratio.is_some()
    }

    /// Finite physical projections distinguish a free scale from ordinary measurement spread.
    pub(super) const fn bounded(&self, quantity: LcQuantity) -> bool {
        match quantity {
            LcQuantity::Inductance | LcQuantity::Capacitance | LcQuantity::AmplitudeRatio => {
                self.element_fixed()
            }
            LcQuantity::VoltageAmplitude => {
                self.voltage.is_some()
                    || (self.element_fixed() && (self.current.is_some() || self.energy.is_some()))
            }
            LcQuantity::CurrentAmplitude => {
                self.current.is_some()
                    || (self.element_fixed() && (self.voltage.is_some() || self.energy.is_some()))
            }
            LcQuantity::Energy => {
                self.energy.is_some()
                    || (self.element_fixed() && (self.voltage.is_some() || self.current.is_some()))
            }
            _ => false,
        }
    }

    /// Every unbounded physical projection names an actual available continuous parameter.
    pub(super) const fn freedom(&self, quantity: LcQuantity) -> &'static [LcFreeDirection] {
        if self.bounded(quantity) {
            return &[];
        }
        match quantity {
            LcQuantity::Inductance | LcQuantity::Capacitance | LcQuantity::AmplitudeRatio => {
                &[LcFreeDirection::ElementScale]
            }
            _ => {
                if self.voltage.is_none() && self.current.is_none() && self.energy.is_none() {
                    &[LcFreeDirection::AmplitudeScale]
                } else {
                    &[LcFreeDirection::ElementScale]
                }
            }
        }
    }

    /// Conservative exact scale metadata is separate from the original shared declarations.
    pub(super) fn bytes(&self) -> u64 {
        [
            &self.inductance,
            &self.capacitance,
            &self.ratio,
            &self.voltage,
            &self.current,
            &self.energy,
        ]
        .into_iter()
        .flatten()
        .fold(512_u64, |bytes, value| {
            bytes.saturating_add(super::storage::rational(value))
        })
    }
}

/// Contradictory repeated or independently derived values cannot be replaced by the latest one.
fn constrain(
    slot: &mut Option<Rational>,
    value: Rational,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let (numerator, denominator) = value.component_bits();
    budget.work(
        numerator
            .div_ceil(64)
            .saturating_add(denominator.div_ceil(64))
            .max(1),
        AT,
    )?;
    value.check(budget, AT)?;
    if value.is_negative() || value.is_zero() {
        return Err(LcRelationError::Positivity);
    }
    if slot.as_ref().is_some_and(|previous| previous != &value) {
        return Err(LcRelationError::Containment);
    }
    *slot = Some(value);
    Ok(())
}

/// Positive roots retain their original real-domain proof and exact CAS expression.
fn root(value: Rational, budget: &mut Budget) -> Result<ExactConstant, LcRelationError> {
    Ok(ExactConstant::from(value)
        .positive()
        .ok_or(LcRelationError::Positivity)?
        .sqrt(budget)?)
}
