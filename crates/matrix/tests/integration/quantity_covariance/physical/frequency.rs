// SPDX-License-Identifier: AGPL-3.0-only
//! Cyclic LC conversion retains the actual quantity composition and its complete correlated sources.

use super::{TestResult, resonance_at_with};
use crate::integration::pi_conversion::{exact_policy, oracle, pi};
use crate::reciprocal_testing::oracle::Rational;
use nrz_bindings::{Budget, Descriptor, Kind, Limits, Measure, Role};
use nrz_matrix::{Heap, JointLimits, MomentAssurance, Quantity};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Both formats retain LC model identity and rounded-point targets through the full-cycle relation.
/// ´test:integration:quantity-frequency-lc-composition´
#[test]
fn quantity_frequency_lc_composition() -> TestResult {
    lc::<f32>()?;
    lc::<f64>()
}

fn lc<P: Scalar + 'static>() -> TestResult {
    for (centers, covariance) in crate::omega_oracle::cases::<P>() {
        let (context, mut heap, values) = resonance_at_with(centers, covariance, true)?;
        let mut budget = Budget::new(Limits::default())?;
        let product = context.multiply(&mut heap, &values[0], &values[1], &mut budget)?;
        let root =
            context.square_root(&mut heap, &product, &mut Budget::new(Limits::default())?)?;
        let inverse =
            context.integer_power(&mut heap, &root, -1, &mut Budget::new(Limits::default())?)?;
        let omega = context.multiply(
            &mut heap,
            &inverse,
            &values[2],
            &mut Budget::new(Limits::default())?,
        )?;
        let retained = [
            product.view(&heap)?.region().get(0, 0)?,
            root.view(&heap)?.region().get(0, 0)?,
        ];
        let (squares, variance) =
            crate::omega_oracle::targets(centers, covariance, Some(retained))?;
        let circle = pi::<P>(
            &ArithmeticContext::new()?,
            &mut Budget::new(Limits::default())?,
        )?;
        let target = Descriptor::new(Kind::frequency()?, Measure::coherent(), Role::Difference)?;
        let cyclic = context.convert_frequency(
            &mut heap,
            &omega,
            &target,
            &circle,
            &exact_policy(),
            &mut Budget::new(Limits::default())?,
        )?;
        check(&mut heap, &omega, &cyclic, &squares, &variance)?;
        let period =
            context.integer_power(&mut heap, &cyclic, -1, &mut Budget::new(Limits::default())?)?;
        let restored =
            context.integer_power(&mut heap, &period, -1, &mut Budget::new(Limits::default())?)?;
        let relation = restored.descriptor().conversion_to(cyclic.descriptor())?;
        assert_eq!(
            relation.rational_scale()?,
            &nrz_bindings::readings::algebra::Rational::one()
        );
        assert_eq!(
            relation.rational_shift()?,
            &nrz_bindings::readings::algebra::Rational::zero()
        );
        let [center, _, error] = crate::integration::quantity_products::channels(&heap, &restored)?;
        let nominal = Rational::binary(omega.view(&heap)?.region().get(0, 0)?.to_f64())?;
        for endpoint in oracle::factor(1, 2, true, 56)? {
            assert!(nominal.multiply(&big(endpoint)?)?.covered(center, error)?);
        }
    }
    Ok(())
}

/// Every parameter oracle uses the retained chain's exact rational target, with an independent pi interval.
fn check<P: Scalar + 'static>(
    heap: &mut Heap,
    omega: &Quantity<P>,
    cyclic: &Quantity<P>,
    squares: &[Rational; 2],
    variance: &Rational,
) -> TestResult {
    let angular = heap
        .companion(omega.view(heap)?.bound().ok_or("bound")?)?
        .target();
    let target = heap
        .companion(cyclic.view(heap)?.bound().ok_or("bound")?)?
        .target();
    let original = heap.moments(heap.moment_companion(angular)?)?;
    let model = original.model();
    let sources = original.source_ids().to_vec();
    let moment = heap.moments(heap.moment_companion(target)?)?;
    assert_eq!(moment.model(), model);
    assert_eq!(moment.source_ids(), sources);
    assert_eq!(moment.source_ids().len(), 3);
    assert_eq!(moment.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moment.means().is_none());
    for endpoint in oracle::factor(1, 2, true, 56)? {
        let factor = big(endpoint)?;
        let scale_product = factor.multiply(&factor)?;
        assert!(variance.multiply(&scale_product)?.covered(
            moment.covariance().get(0, 0)?.to_f64(),
            moment.covariance().error(0, 0)?.to_f64()
        )?);
        let sensitivity = moment.sensitivity().ok_or("sensitivity")?;
        for (column, square) in squares.iter().enumerate() {
            crate::omega_oracle::negative_square(
                sensitivity.get(0, column)?.to_f64(),
                sensitivity.error(0, column)?.to_f64(),
                &square.multiply(&scale_product)?,
            )?;
        }
    }
    for (left, right) in [(angular, target), (target, angular)] {
        let cross = heap.materialize_cross_covariance(left, right, JointLimits::default())?;
        let moment = heap.moments(cross)?;
        for endpoint in oracle::factor(1, 2, true, 56)? {
            assert!(variance.multiply(&big(endpoint)?)?.covered(
                moment.covariance().get(0, 0)?.to_f64(),
                moment.covariance().error(0, 0)?.to_f64()
            )?);
        }
    }
    Ok(())
}

fn big(value: crate::kernel_oracle::Rational) -> TestResult<Rational> {
    Rational::new(
        i64::try_from(value.numerator)?,
        u64::try_from(value.denominator)?,
    )
}
