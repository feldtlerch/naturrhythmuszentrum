// SPDX-License-Identifier: AGPL-3.0-only
//! Independent Machin series uses bounded arbitrary-precision fractions, never native truth.

use super::{TestResult, ledger, number};
use crate::kernel_oracle::Rational as Binary;
use nrz_bindings::readings::algebra::Rational;
use nrz_scalar::{Enclosure, Scalar};
use std::cmp::Ordering;

/// Prepared once per fixture and reused for every endpoint and both numerical formats.
pub(super) struct Pi {
    /// Independent alternating-series lower and upper bounds for mathematical pi.
    bounds: [Rational; 2],
}

impl Pi {
    /// Exact Machin identity `pi=16*atan(1/5)-4*atan(1/239)`.
    pub(super) fn new() -> TestResult<Self> {
        let small = atan(5, 18)?;
        let tiny = atan(239, 6)?;
        let sixteen = number("16")?;
        let four = number("4")?;
        Ok(Self {
            bounds: [
                small[0]
                    .mul(&sixteen, &mut ledger()?)?
                    .sub(&tiny[1].mul(&four, &mut ledger()?)?, &mut ledger()?)?,
                small[1]
                    .mul(&sixteen, &mut ledger()?)?
                    .sub(&tiny[0].mul(&four, &mut ledger()?)?, &mut ledger()?)?,
            ],
        })
    }

    /// Every comparison is exact; rational arithmetic refuses its resource limit.
    pub(super) fn contains<P: Scalar>(
        &self,
        value: Enclosure<P>,
        numerator: i64,
        denominator: u64,
    ) -> TestResult {
        self.affine(
            value,
            &Rational::from_ratio(numerator, denominator, &mut ledger()?)?,
            &Rational::zero(),
        )
    }

    /// Expected coefficients are fixture literals independent of the production projection.
    pub(super) fn affine<P: Scalar>(
        &self,
        value: Enclosure<P>,
        coefficient: &Rational,
        offset: &Rational,
    ) -> TestResult {
        let lower = binary(value.lower().get().to_f64())?;
        let upper = binary(value.upper().get().to_f64())?;
        for pi in &self.bounds {
            let expected = pi
                .mul(coefficient, &mut ledger()?)?
                .add(offset, &mut ledger()?)?;
            assert_ne!(lower.compare(&expected, &mut ledger()?)?, Ordering::Greater);
            assert_ne!(upper.compare(&expected, &mut ledger()?)?, Ordering::Less);
        }
        Ok(())
    }
}

/// Alternating bounds use one more exact term after an even partial sum.
fn atan(denominator: u64, terms: i64) -> TestResult<[Rational; 2]> {
    let x = Rational::from_ratio(1, denominator, &mut ledger()?)?;
    let square = x.mul(&x, &mut ledger()?)?;
    let mut power = x;
    let mut sum = Rational::zero();
    for index in 0..terms {
        let divisor = Rational::from_ratio(2 * index + 1, 1, &mut ledger()?)?;
        let term = power.div(&divisor, &mut ledger()?)?;
        sum = if index % 2 == 0 {
            sum.add(&term, &mut ledger()?)?
        } else {
            sum.sub(&term, &mut ledger()?)?
        };
        power = power.mul(&square, &mut ledger()?)?;
    }
    let divisor = Rational::from_ratio(2 * terms + 1, 1, &mut ledger()?)?;
    let upper = sum.add(&power.div(&divisor, &mut ledger()?)?, &mut ledger()?)?;
    Ok([sum, upper])
}

/// Bit decoding refuses an out-of-range small fraction, before arbitrary-precision arithmetic.
fn binary(value: f64) -> TestResult<Rational> {
    let rational = Binary::binary(value)?;
    number(&format!("{}/{}", rational.numerator, rational.denominator))
}
