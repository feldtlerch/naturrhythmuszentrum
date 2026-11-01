// SPDX-License-Identifier: AGPL-3.0-only
//! The closed angular output reuses original root evidence and publishes all local parameters together.

mod costs;
mod definition;
mod lifecycle;
mod scope;
mod sources;

use super::{reciprocal_statistics as shared, root_support as inputs};
use crate::reading_testing::TestResult;
use crate::spectral_testing::Fixture;
use nrz_bindings::{Budget, readings::LcAngularFrequencyDefinition};
use nrz_matrix::{LcAngularFrequencyPublication, ReciprocalDerivativeContract};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// Original physical law, one shared exact derivative and a fresh output inventory.
struct Prepared {
    fixture: Fixture,
    derivative: ReciprocalDerivativeContract,
    definition: LcAngularFrequencyDefinition,
}

fn prepared() -> TestResult<Prepared> {
    prepared_with_budget(&mut shared::policy()?)
}

fn prepared_with_budget(budget: &mut Budget) -> TestResult<Prepared> {
    let mut fixture =
        Fixture::with_store_limits(["5", "1/10", "1/2000"], shared::policy()?.limits().clone())?;
    let derivative = shared::derivative(&mut fixture, budget)?;
    let graph = Rc::get_mut(&mut fixture.graph).ok_or("unique fixture graph")?;
    let definition = crate::angular_testing::prepare(graph, fixture.law.clone(), budget)?;
    Ok(Prepared {
        fixture,
        derivative,
        definition,
    })
}

fn publication<P: Scalar + 'static>(
    prepared: &Prepared,
    case: &inputs::Case<P>,
    budget: &mut Budget,
) -> TestResult<LcAngularFrequencyPublication<P>> {
    Ok(LcAngularFrequencyPublication::new(
        inputs::plan(&prepared.fixture, case, budget)?,
        prepared.definition.clone(),
        budget,
    )?)
}

/// ´test:integration:lc-angular-frequency-original-center-output´
#[test]
fn lc_angular_frequency_original_center_output() -> TestResult {
    let prepared = prepared()?;
    original::<f32>(&prepared)?;
    original::<f64>(&prepared)
}

fn original<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    for (centers, coefficients) in crate::omega_oracle::cases::<P>() {
        let mut case = sources::case(
            prepared,
            centers.map(|value| [value, P::ZERO, P::ZERO]),
            &[coefficients[0], coefficients[1], P::ZERO, coefficients[1]],
            &[P::ONE, coefficients[2]],
            nrz_matrix::CovarianceBlock::Known,
            &mut budget,
        )?;
        let plan = publication(prepared, &case, &mut budget)?;
        let context = ArithmeticContext::new()?;
        let output = case
            .heap
            .publish_lc_angular_frequency(&plan, &context, &mut budget)?;
        assert_eq!(
            output.definition().descriptor().kind(),
            &nrz_bindings::Kind::angular_frequency()?
        );
        assert_eq!(
            output.assurance(&case.heap, &context)?,
            nrz_matrix::MomentAssurance::LinearizedMoments
        );
        assert_eq!(output.operating_point(&case.heap, &context)?, centers);
        assert_eq!(output.readings().bindings().len(), 2);
        assert_eq!(output.readings().bindings()[0].companions().len(), 3);
        assert_eq!(
            output.source_model(&case.heap, &context)?.model().id(),
            case.heap.statistical(case.companion)?.model().id()
        );
        oracle(&output, &case.heap, centers, coefficients)?;
        assert!(
            output.readings().bindings()[0]
                .lc_angular_frequency()
                .is_some()
        );
        assert!(
            case.heap
                .publish_lc_angular_frequency(&plan, &context, &mut budget)
                .is_err()
        );
        case.heap.compact()?;
        oracle(&output, &case.heap, centers, coefficients)?;
    }
    Ok(())
}

/// Original physical-center rational targets are independent of the retained native rate schedule.
fn oracle<P: Scalar + 'static>(
    output: &nrz_matrix::PublishedLcAngularFrequency<P>,
    heap: &nrz_matrix::Heap,
    centers: [P; 2],
    coefficients: [P; 3],
) -> TestResult {
    let context = ArithmeticContext::new()?;
    let (squares, variance) = crate::omega_oracle::targets(centers, coefficients, None)?;
    let actual = output.covariance(heap, &context)?;
    assert!(variance.covered(actual.center.to_f64(), actual.error.to_f64())?);
    for (actual, expected) in output.jacobian(heap, &context)?.into_iter().zip(&squares) {
        crate::omega_oracle::negative_square(
            actual.center.to_f64(),
            actual.error.to_f64(),
            expected,
        )?;
    }
    Ok(())
}
