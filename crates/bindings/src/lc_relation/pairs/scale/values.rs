// SPDX-License-Identifier: AGPL-3.0-only
//! A total positive scale map extends each retained phase/rate fibre to the full original relation.

use super::{LcPairParameters, LcRelationError, Rational, Scale, root};
use crate::{Budget, ExactConstant};

/// Exact physical coordinates before adding original phase observations.
#[derive(Debug)]
pub(in super::super) struct Physical {
    /// Positive inductance selected by original scale constraints or the free ratio.
    pub(in super::super) inductance: Rational,
    /// Positive capacitance preserving the recovered LC product.
    pub(in super::super) capacitance: Rational,
    /// Positive voltage magnitude with exact roots retained when energy determines it.
    pub(in super::super) voltage: ExactConstant,
    /// Positive current magnitude, from the same ratio and voltage.
    pub(in super::super) current: ExactConstant,
    /// Positive energy, derived from the same capacitance and voltage.
    pub(in super::super) energy: ExactConstant,
}

impl Scale {
    /// Supplied scale constraints determine their coordinates; remaining parameters stay free.
    pub(in super::super) fn values(
        &self,
        parameters: &LcPairParameters,
        budget: &mut Budget,
    ) -> Result<Physical, LcRelationError> {
        let unconstrained = [
            &self.inductance,
            &self.capacitance,
            &self.ratio,
            &self.voltage,
            &self.current,
            &self.energy,
        ]
        .into_iter()
        .all(Option::is_none);
        if unconstrained {
            return free_values(parameters, budget);
        }
        let ratio = if let Some(ratio) = &self.ratio {
            ratio.clone()
        } else if let Some(inductance) = &self.inductance {
            inductance.mul(&parameters.rate, budget)?
        } else if let Some(capacitance) = &self.capacitance {
            Rational::one().div(&capacitance.mul(&parameters.rate, budget)?, budget)?
        } else {
            parameters.ratio.clone()
        };
        let inductance = ratio.div(&parameters.rate, budget)?;
        let capacitance = Rational::one().div(&ratio.mul(&parameters.rate, budget)?, budget)?;
        let voltage = if let Some(voltage) = &self.voltage {
            ExactConstant::from(voltage.clone())
        } else if let Some(current) = &self.current {
            ExactConstant::from(current.mul(&ratio, budget)?)
        } else if let Some(energy) = &self.energy {
            let twice = energy.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
            root(twice.div(&capacitance, budget)?, budget)?
        } else {
            ExactConstant::from(parameters.voltage.clone())
        };
        let current = voltage.div(&ratio.into(), budget)?;
        let energy = ExactConstant::from(capacitance.clone())
            .mul(&voltage.pow(2, budget)?, budget)?
            .div(&Rational::from_ratio(2, 1, budget)?.into(), budget)?;
        Ok(Physical {
            inductance,
            capacitance,
            voltage,
            current,
            energy,
        })
    }
}

/// Preserve the established rational free-scale schedule and its literal alias Work boundaries.
fn free_values(
    parameters: &LcPairParameters,
    budget: &mut Budget,
) -> Result<Physical, LcRelationError> {
    let inductance = parameters.ratio.div(&parameters.rate, budget)?;
    let capacitance =
        Rational::one().div(&parameters.ratio.mul(&parameters.rate, budget)?, budget)?;
    let current = parameters.voltage.div(&parameters.ratio, budget)?;
    let twice_energy = capacitance.mul(&parameters.voltage.pow(2, budget)?, budget)?;
    let energy = twice_energy.div(&Rational::from_ratio(2, 1, budget)?, budget)?;
    Ok(Physical {
        inductance,
        capacitance,
        voltage: parameters.voltage.clone().into(),
        current: current.into(),
        energy: energy.into(),
    })
}
