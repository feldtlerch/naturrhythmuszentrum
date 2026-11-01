// SPDX-License-Identifier: AGPL-3.0-only
//! Resource-only current-use refusals preserve valid authority and consumed caller prefixes.

use super::{
    TestResult,
    fixture::{AT, PRIOR, Source, work},
    ledger, pins,
};
use nrz_bindings::readings::algebra::{FaultKind, limits::BudgetKind};
use nrz_matrix::{DecisionSnapshotError, LiveDecisionError};
use nrz_scalar::Scalar;

/// Current and reciprocal consumers retain literal Work bounds without poisoning live evidence.
/// ´test:integration:lc-axis-phase-current-work-boundaries´
#[test]
fn lc_axis_phase_current_work_boundaries() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

/// Undefined and undecided remain explicit even when enough current-use Work is available.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for block in [false, true] {
        for (index, pin) in pins::pins::<P>(block).into_iter().enumerate() {
            let source = Source::<P>::new(index, block)?;
            let published = source.published(index)?;
            for short in [0, 1] {
                let mut budget = work(PRIOR + pin.work[2] - short)?;
                budget.work(PRIOR, AT)?;
                let result = published.current(source.heap(), source.context(), &mut budget);
                assert_eq!(result.is_ok(), short == 0, "{result:?}");
                assert!(budget.usage().work > PRIOR);
                if let Err(cause) = result {
                    exhausted(&cause)?;
                } else {
                    assert_eq!(budget.usage().work, PRIOR + pin.work[2]);
                }
                let mut budget = work(PRIOR + pin.work[3] - short)?;
                budget.work(PRIOR, AT)?;
                let result = published.reciprocal(source.heap(), source.context(), &mut budget);
                if short == 1 {
                    let Err(cause) = result else {
                        return Err("one-short reciprocal Work succeeded".into());
                    };
                    exhausted(&cause)?;
                } else {
                    assert_eq!(budget.usage().work, PRIOR + pin.work[3]);
                    match index {
                        1 => assert!(matches!(result, Err(LiveDecisionError::UndefinedPhase))),
                        2 => assert!(matches!(result, Err(LiveDecisionError::UndecidedPhase))),
                        0 | 3 => assert!(result.is_ok()),
                        _ => return Err("unknown original case".into()),
                    }
                }
                published.current(source.heap(), source.context(), &mut ledger()?)?;
            }
        }
    }
    Ok(())
}

/// The refusal must be caller Work exhaustion, rather than a changed semantic question.
fn exhausted(cause: &LiveDecisionError) -> TestResult {
    let (LiveDecisionError::Algebra(fault)
    | LiveDecisionError::Snapshot(DecisionSnapshotError::Resources(fault))) = cause
    else {
        return Err(format!("unexpected current-use refusal: {cause}").into());
    };
    assert!(matches!(
        fault.kind,
        FaultKind::Exhausted {
            budget: BudgetKind::Work,
            ..
        }
    ));
    Ok(())
}
