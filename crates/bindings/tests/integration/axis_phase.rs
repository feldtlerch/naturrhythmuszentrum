// SPDX-License-Identifier: AGPL-3.0-only
//! Original LC sign questions retain exact phase branches, proof assumptions and explicit absence.

#[path = "axis_phase/costs.rs"]
mod costs;
#[path = "../../src/testing/axis_phase.rs"]
mod fixture;
use fixture::{TestResult, enclosure, fixture, ledger};
use nrz_bindings::{
    AxisPhaseOutcome, DecisionError, Kind, Role,
    readings::algebra::proof::{HypothesisKind, Statement},
};

/// Independent original LC references select exact axes or undefined phase, never a default zero.
/// ´test:integration:bindings-lc-axis-phase-outcomes´
#[test]
fn bindings_lc_axis_phase_outcomes() -> TestResult {
    for (frequency, lower, upper, expected) in [
        (1000, "-301/4", "-299/4", AxisPhaseOutcome::Negative),
        (2000, "0", "0", AxisPhaseOutcome::Undefined),
        (4000, "299/4", "301/4", AxisPhaseOutcome::Positive),
        (2000, "-1/4", "1/4", AxisPhaseOutcome::Undecided),
        (1000, "-100", "1", AxisPhaseOutcome::Undecided),
        (4000, "-1", "100", AxisPhaseOutcome::Undecided),
    ] {
        let source = fixture(frequency)?;
        let result = source
            .case
            .attempts(1)
            .attempt(&enclosure(&source, lower, upper)?, &mut ledger()?)?;
        assert_eq!(result.outcome(), expected);
        assert_eq!(result.case().angle().kind(), &Kind::Angle);
        assert_eq!(result.case().angle().role(), Role::Difference);
        if matches!(
            expected,
            AxisPhaseOutcome::Negative | AxisPhaseOutcome::Positive
        ) {
            assert_eq!(
                result.phase(),
                Some(&source.case.phases()[usize::from(expected == AxisPhaseOutcome::Positive)])
            );
            let reciprocal = result.reciprocal().ok_or("original reciprocal discharge")?;
            assert!(reciprocal.is_discharged());
            assert!(!reciprocal.is_unconditional());
            assert_eq!(reciprocal.hypotheses().len(), 2);
            let Statement::Equality { left, right, .. } = &reciprocal.judgment().statement else {
                return Err("reciprocal equality".into());
            };
            assert_eq!(left, source.case.reciprocal());
            assert_eq!(right, source.case.reciprocal());
            let cas = source.graph.lock().map_err(|_| "graph poisoned")?;
            let citations = cas.proof_citations(reciprocal.proof(), &mut ledger()?)?;
            drop(cas);
            let discharge = citations.last().ok_or("discharge")?;
            let original = citations
                .iter()
                .find(|citation| citation.proof == discharge.premises[0].proof)
                .ok_or("original")?;
            assert_eq!(original.residuals, [HypothesisKind::NonZero]);
            assert_eq!(discharge.residuals, [] as [HypothesisKind; 0]);
        } else {
            assert!(result.phase().is_none());
            assert!(result.reciprocal().is_none());
        }
        assert_eq!(
            result.membership().is_some(),
            expected != AxisPhaseOutcome::Undecided
        );
    }
    Ok(())
}

/// Refinements share their finite allowance, original target and consumed Work.
/// ´test:integration:bindings-lc-axis-phase-attempt-limits´
#[test]
fn bindings_lc_axis_phase_attempt_limits() -> TestResult {
    let source = fixture(1000)?;
    for maximum in [1, 2] {
        let mut attempts = source.case.attempts(maximum);
        let mut budget = ledger()?;
        let first = attempts.attempt(&enclosure(&source, "-100", "1")?, &mut budget)?;
        assert_eq!(first.outcome(), AxisPhaseOutcome::Undecided);
        let consumed = budget.usage().work;
        let second = attempts.attempt(&enclosure(&source, "-301/4", "-299/4")?, &mut budget);
        assert_eq!(second.is_ok(), maximum == 2);
        if maximum == 1 {
            assert!(matches!(second, Err(DecisionError::Attempts)));
        }
        assert!(budget.usage().work > consumed);
        assert_eq!(attempts.used(), maximum);
    }
    Ok(())
}

/// Invalid origins and changed targets consume attempts without providing any selected evidence.
/// ´test:integration:bindings-lc-axis-phase-original-refusals´
#[test]
fn bindings_lc_axis_phase_original_refusals() -> TestResult {
    let source = fixture(1000)?;
    let other = fixture(4000)?;
    let mut attempts = source.case.attempts(2);
    let mut changed = enclosure(&source, "-301/4", "-299/4")?;
    changed.target = other.case.specification().target.clone();
    assert!(attempts.attempt(&changed, &mut ledger()?).is_err());
    let mut false_origin = enclosure(&source, "-301/4", "-299/4")?;
    false_origin.origin = nrz_bindings::readings::algebra::decision::EnclosureOrigin::Unverified;
    assert!(attempts.attempt(&false_origin, &mut ledger()?).is_err());
    assert_eq!(attempts.used(), 2);
    assert!(matches!(
        attempts.attempt(&enclosure(&source, "-301/4", "-299/4")?, &mut ledger()?),
        Err(DecisionError::Attempts)
    ));
    Ok(())
}
