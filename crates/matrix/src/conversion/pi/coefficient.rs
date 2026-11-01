// SPDX-License-Identifier: AGPL-3.0-only
//! Shared numerical recipe for a recognized exact rational factor of pi.

use super::{ConversionError, PiConstant};
use crate::coordinate_numeric::ConstantCoefficient;
use nrz_bindings::{Budget, PiFactor};
use nrz_scalar::{ArithmeticContext, LiteralLimits, NumericLiteral, Scalar};

/// Literal import and the admitted pi operation retain one complete numerical discrepancy.
pub(crate) fn decode<P: Scalar + 'static>(
    factor: (&str, PiFactor, u64),
    pi: &PiConstant<P>,
    literals: LiteralLimits,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<ConstantCoefficient<P>, ConversionError> {
    let (spelling, action, work) = factor;
    let remaining = budget.limits().work.saturating_sub(budget.usage().work);
    let limits = LiteralLimits::new(
        literals.max_digits(),
        literals.max_exponent(),
        literals.max_work().min(remaining),
    );
    let mut used = 0;
    let decoded = NumericLiteral::new_with_work(spelling, limits, &mut used)
        .and_then(|literal| literal.decode_with_work::<P>(context, &mut used));
    super::charge(budget, used)?;
    let decoded = decoded?;
    let work = work
        .checked_add(decoded.work_used())
        .ok_or(ConversionError::ResourceLimit)?;
    if action == PiFactor::Rational {
        super::charge(budget, 1)?;
        return ConstantCoefficient::enclosed(
            decoded.enclosure(),
            work.checked_add(1).ok_or(ConversionError::ResourceLimit)?,
            context,
        );
    }
    super::charge(budget, 2)?;
    let work = work.checked_add(2).ok_or(ConversionError::ResourceLimit)?;
    let factor = match action {
        PiFactor::Multiply => pi.pi().enclosure(),
        PiFactor::Divide => pi.reciprocal(),
        PiFactor::Rational => return Err(ConversionError::Contract),
    };
    ConstantCoefficient::enclosed(decoded.enclosure().mul(factor, context)?, work, context)
}
