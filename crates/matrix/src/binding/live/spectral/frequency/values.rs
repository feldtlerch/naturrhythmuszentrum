// SPDX-License-Identifier: AGPL-3.0-only
//! Linear full-cycle transport reuses certified affine arithmetic and both covariance-axis factors.

use super::{AT, Error, Evidence, angular, plan::Plan};
use crate::{Heap, OperationError, SpectralValue};
use nrz_bindings::Budget;
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// No root is evaluated here: the original native evidence supplies every angular parameter.
pub(super) fn prepare<P: Scalar + 'static>(
    heap: &Heap,
    plan: &Plan<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Evidence<P>, Error> {
    if plan.map.revision != context.revision()
        || plan.pi.arithmetic_revision() != context.revision()
    {
        return Err(Error::Identity);
    }
    let (angular, rate, covariance) =
        angular::pack(heap, plan.evaluation.clone(), context, budget)?;
    // Prepay the fixed affine, two partial, two variance-axis and cross-axis operations.
    budget.work(12, AT)?;
    let converted = plan
        .map
        .evaluate(rate.center, rate.total, context)
        .map_err(OperationError::from)?;
    let cyclic = SpectralValue {
        center: converted.center,
        total: converted.total,
        error: plan
            .map
            .propagate_error(rate.error, converted.evaluation, context)
            .map_err(OperationError::from)?,
    };
    let factor = plan.map.scale.enclosure();
    let partials = angular
        .evaluation
        .root_partials(heap, context)?
        .ok_or(Error::Identity)?[1];
    let jacobian = [
        angular::parameter(partials[0].mul(factor, context)?, context)?,
        angular::parameter(partials[1].mul(factor, context)?, context)?,
    ];
    let variance = angular
        .evaluation
        .root_covariance(heap, context)?
        .ok_or(Error::Identity)?[1][1];
    let cross = variance.mul(factor, context)?;
    let covariance = [
        covariance,
        angular::parameter(cross.mul(factor, context)?, context)?,
        angular::parameter(cross, context)?,
    ];
    Ok(Evidence {
        angular,
        values: [rate, cyclic],
        jacobian,
        covariance,
        map: Rc::clone(&plan.map),
        pi: plan.pi.clone(),
    })
}

/// The complete scalar region order is sealed by the exact definition and publication adapters.
pub(super) const fn entries<P: Scalar>(evidence: &Evidence<P>) -> [P; 12] {
    let [angular, cyclic] = evidence.values;
    let [omega, frequency, cross] = evidence.covariance;
    [
        angular.center,
        angular.total,
        angular.error,
        omega.center,
        omega.error,
        cyclic.center,
        cyclic.total,
        cyclic.error,
        frequency.center,
        frequency.error,
        cross.center,
        cross.error,
    ]
}
