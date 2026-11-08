// SPDX-License-Identifier: AGPL-3.0-only
//! Private corruption controls authenticate capabilities before inspecting their numerical owners.

use super::{TestResult, empty, fixture, granted};
use crate::loan::{
    CancelOutcome, HandbackOutcome, HostChannel, HostTileDeclaration, LoanError, LoanRequest,
    ProducerOutcome, Residence, ScanOutcome,
};
use crate::{Coverage, Extent, Heap, HeapLimits, Layout};
use nrz_bindings::{BindingSymbols, Budget, Descriptor, Limits};
use nrz_scalar::Scalar;

/// Every inaccessible locator substitution returns the same core and preserves the rightful group.
fn corrupt_returns<P: Scalar + 'static>() -> TestResult {
    for case in 0..7 {
        let (mut heap, mut account, declaration) =
            fixture::<P>(1, 1, [true, true], Limits::default())?;
        let mut loan = granted(heap.request_loan(declaration, &mut account))?;
        let original = loan.core.identity;
        let pointer = std::ptr::from_ref(loan.core.as_ref());
        match case {
            0 => loan.core.identity.root.generation += 1,
            1 => loan.core.identity.channels.swap(1, 2),
            2 => loan.core.identity.layout.key.generation += 1,
            3 => loan.core.identity.mask.key.generation += 1,
            4 => loan.core.identity.receipt += 1,
            5 => loan.core.identity.declaration += 1,
            _ => loan.core.identity.account += 1,
        }
        let mut loan = match heap.handback(loan, &mut account) {
            HandbackOutcome::ReturnRejected(_, loan) => loan,
            other => return Err(format!("corruption {case}: {other:?}").into()),
        };
        assert_eq!(std::ptr::from_ref(loan.core.as_ref()), pointer);
        assert_eq!(heap.loans.group(original)?.identity, original);
        assert!(matches!(
            heap.loans.group(original)?.outcome,
            ProducerOutcome::Pending
        ));
        loan.core.identity = original;
        assert!(matches!(
            heap.cancel_loan(loan, &mut account),
            CancelOutcome::Cancelled(_)
        ));
        empty(&mut heap, &mut account)?;
    }
    Ok(())
}
/// Stale generations, swapped companions and changed target/attachment identities cannot return.
/// ´test:unit:loan-corrupt-returns-preserve-rightful-producer´
#[test]
fn loan_corrupt_returns_preserve_rightful_producer() -> TestResult {
    corrupt_returns::<f32>()?;
    corrupt_returns::<f64>()
}

/// Independent-versus-derived source evidence is checked again after authenticated ownership return.
/// ´test:unit:loan-invalid-dependency-witness-is-repairable´
#[test]
fn loan_invalid_dependency_witness_is_repairable() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(1, 1, [false, false], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    let original = loan.core.declaration.symbols.clone();
    let mut declarations = Budget::new(Limits::default())?;
    loan.core.declaration.symbols = original.copy_result([1, 1], &mut declarations)?;
    let mut loan = match heap.handback(loan, &mut account) {
        HandbackOutcome::Repair(LoanError::StaleDependency, loan) => loan,
        other => return Err(format!("{other:?}").into()),
    };
    loan.core.declaration.symbols = original;
    let tile = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, _) => tile,
        other => return Err(format!("{other:?}").into()),
    };
    drop(tile);
    empty(&mut heap, &mut account)
}

