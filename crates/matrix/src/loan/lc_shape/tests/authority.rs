// SPDX-License-Identifier: AGPL-3.0-only
//! Pending, altered and stale original occupations cannot inherit LC application authority.

use super::{fixture, source};
use crate::{HandbackOutcome, HostChannel, LcShapeRequest, LoanError};
use fixture::{TestResult, request, setup};
use nrz_scalar::Scalar;

/// A pending LC input cannot be pinned; a changed nonce refuses before inspecting host bytes.
/// ´test:unit:loan-lc-shape-pending-handback-authority´
#[test]
fn loan_lc_shape_pending_handback_authority() -> TestResult {
    pending::<f32>()?;
    pending::<f64>()
}
fn pending<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, schema, axes) = setup::<P>()?;
    let declaration = schema.source(&axes, &mut account)?;
    let mut loan = fixture::granted(heap.request_loan(declaration, &mut account))?;
    let ticket = loan.ticket();
    loan.write_component(HostChannel::Center, 0, 0, P::ONE, &mut account)?;
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        heap.acquire_pin::<P>(ticket, &mut account),
        Err(LoanError::Heap(crate::HeapError::OwnershipPending))
    ));
    loan.core.identity.nonce += 1;
    let mut loan = match heap.handback(loan, &mut account) {
        HandbackOutcome::ReturnRejected(LoanError::WrongLoan, loan) => loan,
        other => return Err(format!("{other:?}").into()),
    };
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    drop(injection);
    loan.core.identity.nonce = ticket.identity.nonce;
    assert_eq!(loan.ticket(), ticket);
    assert!(matches!(
        heap.cancel_loan(loan, &mut account),
        crate::CancelOutcome::Cancelled(_)
    ));
    fixture::empty(&mut heap, &mut account)
}

/// Altered companion inventory and unchanged-centre stale input refuse before allocation or reads.
/// ´test:unit:loan-lc-shape-companion-and-stale-refusals´
#[test]
fn loan_lc_shape_companion_and_stale_refusals() -> TestResult {
    current::<f32>()?;
    current::<f64>()
}
fn current<P: Scalar + 'static>() -> TestResult {
    for stale in [false, true] {
        let (mut heap, mut account, schema, axes) = setup::<P>()?;
        let input = source(&schema, &axes, &mut heap, &mut account, 0)?;
        let ticket = input.pin().ticket()?;
        let candidate = request(
            &schema,
            &axes,
            input.into_pin(),
            &[0, 3, 5],
            &mut heap,
            &mut account,
        )?;
        if !stale {
            companion(&schema);
            let demand = candidate.demand;
            drop(candidate);
            fixture::retire(&mut heap, &mut account, demand)?;
            continue;
        }
        set_filled(&mut heap, ticket, false)?;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let candidate = match schema.request(&mut heap, &axes, candidate, &mut account) {
            LcShapeRequest::Refused(crate::ApplicationError::Loan(error), candidate) => {
                assert!(matches!(error, LoanError::StaleDependency));
                candidate
            }
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        set_filled(&mut heap, ticket, true)?;
        let demand = candidate.demand;
        drop(candidate);
        fixture::retire(&mut heap, &mut account, demand)?;
    }
    Ok(())
}
/// A private stale-authority fault changes no numerical centre or companion bytes.
fn set_filled(heap: &mut crate::Heap, ticket: crate::LoanTicket, filled: bool) -> TestResult {
    let crate::heap::descriptors::DescriptorRecord::Managed(record) =
        heap.descriptors.slots.get_mut(ticket.identity.root)?
    else {
        return Err("managed source".into());
    };
    record.filled = filled;
    Ok(())
}

/// The original immutable companion inventory refuses omission at the bindings boundary.
fn companion<P: Scalar>(schema: &super::LcInputShape<P>) {
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        nrz_bindings::BindingSymbols::check(
            schema.symbols[1].value().clone(),
            schema.symbols[1].bound().cloned(),
            None
        ),
        Err(nrz_bindings::AdmissionError::CompanionMismatch)
    ));
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    drop(injection);
}

/// Abandonment retains its original delivery obligation even after numerical elements disappear.
/// ´test:unit:loan-lc-shape-abandoned-outcome-delivery´
#[test]
fn loan_lc_shape_abandoned_outcome_delivery() -> TestResult {
    for evaluated in [false, true] {
        abandoned::<f32>(evaluated)?;
        abandoned::<f64>(evaluated)?;
    }
    Ok(())
}
fn abandoned<P: Scalar + 'static>(evaluated: bool) -> TestResult {
    let (mut heap, mut account, schema, axes) = setup::<P>()?;
    let input = source(&schema, &axes, &mut heap, &mut account, 0)?;
    let candidate = request(
        &schema,
        &axes,
        input.into_pin(),
        &[0, 3, 5],
        &mut heap,
        &mut account,
    )?;
    let demand = candidate.demand;
    let mut application = match schema.request(&mut heap, &axes, candidate, &mut account) {
        LcShapeRequest::Ready(application) => application,
        other @ LcShapeRequest::Refused(..) => return Err(format!("{other:?}").into()),
    };
    if evaluated {
        application.evaluate(&mut account)?;
    }
    let ticket = application.ticket()?;
    drop(application);
    assert!(matches!(
        heap.residence(&mut account, 100_000)?,
        crate::Residence::Pending {
            elements: Some([0, 0]),
            ..
        }
    ));
    assert!(
        matches!(heap.take_producer_outcome(ticket, &mut account)?, crate::ProducerOutcome::Absent(receipt) if receipt.reason() == crate::AbsenceReason::Abandoned)
    );
    assert_eq!(
        heap.take_producer_outcome(ticket, &mut account)?,
        crate::ProducerOutcome::AlreadyDelivered
    );
    fixture::retire(&mut heap, &mut account, demand)
}
