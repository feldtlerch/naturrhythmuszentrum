// SPDX-License-Identifier: AGPL-3.0-only
//! Checked transport retains its subset proof and original predicate.

use crate::testing::budgets::proof_budget as budget;
use crate::testing::common::TestResult;
use crate::testing::scalar_transport::{chain, meet};

/// Supported scalar conversion does not replace the weakening law's subset premise.
/// ´test:integration:scalar-transport-subset-premise´
#[test]
fn scalar_transport_subset_premise() -> TestResult {
    let mut f = chain(false, 1)?;
    let domain = f.fixture.cas.admissible_region(&f.frames[1])?;
    let original = f.fixture.domain.clone();
    assert_eq!(
        meet(&mut f.fixture, &[original, f.restriction.clone()])?,
        domain
    );
    assert_eq!(f.expected.residuals[0].predicate, f.predicate);
    let before = f.fixture.cas.canonical_snapshot(&mut budget()?)?;
    let mut broken = f.steps.clone();
    broken[2].premises.pop();
    assert!(
        f.fixture
            .cas
            .check_proof(&f.frames[1], &broken, &mut budget()?)
            .is_err()
    );
    assert_eq!(f.fixture.cas.canonical_snapshot(&mut budget()?)?, before);
    let witness = f
        .fixture
        .cas
        .check_proof(&f.frames[1], &f.steps, &mut budget()?)?;
    assert_eq!(witness.judgment(), &f.expected);
    Ok(())
}

/// Complete checks independently bound staging, traversal depth and condition visits.
/// ´test:integration:scalar-transport-checking-nonwork-boundaries´
#[test]
fn scalar_transport_checking_nonwork_boundaries() -> TestResult {
    use crate::testing::scalar_transport_work::{Operation, bounded_attempt};
    use nrz_cas::limits::BudgetKind::{ConditionSteps, Depth, StagedPayloadBytes};
    // One edge, two nested restrictions, and the deeper three-edge chain.
    // Each operation consumes its prefix rather than prepaying the complete check.
    for (depth, work, rows) in [
        (
            1,
            65_016,
            [
                (StagedPayloadBytes, 24_435, 59_143),
                (Depth, 3, 20_797),
                (ConditionSteps, 62, 23_997),
            ],
        ),
        (
            2,
            86_509,
            [
                (StagedPayloadBytes, 32_227, 77_161),
                (Depth, 4, 29_058),
                (ConditionSteps, 97, 32_260),
            ],
        ),
        (
            3,
            109_356,
            [
                (StagedPayloadBytes, 40_019, 95_754),
                (Depth, 5, 37_894),
                (ConditionSteps, 141, 41_098),
            ],
        ),
    ] {
        for (resource, amount, prefix) in rows {
            assert_eq!(
                bounded_attempt(depth, Operation::Check, resource, amount)?,
                (true, work)
            );
            assert_eq!(
                bounded_attempt(depth, Operation::Check, resource, amount - 1)?,
                (false, prefix)
            );
        }
    }
    Ok(())
}

/// Complete proof checking retains prior usage and rolls back the measured refusal prefix.
/// ´test:integration:scalar-transport-checking-resource-boundaries´
#[test]
fn scalar_transport_checking_resource_boundaries() -> TestResult {
    use crate::testing::scalar_transport_work::{Operation, attempt};
    // The complete check is not prepaid; these totals include 37 prior Work and 19 live bytes.
    for (depth, total, prefix) in [
        (0, 33_628, 33_620),
        (1, 65_016, 65_012),
        (3, 109_356, 109_352),
    ] {
        assert_eq!(attempt(depth, Operation::Check, total)?, (true, total));
        assert_eq!(
            attempt(depth, Operation::Check, total - 1)?,
            (false, prefix)
        );
    }
    Ok(())
}
