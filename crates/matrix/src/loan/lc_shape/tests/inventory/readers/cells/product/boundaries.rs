// SPDX-License-Identifier: AGPL-3.0-only
//! Literal metadata and observation visits are reserved before any value is touched.

use super::{TestResult, publish, setup};
use crate::LcProductError;
use crate::{ExecutionAccount, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Binding costs 1+2+1=4; one complete observation costs 2+7*5=37 and loads 15 scalars.
/// Both refused neighbours spend zero Work and perform zero native allocations or loads.
/// ´test:unit:loan-lc-input-product-work-boundaries´
#[test]
fn loan_lc_input_product_work_boundaries() -> TestResult {
    for binding in [false, true] {
        work::<f32>(binding)?;
        work::<f64>(binding)?;
    }
    Ok(())
}
fn work<P: Scalar + 'static>(binding: bool) -> TestResult {
    let required = if binding { 4 } else { 37 };
    for remaining in [required - 1, required] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
        let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
        let references = members.references();
        let readers = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?;
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let injection = crate::loan::fault::Injection::after(0);
        let before;
        let result = if binding {
            before = limit(&mut account, remaining)?;
            readers.into_product([0, 1], &mut account).map(|_| ())
        } else {
            let cells = readers.into_product([0, 1], &mut account)?;
            before = limit(&mut account, remaining)?;
            cells.read_cell(3, &mut account).map(|_| ())
        };
        let success = remaining == required;
        if success {
            result?;
        } else {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::Budget(_)))
            ));
        }
        assert_eq!(
            account.usage().work - before,
            if success { required } else { 0 }
        );
        let observed = success && !binding;
        assert_eq!(crate::loan::fault::reads(), if observed { 15 } else { 0 });
        assert_eq!(crate::loan::fault::scalars(), usize::from(observed));
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

/// Binding resolves native planes once; each later complete-cell span prepays 37 Work.
/// ´test:unit:loan-lc-input-product-phase-receipts´
#[test]
fn loan_lc_input_product_phase_receipts() -> TestResult {
    phases::<f32>()?;
    phases::<f64>()
}
fn phases<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
    let references = members.references();
    let readers = inventory
        .readers(&heap, &references, &mut account)?
        .into_cells(&mut account)?;
    let (result, phases) = super::super::super::super::super::observation::recorded(|| {
        let cells = readers.into_product([0, 1], &mut account)?;
        cells.read_cell(0, &mut account)?;
        cells.read_cell(11, &mut account)?;
        Ok::<_, LcProductError>(())
    })?;
    result?;
    assert_eq!(
        phases,
        vec![
            ("LC input product binding".to_owned(), 4),
            ("LC input product evaluation".to_owned(), 37),
            ("LC input product evaluation".to_owned(), 37)
        ]
    );
    drop(references);
    members.finish(&mut heap, &mut account)
}
