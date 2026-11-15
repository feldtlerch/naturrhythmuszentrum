// SPDX-License-Identifier: AGPL-3.0-only
//! Complete means all original positions; same-valued repeats cannot exchange authority.

use super::{TestResult, fixture, setup, source};
use crate::{ExecutionAccount, LcInputInventory, LcShapeError, LcShapeRequest, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Too few and too many slots refuse after one count visit, before declarations or native work.
/// Optional energy is not invented when the original question omits it.
/// ´test:unit:loan-lc-input-inventory-completeness-refusals´
#[test]
fn loan_lc_input_inventory_completeness_refusals() -> TestResult {
    completeness::<f32>()?;
    completeness::<f64>()
}
fn completeness<P: Scalar>() -> TestResult {
    let mut budget = Budget::new(Limits::default())?;
    let question = fixture::inventory_question(false, false, &mut budget)?;
    let mut account = ExecutionAccount::new(budget)?;
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    let before = account.usage().work;
    for result in [
        LcInputInventory::<P, 0>::declare(&question, [2, 3, 2], &mut account).map(|_| ()),
        LcInputInventory::<P, 2>::declare(&question, [2, 3, 2], &mut account).map(|_| ()),
        LcInputInventory::<P, 4>::declare(&question, [2, 3, 2], &mut account).map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(LcShapeError::Loan(LoanError::UnsupportedContract))
        ));
    }
    assert_eq!(account.usage().work - before, 3);
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    drop(injection);
    let inventory = LcInputInventory::<P, 3>::declare(&question, [2, 3, 2], &mut account)?;
    assert!(inventory.input(3).is_none());
    for (index, role) in super::ROLES[..3].iter().enumerate() {
        let schema = inventory.input(index).ok_or("missing original input")?;
        assert_eq!(
            &schema.question().relation().inputs()[index].quantity(),
            role
        );
        assert_eq!(schema.input_index(), index);
    }
    Ok(())
}

/// Repeated L constraints have identical descriptors and values but distinct source/output symbols.
/// ´test:unit:loan-lc-input-inventory-repeated-role-refusals´
#[test]
fn loan_lc_input_inventory_repeated_role_refusals() -> TestResult {
    repeated::<f32>()?;
    repeated::<f64>()
}
fn repeated<P: Scalar + 'static>() -> TestResult {
    for destination in [false, true] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let original = inventory.input(0).ok_or("first L")?;
        let repeated = inventory.input(4).ok_or("repeated L")?;
        assert_ne!(original.specialization_key(), repeated.specialization_key());
        let source_schema = if destination { original } else { repeated };
        let input = source(source_schema, &axes, &mut heap, &mut account, 1)?;
        let mut request = fixture::request(
            original,
            &axes,
            input.into_pin(),
            &[0, 3, 5],
            &mut heap,
            &mut account,
        )?;
        if destination {
            request.destination = repeated.destination(&axes, &mut account)?;
        }
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let request = match original.request(&mut heap, &axes, request, &mut account) {
            LcShapeRequest::Refused(
                crate::ApplicationError::Loan(LoanError::TargetMismatch),
                request,
            ) => request,
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(account.usage().work - before, 16);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        let demand = request.demand;
        drop(request);
        fixture::retire(&mut heap, &mut account, demand)?;
    }
    Ok(())
}
