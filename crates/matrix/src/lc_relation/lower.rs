// SPDX-License-Identifier: AGPL-3.0-only
//! Exact rational spellings and the shared pi enclosure bound every affine endpoint.

use super::{AT, LcNumericalArc, LcProjectionError};
use crate::PiConstant;
use nrz_bindings::readings::algebra::Rational;
use nrz_bindings::{Budget, LcForwardCover, LcPhaseEndpoint};
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, NumericLiteral, Scalar};

/// The preflighted complete family is staged locally before one immutable publication.
pub(super) fn family<P: Scalar + 'static>(
    cover: &LcForwardCover,
    pi: &PiConstant<P>,
    count: usize,
    literals: LiteralLimits,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Vec<LcNumericalArc<P>>, LcProjectionError> {
    let mut result = Vec::with_capacity(count);
    let mut index = cover.cycles()[0].clone();
    for _ in 0..count {
        budget.work(1, AT)?;
        let exact = cover.arc(index.clone(), budget)?;
        let (lower, upper) = exact.endpoints();
        let lower = endpoint(lower, pi, literals, context, budget)?;
        let upper = endpoint(upper, pi, literals, context, budget)?;
        let enclosure = Enclosure::new(lower.lower().get(), upper.upper().get())?;
        result.push(LcNumericalArc {
            exact,
            endpoints: [lower, upper],
            enclosure,
        });
        index = index.add(&Rational::one(), budget)?;
    }
    Ok(result)
}

/// Affine endpoint evaluation carries both pi error and every rounded arithmetic operation.
fn endpoint<P: Scalar + 'static>(
    endpoint: &LcPhaseEndpoint,
    pi: &PiConstant<P>,
    literals: LiteralLimits,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Enclosure<P>, LcProjectionError> {
    let (rational, coefficient) = endpoint.coefficients();
    let offset = decode(rational, literals, context, budget)?;
    let coefficient = decode(coefficient, literals, context, budget)?;
    budget.work(2, AT)?;
    Ok(offset.add(coefficient.mul(pi.pi().enclosure(), context)?, context)?)
}

/// One cumulative exact/numerical ledger reconciles literal failure work as well as success.
pub(super) fn decode<P: Scalar>(
    value: &Rational,
    literals: LiteralLimits,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Enclosure<P>, LcProjectionError> {
    let spelling = value.exact_string(budget)?;
    let remaining = budget.limits().work.saturating_sub(budget.usage().work);
    let limits = LiteralLimits::new(
        literals.max_digits(),
        literals.max_exponent(),
        literals.max_work().min(remaining),
    );
    let mut used = 0;
    let result = NumericLiteral::new_with_work(&spelling, limits, &mut used)
        .and_then(|literal| literal.decode_with_work::<P>(context, &mut used));
    budget.work(used, AT)?;
    Ok(result?.enclosure())
}
