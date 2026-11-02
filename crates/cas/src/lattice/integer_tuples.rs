// SPDX-License-Identifier: AGPL-3.0-only
//! Unpruned integer tuple domains carry no feasibility or branch-selection assertion.

use crate::{
    Fault, Location, Rational,
    limits::{Budget, BudgetKind},
};

/// Mathematical index-domain checks share the caller's exact ledger.
const AT: Location = Location::new("unrestricted integer tuple domain");

/// The complete mathematical Cartesian domain `Z^d`, without machine-integer cutoffs.
#[derive(Clone, Debug)]
pub struct IntegerTupleDomain {
    /// Number of named original index coordinates.
    coordinates: usize,
}

impl IntegerTupleDomain {
    /// Retain every integer tuple of the declared dimension, with no omitted prefix or suffix.
    /// # Errors
    /// Refuses caller metadata or work exhaustion.
    pub fn unrestricted(coordinates: usize, budget: &mut Budget) -> Result<Self, Fault> {
        budget.work(1, AT)?;
        budget.check(BudgetKind::OutputBytes, 128, AT)?;
        budget.check(BudgetKind::StagedPayloadBytes, 128, AT)?;
        Ok(Self { coordinates })
    }

    /// Mathematical coordinate count; it imposes no magnitude bound on any index.
    #[must_use]
    pub const fn coordinates(&self) -> usize {
        self.coordinates
    }

    /// Exact membership without converting any coordinate to a machine integer.
    /// # Errors
    /// Refuses exact component or cumulative work exhaustion.
    pub fn contains(&self, tuple: &[Rational], budget: &mut Budget) -> Result<bool, Fault> {
        budget.work(1, AT)?;
        if tuple.len() != self.coordinates {
            return Ok(false);
        }
        for value in tuple {
            let (top, bottom) = value.component_bits();
            budget.work(
                top.div_ceil(64).saturating_add(bottom.div_ceil(64)).max(1),
                AT,
            )?;
            value.check(budget, AT)?;
            if !value.is_integer() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