/// Unsupported shapes and source relations refuse before any allocation fault boundary is reached.
/// ´test:unit:loan-adapter-refusals-precede-native-allocation´
#[test]
fn loan_adapter_refusals_precede_native_allocation() -> TestResult {
    let heap = Heap::new(HeapLimits::default())?;
    let mut budget = Budget::new(Limits::default())?;
    let axis = heap.create_cell_axis(1, &mut budget)?;
    let mut account = crate::ExecutionAccount::new(Budget::new(Limits::default())?)?;
    let original =
        BindingSymbols::declare(&Descriptor::ratio(), [1, 1], [false, false], &mut budget)?;
    let derived = original.copy_result([1, 1], &mut budget)?;
    let wrong_shape =
        BindingSymbols::declare(&Descriptor::ratio(), [2, 1], [false, false], &mut budget)?;
    for symbols in [derived, wrong_shape] {
        let injection = crate::loan::fault::Injection::after(0);
        assert!(matches!(
            HostTileDeclaration::<f64>::check(
                symbols,
                axis,
                Coverage::RepresentedInputs,
                &mut account
            ),
            Err(LoanError::UnsupportedContract)
        ));
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
    }
    assert!(matches!(
        HostTileDeclaration::<f32>::check(original, axis, Coverage::DeclaredBounds, &mut account),
        Err(LoanError::UnsupportedContract)
    ));
    Ok(())
}

/// Current authority can become stale while its immutable historical owner remains readable.
/// ´test:unit:loan-current-and-historical-reader-authority´
#[test]
fn loan_current_and_historical_reader_authority() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(1, 1, [false, false], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    loan.write_cell(HostChannel::Center, 0, &[13.0], &mut account)?;
    let (tile, receipt) = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, receipt) => (tile, receipt),
        other => return Err(format!("{other:?}").into()),
    };
    let native = receipt
        .filled(HostChannel::Center)
        .ok_or("filled")?
        .native();
    assert!(matches!(
        heap.replace_cell_layout(
            native.layout(),
            Layout::contiguous(Extent::new(1, 1)?)?,
            &mut account.ledger
        ),
        Err(crate::MaskError::Heap(crate::HeapError::PinnedUses))
    ));
    let key = receipt.ticket().identity.root;
    let record = heap.descriptors.slots.get_mut(key)?;
    let crate::heap::descriptors::DescriptorRecord::Managed(record) = record else {
        return Err("managed record".into());
    };
    record.filled = false;
    assert!(matches!(
        tile.reader(&heap, &mut account),
        Err(LoanError::StaleDependency)
    ));
    assert_eq!(
        tile.pin().historical_reader(&mut account)?.read_component(
            HostChannel::Center,
            0,
            0,
            &mut account
        )?,
        13.0
    );
    let crate::heap::descriptors::DescriptorRecord::Managed(record) =
        heap.descriptors.slots.get_mut(key)?
    else {
        return Err("managed record".into());
    };
    record.filled = true;
    let mut other_account = crate::ExecutionAccount::new(Budget::new(Limits::default())?)?;
    assert!(matches!(
        tile.reader(&heap, &mut other_account),
        Err(LoanError::ForeignAccount)
    ));
    let other_pin = heap.acquire_pin::<f64>(receipt.ticket(), &mut other_account)?;
    assert_eq!(
        other_pin
            .historical_reader(&mut other_account)?
            .read_component(HostChannel::Center, 0, 0, &mut other_account)?,
        13.0
    );
    drop(tile);
    assert!(matches!(
        heap.residence(&mut account, 1000)?,
        Residence::Pending { .. }
    ));
    drop(other_pin);
    empty(&mut heap, &mut account)
}

/// Every fallible staging prefix leaves the original core and witnesses available for cancellation.
/// ´test:unit:loan-all-publication-prefixes-preserve-repair´
#[test]
fn loan_all_publication_prefixes_preserve_repair() -> TestResult {
    for prefix in 0..3 {
        let (mut heap, mut account, declaration) =
            fixture::<f64>(1, 1, [true, true], Limits::default())?;
        let mut loan = granted(heap.request_loan(declaration, &mut account))?;
        for channel in [HostChannel::Center, HostChannel::Total, HostChannel::Error] {
            loan.write_cell(channel, 0, &[1.0], &mut account)?;
        }
        let ticket = loan.ticket();
        let witness = std::rc::Rc::downgrade(&loan.core.witness);
        let injection = crate::loan::fault::Injection::after(prefix);
        let loan = match heap.handback(loan, &mut account) {
            HandbackOutcome::Repair(LoanError::AllocationRefused, loan) => loan,
            other => return Err(format!("prefix {prefix}: {other:?}").into()),
        };
        assert_eq!(crate::loan::fault::Injection::attempts(), prefix + 1);
        let [acquired, retained, destroyed] = crate::loan::fault::Injection::prefix();
        assert_eq!(acquired, prefix);
        assert_eq!(destroyed + retained, acquired);
        drop(injection);
        assert_eq!(loan.ticket(), ticket);
        assert_eq!(witness.strong_count(), 1);
        assert!(matches!(
            heap.take_producer_outcome(ticket, &mut account)?,
            ProducerOutcome::Pending
        ));
        let remaining = account.limits().work - account.usage().work;
        account.ledger.work(remaining, crate::loan::AT)?;
        assert!(matches!(
            heap.cancel_loan(loan, &mut account),
            CancelOutcome::Cancelled(_)
        ));
        assert_eq!(heap.loans.elements()?, [0, 0]);
    }
    Ok(())
}

