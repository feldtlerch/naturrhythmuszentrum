// SPDX-License-Identifier: AGPL-3.0-only
//! Each mutation and original-domain boundary retains its own typed cause.

use super::{
    CoordinateBound, Counts, Fixture, Interval, InvalidCode, Operator, Policy, Rational,
    TestResult, expression, invalid, ledger, number, power, prepared, request,
};

/// Reordering, stage understatement and both export obligations fail independently.
/// # Panics
/// Panics if a mutation changes or loses its own cause.
/// ´claim:cas:prepared-expm1-extra-mutations´
/// ´test:integration:prepared-expm1-extra-mutations´
#[test]
fn prepared_expm1_extra_mutations() -> TestResult {
    let mut f = Fixture::new()?;
    let r = request(&mut f, number(-1, 1)?, 40)?;
    let p = prepared(&mut f, &r)?;
    for (which, cause) in [
        (0, InvalidCode::Expm1Rounding),
        (1, InvalidCode::Expm1Rounding),
        (2, InvalidCode::Expm1Export),
        (3, InvalidCode::Expm1Export),
    ] {
        let mut proof = p.proof().clone();
        match which {
            0 => proof.stages.swap(0, 1),
            1 => proof.stages[0].error = Rational::zero(),
            2 => proof.radius_bits += 30,
            _ => proof.endpoint_ratio = Rational::zero(),
        }
        invalid(
            f.cas
                .check_expm1(
                    &r,
                    Policy::default(),
                    &proof,
                    &mut ledger()?,
                    &mut Counts::default(),
                )
                .expect_err("mutated obligation"),
            cause,
        );
    }
    let mut other = r;
    other.bits = 39;
    invalid(
        f.cas
            .check_expm1(
                &other,
                Policy::default(),
                p.proof(),
                &mut ledger()?,
                &mut Counts::default(),
            )
            .expect_err("target binding"),
        InvalidCode::Expm1Claim,
    );
    Ok(())
}

/// Immediate signed normal neighbours and both outside box faces isolate operand causes.
/// # Panics
/// Panics if an invalid endpoint is admitted or a valid normal endpoint is refused.
/// ´claim:cas:prepared-expm1-domain-neighbours´
/// ´test:integration:prepared-expm1-domain-neighbours´
#[test]
fn prepared_expm1_domain_neighbours() -> TestResult {
    let mut f = Fixture::new()?;
    let mut b = ledger()?;
    let minimum = power(-1022)?;
    let below = minimum.sub(&power(-1074)?, &mut b)?.negated(&mut b)?;
    let r = request(&mut f, below, 40)?;
    invalid(
        f.cas
            .prepare_expm1(&r, Policy::default(), &mut b, &mut Counts::default())
            .expect_err("below-normal neighbour"),
        InvalidCode::Expm1BelowNormal,
    );
    let r = request(&mut f, minimum.negated(&mut b)?, 40)?;
    prepared(&mut f, &r)?;
    for (lower, upper) in [
        (number(-2, 1)?.sub(&power(-51)?, &mut b)?, number(-1, 1)?),
        (number(-1, 1)?, minimum.clone()),
        (minimum.negated(&mut b)?, minimum),
    ] {
        let x = f.term(Operator::Free("x".into()), &[])?;
        let mut r = expression(&mut f, x.clone(), 40)?;
        r.original.coordinates.push(CoordinateBound {
            coordinate: x,
            interval: Interval::new(lower, upper, &mut b)?,
        });
        invalid(
            f.cas
                .prepare_expm1(&r, Policy::default(), &mut b, &mut Counts::default())
                .expect_err("outside endpoint"),
            InvalidCode::Expm1OutsideDomain,
        );
    }
    Ok(())
}
