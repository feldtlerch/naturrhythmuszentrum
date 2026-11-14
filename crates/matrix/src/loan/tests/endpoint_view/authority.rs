// SPDX-License-Identifier: AGPL-3.0-only
//! Endpoint reads inherit repair, qualified coverage and current versus historical authority.

use super::{TestResult, empty, fixture, granted, oracle};
use crate::{EndpointReadError, HandbackOutcome, HostChannel, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Repair the original error/total invariant before admitting endpoint reads in either format.
/// ´test:unit:loan-endpoint-view-repair-and-authority´
#[test]
fn loan_endpoint_view_repair_and_authority() -> TestResult {
    repair::<f32>()?;
    repair::<f64>()
}
fn repair<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, declaration) = fixture::<P>(1, 1, [true, true], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    let ticket = loan.ticket();
    for (role, value) in HostChannel::ALL.into_iter().zip([P::ONE, P::ZERO, P::ONE]) {
        loan.write_component(role, 0, 0, value, &mut account)?;
    }
    let mut loan = match heap.handback(loan, &mut account) {
        HandbackOutcome::Repair(
            LoanError::InvalidPayload {
                channel: HostChannel::Error,
                ..
            },
            loan,
        ) => loan,
        other => return Err(format!("{other:?}").into()),
    };
    assert_eq!(loan.ticket(), ticket);
    loan.write_component(HostChannel::Total, 0, 0, P::ONE, &mut account)?;
    let (tile, receipt) = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, receipt) => (tile, receipt),
        other => return Err(format!("{other:?}").into()),
    };
    let mask = receipt.filled(HostChannel::Center).ok_or("mask")?.native();
    let key = ticket.identity.root;
    let crate::heap::descriptors::DescriptorRecord::Managed(record) =
        heap.descriptors.slots.get_mut(key)?
    else {
        return Err("managed".into());
    };
    record.filled = false;
    crate::loan::fault::reset_reads();
    assert!(matches!(
        tile.reader(&heap, &mut account),
        Err(LoanError::StaleDependency)
    ));
    assert_eq!(crate::loan::fault::reads(), 0);
    let source = tile.pin().historical_reader(&mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    oracle(&reader.read_component(0, 0, &mut account)?, [P::ONE; 3])?;
    let mut foreign = crate::ExecutionAccount::new(Budget::new(Limits::default())?)?;
    crate::loan::fault::reset_reads();
    assert!(matches!(
        reader.read_component(0, 0, &mut foreign),
        Err(EndpointReadError::Loan(LoanError::ForeignAccount))
    ));
    assert!(matches!(
        reader.read_component(0, 1, &mut account),
        Err(EndpointReadError::Loan(LoanError::LayoutMismatch))
    ));
    assert_eq!(crate::loan::fault::reads(), 0);
    let crate::heap::descriptors::DescriptorRecord::Managed(record) =
        heap.descriptors.slots.get_mut(key)?
    else {
        return Err("managed".into());
    };
    record.filled = true;
    drop(tile);
    empty(&mut heap, &mut account)
}

/// Demand cannot promote a source hole, and foreign axes fail before endpoint binding.
/// ´test:unit:loan-endpoint-view-selection-refusals´
#[test]
fn loan_endpoint_view_selection_refusals() -> TestResult {
    selection::<f32>()?;
    selection::<f64>()
}
fn selection<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<P>(65, 2, [true, true], Limits::default())?;
    let (tile, mask) =
        super::super::owned_selection::publish(&mut heap, &mut account, declaration)?;
    let selected = heap.create_mask(mask.layout(), &[0, 1], &mut account.ledger)?;
    let foreign_axis = heap.create_cell_axis(65, &mut account.ledger)?;
    let foreign_layout = heap.create_cell_layout(
        foreign_axis,
        crate::Layout::contiguous(crate::Extent::new(65, 2)?)?,
        &mut account.ledger,
    )?;
    let foreign_mask = heap.create_mask(foreign_layout, &[1], &mut account.ledger)?;
    let source = tile.reader(&heap, &mut account)?;
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        source.masked_view(heap.mask(foreign_mask, foreign_layout)?, &mut account),
        Err(LoanError::LayoutMismatch)
    ));
    let view = source
        .masked_view(heap.mask(selected, selected.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    assert!(matches!(
        view.read_component(0, 0, &mut account),
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
    oracle(
        &view.read_component(1, 0, &mut account)?,
        [P::ONE, P::from_u8(2), P::ONE],
    )?;
    drop(tile);
    heap.retire_mask(foreign_mask, &mut account.ledger)?;
    heap.retire_cell_layout(foreign_layout, &mut account.ledger)?;
    heap.retire_mask(selected, &mut account.ledger)?;
    empty(&mut heap, &mut account)
}

/// Formatting an endpoint view cannot inspect native planes or expose a cleared NaN hole.
/// ´test:unit:loan-endpoint-view-formatting-holes´
#[test]
fn loan_endpoint_view_formatting_holes() -> TestResult {
    formatting::<f32>()?;
    formatting::<f64>()
}
fn formatting<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, declaration) = fixture::<P>(1, 1, [true, true], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    loan.write_component(
        HostChannel::Center,
        0,
        0,
        P::parse_nearest("NaN").ok_or("NaN")?,
        &mut account,
    )?;
    loan.clear_cell(HostChannel::Center, 0, &mut account)?;
    let (tile, mask) = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, receipt) => (
            tile,
            receipt.filled(HostChannel::Center).ok_or("mask")?.native(),
        ),
        other => return Err(format!("{other:?}").into()),
    };
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    let remaining = account.limits().work - account.usage().work;
    account.ledger.work(remaining, crate::loan::AT)?;
    crate::loan::fault::reset_reads();
    assert!(!format!("{reader:?}").contains("NaN"));
    assert_eq!(crate::loan::fault::reads(), 0);
    assert_eq!(account.usage().work, account.limits().work);
    drop(tile);
    let mut scanner = crate::ExecutionAccount::new(Budget::new(Limits::default())?)?;
    empty(&mut heap, &mut scanner)
}
