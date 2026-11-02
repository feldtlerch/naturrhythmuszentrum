// SPDX-License-Identifier: AGPL-3.0-only
//! Shared-clock identity, declared support and cumulative budgets remain explicit.

use super::{TestResult, clock, integer, ledger, number, pi_certificate, relation};
use nrz_bindings::{Budget, LcInterval, LcSharedClock, Limits, LimitsOverride};
use nrz_matrix::LcClockProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits};

/// A larger shared offset cannot silently discard a narrower original timestamp constraint.
/// ´test:integration:lc-relation-shared-clock-refusals-and-work´
#[test]
fn lc_relation_shared_clock_refusals_and_work() -> TestResult {
    let offset = LcInterval::new(
        Some(number("-1/5")?.into()),
        Some(number("1/5")?.into()),
        &mut ledger()?,
    )?;
    assert!(
        LcSharedClock::new(
            relation()?,
            vec![number("1")?, number("2")?],
            &offset,
            &mut ledger()?
        )
        .is_err()
    );
    let clock = clock()?;
    assert!(
        clock
            .pairs_cover(
                vec![integer(0)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    let cover = clock.pairs_cover(
        vec![integer(0)?, integer(0)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?;
    let context = ArithmeticContext::new()?;
    let quantities = [
        nrz_bindings::LcQuantity::Rate,
        nrz_bindings::LcQuantity::Product,
    ];
    let mut measured = ledger()?;
    LcClockProjection::<f64>::new(
        cover.clone(),
        &quantities,
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
            nrz_bindings::readings::algebra::Location::new("shared clock prefix"),
        )?;
        assert_eq!(
            LcClockProjection::<f64>::new(
                cover.clone(),
                &quantities,
                LiteralLimits::default(),
                &context,
                &mut budget
            )
            .is_ok(),
            succeeds
        );
        assert!(cover.clock().shares_allocation(&clock));
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}
