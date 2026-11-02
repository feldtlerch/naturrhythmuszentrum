// SPDX-License-Identifier: AGPL-3.0-only
//! Exact finite or unbounded deterministic support; no floating infinity or probability.

use super::{AT, LcRelationError, ordered, storage};
use crate::{Budget, ExactConstant};

/// Closed exact support, intersected with the relation's original physical domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LcInterval {
    /// Included finite lower bound, or a mathematical negative unbounded endpoint.
    lower: Option<ExactConstant>,
    /// Included finite upper bound, or a mathematical positive unbounded endpoint.
    upper: Option<ExactConstant>,
}

impl LcInterval {
    /// Admit ordered exact endpoints; absent bounds remain absent.
    /// # Errors
    /// Refuses false or unproved finite endpoint order and exhausted caller resources.
    pub fn new(
        lower: Option<ExactConstant>,
        upper: Option<ExactConstant>,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let result = Self { lower, upper };
        storage::retained(result.bytes(), budget)?;
        if let Some((lower, upper)) = result.lower.as_ref().zip(result.upper.as_ref())
            && !ordered(lower, upper, budget)?
        {
            return Err(LcRelationError::UnprovedOrder);
        }
        Ok(result)
    }

    /// An exact value is a singleton support, including exact irrational constants.
    /// # Errors
    /// Refuses exhausted caller resources.
    pub fn exact(value: ExactConstant, budget: &mut Budget) -> Result<Self, LcRelationError> {
        Self::new(Some(value.clone()), Some(value), budget)
    }

    /// No declared deterministic bounds; the relation's physical domain still applies.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    /// Included lower endpoint; `None` is a mathematical unbounded endpoint.
    #[must_use]
    pub const fn lower(&self) -> Option<&ExactConstant> {
        self.lower.as_ref()
    }

    /// Included upper endpoint; `None` is a mathematical unbounded endpoint.
    #[must_use]
    pub const fn upper(&self) -> Option<&ExactConstant> {
        self.upper.as_ref()
    }

    /// Check a proposed exact coordinate without interpreting a failed check as emptiness.
    pub(super) fn contains(
        &self,
        value: &ExactConstant,
        budget: &mut Budget,
    ) -> Result<bool, LcRelationError> {
        budget.work(1, AT)?;
        if let Some(lower) = &self.lower
            && !ordered(lower, value, budget)?
        {
            return Ok(false);
        }
        if let Some(upper) = &self.upper
            && !ordered(value, upper, budget)?
        {
            return Ok(false);
        }
        Ok(true)
    }

    /// Conservative exact endpoint ownership, including shared constant graph envelopes.
    pub(super) fn bytes(&self) -> u64 {
        self.lower
            .iter()
            .chain(&self.upper)
            .fold(128_u64, |bytes, value| {
                bytes.saturating_add(storage::constant(value))
            })
    }
}