/// Fixed-capacity cycles reuse tables and monotonically change occupations without a replay log.
/// ´test:unit:loan-repeated-release-reuse-and-stale-cursor´
#[test]
fn loan_repeated_release_reuse_and_stale_cursor() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(2, 1, [false, false], Limits::default())?;
    let symbols = declaration.symbols().clone();
    let axis = declaration.axis;
    let loan = granted(heap.request_loan(declaration, &mut account))?;
    let old = loan.ticket();
    let cursor = match heap.reconcile_detached(&mut account, 0, None) {
        ScanOutcome::Pending(cursor) => cursor,
        other => return Err(format!("{other:?}").into()),
    };
    assert!(matches!(
        heap.cancel_loan(loan, &mut account),
        CancelOutcome::Cancelled(_)
    ));
    empty(&mut heap, &mut account)?;
    let mut retained = None;
    for _ in 0..32 {
        let declaration = HostTileDeclaration::check(
            symbols.clone(),
            axis,
            Coverage::RepresentedInputs,
            &mut account,
        )?;
        let mut loan = granted(heap.request_loan(declaration, &mut account))?;
        assert_ne!(loan.ticket(), old);
        assert!(matches!(
            heap.probe_loan(old, &mut account),
            Err(LoanError::StaleOccupation)
        ));
        assert!(matches!(
            heap.reconcile_detached(&mut account, 1, Some(cursor)),
            ScanOutcome::Pending(_)
        ));
        loan.write_cell(HostChannel::Center, 1, &[23.0_f64], &mut account)?;
        let (tile, receipt) = match heap.handback(loan, &mut account) {
            HandbackOutcome::Published(tile, receipt) => (tile, receipt),
            other => return Err(format!("{other:?}").into()),
        };
        let payload = tile.pin().record.payload.as_ref().ok_or("pin")?;
        let count = std::rc::Rc::strong_count(&payload.bundle);
        let locators = vec![receipt; 128];
        assert_eq!(locators[0], locators[127]);
        assert_eq!(std::rc::Rc::strong_count(&payload.bundle), count);
        drop(tile);
        empty(&mut heap, &mut account)?;
        let bytes = heap.resource_bytes()?;
        if let Some(expected) = retained {
            assert_eq!(bytes, expected);
        }
        retained = Some(bytes);
    }
    assert_eq!(heap.loans.groups.len(), 1);
    assert_eq!(heap.loans.pins.len(), 1);
    assert_eq!(heap.loans.debits.len(), 9);
    Ok(())
}

/// Forgetting a capability cannot create an abandonment transition or free its occupied slots.
/// ´test:unit:loan-forgotten-owner-remains-occupied´
#[test]
fn loan_forgotten_owner_remains_occupied() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(1, 1, [false, false], Limits::default())?;
    let symbols = declaration.symbols().clone();
    let axis = declaration.axis;
    heap.limits = HeapLimits::new(1, 1, 4096);
    let loan = granted(heap.request_loan(declaration, &mut account))?;
    let ticket = loan.ticket();
    std::mem::forget(loan);
    assert!(matches!(
        heap.residence(&mut account, 1000)?,
        Residence::Pending { .. }
    ));
    assert!(matches!(
        heap.take_producer_outcome(ticket, &mut account)?,
        ProducerOutcome::Pending
    ));
    let declaration = HostTileDeclaration::<f64>::check(
        symbols,
        axis,
        Coverage::RepresentedInputs,
        &mut account,
    )?;
    assert!(matches!(
        heap.request_loan(declaration, &mut account),
        LoanRequest::Refused(LoanError::Heap(crate::HeapError::Capacity))
    ));
    assert_eq!(heap.loans.elements()?, [0, 1]);
    Ok(())
}

