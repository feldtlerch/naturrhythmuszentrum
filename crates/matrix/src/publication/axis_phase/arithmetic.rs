// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit rational spellings preserve the scalar decoder's proved exact-dyadic path.

use super::{AT, LiveDecisionError};
use nrz_bindings::{Budget, readings::algebra::Rational};
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, NumericLiteral, Scalar};

/// Integer denominators stay explicit; representable dyadics then need no artificial error.
pub(super) fn endpoint<P: Scalar>(
    exact: &Rational,
    arithmetic: &ArithmeticContext,
    literals: LiteralLimits,
    budget: &mut Budget,
    at: nrz_bindings::readings::algebra::Location,
) -> Result<Enclosure<P>, LiveDecisionError> {
    let mut text = exact.exact_string(budget)?;
    budget.work(2, AT)?;
    if exact.is_integer() {
        text.push_str("/1");
    }
    budget.work(literals.max_work(), at)?;
    Ok(NumericLiteral::new(&text, literals)?
        .decode::<P>(arithmetic)?
        .enclosure())
}
