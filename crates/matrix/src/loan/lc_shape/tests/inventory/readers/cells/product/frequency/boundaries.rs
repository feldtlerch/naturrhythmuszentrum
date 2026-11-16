// SPDX-License-Identifier: AGPL-3.0-only
//! Literal metadata and observation visits are reserved before any value is touched.

use super::{TestResult, publish, setup};
use crate::LcProductError;
use crate::{ExecutionAccount, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Complete observation 36 plus Product, three signs, root, root sign and reciprocal = 43.
/// The refused neighbour spends zero Work and performs zero native allocations or loads.
/// ´test:unit:loan-lc-frequency-work-boundaries´
#[test]
fn loan_lc_frequency_work_boundaries() -> TestResult {
    work::<f32>()?;
    work::<f64>()?;
    Ok(())
}
fn work<P: Scalar + 'static>() -> TestResult {
    let required = 43;
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
        let cells = readers.into_product([0, 1], &mut account)?;
        let before = limit(&mut account, remaining)?;
        let result = cells
            .read_requested(3, crate::LcQuantityDemand::Omega, None, &mut account)
            .map(|_| ());
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
        let observed = success;
        assert_eq!(crate::loan::fault::reads(), if observed { 15 } else { 0 });
        assert_eq!(crate::loan::fault::scalars(), if observed { 3 } else { 0 });
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

/// Product binding is once per tile; each frequency span prepays 43 Work.
/// ´test:unit:loan-lc-frequency-phase-receipts´
#[test]
fn loan_lc_frequency_phase_receipts() -> TestResult {
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
    let (result, phases) = super::super::super::super::super::super::observation::recorded(|| {
        let cells = readers.into_product([0, 1], &mut account)?;
        cells.read_requested(0, crate::LcQuantityDemand::Omega, None, &mut account)?;
        cells.read_requested(11, crate::LcQuantityDemand::Omega, None, &mut account)?;
        Ok::<_, LcProductError>(())
    })?;
    result?;
    assert_eq!(
        phases,
        vec![
            ("LC input product binding".to_owned(), 4),
            ("LC requested quantities".to_owned(), 43),
            ("LC requested quantities".to_owned(), 43)
        ]
    );
    drop(references);
    members.finish(&mut heap, &mut account)
}
