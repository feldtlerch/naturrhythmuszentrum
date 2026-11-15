// SPDX-License-Identifier: AGPL-3.0-only
//! Original-question schema refusals precede numerical reads and heap allocation.

use super::fixture::{self, TestResult};
use crate::{ExecutionAccount, LcInputShape, LcShapeError, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Missing original inputs, empty/wrong branch counts and overflowing extents refuse.
/// Indexed constraints and empty covers are already excluded by checked forward-cover admission.
/// ´test:unit:loan-lc-shape-schema-refusals´
#[test]
fn loan_lc_shape_schema_refusals() -> TestResult {
    refusals::<f32>()?;
    refusals::<f64>()
}
fn refusals<P: Scalar>() -> TestResult {
    let native_max = usize::try_from(u32::MAX)?;
    for (input, counts) in [
        (4, [2, 3, 2]),
        (0, [2, 3, 0]),
        (0, [2, 3, 1]),
        (0, [usize::MAX, 2, 2]),
        // Product fits usize on the supported 64-bit target but exceeds native u32 rows.
        (0, [1, native_max, 2]),
        // Zero samples isolate conversion of the source row count itself.
        (0, [native_max + 1, 0, 2]),
    ] {
        let mut budget = Budget::new(Limits::default())?;
        let question = fixture::question(&mut budget)?;
        let mut account = ExecutionAccount::new(budget)?;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let error = LcInputShape::<P>::declare(question, input, counts, &mut account)
            .expect_err("original schema must refuse");
        if input == 4 {
            assert!(matches!(
                error,
                LcShapeError::Loan(LoanError::UnsupportedContract)
            ));
        } else if counts[0] > native_max || counts[1] == native_max {
            assert!(matches!(
                error,
                LcShapeError::Loan(LoanError::CountOverflow)
            ));
        } else {
            assert!(matches!(
                error,
                LcShapeError::Loan(LoanError::LayoutMismatch)
            ));
        }
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
    }
    Ok(())
}

/// Six fixed schema visits precede inherited exact arithmetic and symbol tariffs.
/// A missing input isolates this prefix: five refuses Work, six reaches the input refusal.
/// ´test:unit:loan-lc-shape-schema-work-boundaries´
#[test]
fn loan_lc_shape_schema_work_boundaries() -> TestResult {
    work::<f32>()?;
    work::<f64>()
}
fn work<P: Scalar>() -> TestResult {
    for remaining in [5, 6] {
        let mut budget = Budget::new(Limits::default())?;
        let question = fixture::question(&mut budget)?;
        let mut account = ExecutionAccount::new(budget)?;
        let spend = account.limits().work - account.usage().work - remaining;
        account.ledger.work(spend, crate::loan::AT)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let error = LcInputShape::<P>::declare(question, 4, [2, 3, 2], &mut account)
            .expect_err("prefix control has no original input");
        if remaining == 6 {
            assert!(matches!(
                error,
                LcShapeError::Loan(LoanError::UnsupportedContract)
            ));
            assert_eq!(account.usage().work - before, 6);
        } else {
            assert!(matches!(error, LcShapeError::Loan(LoanError::Budget(_))));
        }
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
    }
    Ok(())
}
