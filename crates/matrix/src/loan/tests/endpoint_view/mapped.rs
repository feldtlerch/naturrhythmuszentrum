// SPDX-License-Identifier: AGPL-3.0-only
//! Noncontiguous native planes preserve logical endpoint targets and ignore physical padding.

use super::{TestResult, empty, fixture, granted, oracle};
use crate::{EndpointReadError, HandbackOutcome, HostChannel, LoanError};
use nrz_bindings::Limits;
use nrz_scalar::Scalar;

/// The independent address sets are [2,5,8] and [12,15,18]; cell one is a source hole.
/// ´test:unit:loan-endpoint-view-gapped-layout´
#[test]
fn loan_endpoint_view_gapped_layout() -> TestResult {
    mapped::<f32>()?;
    mapped::<f64>()
}
fn mapped<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, declaration) = fixture::<P>(3, 2, [true, true], Limits::default())?;
    let layout = crate::Layout::strided(crate::Extent::new(3, 2)?, 20, 2, 3, 10)?;
    let mut loan = granted(heap.request_loan_with_layout(declaration, layout, &mut account))?;
    for cell in [0, 2] {
        for component in 0..2 {
            for (role, value) in HostChannel::ALL
                .into_iter()
                .zip(values::<P>(cell, component)?)
            {
                loan.write_component(role, cell, component, value, &mut account)?;
            }
        }
    }
    loan.write_component(
        HostChannel::Center,
        1,
        0,
        P::parse_nearest("NaN").ok_or("NaN")?,
        &mut account,
    )?;
    let (tile, mask) = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, receipt) => (
            tile,
            receipt.filled(HostChannel::Center).ok_or("mask")?.native(),
        ),
        other => return Err(format!("{other:?}").into()),
    };
    let demand = heap.create_mask(mask.layout(), &[0, 1, 2], &mut account.ledger)?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(demand, demand.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    for cell in [0, 2] {
        for component in 0..2 {
            oracle(
                &reader.read_component(cell, component, &mut account)?,
                values::<P>(cell, component)?,
            )?;
        }
    }
    crate::loan::fault::reset_reads();
    assert!(matches!(
        reader.read_component(1, 0, &mut account),
        Err(EndpointReadError::Loan(LoanError::CoverageMismatch))
    ));
    assert_eq!(crate::loan::fault::reads(), 0);
    drop(tile);
    heap.retire_mask(demand, &mut account.ledger)?;
    empty(&mut heap, &mut account)
}
fn values<P: Scalar>(cell: usize, component: usize) -> TestResult<[P; 3]> {
    Ok([
        P::from_u8(u8::try_from(5 + cell + 10 * component)?),
        P::from_u8(u8::try_from(1 + component)?),
        P::ONE,
    ])
}
