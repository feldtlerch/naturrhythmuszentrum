// SPDX-License-Identifier: AGPL-3.0-only
//! Late holes cannot expose early roles, and foreign accounts never pay for observations.

use super::{TestResult, publish, setup};
use crate::{ExecutionAccount, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// A hole in the first or final original role refuses after the fixed prepay with no loads.
/// Out-of-capacity tuples have the same preflight boundary, without touching any numerical plane.
/// ´test:unit:loan-lc-input-cells-coverage-refusals´
#[test]
fn loan_lc_input_cells_coverage_refusals() -> TestResult {
    coverage::<f32>()?;
    coverage::<f64>()
}
fn coverage<P: Scalar + 'static>() -> TestResult {
    for missing in [0, 4, 5] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let mut masks = std::array::from_fn(|_| vec![0, 3, 11]);
        if missing < 5 {
            masks[missing] = vec![3, 11];
        }
        let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
        let references = members.references();
        let cells = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = cells.read_cell(if missing == 5 { 12 } else { 0 }, &mut account);
        if missing == 5 {
            assert!(matches!(result, Err(LoanError::LayoutMismatch)));
        } else {
            assert!(matches!(result, Err(LoanError::CoverageMismatch)));
        }
        assert_eq!(account.usage().work - before, 36);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        // A refused tuple does not consume or corrupt the remaining complete cells.
        cells.read_cell(3, &mut account)?;
        assert_eq!(crate::loan::fault::reads(), 15);
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}

/// Binding and observations authenticate the original ledger before charging, with repair/retry.
/// ´test:unit:loan-lc-input-cells-account-refusals´
#[test]
fn loan_lc_input_cells_account_refusals() -> TestResult {
    account::<f32>()?;
    account::<f64>()
}
fn account<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
    let references = members.references();
    let readers = inventory.readers(&heap, &references, &mut account)?;
    let mut other = ExecutionAccount::new(Budget::new(Limits::default())?)?;
    let before = [account.usage().work, other.usage().work];
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        readers.into_cells(&mut other),
        Err(LoanError::ForeignAccount)
    ));
    assert_eq!([account.usage().work, other.usage().work], before);
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    let cells = inventory
        .readers(&heap, &references, &mut account)?
        .into_cells(&mut account)?;
    let before = [account.usage().work, other.usage().work];
    assert!(matches!(
        cells.read_cell(0, &mut other),
        Err(LoanError::ForeignAccount)
    ));
    assert_eq!([account.usage().work, other.usage().work], before);
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    cells.read_cell(0, &mut account)?;
    assert_eq!(crate::loan::fault::reads(), 15);
    drop(injection);
    drop(references);
    members.finish(&mut heap, &mut account)
}
