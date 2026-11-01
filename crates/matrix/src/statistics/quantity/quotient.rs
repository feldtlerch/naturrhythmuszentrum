// SPDX-License-Identifier: AGPL-3.0-only
//! A separated divisor authorizes independently enclosed quotient coefficients.

use super::{
    Affine, ArithmeticContext, CheckedSensitivity, Extent, Matrix, OperationError, Role, Scalar,
    SensitivityKernel, StatisticsError, singleton,
};

/// The numerical coefficients use represented coordinates, never the physical input box as error.
pub(super) fn map<P: Scalar + 'static, Q: Scalar + 'static>(
    points: &[Matrix<P>],
    proof: &CheckedSensitivity,
    context: &ArithmeticContext,
    run: &super::super::inventory::Run,
) -> Result<Affine<Q>, OperationError> {
    let [left, right] = points else {
        return Err(StatisticsError::Contract.into());
    };
    if proof.entries().len() != 2 || points.iter().any(|point| point.extent.entries() != 1) {
        return Err(StatisticsError::Contract.into());
    }
    let domain = right.entry(0, 0)?;
    if domain.lower().get() <= P::ZERO && domain.upper().get() >= P::ZERO {
        return Err(StatisticsError::Domain {
            function: crate::Elementwise::Reciprocal,
            entry: 0,
        }
        .into());
    }
    let x: Q =
        crate::conversion::same_format(*left.centers.first().ok_or(StatisticsError::Contract)?)?;
    let y: Q =
        crate::conversion::same_format(*right.centers.first().ok_or(StatisticsError::Contract)?)?;
    let numerator = singleton(x)?;
    let divisor = singleton(y)?;
    let mut result = Affine {
        jacobian: run.matrix_at(Role::Jacobian, 0, Extent::new(1, 2)?)?,
        offsets: run.matrix_at(Role::Offsets, 0, Extent::new(1, 1)?)?,
    };
    for (coordinate, entry) in proof.entries().iter().enumerate() {
        if entry.kernel() != (SensitivityKernel::QuantityQuotient { coordinate }) {
            return Err(StatisticsError::Contract.into());
        }
        let (candidate, truth) = if coordinate == 0 {
            (Q::ONE / y, singleton(Q::ONE)?.div(divisor, context)?)
        } else {
            (
                -x / (y * y),
                numerator.negated().div(divisor.square(context)?, context)?,
            )
        };
        result.jacobian.set(0, coordinate, candidate, truth)?;
    }
    Ok(result)
}
