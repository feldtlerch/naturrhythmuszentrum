// SPDX-License-Identifier: AGPL-3.0-only
//! Certified power coefficients use represented coordinates and a bounded binary schedule.

use super::{
    Affine, ArithmeticContext, CheckedSensitivity, Extent, Matrix, OperationError, Role, Scalar,
    SensitivityKernel, SensitivityRecipe, StatisticsError, singleton,
};
use nrz_scalar::Enclosure;

/// Discharge the retained whole-box premise before evaluating the fixed derivative instruction.
pub(super) fn map<P: Scalar + 'static, Q: Scalar + 'static>(
    points: &[Matrix<P>],
    proof: &CheckedSensitivity,
    context: &ArithmeticContext,
    run: &super::super::inventory::Run,
) -> Result<Affine<Q>, OperationError> {
    let [point] = points else {
        return Err(StatisticsError::Contract.into());
    };
    let [entry] = proof.entries() else {
        return Err(StatisticsError::Contract.into());
    };
    if point.extent != Extent::new(1, 1)? {
        return Err(StatisticsError::Contract.into());
    }
    let domain = point.entry(0, 0)?;
    let center: Q =
        crate::conversion::same_format(*point.centers.first().ok_or(StatisticsError::Contract)?)?;
    let (candidate, truth) = match (proof.recipe(), entry.kernel()) {
        (
            SensitivityRecipe::QuantityIntegerPower { exponent },
            SensitivityKernel::QuantityIntegerPower { exponent: actual },
        ) if actual == exponent => {
            if exponent <= 0 && domain.lower().get() <= P::ZERO && domain.upper().get() >= P::ZERO {
                return Err(StatisticsError::Domain {
                    function: crate::Elementwise::Reciprocal,
                    entry: 0,
                }
                .into());
            }
            coefficient(center, exponent, context)?
        }
        (SensitivityRecipe::QuantityPositiveRoot, SensitivityKernel::QuantityPositiveRoot) => {
            if domain.lower().get() <= P::ZERO {
                return Err(StatisticsError::Domain {
                    function: crate::Elementwise::SquareRoot,
                    entry: 0,
                }
                .into());
            }
            let two = Q::from_u8(2);
            (
                Q::ONE / (two * center.sqrt()),
                singleton(Q::ONE)?.div(
                    singleton(two)?.mul(singleton(center)?.sqrt(context)?, context)?,
                    context,
                )?,
            )
        }
        _ => return Err(StatisticsError::Contract.into()),
    };
    let mut result = Affine {
        jacobian: run.matrix_at(Role::Jacobian, 0, Extent::new(1, 1)?)?,
        offsets: run.matrix_at(Role::Offsets, 0, Extent::new(1, 1)?)?,
    };
    result.jacobian.set(0, 0, candidate, truth)?;
    Ok(result)
}

/// Enclose the exact signed integer without a potentially inexact native integer cast.
fn integer<P: Scalar>(
    value: i64,
    context: &ArithmeticContext,
) -> Result<(P, Enclosure<P>), OperationError> {
    let magnitude = value.unsigned_abs();
    let mut candidate = P::ZERO;
    let mut truth = singleton(P::ZERO)?;
    let two = P::from_u8(2);
    for bit in (0..(64 - magnitude.leading_zeros())).rev() {
        let digit = P::from_u8(u8::from((magnitude >> bit) & 1 != 0));
        candidate = candidate * two + digit;
        truth = truth
            .mul(singleton(two)?, context)?
            .add(singleton(digit)?, context)?;
    }
    if value < 0 {
        candidate = -candidate;
        truth = singleton(P::ZERO)?.sub(truth, context)?;
    }
    Ok((candidate, truth))
}

/// Binary exponentiation bounds at most sixty-four bits; special powers need no artificial division.
fn coefficient<P: Scalar>(
    center: P,
    exponent: i64,
    context: &ArithmeticContext,
) -> Result<(P, Enclosure<P>), OperationError> {
    if exponent == 0 {
        return Ok((P::ZERO, singleton(P::ZERO)?));
    }
    if exponent == 1 {
        return Ok((P::ONE, singleton(P::ONE)?));
    }
    let power = exponent
        .checked_sub(1)
        .ok_or(StatisticsError::ResourceLimit)?;
    let mut remaining = power.unsigned_abs();
    let (mut factor, mut enclosure) = (center, singleton(center)?);
    let (mut candidate, mut truth) = (P::ONE, singleton(P::ONE)?);
    while remaining != 0 {
        if remaining & 1 != 0 {
            candidate = candidate * factor;
            truth = truth.mul(enclosure, context)?;
        }
        remaining >>= 1;
        if remaining != 0 {
            factor = factor * factor;
            enclosure = enclosure.square(context)?;
        }
    }
    if power < 0 {
        candidate = P::ONE / candidate;
        truth = singleton(P::ONE)?.div(truth, context)?;
    }
    let (integer, exact) = integer(exponent, context)?;
    Ok((integer * candidate, exact.mul(truth, context)?))
}
