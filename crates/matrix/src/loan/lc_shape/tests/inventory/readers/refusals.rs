// SPDX-License-Identifier: AGPL-3.0-only
//! Individually valid copied tiles cannot impersonate a complete original inventory.

use super::{TestResult, fixture, member, publish, setup};
use crate::{ExecutionAccount, Heap, LcInputInventory, LcInputShape, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Missing/extra, swapped/repeated, standalone/foreign-inventory and differently qualified
/// tiles refuse in the eleven-unit membership phase, before native reader admission.
/// ´test:unit:loan-lc-inventory-readers-membership-refusals´
#[test]
fn loan_lc_inventory_readers_membership_refusals() -> TestResult {
    membership::<f32>()?;
    membership::<f64>()
}
fn membership<P: Scalar + 'static>() -> TestResult {
    for case in 0..7 {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let mut members = publish(&inventory, &axes, &mut heap, &mut account, 1, 1)?;
        let question = inventory.input(0).ok_or("first member")?.question();
        let standalone = LcInputShape::declare(question.clone(), 0, [2, 3, 2], &mut account)?;
        let foreign = LcInputInventory::<P, 5>::declare(question, [2, 3, 2], &mut account)?;
        let alternate = if case == 6 {
            fixture::axes(&heap, &mut account)?
        } else {
            axes
        };
        let extra = match case {
            4 => Some(member(
                &standalone,
                &axes,
                &mut heap,
                &mut account,
                &[0, 3, 5],
                1,
            )?),
            5 => Some(member(
                foreign.input(0).ok_or("foreign member")?,
                &axes,
                &mut heap,
                &mut account,
                &[0, 3, 5],
                1,
            )?),
            6 => Some(member(
                inventory.input(0).ok_or("original member")?,
                &alternate,
                &mut heap,
                &mut account,
                &[0, 3, 5],
                1,
            )?),
            _ => None,
        };
        if let Some((_, layout)) = &extra {
            members.layouts.push(*layout);
        }
        let mut references = members.references();
        match case {
            0 => references.truncate(4),
            1 => references.push(references[0]),
            2 => references.swap(0, 4),
            3 => references[4] = references[0],
            4..=6 => references[0] = &extra.as_ref().ok_or("replacement")?.0,
            _ => return Err("fixture case".into()),
        }
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = inventory
            .readers(&heap, &references, &mut account)
            .map(|_| ());
        match case {
            0 | 1 => assert!(matches!(result, Err(LoanError::UnsupportedContract))),
            6 => assert!(matches!(result, Err(LoanError::LayoutMismatch))),
            _ => assert!(matches!(result, Err(LoanError::TargetMismatch))),
        }
        assert_eq!(account.usage().work - before, 11);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        drop(references);
        drop(extra);
        // None of the rightful tiles was consumed or invalidated by the refused borrowed call.
        inventory.readers(&heap, &members.references(), &mut account)?;
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}

/// Complete membership cannot bypass account, heap or current publication authentication.
/// A historical member remains readable during a private stale-attachment fault, while the
/// complete current-reader capability refuses; restoring the attachment permits the same tiles.
/// ´test:unit:loan-lc-inventory-readers-current-authority-refusals´
#[test]
fn loan_lc_inventory_readers_current_authority_refusals() -> TestResult {
    authority::<f32>()?;
    authority::<f64>()
}
fn authority<P: Scalar + 'static>() -> TestResult {
    for case in 0..3 {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let members = publish(&inventory, &axes, &mut heap, &mut account, 1, 1)?;
        let foreign = Heap::new(crate::HeapLimits::default())?;
        let mut other = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        let ticket = members.tiles[4].receipt().ticket();
        if case == 2 {
            filled(&mut heap, ticket, false)?;
        }
        let references = members.references();
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = match case {
            0 => inventory.readers(&heap, &references, &mut other),
            1 => inventory.readers(&foreign, &references, &mut account),
            2 => inventory.readers(&heap, &references, &mut account),
            _ => return Err("fixture case".into()),
        }
        .map(|_| ());
        match case {
            0 => assert!(matches!(result, Err(LoanError::ForeignAccount))),
            1 => assert!(matches!(result, Err(LoanError::ForeignOwner))),
            2 => assert!(matches!(result, Err(LoanError::StaleDependency))),
            _ => return Err("fixture case".into()),
        }
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        drop(references);
        if case == 2 {
            assert!(
                members.tiles[4]
                    .reader(&mut account)?
                    .contains(3, &mut account)?
            );
            filled(&mut heap, ticket, true)?;
        }
        let references = members.references();
        let readers = inventory.readers(&heap, &references, &mut account)?;
        crate::loan::fault::reset_reads();
        assert!(matches!(
            readers.reader(0).ok_or("reader")?.read_component(
                crate::HostChannel::Center,
                0,
                0,
                &mut other
            ),
            Err(LoanError::ForeignAccount)
        ));
        assert_eq!(crate::loan::fault::reads(), 0);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}
fn filled(heap: &mut Heap, ticket: crate::LoanTicket, value: bool) -> TestResult {
    let crate::heap::descriptors::DescriptorRecord::Managed(record) =
        heap.descriptors.slots.get_mut(ticket.identity.root)?
    else {
        return Err("managed tile".into());
    };
    record.filled = value;
    Ok(())
}
