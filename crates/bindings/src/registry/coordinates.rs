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
    /// Admit an integer coordinate power without converting the original coordinate.
    /// # Errors
    /// Refuses points, counts, unsupported semantic powers and exhausted exact resources.
    pub fn coordinate_integer_power(
        &self,
        base: &Descriptor,
        exponent: i64,
        budget: &mut Budget,
    ) -> Result<Descriptor, UnitError> {
        let exact = Rational::from_ratio(exponent, 1, budget)?;
        let derived = self.power(base, &exact)?;
        let scale = base.measure().scale().pow(exponent, budget)?;
        let digits = exponent
            .unsigned_abs()
            .checked_ilog10()
            .map_or(1, |value| value + 1);
        let length = base
            .measure()
            .name()
            .len()
            .saturating_add(usize::try_from(digits).unwrap_or(usize::MAX))
            .saturating_add(usize::from(exponent < 0))
            .saturating_add(3);
        coordinate_name(length, budget)?;
        let measure = Measure::new(
            format!("({}^{exponent})", base.measure().name()),
            Scale::Expression(scale),
            Rational::zero(),
        )?;
        Descriptor::new(derived.kind().clone(), measure, Role::Difference)
    }

    /// Admit the exact positive coordinate scale and half-dimensional square-root kind.
    /// # Errors
    /// Refuses unadmitted fractional dimensions, unsupported scale roots, points and counts.
    pub fn coordinate_square_root(
        &self,
        base: &Descriptor,
        budget: &mut Budget,
    ) -> Result<Descriptor, UnitError> {
        self.with_coordinate_square_root(base, budget, |descriptor, _| Ok(descriptor))
    }

    /// Prepare an exact root and retain its checked scale only after its consumer succeeds.
    ///
    /// This keeps a caller's later numerical or publication refusal from warming the cache.
    /// Work remains spent on every path; the callback shares the original caller ledger.
    /// # Errors
    /// Preserves descriptor, exact resource and consumer refusals without publishing a cache entry.
    pub fn with_coordinate_square_root<T, E>(
        &self,
        base: &Descriptor,
        budget: &mut Budget,
        consume: impl FnOnce(Descriptor, &mut Budget) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<UnitError> + From<nrz_cas::Fault>,
    {
        let half = Rational::from_ratio(1, 2, budget).map_err(UnitError::from)?;
        let derived = self.power(base, &half)?;
        let root = self
            .scale_roots
            .prepare(base.measure().scale_witness(), budget)?;
        let result = budget.with_retained_payload(
            root.reserved(),
            Location::new("coordinate scale root consumer"),
            |budget| {
                let scale = root.value(budget)?;
                coordinate_name(base.measure().name().len().saturating_add(6), budget)?;
                let measure = Measure::new(
                    format!("sqrt({})", base.measure().name()),
                    Scale::Expression(scale),
                    Rational::zero(),
                )?;
                let descriptor =
                    Descriptor::new(derived.kind().clone(), measure, Role::Difference)?;
                consume(descriptor, budget)
            },
        )?;
        self.scale_roots.publish(root);
        Ok(result)
    }
}

/// Prepay the exact unary measure spelling before allocating its reporting name.
fn coordinate_name(length: usize, budget: &mut Budget) -> Result<(), UnitError> {
    let bytes = u64::try_from(length).unwrap_or(u64::MAX);
    let at = Location::new("coordinate power measure name");
    budget.check(BudgetKind::OutputBytes, bytes, at)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, at)?;
    budget.work(bytes, at)?;
    Ok(())
}
