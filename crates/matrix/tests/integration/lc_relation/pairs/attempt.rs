// SPDX-License-Identifier: AGPL-3.0-only
//! Unproved branch coverage preserves every original constraint and every integer tuple.

mod refusals;

use super::super::support::{input, integer, pi_certificate};
use super::{TestResult, ledger, number, support as pairs};
use nrz_bindings::readings::algebra::Rational;
use nrz_bindings::{
    LcBranchObligation, LcCertainty, LcDeterminacy, LcPairAttempt, LcQuantity as Q, LcRelation,
    LcUnresolvedBranches,
};

/// The same finite-determined question remains unresolved with weak evidence and certifies with more.
/// ´test:integration:lc-relation-unresolved-branches´
#[test]
fn lc_relation_unresolved_branches() -> TestResult {
    let relation = pairs::relation(pairs::inputs(true, "1/20")?)?;
    let mut coarse = pi_certificate()?;
    coarse.lower = number("1/10")?;
    let attempt = relation.attempt_pairs_cover(cycles()?, &coarse, &mut ledger()?)?;
    let LcPairAttempt::Unresolved(unresolved) = attempt else {
        return Err("unresolved coarse branch evidence".into());
    };
    assert_eq!(unresolved.obligation(), LcBranchObligation::CycleCoverage);
    retained(&unresolved, &relation)?;
    let fine = relation.attempt_pairs_cover(cycles()?, &pi_certificate()?, &mut ledger()?)?;
    let LcPairAttempt::Certified(cover) = fine else {
        return Err("certified fine branch evidence".into());
    };
    assert_eq!(cover.determinacy(Q::Rate)?, LcDeterminacy::DeterminedFinite);
    assert!(cover.relation().shares_allocation(unresolved.relation()));
    assert_eq!(cover.rate(), &[number("15875/8")?, number("16125/8")?]);
    assert_eq!(unresolved.certainty(), LcCertainty::Undecided);
    Ok(())
}

/// Broad original frequency support is kept even when the proposed bounded branch is incomplete.
/// ´test:integration:lc-relation-unresolved-original-domain´
#[test]
fn lc_relation_unresolved_original_domain() -> TestResult {
    let mut inputs = pairs::inputs(true, "1/20")?;
    inputs[0] = input(Q::AngularFrequency, ["1", "100000"])?;
    let relation = pairs::relation(inputs)?;
    let attempt = relation.attempt_pairs_cover(cycles()?, &pi_certificate()?, &mut ledger()?)?;
    let LcPairAttempt::Unresolved(unresolved) = attempt else {
        return Err("unresolved broad branch domain".into());
    };
    retained(&unresolved, &relation)?;
    assert_eq!(
        unresolved.relation().inputs()[0].interval().upper(),
        Some(&number("100000")?.into())
    );
    assert_eq!(unresolved.obligation(), LcBranchObligation::CycleCoverage);
    assert!(matches!(
        unresolved.reason(),
        nrz_bindings::LcRelationError::CycleCover
    ));
    Ok(())
}

/// Complete candidate identity and arbitrary-size indices are independent of the failed proposal.
fn retained(unresolved: &LcUnresolvedBranches, relation: &LcRelation) -> TestResult {
    assert!(unresolved.relation().shares_allocation(relation));
    assert_eq!(unresolved.relation().inputs(), relation.inputs());
    assert_eq!(unresolved.proposal(), &cycles()?);
    assert_eq!(unresolved.domain().coordinates(), relation.observations());
    assert_eq!(unresolved.certainty(), LcCertainty::Undecided);
    assert_eq!(unresolved.determinacy(), None);
    assert!(unresolved.retained_payload_bytes() > 0);
    assert!(unresolved.usage()[1].work > unresolved.usage()[0].work);
    let huge = number("1000000000000000000000000000000000000000000000000000000000000")?;
    let negative = integer(0)?.sub(&huge, &mut ledger()?)?;
    assert!(
        unresolved
            .domain()
            .contains(&[huge, negative, integer(17)?], &mut ledger()?)?
    );
    assert!(
        !unresolved
            .domain()
            .contains(&[integer(0)?], &mut ledger()?)?
    );
    assert!(
        !unresolved
            .domain()
            .contains(&[integer(0)?, number("1/2")?, integer(1)?], &mut ledger()?)?
    );
    Ok(())
}

/// This finite tuple is only a proposal, never the unresolved domain itself.
fn cycles() -> TestResult<Vec<Rational>> {
    Ok(vec![integer(0)?, integer(0)?, integer(1)?])
}

/// A failed joint solve retains the original contradictory inputs without declaring global emptiness.
/// ´test:integration:lc-relation-unresolved-feasibility´
#[test]
fn lc_relation_unresolved_feasibility() -> TestResult {
    const FIRST: usize = 0;
    let mut inputs = pairs::inputs(true, "1/20")?;
    inputs.push(input(Q::Phase(FIRST), ["3", "3"])?);
    let relation = pairs::relation(inputs)?;
    let attempt = relation.attempt_pairs_cover(cycles()?, &pi_certificate()?, &mut ledger()?)?;
    let LcPairAttempt::Unresolved(unresolved) = attempt else {
        return Err("unresolved joint feasibility".into());
    };
    retained(&unresolved, &relation)?;
    assert_eq!(
        unresolved.obligation(),
        LcBranchObligation::JointFeasibility
    );
    assert!(matches!(
        unresolved.reason(),
        nrz_bindings::LcRelationError::Containment
    ));
    Ok(())
}
