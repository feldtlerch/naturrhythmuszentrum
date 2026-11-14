// SPDX-License-Identifier: AGPL-3.0-only
//! Conservative finite-limit refusals and empty selections are explicit in both formats.

use super::{TestResult, empty, oracle, publish};
use crate::{EndpointReadError, LoanError};
use nrz_scalar::Scalar;

/// Neighbours inside the finite range succeed; exact range-limit targets still refuse.
pub(super) fn finite_edges<P: Scalar + 'static>() -> TestResult {
    let maximum = P::max_finite();
    let half = maximum / P::from_u8(2);
    let inputs = [
        [maximum.next_down(), P::ZERO, P::ZERO],
        [(-maximum).next_up(), -P::ZERO, P::ZERO],
        [maximum, P::ZERO, P::ZERO],
        [-maximum, -P::ZERO, P::ZERO],
        [half, half, P::ZERO],
        [-half, half, P::ZERO],
    ];
    let (mut heap, mut account, tile, mask) = publish(&inputs)?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    for (cell, input) in inputs.into_iter().enumerate() {
        crate::loan::fault::reset_reads();
        let result = reader.read_component(cell, 0, &mut account);
        assert_eq!(crate::loan::fault::reads(), 3);
        if cell < 2 {
            oracle(&result?, input)?;
        } else {
            assert!(matches!(result, Err(EndpointReadError::Arithmetic(_))));
        }
    }
    drop(tile);
    empty(&mut heap, &mut account)
}

/// A real filled tile with no demanded cells refuses coverage without observation or allocation.
pub(super) fn empty_demand<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, tile, mask) = publish(&[[P::ONE; 3]])?;
    let selected = heap.create_mask(mask.layout(), &[], &mut account.ledger)?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(selected, selected.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    assert_eq!(reader.capacity(), 1);
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        reader.read_component(0, 0, &mut account),
        Err(EndpointReadError::Loan(LoanError::CoverageMismatch))
    ));
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    drop(injection);
    drop(tile);
    heap.retire_mask(selected, &mut account.ledger)?;
    empty(&mut heap, &mut account)
}

/// All signed-zero pairs represent the exact singleton zero with zero conversion discrepancy.
/// ´test:unit:loan-endpoint-view-signed-zero´
#[test]
fn loan_endpoint_view_signed_zero() -> TestResult {
    zeros::<f32>()?;
    zeros::<f64>()
}
fn zeros<P: Scalar + 'static>() -> TestResult {
    let inputs = [
        [P::ZERO, P::ZERO, P::ZERO],
        [-P::ZERO, P::ZERO, P::ZERO],
        [P::ZERO, -P::ZERO, P::ZERO],
        [-P::ZERO, -P::ZERO, P::ZERO],
    ];
    assert!((-P::ZERO).to_f64().is_sign_negative());
    let (mut heap, mut account, tile, mask) = publish(&inputs)?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    for cell in 0..4 {
        let reading = reader.read_component(cell, 0, &mut account)?;
        assert_eq!(
            [
                reading.bounds().lower().get(),
                reading.bounds().upper().get()
            ],
            [P::ZERO; 2]
        );
        assert_eq!(
            reading.conversion_error().map(nrz_scalar::Finite::get),
            [P::ZERO; 2]
        );
        assert_eq!(
            [reading.bounds().lower(), reading.bounds().upper()]
                .map(|bound| bound.get().to_f64().to_bits()),
            [0_u64; 2]
        );
    }
    drop(tile);
    empty(&mut heap, &mut account)
}
