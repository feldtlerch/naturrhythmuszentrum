// SPDX-License-Identifier: AGPL-3.0-only
//! Failed solving retains its spent ledger and whole question; malformed evidence stays refused.

use super::{TestResult, cycles, ledger, number, pairs, pi_certificate, retained};
use nrz_bindings::{Budget, LcBranchObligation, LcPairAttempt, Limits, LimitsOverride};

/// Exact N/N-minus-one work boundaries do not erase the original candidates.
/// ´test:integration:lc-relation-unresolved-work´
#[test]
fn lc_relation_unresolved_work() -> TestResult {
    let relation = pairs::relation(pairs::inputs(true, "1/20")?)?;
    let pi = pi_certificate()?;
    let mut measured = ledger()?;
    let complete = relation.attempt_pairs_cover(cycles()?, &pi, &mut measured)?;
    assert!(matches!(complete, LcPairAttempt::Certified(_)));
    let work = measured.usage().work;
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("unresolved attempt prefix"),
        )?;
        let attempt = relation.attempt_pairs_cover(cycles()?, &pi, &mut budget)?;
        assert_eq!(matches!(attempt, LcPairAttempt::Certified(_)), success);
        if let LcPairAttempt::Unresolved(unresolved) = attempt {
            retained(&unresolved, &relation)?;
            assert_eq!(unresolved.obligation(), LcBranchObligation::Resources);
            assert_eq!(unresolved.usage()[0].work, 17);
            assert_eq!(unresolved.usage()[1].work, budget.usage().work);
        }
    }
    Ok(())
}

/// False pi and noninteger proposals are invalid evidence, not new certified candidates.
/// ´test:integration:lc-relation-unresolved-malformed´
#[test]
fn lc_relation_unresolved_malformed() -> TestResult {
    let relation = pairs::relation(pairs::inputs(true, "1/20")?)?;
    let mut pi = pi_certificate()?;
    pi.upper = number("3")?;
    assert!(
        relation
            .attempt_pairs_cover(cycles()?, &pi, &mut ledger()?)
            .is_err()
    );
    let mut proposal = cycles()?;
    proposal[0] = number("1/2")?;
    assert!(
        relation
            .attempt_pairs_cover(proposal, &pi_certificate()?, &mut ledger()?)
            .is_err()
    );
    assert!(
        relation
            .attempt_pairs_cover(Vec::new(), &pi_certificate()?, &mut ledger()?)
            .is_err()
    );
    let limits = Limits::default().resolve(&LimitsOverride {
        work: Some(1),
        ..LimitsOverride::default()
    })?;
    assert!(
        relation
            .attempt_pairs_cover(cycles()?, &pi_certificate()?, &mut Budget::new(limits)?)
            .is_err()
    );
    Ok(())
}
