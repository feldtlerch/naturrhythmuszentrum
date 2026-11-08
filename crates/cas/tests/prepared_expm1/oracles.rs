// SPDX-License-Identifier: AGPL-3.0-only
//! Exact point oracles use binary64 bit-order search and alternating partial sums.

use super::{
    Counts, Fixture, InvalidCode, Policy, Proof, Rational, TestResult, invalid, ledger, number,
    power, prepared, request,
};
use nrz_cas::limits::{Budget, Limits};
use std::cmp::Ordering;

/// Exact value of finite positive binary64 bits, without a floating operation.
fn value(bits: u64, b: &mut Budget) -> TestResult<Rational> {
    let field = (bits >> 52) & 2047;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if field == 0 {
        (fraction, -1074)
    } else {
        (fraction + (1_u64 << 52), i64::try_from(field)? - 1075)
    };
    Ok(Rational::from_ratio(i64::try_from(significand)?, 1, b)?
        .mul(&Rational::from_ratio(2, 1, b)?.pow(exponent, b)?, b)?)
}

/// Binary search over the ordered finite encodings, then compare exact distances.
fn nearest(q: &Rational, b: &mut Budget) -> TestResult<Rational> {
    let negative = q.is_negative();
    let magnitude = absolute(q, b)?;
    let mut low = 0;
    let mut high = 0x4030_0000_0000_0000_u64; // 16
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if value(middle, b)?.compare(&magnitude, b)? == Ordering::Greater {
            high = middle;
        } else {
            low = middle;
        }
    }
    let lower = value(low, b)?;
    let upper = value(high, b)?;
    let left = magnitude.sub(&lower, b)?;
    let right = upper.sub(&magnitude, b)?;
    let order = left.compare(&right, b)?;
    let chosen = if order == Ordering::Greater || (order == Ordering::Equal && low % 2 != 0) {
        upper
    } else {
        lower
    };
    Ok(if negative { chosen.negated(b)? } else { chosen })
}

/// Find the neighbouring exact binary64 value of an already rounded number.
/// # Panics
/// Panics if its caller supplies an unrounded value.
fn neighbour(q: &Rational, upward: bool, b: &mut Budget) -> TestResult<Rational> {
    if q.is_zero() {
        return Ok(if upward {
            power(-1074)?
        } else {
            power(-1074)?.negated(b)?
        });
    }
    let magnitude = absolute(q, b)?;
    let mut low = 0;
    let mut high = 0x4030_0000_0000_0000_u64;
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if value(middle, b)?.compare(&magnitude, b)? == Ordering::Greater {
            high = middle;
        } else {
            low = middle;
        }
    }
    assert_eq!(value(low, b)?, magnitude);
    let bits = if upward == q.is_negative() {
        low - 1
    } else {
        low + 1
    };
    let result = value(bits, b)?;
    Ok(if q.is_negative() {
        result.negated(b)?
    } else {
        result
    })
}

/// Exact alternating partial sums bracket exp(-a)-1; the next term bounds the remainder.
fn truth(a: &Rational, b: &mut Budget) -> TestResult<[Rational; 2]> {
    if a.is_zero() {
        return Ok([Rational::zero(), Rational::zero()]);
    }
    let negative = a.negated(b)?;
    let threshold = a.mul(&power(-160)?, b)?;
    let mut term = negative.clone();
    let mut sum = term.clone();
    for n in 1..192 {
        let next = term.mul(&negative, b)?.div(&number(n + 1, 1)?, b)?;
        let adjacent = sum.add(&next, b)?;
        if n >= 2 && absolute(&next, b)?.compare(&threshold, b)? != Ordering::Greater {
            return Ok(if next.is_negative() {
                [adjacent, sum]
            } else {
                [sum, adjacent]
            });
        }
        sum = adjacent;
        term = next;
    }
    Err("oracle did not reach its proved remainder threshold".into())
}

/// Exact absolute value for independent test arithmetic.
fn absolute(q: &Rational, b: &mut Budget) -> TestResult<Rational> {
    Ok(if q.is_negative() {
        q.negated(b)?
    } else {
        q.clone()
    })
}

/// Require an exact inequality rather than comparing rounded decimal output.
/// # Panics
/// Panics if the independently computed inequality fails.
fn le(a: &Rational, c: &Rational, b: &mut Budget) -> TestResult {
    assert_ne!(a.compare(c, b)?, Ordering::Greater);
    Ok(())
}

