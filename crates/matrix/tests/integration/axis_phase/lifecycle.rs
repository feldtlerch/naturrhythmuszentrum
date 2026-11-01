// SPDX-License-Identifier: AGPL-3.0-only
//! Source and radius changes retire every dependent use without reviving archived proofs.

use super::{Case, TestResult, enclosure, ledger};
use nrz_matrix::{Heap, HeapLimits, LiveDecisionError};
use nrz_scalar::Scalar;

/// Equal source writes and radius-only changes retire phase and reciprocal together in both formats.
/// ´test:integration:lc-axis-phase-source-radius-retirement´
#[test]
fn lc_axis_phase_source_radius_retirement() -> TestResult {
    retirement::<f32>()?;
    retirement::<f64>()
}

/// The existing version owner authenticates original values and companions, never center bytes alone.
fn retirement<P: Scalar + 'static>() -> TestResult {
    for radius_only in [false, true] {
        let mut case = Case::<P>::new(1000, "-75", "0.25")?;
        let published = case.publish("-76", "-74", &mut ledger()?)?;
        let historical = published
            .current(&case.heap, &case.context, &mut ledger()?)?
            .clone();
        let foreign = Heap::new(HeapLimits::default())?;
        assert!(
            published
                .phase(&foreign, &case.context, &mut ledger()?)
                .is_err()
        );
        assert!(
            published
                .phase(&case.heap, &case.context, &mut ledger()?)
                .is_ok()
        );
        let input = &published.inputs().operands()[0];
        let version = input.input().stamp().version();
        let handle = input.input().handle();
        let center = input.coordinates()[0];
        if radius_only {
            case.heap.replace_declared_bounds(
                handle,
                version,
                &[center],
                &[super::scalar("76")?],
            )?;
        } else {
            case.heap.replace_represented(handle, version, &[center])?;
        }
        assert!(
            published
                .phase(&case.heap, &case.context, &mut ledger()?)
                .is_err()
        );
        assert!(matches!(
            published.reciprocal(&case.heap, &case.context, &mut ledger()?),
            Err(LiveDecisionError::Retired)
        ));
        assert!(
            historical
                .reciprocal()
                .ok_or("historical proof")?
                .is_discharged()
        );
        assert_eq!(case.heap.value(handle)?.region().get(0, 0)?, center);
    }
    Ok(())
}

/// A mathematical strict sign and `NonZero` discharge cannot authenticate a straddling numerical box.
/// ´test:integration:lc-axis-phase-numerical-guard-and-original-refusals´
#[test]
fn lc_axis_phase_numerical_guard_and_original_refusals() -> TestResult {
    refusals::<f32>()?;
    refusals::<f64>()
}

/// Full-box containment and closed original value correspondence are checked independently.
fn refusals<P: Scalar + 'static>() -> TestResult {
    let case = Case::<P>::new(1000, "-75", "76")?;
    let proof = case
        .source
        .case
        .attempts(1)
        .attempt(&enclosure(&case.source, "-76", "-74")?, &mut ledger()?)?;
    assert!(
        proof
            .reciprocal()
            .ok_or("symbolic nonzero")?
            .is_discharged()
    );
    assert!(case.publish("-76", "-74", &mut ledger()?).is_err());
    let wrong = Case::<P>::new(1000, "-74", "0.25")?;
    assert!(wrong.publish("-100", "-1", &mut ledger()?).is_err());
    let other = Case::<P>::new(1000, "-75", "0.25")?;
    assert!(
        other
            .heap
            .freeze_axis_phase_input(&case.source.case, &other.input, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// A late source mutation refuses complete publication and preserves its consumed prefix.
/// ´test:integration:lc-axis-phase-late-publication-refusal´
#[test]
fn lc_axis_phase_late_publication_refusal() -> TestResult {
    let mut case = Case::<f64>::new(1000, "-75", "0.25")?;
    let mut budget = ledger()?;
    let prepared = case.prepare("-76", "-74", &mut budget)?;
    let prefix = budget.usage().work;
    let frozen =
        case.heap
            .freeze_axis_phase_input(&case.source.case, &case.input, &mut ledger()?)?;
    let handle = frozen.operands()[0].input().handle();
    let version = frozen.operands()[0].input().stamp().version();
    case.heap
        .replace_declared_bounds(handle, version, &[-75.0], &[76.0])?;
    let usage = case.heap.provenance_usage();
    assert!(case.heap.publish_axis_phase(prepared, &mut budget).is_err());
    assert!(budget.usage().work > prefix);
    assert_eq!(case.heap.provenance_usage(), usage);
    Ok(())
}