/// The detached safe owner is independent of the lifetime of its original destination heap.
/// ´test:unit:loan-owner-survives-heap-drop-without-publication-authority´
#[test]
fn loan_owner_survives_heap_drop_without_publication_authority() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f32>(1, 1, [false, false], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    let witness = std::rc::Rc::downgrade(&loan.core.witness);
    drop(heap);
    loan.write_cell(HostChannel::Center, 0, &[31.0], &mut account)?;
    let mut foreign = Heap::new(HeapLimits::default())?;
    let loan = match foreign.handback(loan, &mut account) {
        HandbackOutcome::ReturnRejected(LoanError::ForeignOwner, loan) => loan,
        other => return Err(format!("{other:?}").into()),
    };
    assert_eq!(witness.strong_count(), 1);
    drop(loan);
    assert_eq!(witness.strong_count(), 0);
    Ok(())
}

/// Invalid selected native scalars return the original core for repair or fully prepaid cancellation.
fn invalid_native<P: Scalar + 'static>(value: P, channel: HostChannel, cancel: bool) -> TestResult {
    let (mut heap, mut account, declaration) = fixture::<P>(1, 1, [true, true], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    let ticket = loan.ticket();
    for target in HostChannel::ALL {
        loan.write_cell(
            target,
            0,
            &[if target == channel { value } else { P::ONE }],
            &mut account,
        )?;
    }
    let mut loan = match heap.handback(loan, &mut account) {
        HandbackOutcome::Repair(
            LoanError::InvalidPayload {
                channel: actual, ..
            },
            loan,
        ) if actual == channel => loan,
        other => return Err(format!("{other:?}").into()),
    };
    assert_eq!(loan.ticket(), ticket);
    if cancel {
        let remaining = account.limits().work - account.usage().work;
        account.ledger.work(remaining, crate::loan::AT)?;
        assert!(matches!(
            heap.cancel_loan(loan, &mut account),
            CancelOutcome::Cancelled(_)
        ));
        assert_eq!(heap.loans.elements()?, [0, 0]);
    } else {
        loan.write_cell(channel, 0, &[P::ONE], &mut account)?;
        let tile = match heap.handback(loan, &mut account) {
            HandbackOutcome::Published(tile, _) => tile,
            other => return Err(format!("{other:?}").into()),
        };
        drop(tile);
        empty(&mut heap, &mut account)?;
    }
    Ok(())
}
/// Both formats refuse nonfinite centers and invalid total/error radii without losing the capability.
/// ´test:unit:loan-native-payload-refusal-matrix-repair-and-cancel´
#[test]
fn loan_native_payload_refusal_matrix_repair_and_cancel() -> TestResult {
    for cancel in [false, true] {
        invalid_native(f32::NAN, HostChannel::Center, cancel)?;
        invalid_native(f64::NAN, HostChannel::Center, cancel)?;
        invalid_native(-1.0_f32, HostChannel::Total, cancel)?;
        invalid_native(-1.0_f64, HostChannel::Total, cancel)?;
        invalid_native(-1.0_f32, HostChannel::Error, cancel)?;
        invalid_native(-1.0_f64, HostChannel::Error, cancel)?;
        invalid_native(f32::INFINITY, HostChannel::Total, cancel)?;
        invalid_native(f64::INFINITY, HostChannel::Total, cancel)?;
    }
    Ok(())
}