/// Evaluate a supplied schedule by independent exact rounding and endpoint-specific truth.
/// # Panics
/// Panics if a coefficient, operation, bound or output violates its independent oracle.
fn evaluate(x: &Rational, p: &Proof, b: &mut Budget) -> TestResult {
    let a = x.negated(b)?;
    let alpha = nearest(&a, b)?;
    let imported_error = absolute(&alpha.sub(&a, b)?, b)?;
    le(&imported_error, &a.mul(&p.import_ratio, b)?, b)?;
    if x == &number(-1, 3)? {
        assert_ne!(alpha, a);
    }
    let oracle = truth(&a, b)?;
    le(&oracle[1].sub(&oracle[0], b)?, &power(-120)?, b)?;
    if a.is_zero() {
        assert_eq!(oracle, [Rational::zero(), Rational::zero()]);
        return Ok(());
    }
    let mut factorial = 1_u128;
    let mut ideals = Vec::new();
    for n in 1..=p.degree {
        factorial = factorial
            .checked_mul(u128::from(n))
            .ok_or("oracle factorial")?;
        let exact = Rational::one().div(&Rational::parse(&factorial.to_string(), b)?, b)?;
        assert_eq!(
            nearest(&exact, b)?,
            p.coefficients[n as usize - 1].exact(b)?
        );
        ideals.push(exact);
    }
    let last = ideals.len() - 1;
    let mut actual = p.coefficients[last].exact(b)?;
    let mut ideal = ideals[last].clone();
    let negative = alpha.negated(b)?;
    let mut cursor = 0;
    for index in (0..last).rev() {
        actual = nearest(&actual.mul(&negative, b)?, b)?;
        ideal = ideal.mul(&negative, b)?;
        le(
            &absolute(&actual.sub(&ideal, b)?, b)?,
            &p.stages[cursor].error,
            b,
        )?;
        cursor += 1;
        actual = nearest(&actual.add(&p.coefficients[index].exact(b)?, b)?, b)?;
        ideal = ideal.add(&ideals[index], b)?;
        le(
            &absolute(&actual.sub(&ideal, b)?, b)?,
            &p.stages[cursor].error,
            b,
        )?;
        cursor += 1;
    }
    actual = nearest(&actual.mul(&negative, b)?, b)?;
    ideal = ideal.mul(&negative, b)?;
    le(
        &absolute(&actual.sub(&ideal, b)?, b)?,
        &p.stages[cursor].error,
        b,
    )?;
    // Reconstruct the first omitted term at a=2 by successive terms, not factorial/power formulas.
    let mut omitted = number(2, 1)?;
    for n in 2..=p.degree + 1 {
        omitted = omitted
            .mul(&number(2, 1)?, b)?
            .div(&number(i64::from(n), 1)?, b)?;
    }
    assert_eq!(omitted, p.absolute_tail);
    assert_eq!(omitted.mul(&number(3, 2)?, b)?, p.relative_tail);
    let nominal = a.mul(&p.central_ratio, b)?;
    for endpoint in &oracle {
        le(&absolute(&actual.sub(endpoint, b)?, b)?, &nominal, b)?;
    }
    let radius = neighbour(
        &nearest(&alpha.mul(&power(-i64::from(p.radius_bits))?, b)?, b)?,
        true,
        b,
    )?;
    let lower = neighbour(&nearest(&actual.sub(&radius, b)?, b)?, false, b)?;
    let upper = neighbour(&nearest(&actual.add(&radius, b)?, b)?, true, b)?;
    le(&lower, &oracle[0], b)?;
    le(&oracle[1], &upper, b)?;
    let magnitude_lower = oracle[1].negated(b)?;
    for slack in [oracle[1].sub(&lower, b)?, upper.sub(&oracle[0], b)?] {
        le(&slack, &magnitude_lower.mul(&p.endpoint_ratio, b)?, b)?;
        le(&slack, &magnitude_lower.mul(&power(-40)?, b)?, b)?;
    }
    if a == power(-1022)? {
        assert_eq!(oracle[0].compare(&a.negated(b)?, b)?, Ordering::Greater);
        le(&absolute(&oracle[0], b)?, &power(-1022)?, b)?;
    }
    Ok(())
}

/// Endpoint-specific exact oracles check imports, coefficients, native steps and exports.
/// # Errors
/// Returns exact oracle or preparation failures.
/// # Panics
/// Panics if a proved bound misses an independently computed value or mutation cause.
/// ´claim:cas:prepared-expm1-independent-oracles´
/// ´test:integration:prepared-expm1-independent-oracles´
#[test]
fn prepared_expm1_independent_oracles() -> TestResult {
    let mut f = Fixture::new()?;
    let mut b = Budget::new(Limits {
        work: 20_000_000_000,
        arithmetic_steps: 50_000_000,
        component_bits: 32_768,
        intermediate_bits: 131_072,
        integer_exponent: 16_384,
        ..Limits::default()
    })?;
    for x in [
        number(-1, 3)?,
        power(-1022)?.negated(&mut b)?,
        number(-2, 1)?,
        Rational::zero(),
        power(-40)?.negated(&mut b)?,
        number(-1, 1)?,
    ] {
        let r = request(&mut f, x.clone(), 40)?;
        let p = prepared(&mut f, &r)?;
        evaluate(&x, p.proof(), &mut b)?;
        if x == number(-1, 3)? {
            let mut wrong = p.proof().clone();
            wrong.import_ratio = Rational::zero();
            invalid(
                f.cas
                    .check_expm1(
                        &r,
                        Policy::default(),
                        &wrong,
                        &mut ledger()?,
                        &mut Counts::default(),
                    )
                    .expect_err("rounded import"),
                InvalidCode::Expm1Import,
            );
        }
    }
    Ok(())
}
