// SPDX-License-Identifier: AGPL-3.0-only
//! Both native formats retain the rest question, with independent exact endpoint comparisons.

use super::support::{TestResult, input, ledger, number};
use nrz_bindings::readings::algebra::{Location, Rational};
use nrz_bindings::{
    Budget, LcCertainty, LcDeterminacy, LcQuantity as Q, LcQuiescentCover, LcQuiescentOutcome,
    LcRelation, Limits, LimitsOverride,
};
use nrz_matrix::LcQuiescentProjection;
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, Scalar};
use std::cmp::Ordering;

/// One timestamp is retained beside explicitly undefined physical phase.
const SAMPLE: usize = 0;

/// Original repeated inputs are independent of the hand-derived endpoint list.
fn cover(observed: bool) -> TestResult<LcQuiescentCover> {
    let mut inputs = vec![
        input(Q::Inductance, ["1/4", "1"])?,
        input(Q::Capacitance, ["1/4", "1"])?,
        input(Q::Energy, ["-1", "0"])?,
        input(Q::Energy, ["0", "1"])?,
    ];
    if observed {
        inputs.push(input(Q::Time(SAMPLE), ["-1/3", "2/3"])?);
        inputs.push(input(Q::Time(SAMPLE), ["0", "1"])?);
    }
    Ok(LcRelation::new(
        "native rest epoch".into(),
        usize::from(observed),
        inputs,
        &mut ledger()?,
    )?
    .quiescent_cover(&mut ledger()?)?)
}

/// Rational endpoint truth is decoded and compared using bounded arbitrary-precision arithmetic.
fn contains<P: Scalar>(value: Enclosure<P>, expected: &Rational) -> TestResult {
    let decode = |value| -> TestResult<Rational> {
        let value = crate::kernel_oracle::Rational::binary(value)?;
        number(&format!("{}/{}", value.numerator, value.denominator))
    };
    assert_ne!(
        decode(value.lower().get().to_f64())?.compare(expected, &mut ledger()?)?,
        Ordering::Greater
    );
    assert_ne!(
        decode(value.upper().get().to_f64())?.compare(expected, &mut ledger()?)?,
        Ordering::Less
    );
    Ok(())
}

/// Both formats enclose each complete scalar image and retain undefined phase/ratio separately.
/// ´test:integration:lc-relation-quiescent-native-queries´
#[test]
fn lc_relation_quiescent_native_queries() -> TestResult {
    for observed in [false, true] {
        let cover = cover(observed)?;
        render::<f32>(&cover)?;
        render::<f64>(&cover)?;
    }
    Ok(())
}

/// Literal endpoints are independent of the production rectangle projection formulas.
fn render<P: Scalar>(cover: &LcQuiescentCover) -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut expected = vec![
        (Q::Inductance, ["1/4", "1"]),
        (Q::Capacitance, ["1/4", "1"]),
        (Q::Product, ["1/16", "1"]),
        (Q::NaturalTime, ["1/4", "1"]),
        (Q::Rate, ["1", "4"]),
        (Q::AngularFrequency, ["1", "4"]),
        (Q::VoltageAmplitude, ["0", "0"]),
        (Q::CurrentAmplitude, ["0", "0"]),
        (Q::Energy, ["0", "0"]),
    ];
    let mut undefined = vec![Q::InitialPhase, Q::AmplitudeRatio];
    if cover.relation().observations() == 1 {
        expected.push((Q::Time(SAMPLE), ["0", "2/3"]));
        undefined.push(Q::Phase(SAMPLE));
    }
    for (quantity, endpoints) in expected {
        let exact = cover.query(quantity, &mut ledger()?)?;
        let native = LcQuiescentProjection::<P>::new(
            exact,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(
            native
                .exact()
                .cover()
                .relation()
                .shares_allocation(cover.relation())
        );
        assert_eq!(native.exact().quantity(), quantity);
        assert_eq!(
            native.exact().determinacy(),
            Some(LcDeterminacy::DeterminedFinite)
        );
        assert_eq!(native.exact().certainty(), LcCertainty::Certified);
        assert_eq!(native.arithmetic_revision(), context.revision());
        assert!(native.owned_bytes() > 0);
        let scalar = native.scalar().ok_or("defined native scalar")?;
        assert_eq!(scalar.quantity(), quantity);
        for (enclosure, expected) in scalar.endpoints().iter().zip(endpoints) {
            contains(
                enclosure.ok_or("finite exact endpoint")?,
                &number(expected)?,
            )?;
            contains(
                scalar.enclosure().ok_or("bounded scalar")?,
                &number(expected)?,
            )?;
        }
    }
    for quantity in undefined {
        let exact = cover.query(quantity, &mut ledger()?)?;
        let native = LcQuiescentProjection::<P>::new(
            exact,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(native.scalar().is_none());
        assert_eq!(native.exact().determinacy(), None);
        assert!(matches!(
            native.exact().outcome(),
            LcQuiescentOutcome::UndefinedPhase | LcQuiescentOutcome::UndefinedRatio
        ));
    }
    Ok(())
}

/// Native scalar and undefined publications have exact cumulative N/N-1 Work boundaries.
/// ´test:integration:lc-relation-quiescent-native-work´
#[test]
fn lc_relation_quiescent_native_work() -> TestResult {
    work::<f32>()?;
    work::<f64>()
}

/// Exhausted publication leaves the original query and full witness reusable.
fn work<P: Scalar>() -> TestResult {
    const PREFIX: u64 = 17;
    let cover = cover(true)?;
    let context = ArithmeticContext::new()?;
    for quantity in [Q::Time(SAMPLE), Q::Phase(SAMPLE), Q::AmplitudeRatio] {
        let exact = cover.query(quantity, &mut ledger()?)?;
        let mut measured = ledger()?;
        let native = LcQuiescentProjection::<P>::new(
            exact.clone(),
            LiteralLimits::default(),
            &context,
            &mut measured,
        )?;
        let used = measured.usage().work;
        assert!(used > 0);
        assert_eq!(native.usage()[1].work - native.usage()[0].work, used);
        for (limit, succeeds) in [(used + PREFIX, true), (used + PREFIX - 1, false)] {
            let limits = Limits::default().resolve(&LimitsOverride {
                work: Some(limit),
                ..LimitsOverride::default()
            })?;
            let mut budget = Budget::new(limits)?;
            budget.work(PREFIX, Location::new("quiescent native prefix"))?;
            assert_eq!(
                LcQuiescentProjection::<P>::new(
                    exact.clone(),
                    LiteralLimits::default(),
                    &context,
                    &mut budget
                )
                .is_ok(),
                succeeds
            );
        }
        cover
            .relation()
            .check_witness(cover.feasible().values().clone(), &mut ledger()?)?;
    }
    Ok(())
}
