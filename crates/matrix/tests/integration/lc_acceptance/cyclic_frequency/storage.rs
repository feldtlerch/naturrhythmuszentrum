// SPDX-License-Identifier: AGPL-3.0-only
//! Final descriptor and scalar-storage refusal leaves the complete symbol inventory available.

use super::{Prepared, TestResult, models, prepared, publication, shared};
use nrz_matrix::{CovarianceBlock, Extent, Heap, HeapLimits};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Descriptor and element N/N-1 twins keep one immutable heap limit through refusal and retry.
/// ´test:integration:lc-frequency-heap-storage-boundaries´
#[test]
fn lc_frequency_heap_storage_boundaries() -> TestResult {
    let prepared = prepared()?;
    boundaries::<f32>(&prepared)?;
    boundaries::<f64>(&prepared)
}

fn boundaries<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    for (kind, required, prefix) in [(0, 20, 270), (1, 25, 3_844)] {
        assert_eq!(attempt::<P>(prepared, kind, required)?, (true, 3_844));
        assert_eq!(attempt::<P>(prepared, kind, required - 1)?, (false, prefix));
    }
    Ok(())
}

/// A disposable unrelated scalar permits same-limit retry after a late one-slot refusal.
fn attempt<P: Scalar + 'static>(
    prepared: &Prepared,
    kind: u8,
    available: usize,
) -> TestResult<(bool, u64)> {
    let limits = if kind == 0 {
        HeapLimits::new(available, 4096, 4096)
    } else {
        HeapLimits::new(4096, available, 4096)
    };
    let mut heap = Heap::new(limits)?;
    let spare = heap.import_represented(
        Extent::new(1, 1)?,
        &nrz_bindings::Descriptor::ratio(),
        &[P::ZERO],
    )?;
    let mut budget = shared::policy()?;
    let one = P::ONE;
    let mut case = models::in_heap(
        prepared,
        heap,
        [
            [one / P::from_u8(4), one / P::from_u8(64), P::ZERO],
            [
                one / P::from_u8(16),
                one / (P::from_u8(128) * P::from_u8(2)),
                P::ZERO,
            ],
        ],
        [
            one / P::from_u8(128),
            one / (P::from_u8(128) * P::from_u8(4)),
            P::ZERO,
            P::ZERO,
        ],
        [one; 2],
        CovarianceBlock::Known,
        &mut budget,
    )?;
    let plan = publication(prepared, &case, &mut budget)?;
    let before = case.heap.provenance_usage();
    let stamps = case.heap.statistical(case.companion)?.targets();
    let spare_stamp = case.heap.value(spare.value())?.stamp();
    let context = ArithmeticContext::new()?;
    let mut execution = shared::policy()?;
    let result = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut execution);
    let success = result.is_ok();
    if !success {
        assert_eq!(case.heap.provenance_usage(), before);
        assert_eq!(case.heap.statistical(case.companion)?.targets(), stamps);
        assert_eq!(case.heap.value(spare.value())?.stamp(), spare_stamp);
        case.heap.retire(spare.value())?;
        let result = case
            .heap
            .publish_lc_frequency(&plan, &context, &mut shared::policy()?)?;
        assert_eq!(result.readings().bindings().len(), 5);
        result.covariance(&case.heap, &context)?;
    }
    Ok((success, execution.usage().work))
}
