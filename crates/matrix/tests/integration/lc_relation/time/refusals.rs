// SPDX-License-Identifier: AGPL-3.0-only
//! Failed overlap or numerical work preserves the complete certified question.

use super::{TestResult, cover, integer, ledger, number, tail};
use nrz_bindings::{Budget, LcTailOutcome, Limits, LimitsOverride};
use nrz_matrix::LcTimeProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits};

/// Wrong overlap proposals are undecided with all original numbered boxes preserved.
/// ´test:integration:lc-relation-zero-time-refusals´
#[test]
fn lc_relation_zero_time_refusals() -> TestResult {
    let exact = cover(["5800/3", "6200/3"])?;
    for proposed in ["0", "14", "16", "-1", "29/2"] {
        let LcTailOutcome::Undecided {
            cover: retained, ..
        } = exact.overlap_tail(number(proposed)?, &mut ledger()?)
        else {
            return Err("false overlap accepted".into());
        };
        assert!(retained.shares_allocation(&exact));
        assert!(retained.member(integer(100)?, &mut ledger()?).is_ok());
    }
    for invalid in ["-1", "1/2"] {
        assert!(exact.member(number(invalid)?, &mut ledger()?).is_err());
    }
    for invalid in [["0", "1"], ["-1", "1"]] {
        assert!(cover(invalid).is_err());
    }
    let mut inputs = vec![super::super::support::input(
        nrz_bindings::LcQuantity::Rate,
        ["1", "2"],
    )?];
    assert!(
        super::super::support::relation(inputs.clone())?
            .zero_times(&mut ledger()?)
            .is_err()
    );
    inputs.push(super::super::support::input(
        nrz_bindings::LcQuantity::InitialPhase,
        ["0", "1"],
    )?);
    assert!(
        super::super::support::relation(inputs)?
            .zero_times(&mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Exact cumulative boundaries and foreign question identity cannot erase earlier coverage.
/// ´test:integration:lc-relation-zero-time-resource-preservation´
#[test]
fn lc_relation_zero_time_resource_preservation() -> TestResult {
    let exact = cover(["5800/3", "6200/3"])?;
    let limits = Limits::default().resolve(&LimitsOverride {
        work: Some(1),
        ..LimitsOverride::default()
    })?;
    let mut exhausted = Budget::new(limits)?;
    let LcTailOutcome::Undecided {
        cover: retained, ..
    } = exact.overlap_tail(integer(15)?, &mut exhausted)
    else {
        return Err("exhaustion asserted overlap".into());
    };
    assert!(retained.shares_allocation(&exact));
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::pi_conversion::pi::<f64>(&context, &mut ledger()?)?;
    let native = LcTimeProjection::new(exact.clone(), pi, &mut ledger()?)?;
    let mut measured = ledger()?;
    native.member(
        integer(14)?,
        LiteralLimits::default(),
        &context,
        &mut measured,
    )?;
    let work = measured.usage().work;
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("time query prefix"),
        )?;
        assert_eq!(
            native
                .member(
                    integer(14)?,
                    LiteralLimits::default(),
                    &context,
                    &mut budget
                )
                .is_ok(),
            succeeds
        );
        assert!(budget.usage().work >= 17);
        assert!(native.cover().shares_allocation(&exact));
    }
    let foreign = tail(&cover(["5800/3", "6200/3"])?, 15)?;
    assert!(
        native
            .tail(foreign, LiteralLimits::default(), &context, &mut ledger()?)
            .is_err()
    );
    Ok(())
}
