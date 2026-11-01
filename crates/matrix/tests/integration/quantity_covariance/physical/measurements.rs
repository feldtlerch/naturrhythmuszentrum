// SPDX-License-Identifier: AGPL-3.0-only
//! Measurements distinguish the original physical center from retained rounded intermediate points.

use super::{TestResult, resonance_at};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// ´test:integration:lc-omega-route-measurement-quantity´
#[test]
fn lc_omega_route_measurement_quantity() -> TestResult {
    measure::<f32>()?;
    measure::<f64>()
}

fn measure<P: Scalar + 'static>() -> TestResult {
    for (index, (centers, covariance)) in crate::omega_oracle::cases::<P>().into_iter().enumerate()
    {
        let (context, mut heap, values) = resonance_at(centers, covariance)?;
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
        let product_center = product.view(&heap)?.region().get(0, 0)?;
        let root_center = root.view(&heap)?.region().get(0, 0)?;
        let target = heap
            .companion(omega.view(&heap)?.bound().ok_or("bound")?)?
            .target();
        let moments = heap.moments(heap.moment_companion(target)?)?;
        let sensitivity = moments.sensitivity().ok_or("sensitivity")?;
        let slopes = [
            (
                sensitivity.get(0, 0)?.to_f64(),
                sensitivity.error(0, 0)?.to_f64(),
            ),
            (
                sensitivity.get(0, 1)?.to_f64(),
                sensitivity.error(0, 1)?.to_f64(),
            ),
        ];
        let (squares, expected) =
            crate::omega_oracle::targets(centers, covariance, Some([product_center, root_center]))?;
        for ((center, error), square) in slopes.iter().zip(&squares) {
            crate::omega_oracle::negative_square(*center, *error, square)?;
        }
        assert!(expected.covered(
            moments.covariance().get(0, 0)?.to_f64(),
            moments.covariance().error(0, 0)?.to_f64()
        )?);
        let (direct_squares, direct) = crate::omega_oracle::targets(centers, covariance, None)?;
        report::<P>(index, &squares, &direct_squares, &expected, &direct)?;
        assert_eq!(crate::omega_oracle::equal(&expected, &direct)?, index == 0);
        assert_eq!(
            moments.assurance(),
            nrz_matrix::MomentAssurance::LinearizedMoments
        );
        assert!(moments.means().is_none());
    }
    Ok(())
}

/// Exact rational differences quantify the two policies; decimal derivative gaps are reported estimates.
fn report<P: Scalar>(
    index: usize,
    chain: &[crate::reciprocal_testing::oracle::Rational; 2],
    direct: &[crate::reciprocal_testing::oracle::Rational; 2],
    chain_variance: &crate::reciprocal_testing::oracle::Rational,
    direct_variance: &crate::reciprocal_testing::oracle::Rational,
) -> TestResult {
    use crate::reciprocal_testing::oracle::Rational;
    use std::io::Write;
    let minus_one = Rational::new(-1, 1)?;
    let ratio_delta = chain[0]
        .divide(&direct[0])?
        .add(&minus_one)?
        .binary64_bounds()?;
    let variance_delta = chain_variance
        .add(&direct_variance.multiply(&minus_one)?)?
        .binary64_bounds()?;
    let delta = f64::midpoint(ratio_delta[0], ratio_delta[1]);
    let relative = delta / ((1.0 + delta).sqrt() + 1.0);
    let mut partial_gaps = Vec::new();
    for square in direct {
        let bounds = square.binary64_bounds()?;
        partial_gaps.push(-bounds[0].sqrt() * relative);
    }
    writeln!(
        std::io::stdout().lock(),
        "omega target gap {:?} case={index}: (J_chain/J_direct)^2-1 in {ratio_delta:?}; U_chain-U_direct in {variance_delta:?}; approximate J_chain-J_direct={partial_gaps:?}",
        P::PRECISION
    )?;
    Ok(())
}
