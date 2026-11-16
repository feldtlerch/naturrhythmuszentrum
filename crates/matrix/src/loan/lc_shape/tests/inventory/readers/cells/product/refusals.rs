// SPDX-License-Identifier: AGPL-3.0-only
//! Late holes cannot expose early roles, and foreign accounts never pay for observations.

use super::{TestResult, publish, setup};
use crate::{ExecutionAccount, LcProductError, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// A hole in the first or final original role refuses after the fixed prepay with no loads.
/// Out-of-capacity tuples have the same preflight boundary, without touching any numerical plane.
/// ´test:unit:loan-lc-input-product-coverage-refusals´
#[test]
fn loan_lc_input_product_coverage_refusals() -> TestResult {
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
            .into_cells(&mut account)?
            .into_product([0, 1], &mut account)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let injection = crate::loan::fault::Injection::after(0);
        let result = cells.read_cell(if missing == 5 { 12 } else { 0 }, &mut account);
        if missing == 5 {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::LayoutMismatch))
            ));
        } else {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::CoverageMismatch))
            ));
        }
        assert_eq!(account.usage().work - before, 37);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts(),
                crate::loan::fault::scalars()
            ],
            [0; 3]
        );
        // A refused tuple does not consume or corrupt the remaining complete cells.
        cells.read_cell(3, &mut account)?;
        assert_eq!(crate::loan::fault::reads(), 15);
        assert_eq!(crate::loan::fault::scalars(), 1);
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}

/// Binding and observations authenticate the original ledger before charging, with repair/retry.
/// ´test:unit:loan-lc-input-product-account-refusals´
#[test]
fn loan_lc_input_product_account_refusals() -> TestResult {
    account::<f32>()?;
    account::<f64>()
}
fn account<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
    let references = members.references();
    let readers = inventory
        .readers(&heap, &references, &mut account)?
        .into_cells(&mut account)?;
    let mut other = ExecutionAccount::new(Budget::new(Limits::default())?)?;
    let before = [account.usage().work, other.usage().work];
    crate::loan::fault::reset_reads();
    crate::loan::fault::reset_scalars();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        readers.into_product([0, 1], &mut other),
        Err(LcProductError::Loan(LoanError::ForeignAccount))
    ));
    assert_eq!([account.usage().work, other.usage().work], before);
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts(),
            crate::loan::fault::scalars()
        ],
        [0; 3]
    );
    let cells = inventory
        .readers(&heap, &references, &mut account)?
        .into_cells(&mut account)?
        .into_product([0, 1], &mut account)?;
    let before = [account.usage().work, other.usage().work];
    assert!(matches!(
        cells.read_cell(0, &mut other),
        Err(LcProductError::Loan(LoanError::ForeignAccount))
    ));
    assert_eq!([account.usage().work, other.usage().work], before);
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts(),
            crate::loan::fault::scalars()
        ],
        [0; 3]
    );
    cells.read_cell(0, &mut account)?;
    assert_eq!(crate::loan::fault::reads(), 15);
    assert_eq!(crate::loan::fault::scalars(), 1);
    drop(injection);
    drop(references);
    members.finish(&mut heap, &mut account)
}

/// Original quantity positions, including repeated roles, are checked once before any load.
/// ´test:unit:loan-lc-input-product-role-refusals´
#[test]
fn loan_lc_input_product_role_refusals() -> TestResult {
    roles::<f32>()?;
    roles::<f64>()
}
fn roles<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
    let references = members.references();
    for positions in [[1, 0], [0, 0], [2, 1], [5, 1]] {
        let cells = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?;
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let injection = crate::loan::fault::Injection::after(0);
        let before = account.usage().work;
        let result = cells.into_product(positions, &mut account);
        if positions[0] == 5 {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::UnsupportedContract))
            ));
        } else {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::TargetMismatch))
            ));
        }
        assert_eq!(account.usage().work - before, 4);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::scalars(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 3]
        );
        drop(injection);
    }
    drop(references);
    members.finish(&mut heap, &mut account)
}
