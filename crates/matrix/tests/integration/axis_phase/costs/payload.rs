// SPDX-License-Identifier: AGPL-3.0-only
//! Caller-retained storage composes with full preparation and atomic publication allowances.

use super::{
    TestResult,
    fixture::{AT, PRIOR, Pin, Source},
    pins,
};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Every outcome has literal staging/output boundaries, both alone and beside retained caller data.
/// ´test:integration:lc-axis-phase-payload-boundaries´
#[test]
fn lc_axis_phase_payload_boundaries() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

/// No measured ceiling is raised; the caller's existing payload is accounted separately.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for block in [false, true] {
        for (index, pin) in pins::pins::<P>(block).into_iter().enumerate() {
            for staged in [true, false] {
                for prior in [0, PRIOR] {
                    for short in [0, 1] {
                        peak::<P>(index, block, pin, [staged, false], prior, short)?;
                        peak::<P>(index, block, pin, [staged, true], prior, short)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Publication alone starts with a prepared result; the peak route includes the complete attempt.
fn peak<P: Scalar + 'static>(
    index: usize,
    block: bool,
    pin: Pin,
    checks: [bool; 2],
    prior: u64,
    short: u64,
) -> TestResult {
    let [staged, publication] = checks;
    let source = Source::<P>::new(index, block)?;
    let usage = source.heap().provenance_usage();
    let required = if publication {
        pin.bytes[2]
    } else {
        pin.bytes[usize::from(!staged)]
    };
    let mut limits = Limits::default();
    if staged {
        limits.staged_payload_bytes = required + prior - short;
    } else {
        limits.output_bytes = required - short;
    }
    let mut budget = Budget::new(limits)?;
    budget.work(PRIOR, AT)?;
    let prepared = publication
        .then(|| source.prepare(index, &mut super::ledger()?))
        .transpose()?;
    let result = budget.with_retained_payload(prior, AT, |budget| -> TestResult<_> {
        let prepared = prepared.map_or_else(|| source.prepare(index, budget), Ok)?;
        Ok(source.heap().publish_axis_phase(prepared, budget)?)
    });
    assert_eq!(result.is_ok(), short == 0, "{result:?}");
    assert_eq!(source.heap().provenance_usage(), usage);
    if publication && short == 1 {
        assert_eq!(budget.usage().work, PRIOR);
    } else {
        assert!(budget.usage().work > PRIOR);
    }
    if let Ok(published) = result {
        assert_eq!(published.retained_payload_bytes(), pin.bytes[2]);
    }
    Ok(())
}
