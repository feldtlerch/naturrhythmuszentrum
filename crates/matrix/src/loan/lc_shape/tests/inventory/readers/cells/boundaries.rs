// SPDX-License-Identifier: AGPL-3.0-only
//! Literal metadata and observation visits are reserved before any value is touched.

use super::{TestResult, publish, setup};
use crate::{ExecutionAccount, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Binding costs 1+5*5=26; one complete observation costs 1+7*5=36 and loads 15 scalars.
/// Both refused neighbours spend zero Work and perform zero native allocations or loads.
/// ´test:unit:loan-lc-input-cells-work-boundaries´
#[test]
fn loan_lc_input_cells_work_boundaries() -> TestResult {
    for binding in [false, true] {
        work::<f32>(binding)?;
        work::<f64>(binding)?;
    }
    Ok(())
}
fn work<P: Scalar + 'static>(binding: bool) -> TestResult {
    let required = if binding { 26 } else { 36 };
    for remaining in [required - 1, required] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
        let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
        let references = members.references();
        let readers = inventory.readers(&heap, &references, &mut account)?;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let before;
        let result = if binding {
            before = limit(&mut account, remaining)?;
            readers.into_cells(&mut account).map(|_| ())
        } else {
            let cells = readers.into_cells(&mut account)?;
            before = limit(&mut account, remaining)?;
            cells.read_cell(3, &mut account).map(|_| ())
        };
        let success = remaining == required;
        if success {
            result?;
        } else {
            assert!(matches!(result, Err(LoanError::Budget(_))));
        }
        assert_eq!(
            account.usage().work - before,
            if success { required } else { 0 }
        );
        let observed = success && !binding;
        assert_eq!(crate::loan::fault::reads(), if observed { 15 } else { 0 });
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        drop(references);
        let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        members.finish(&mut heap, &mut scanner)?;
    }
    Ok(())
}
fn limit(account: &mut ExecutionAccount, remaining: u64) -> TestResult<u64> {
    let spend = account.limits().work - account.usage().work - remaining;
    account.ledger.work(spend, crate::loan::AT)?;
    Ok(account.usage().work)
}

/// Binding resolves native planes once; each later complete-cell span prepays 36 Work.
/// ´test:unit:loan-lc-input-cells-phase-receipts´
#[test]
fn loan_lc_input_cells_phase_receipts() -> TestResult {
    phases::<f32>()?;
    phases::<f64>()
}
fn phases<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
    let references = members.references();
    let readers = inventory.readers(&heap, &references, &mut account)?;
    let (result, phases) = super::super::super::super::observation::recorded(|| {
        let cells = readers.into_cells(&mut account)?;
        cells.read_cell(0, &mut account)?;
        cells.read_cell(11, &mut account)?;
        Ok::<_, LoanError>(())
    })?;
    result?;
    assert_eq!(
        phases,
        vec![
            ("LC input cell binding".to_owned(), 26),
            ("LC input cell observation".to_owned(), 36),
            ("LC input cell observation".to_owned(), 36)
        ]
    );
    drop(references);
    members.finish(&mut heap, &mut account)
}
