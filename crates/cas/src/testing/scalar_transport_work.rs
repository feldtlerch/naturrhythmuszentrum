// SPDX-License-Identifier: AGPL-3.0-only
//! Complete checking and portable operations account for caller usage and rollback.

use super::budgets::proof_budget as budget;
use super::common::TestResult;
use super::scalar_transport::chain;
use crate::test_api::limits::{Budget, BudgetKind};
use crate::test_api::{Fault, FaultKind, InvalidCode, Location};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Check,
    Export,
    Replay,
}

pub(crate) fn attempt(depth: usize, operation: Operation, amount: u64) -> TestResult<(bool, u64)> {
    bounded_attempt(depth, operation, BudgetKind::Work, amount)
}

pub(crate) fn bounded_attempt(
    depth: usize,
    operation: Operation,
    resource: BudgetKind,
    amount: u64,
) -> TestResult<(bool, u64)> {
    let mut f = chain(false, depth)?;
    let context = f.frames.last().ok_or("missing frame")?.clone();
    let witness = if matches!(operation, Operation::Check) {
        None
    } else {
        Some(
            f.fixture
                .cas
                .check_proof(&context, &f.steps, &mut budget()?)?,
        )
    };
    let bytes = if let Some(witness) = &witness {
        f.fixture.cas.export_witness(witness, &mut budget()?)?
    } else {
        vec![]
    };
    if matches!(operation, Operation::Replay) {
        f = chain(true, depth)?;
    }
    let before = f.fixture.cas.canonical_snapshot(&mut budget()?)?;
    let mut limits = budget()?.limits().clone();
    match resource {
        BudgetKind::Work => limits.work = amount,
        BudgetKind::StagedPayloadBytes => limits.staged_payload_bytes = amount,
        BudgetKind::ConditionSteps => limits.condition_steps = amount,
        BudgetKind::Depth => limits.depth = amount,
        _ => return Err("unsupported transport resource".into()),
    }
    let mut ledger = Budget::new(limits.clone())?;
    ledger.work(37, Location::new("prior scalar transport Work"))?;
    let result = ledger.with_retained_payload(
        19,
        Location::new("prior scalar transport payload"),
        |ledger| match operation {
            Operation::Check => f
                .fixture
                .cas
                .check_proof(&context, &f.steps, ledger)
                .map(|w| assert_eq!(w.judgment(), &f.expected)),
            Operation::Export => f
                .fixture
                .cas
                .export_witness(
                    witness.as_ref().ok_or_else(|| {
                        Fault::invalid(InvalidCode::Reference, Location::new("export fixture"))
                    })?,
                    ledger,
                )
                .map(|_| ()),
            Operation::Replay => f
                .fixture
                .cas
                .replay(&bytes, &f.expected, ledger)
                .map(|w| assert_eq!(w.judgment(), &f.expected)),
        },
    );
    assert_eq!(ledger.limits(), &limits);
    if let Err(fault) = &result {
        assert!(
            matches!(
                fault.kind,
                FaultKind::Exhausted {
                    budget,
                    ..
                } if budget == resource
            ),
            "{fault:?}"
        );
        assert_eq!(f.fixture.cas.canonical_snapshot(&mut budget()?)?, before);
    }
    Ok((result.is_ok(), ledger.usage().work))
}
