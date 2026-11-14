// SPDX-License-Identifier: AGPL-3.0-only
//! A genuine prepared publication cannot be rebound as an ordinary radius endpoint view.

use super::{Fixture, TestResult, candidate, fixture, published, ready};
use crate::loan::fault;
use crate::{EndpointReadError, HostChannel, LoanError, Residence};

/// Prepared binary64 publication carries all ordinary roles plus its original endpoint owners.
/// ´test:unit:loan-endpoint-view-prepared-publication-refusal´
#[test]
fn loan_endpoint_view_prepared_publication_refusal() -> TestResult {
    let Fixture {
        mut heap,
        mut account,
        request,
    } = fixture(1, 1, [-2.0, 0.0], 2, false)?;
    let demand = request.demand;
    let owner = candidate(
        ready(heap.request_pointwise_expm1(request, &mut account))?,
        &mut account,
    )?;
    let (tile, receipt) = published(heap.publish_application(owner, &mut account))?;
    let mask = receipt.filled(HostChannel::Center).ok_or("mask")?.native();
    let source = tile.reader(&heap, &mut account)?;
    assert!(source.bundle.channels.iter().all(Option::is_some));
    assert!(source.bundle.endpoints.is_some());
    let view = source.masked_view(heap.mask(mask, mask.layout())?, &mut account)?;
    fault::reset_reads();
    let injection = fault::Injection::after(0);
    assert!(matches!(
        view.into_endpoint_view(&mut account),
        Err(EndpointReadError::Loan(
            LoanError::UnsupportedEndpointRepresentation
        ))
    ));
    assert_eq!([fault::reads(), fault::Injection::attempts()], [0; 2]);
    drop(injection);
    drop(tile);
    heap.retire_mask(demand, &mut account.ledger)?;
    assert!(matches!(
        heap.reconcile_detached(&mut account, 100_000, None),
        crate::ScanOutcome::Complete
    ));
    assert!(matches!(
        heap.reclaim(&mut account, 100_000, None),
        crate::ScanOutcome::Complete
    ));
    assert!(matches!(
        heap.reconcile_detached(&mut account, 100_000, None),
        crate::ScanOutcome::Complete
    ));
    assert!(matches!(
        heap.residence(&mut account, 100_000)?,
        Residence::Empty { .. }
    ));
    Ok(())
}
