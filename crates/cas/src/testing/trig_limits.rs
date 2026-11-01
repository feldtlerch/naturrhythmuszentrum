// SPDX-License-Identifier: AGPL-3.0-only
//! Independent single-dimension ceilings for graph lifecycle boundary fixtures.

use super::common::TestResult;
use crate::test_api::limits::{BudgetKind, Limits};

/// Work pairs include prior cumulative usage; other sizes retain their literal ceilings.
pub(crate) fn boundary(
    kind: BudgetKind,
    cost: u64,
    mut attempt: impl FnMut(u64) -> TestResult<(bool, u64)>,
) -> TestResult {
    if kind == BudgetKind::Work {
        assert!(!attempt(cost + 36)?.0);
        let (accepted, spent) = attempt(cost + 37)?;
        assert!(accepted);
        assert_eq!(spent, cost + 37);
    } else {
        if cost > 0 {
            assert!(!attempt(cost - 1)?.0);
        }
        assert!(attempt(cost)?.0);
    }
    Ok(())
}

pub(crate) const KINDS: [BudgetKind; 9] = [
    BudgetKind::Work,
    BudgetKind::Depth,
    BudgetKind::StagedPayloadBytes,
    BudgetKind::PayloadBytes,
    BudgetKind::WitnessBytes,
    BudgetKind::OutputBytes,
    BudgetKind::SourceBytes,
    BudgetKind::OriginalOccurrences,
    BudgetKind::ReplayWork,
];

pub(crate) fn limits(kind: BudgetKind, amount: u64) -> TestResult<Limits> {
    let mut limits = Limits::default();
    match kind {
        BudgetKind::Work => limits.work = amount,
        BudgetKind::Depth => limits.depth = amount,
        BudgetKind::StagedPayloadBytes => limits.staged_payload_bytes = amount,
        BudgetKind::PayloadBytes => limits.payload_bytes = amount,
        BudgetKind::WitnessBytes => limits.witness_bytes = amount,
        BudgetKind::OutputBytes => limits.output_bytes = amount,
        BudgetKind::SourceBytes => limits.source_bytes = amount,
        BudgetKind::OriginalOccurrences => limits.original_occurrences = amount,
        BudgetKind::ReplayWork => limits.replay_work = amount,
        _ => return Err("unmeasured trig resource".into()),
    }
    Ok(limits)
}