/// Another ledger cannot reset a detached continuation or consume its original cancellation receipt.
/// ´test:unit:loan-foreign-account-cannot-fund-continuation´
#[test]
fn loan_foreign_account_cannot_fund_continuation() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(1, 1, [false, false], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    let mut foreign = crate::ExecutionAccount::new(Budget::new(Limits::default())?)?;
    let original_work = account.usage().work;
    assert!(matches!(
        loan.write_cell(HostChannel::Center, 0, &[3.0], &mut foreign),
        Err(LoanError::ForeignAccount)
    ));
    let loan = match heap.handback(loan, &mut foreign) {
        HandbackOutcome::ReturnRejected(LoanError::ForeignAccount, loan) => loan,
        other => return Err(format!("{other:?}").into()),
    };
    let loan = match heap.cancel_loan(loan, &mut foreign) {
        CancelOutcome::Rejected(error, loan) => {
            assert!(matches!(error, LoanError::ForeignAccount));
            loan
        }
        other @ CancelOutcome::Cancelled(_) => return Err(format!("{other:?}").into()),
    };
    assert_eq!(account.usage().work, original_work);
    assert!(matches!(
        heap.cancel_loan(loan, &mut account),
        CancelOutcome::Cancelled(_)
    ));
    assert_eq!(account.usage().work, original_work);
    empty(&mut heap, &mut account)
}

/// Explicit release spends its original entitlement even after the pin's account exhausts Work.
/// ´test:unit:loan-pin-release-is-prepaid-and-authenticated´
#[test]
fn loan_pin_release_is_prepaid_and_authenticated() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(0, 1, [false, false], Limits::default())?;
    let loan = granted(heap.request_loan(declaration, &mut account))?;
    let tile = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, _) => tile,
        other => return Err(format!("{other:?}").into()),
    };
    let pin = tile.into_pin();
    let mut foreign = crate::ExecutionAccount::new(Budget::new(Limits::default())?)?;
    let pin = match heap.release_pin(pin, &mut foreign) {
        Err((LoanError::ForeignAccount, pin)) => pin,
        other => return Err(format!("{other:?}").into()),
    };
    let remaining = account.limits().work - account.usage().work;
    account.ledger.work(remaining, crate::loan::AT)?;
    let before = account.usage().work;
    assert!(heap.release_pin(pin, &mut account).is_ok());
    assert_eq!(account.usage().work, before);
    empty(&mut heap, &mut foreign)
}

/// Instrumented numerical access distinguishes initialized selected cells from invalid hole sentinels.
/// ´test:unit:loan-empty-and-sparse-selected-loads-visit-no-holes´
#[test]
fn loan_empty_and_sparse_selected_loads_visit_no_holes() -> TestResult {
    for selected in [false, true] {
        let (mut heap, mut account, declaration) =
            fixture::<f64>(4, 2, [true, true], Limits::default())?;
        let mut loan = granted(heap.request_loan(declaration, &mut account))?;
        for cell in [0, 2] {
            loan.write_component(HostChannel::Center, cell, 0, f64::NAN, &mut account)?;
        }
        if selected {
            for cell in [1, 3] {
                for channel in HostChannel::ALL {
                    loan.write_cell(channel, cell, &[1.0, 1.0], &mut account)?;
                }
            }
        }
        crate::loan::fault::reset_reads();
        let tile = match heap.handback(loan, &mut account) {
            HandbackOutcome::Published(tile, _) => tile,
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(crate::loan::fault::reads(), if selected { 12 } else { 0 });
        crate::loan::fault::reset_reads();
        let reader = tile.reader(&heap, &mut account)?;
        for cell in 0..4 {
            let value = reader.read_component(HostChannel::Center, cell, 1, &mut account);
            if selected && cell % 2 == 1 {
                assert_eq!(value?, 1.0);
            } else {
                assert!(matches!(value, Err(LoanError::CoverageMismatch)));
            }
        }
        assert_eq!(crate::loan::fault::reads(), if selected { 2 } else { 0 });
        drop(tile);
        empty(&mut heap, &mut account)?;
    }
    Ok(())
}
