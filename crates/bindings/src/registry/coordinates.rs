// SPDX-License-Identifier: AGPL-3.0-only
//! Coordinate products retain exact measures after the ordinary semantic admission.

use crate::{BinaryOp, Budget, Descriptor, Measure, Registry, Role, Scale, UnitError};
use nrz_cas::{Location, Rational, limits::BudgetKind};

impl Registry {
    /// Admit a product or quotient while preserving the operands' coordinate measures.
    ///
    /// The ordinary registry operation still derives coherent-coordinate meaning. This
    /// adapter composes exact scales without converting either numerical coordinate.
    /// # Errors
    /// Refuses additive operations, invalid kinds or roles, and exhausted exact arithmetic.
    pub fn coordinate_binary(
        &self,
        operation: BinaryOp,
        left: &Descriptor,
        right: &Descriptor,
        budget: &mut Budget,
    ) -> Result<Descriptor, UnitError> {
        let separator = match operation {
            BinaryOp::Multiply => '*',
            BinaryOp::Divide => '/',
            BinaryOp::Add | BinaryOp::Subtract => return Err(UnitError::UnsupportedKindRule),
        };
        let derived = self.binary(operation, left, right)?;
        let scale = match operation {
            BinaryOp::Multiply => left
                .measure()
                .scale()
                .mul(right.measure().scale(), budget)?,
            BinaryOp::Divide => left
                .measure()
                .scale()
                .div(right.measure().scale(), budget)?,
            BinaryOp::Add | BinaryOp::Subtract => return Err(UnitError::UnsupportedKindRule),
        };
        let length = left
            .measure()
            .name()
            .len()
            .saturating_add(right.measure().name().len())
            .saturating_add(3);
        let bytes = u64::try_from(length).unwrap_or(u64::MAX);
        let at = Location::new("coordinate product measure name");
        budget.check(BudgetKind::OutputBytes, bytes, at)?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, at)?;
        budget.work(bytes, at)?;
        let name = format!(
            "({}{separator}{})",
            left.measure().name(),
            right.measure().name()
        );
        let measure = Measure::new(name, Scale::Expression(scale), Rational::zero())?;
        Descriptor::new(derived.kind().clone(), measure, Role::Difference)
    }
}
