// SPDX-License-Identifier: AGPL-3.0-only
//! Native root statistics enclose the exact law at the original represented physical centers.

use super::{reciprocal_statistics as shared, selected_joint};
use crate::reading_testing::TestResult;
use crate::spectral_testing::Fixture;
use nrz_bindings::Budget;
use nrz_matrix::{
    CompanionSetDraft, CovarianceBlock, JointLimits, JointModel, ReciprocalDerivativeContract,
    Source, SourceFactor,
};
use nrz_scalar::{ArithmeticContext, Scalar};

/// ´test:integration:lc-omega-route-measurement-native´
#[test]
fn lc_omega_route_measurement_native() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_store_limits(["5", "1/10", "1/2000"], budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    measure::<f32>(&fixture, &contract, &mut budget)?;
    measure::<f64>(&fixture, &contract, &mut budget)
}

fn measure<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
    budget: &mut Budget,
) -> TestResult {
    for (centers, covariance) in crate::omega_oracle::cases::<P>() {
        let (mut heap, original) =
            fixture.import_with_budget(centers.map(|center| [center, P::ZERO, P::ZERO]), budget)?;
        let selected = original
            .iter()
            .map(|source| heap.select_reading(source, budget))
            .collect::<Result<Vec<_>, _>>()?;
        let handles = selected_joint::capture_handles(&mut heap, &selected)?;
        let sources = selected
            .iter()
            .map(|value| Source::new(value.symbol().descriptor(), 1))
            .collect::<Result<Vec<_>, _>>()?;
        let limits = JointLimits::default();
        let context = ArithmeticContext::new()?;
        let model = JointModel::from_factor(
            &sources,
            SourceFactor::new(
                2,
                &[covariance[0], covariance[1], P::ZERO, covariance[1]],
                &[P::ONE, covariance[2]],
                limits,
            )?,
            &[const { CovarianceBlock::Known }; 4],
            limits,
        )?;
        let companion = heap.publish_joint(
            CompanionSetDraft::new(
                model,
                selected_joint::capture_targets(&heap, handles)?,
                [vec![0], vec![1]],
            ),
            limits,
        )?;
        let derivative =
            heap.evaluate_reciprocal_derivative(contract, &original, &context, budget)?;
        let plan = fixture
            .plan(original, budget)?
            .with_reciprocal_statistics(&heap, derivative, companion, budget)?;
        let evaluation = heap.evaluate_lc_spectrum(&plan, &context, budget)?;
        let partials = evaluation
            .root_partials(&heap, &context)?
            .ok_or("partials")?[1];
        let variance = evaluation
            .root_covariance(&heap, &context)?
            .ok_or("covariance")?[1][1];
        let (squares, expected) = crate::omega_oracle::targets(centers, covariance, None)?;
        for (partial, square) in partials.into_iter().zip(&squares) {
            let low = crate::reciprocal_testing::oracle::Rational::binary(
                -partial.upper().get().to_f64(),
            )?;
            let high = crate::reciprocal_testing::oracle::Rational::binary(
                -partial.lower().get().to_f64(),
            )?;
            assert!(partial.upper().get() < P::ZERO);
            assert!(low.multiply(&low)?.at_most(square)?);
            assert!(square.at_most(&high.multiply(&high)?)?);
        }
        assert!(expected.enclosed(
            variance.lower().get().to_f64(),
            variance.upper().get().to_f64()
        )?);
        assert_eq!(
            evaluation.moment_assurance(),
            Some(nrz_matrix::MomentAssurance::LinearizedMoments)
        );
    }
    Ok(())
}

/// ´test:integration:lc-omega-exact-physical-reference´
#[test]
fn lc_omega_exact_physical_reference() -> TestResult {
    use crate::omega_oracle::equal;
    use crate::reciprocal_testing::oracle::Rational;
    let left = Rational::new(1, 40)?;
    let right = Rational::new(1, 100_000)?;
    let rate_squared = Rational::new(1, 1)?.divide(&left.multiply(&right)?)?;
    assert!(equal(&rate_squared, &Rational::new(4_000_000, 1)?)?);
    let relative = Rational::new(1, 10_000)?
        .add(&Rational::new(1, 2_500)?)?
        .add(&Rational::new(1, 5_000)?)?;
    let variance = rate_squared
        .multiply(&relative)?
        .divide(&Rational::new(4, 1)?)?;
    assert!(equal(&variance, &Rational::new(700, 1)?)?);
    for (input, target) in [(&left, 40_000), (&right, 100_000_000)] {
        let derivative =
            Rational::new(-2_000, 1)?.divide(&Rational::new(2, 1)?.multiply(input)?)?;
        assert!(equal(&derivative, &Rational::new(-target, 1)?)?);
    }
    Ok(())
}
