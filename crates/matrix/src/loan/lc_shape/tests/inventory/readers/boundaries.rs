// SPDX-License-Identifier: AGPL-3.0-only
//! Five original positions cost eleven metadata visits plus five current-reader tariffs.

use super::{TestResult, publish, setup};
use crate::{ExecutionAccount, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Design inventory: 1+2*5=11 metadata units and 5*(3+9+4+3+3+3)=125 native units.
/// 135/136 pin complete public admission; 10/11 pin the allocation-free metadata phase.
/// ´test:unit:loan-lc-inventory-readers-work-boundaries´
#[test]
fn loan_lc_inventory_readers_work_boundaries() -> TestResult {
    for metadata in [false, true] {
        work::<f32>(metadata)?;
        work::<f64>(metadata)?;
    }
    Ok(())
}
fn work<P: Scalar + 'static>(metadata: bool) -> TestResult {
    let required = if metadata { 11 } else { 136 };
    for remaining in [required - 1, required] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let members = publish(&inventory, &axes, &mut heap, &mut account, 1, 1)?;
        let references = members.references();
        let spend = account.limits().work - account.usage().work - remaining;
        account.ledger.work(spend, crate::loan::AT)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = if metadata {
            inventory.membership(&references, &mut account).map(|_| ())
        } else {
            inventory
                .readers(&heap, &references, &mut account)
                .map(|_| ())
        };
        if remaining == required {
            result?;
            assert_eq!(account.usage().work - before, required);
        } else {
            assert!(matches!(result, Err(LoanError::Budget(_))));
            // Fixed metadata prepay refuses atomically; the last reader refuses its final 3 units.
            assert_eq!(
                account.usage().work - before,
                if metadata { 0 } else { 133 }
            );
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
        let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        members.finish(&mut heap, &mut scanner)?;
    }
    Ok(())
}

/// Whole metadata admission precedes the five independently charged current-reader phases.
/// ´test:unit:loan-lc-inventory-readers-phase-receipts´
#[test]
fn loan_lc_inventory_readers_phase_receipts() -> TestResult {
    phases::<f32>()?;
    phases::<f64>()
}
fn phases<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let members = publish(&inventory, &axes, &mut heap, &mut account, 1, 1)?;
    let references = members.references();
    let (result, phases) = super::super::super::observation::recorded(|| {
        inventory.readers(&heap, &references, &mut account)
    })?;
    result?;
    assert_eq!(phases.len(), 6);
    assert_eq!(phases[0], ("LC inventory membership".to_owned(), 11));
    for phase in &phases[1..] {
        assert_eq!(phase, &("current reader".to_owned(), 25));
    }
    drop(references);
    members.finish(&mut heap, &mut account)
}
