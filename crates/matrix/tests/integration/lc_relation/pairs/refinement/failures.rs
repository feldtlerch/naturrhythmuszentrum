// SPDX-License-Identifier: AGPL-3.0-only
//! Changed questions, certified emptiness and failed refinement have distinct meanings.

use super::{TestResult, integer, ledger, pairs, support};
use nrz_bindings::{Budget, LcRefinementState, LcRelation, Limits, LimitsOverride};

/// A separately certified empty included family has no physical horizon and retains previous facts.
/// ´test:integration:lc-relation-refinement-impossible´
#[test]
fn lc_relation_refinement_impossible() -> TestResult {
    const FIRST: usize = 0;
    const ADDED: usize = 3;
    let previous = pairs::cover(true, "1/20")?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let refined = support::impossible_inputs()?;
    let inclusion = previous
        .relation()
        .refinement_inputs(&refined, &mut ledger()?)?;
    let pi = previous.pi_bounds().contract();
    let empty = refined.impossible_pair(
        [FIRST, ADDED],
        [integer(0)?, integer(0)?],
        pi,
        &mut ledger()?,
    )?;
    let attempt = previous.refine_impossible(&inclusion, &horizon, empty, &mut ledger()?)?;
    let LcRefinementState::Impossible(certificate) = attempt.state() else {
        return Err("certified impossible refinement".into());
    };
    assert!(certificate.relation().shares_allocation(&refined));
    assert!(attempt.previous().shares_allocation(&previous));
    assert!(
        attempt
            .previous_horizon()
            .relation()
            .shares_allocation(previous.relation())
    );
    assert!(attempt.refined_horizon().is_none());
    assert_eq!(
        attempt.inclusion().counterparts().len(),
        previous.relation().inputs().len()
    );
    Ok(())
}

/// Cumulative exhaustion records its new obligation while preserving cover, horizon and input proof.
/// ´test:integration:lc-relation-refinement-exhaustion´
#[test]
fn lc_relation_refinement_exhaustion() -> TestResult {
    let previous = pairs::cover(true, "1/20")?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let refined = support::refined(true, "1/40", true)?;
    let inclusion = previous
        .relation()
        .refinement_inputs(&refined, &mut ledger()?)?;
    let mut cycles = previous.cycles().to_vec();
    cycles.push(integer(1)?);
    let mut measured = ledger()?;
    let complete = previous.refine(&inclusion, &horizon, cycles.clone(), &mut measured)?;
    assert!(matches!(complete.state(), LcRefinementState::Refined(_)));
    let work = measured.usage().work;
    for (limit, success) in [(work + 17, true), (work + 16, false), (117, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("refinement prefix"),
        )?;
        let attempt = previous.refine(&inclusion, &horizon, cycles.clone(), &mut budget)?;
        assert_eq!(
            matches!(attempt.state(), LcRefinementState::Refined(_)),
            success
        );
        if !success {
            assert!(matches!(attempt.state(), LcRefinementState::Undecided(_)));
        }
        assert!(attempt.previous().shares_allocation(&previous));
        assert!(attempt.inclusion().refined().shares_allocation(&refined));
        assert!(
            attempt
                .previous_horizon()
                .relation()
                .shares_allocation(horizon.relation())
        );
        assert_eq!(attempt.proposed_cycles(), &cycles);
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

/// Weaker inputs, changed epochs and foreign equal-value horizons are different questions.
/// ´test:integration:lc-relation-refinement-identity-refusals´
#[test]
fn lc_relation_refinement_identity_refusals() -> TestResult {
    let previous = pairs::cover(true, "1/20")?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let weaker = support::refined(true, "1/10", false)?;
    assert!(
        previous
            .relation()
            .refinement_inputs(&weaker, &mut ledger()?)
            .is_err()
    );
    let changed_clock = LcRelation::new(
        "different epoch".into(),
        3,
        pairs::inputs(true, "1/40")?,
        &mut ledger()?,
    )?;
    assert!(
        previous
            .relation()
            .refinement_inputs(&changed_clock, &mut ledger()?)
            .is_err()
    );
    let mut dropped = pairs::inputs(true, "1/40")?;
    dropped.pop();
    let dropped = pairs::relation(dropped)?;
    assert!(
        previous
            .relation()
            .refinement_inputs(&dropped, &mut ledger()?)
            .is_err()
    );
    let refined = support::refined(true, "1/40", false)?;
    let inclusion = previous
        .relation()
        .refinement_inputs(&refined, &mut ledger()?)?;
    let foreign = pairs::cover(true, "1/20")?.forward_horizon(&mut ledger()?)?;
    assert!(
        previous
            .refine(
                &inclusion,
                &foreign,
                previous.cycles().to_vec(),
                &mut ledger()?
            )
            .is_err()
    );
    let mut cycles = previous.cycles().to_vec();
    cycles[0] = integer(1)?;
    let wrong_branch = previous.refine(&inclusion, &horizon, cycles, &mut ledger()?)?;
    assert!(matches!(
        wrong_branch.state(),
        LcRefinementState::Undecided(_)
    ));
    assert!(wrong_branch.previous().shares_allocation(&previous));
    assert!(previous.zero_times(&mut ledger()?).is_ok());
    assert!(
        pairs::cover(false, "1/20")?
            .zero_times(&mut ledger()?)
            .is_err()
    );
    Ok(())
}
