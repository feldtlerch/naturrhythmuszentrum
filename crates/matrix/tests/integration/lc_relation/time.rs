// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact zero-time endpoints, unbounded domains and assurance controls.

pub(super) mod oracle;
mod refusals;

use super::support::{self, TestResult, input, integer, ledger, number};
use crate::kernel_oracle::Rational as Oracle;
use nrz_bindings::{LcQuantity as Q, LcTailOutcome, LcTailStatus, LcTimeTail, LcZeroTimes};
use nrz_matrix::LcTimeProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Original rate rectangle is distinct from the correlated measurement fixture.
pub(super) fn cover(rate: [&str; 2]) -> TestResult<LcZeroTimes> {
    Ok(support::relation(vec![
        input(Q::Rate, rate)?,
        input(Q::InitialPhase, ["0", "0"])?,
    ])?
    .zero_times(&mut ledger()?)?)
}

/// Only the successful checked outcome is extracted in these fixture tests.
pub(super) fn tail(cover: &LcZeroTimes, first: i64) -> TestResult<LcTimeTail> {
    match cover.overlap_tail(integer(first)?, &mut ledger()?) {
        LcTailOutcome::Certified(tail) => Ok(*tail),
        LcTailOutcome::SeparatedInfinite(_) | LcTailOutcome::Undecided { .. } => {
            Err("expected checked overlap".into())
        }
    }
}

/// Complete numbered boxes, exact first overlap and both tail strengths match the design fixture.
/// ´test:integration:lc-relation-zero-boxes´
#[test]
fn lc_relation_zero_boxes() -> TestResult {
    let oracle = oracle::Pi::new()?;
    zero_boxes::<f32>(&oracle)?;
    zero_boxes::<f64>(&oracle)
}

/// Expected endpoint fractions are fixed independent values, not production-formula calls.
fn zero_boxes<P: Scalar + 'static>(oracle: &oracle::Pi) -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    let exact = cover(["5800/3", "6200/3"])?;
    let native = LcTimeProjection::new(exact.clone(), pi.clone(), &mut ledger()?)?;
    assert!(native.pi().same_region(&pi));
    assert_eq!(native.cover().relation().epoch(), "forward epoch");
    assert_eq!(native.cover().descriptor(), &Q::NaturalTime.descriptor()?);
    assert_eq!(native.cover().family().last_index(), None);
    for (index, expected) in [
        (0, [(0, 1), (0, 1)]),
        (1, [(3, 3100), (3, 2900)]),
        (14, [(21, 1550), (21, 1450)]),
        (15, [(9, 620), (9, 580)]),
        (100, [(3, 31), (3, 29)]),
    ] {
        let member = native.member(
            integer(index)?,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert_eq!(member.exact().index(), &integer(index)?);
        for (value, (top, bottom)) in member.endpoints().iter().zip(expected) {
            oracle.contains(*value, top, bottom)?;
        }
    }
    let covering = tail(&exact, 15)?;
    assert_eq!(covering.status(), LcTailStatus::CoverageOnly);
    assert!(covering.existence_witness().is_none());
    assert_eq!(covering.mathematical().first_index(), &integer(15)?);
    assert_eq!(covering.mathematical().last_separate_index(), &integer(14)?);
    assert_eq!(
        covering.mathematical().overlap_products(),
        &[number("5600/3")?, number("2000")?]
    );
    let feasible = covering.certify_feasibility(&mut ledger()?)?;
    assert_eq!(feasible.status(), LcTailStatus::FeasibleEverywhere);
    assert!(feasible.existence_witness().is_some());
    for exact_tail in [covering, feasible] {
        let tail = native.tail(
            exact_tail,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        oracle.contains(tail.start(), 9, 620)?;
    }
    Ok(())
}

/// Zero spread keeps arbitrarily large exact indices and never manufactures a finite tail.
/// ´test:integration:lc-relation-zero-spread-family´
#[test]
fn lc_relation_zero_spread_family() -> TestResult {
    let exact = cover(["2000", "2000"])?;
    let LcTailOutcome::SeparatedInfinite(retained) =
        exact.overlap_tail(integer(1)?, &mut ledger()?)
    else {
        return Err("infinite separated family".into());
    };
    assert!(retained.shares_allocation(&exact));
    let index = number("100000000000000000000000000000000000000000000000000")?;
    let member = retained.member(index.clone(), &mut ledger()?)?;
    assert_eq!(member.index(), &index);
    assert_eq!(
        member.endpoints()[0].coefficients(),
        member.endpoints()[1].coefficients()
    );
    assert_eq!(retained.family().last_index(), None);
    Ok(())
}

/// Rational parameter witnesses and exact endpoint inequalities are independent of native overlap.
/// ´test:integration:lc-relation-zero-time-parametric-witnesses´
#[test]
fn lc_relation_zero_time_parametric_witnesses() -> TestResult {
    let exact = cover(["5800/3", "6200/3"])?;
    for rate in ["5800/3", "1950", "2000", "2050", "6200/3"] {
        let witness = exact.witness(number(rate)?, &mut ledger()?)?;
        assert!(witness.relation().shares_allocation(exact.relation()));
        assert_eq!(
            witness
                .coordinate(Q::Rate)
                .and_then(nrz_bindings::ExactConstant::as_rational),
            Some(&number(rate)?)
        );
    }
    assert!(Oracle::new(5600, 3)?.at_most(Oracle::new(5800, 3)?)?);
    assert!(Oracle::new(5800, 3)?.at_most(Oracle::new(2000, 1)?)?);
    Ok(())
}
