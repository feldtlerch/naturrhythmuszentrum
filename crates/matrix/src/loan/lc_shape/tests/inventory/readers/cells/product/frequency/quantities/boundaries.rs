// SPDX-License-Identifier: AGPL-3.0-only
//! Literal local prelude and per-cell tariffs retain refusal and original-input semantics.

use super::{
    ArithmeticContext, Budget, LcFrequencyPrelude, Limits, Scalar, TestResult, VALUES, pi, publish,
    publish_values, setup,
};
use crate::{ExecutionAccount, LcFrequencyState, LcFrequencyTarget, LcProductError, LoanError};

/// Fixed factor construction prepays five visits and checks its actual fixed storage.
/// Existing pi storage is shared; renewed construction is separately charged.
/// ´test:unit:loan-lc-quantity-prelude-boundaries´
#[test]
fn loan_lc_quantity_prelude_boundaries() -> TestResult {
    prelude::<f32>()?;
    prelude::<f64>()
}
fn prelude<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = pi::<P>(&context, &mut Budget::new(Limits::default())?)?;
    let bytes = if P::PRECISION == nrz_scalar::Precision::F32 {
        32
    } else {
        56
    };
    assert_eq!(LcFrequencyPrelude::<P>::stored_bytes(), bytes);
    for remaining in [4, 5] {
        let mut budget = Budget::new(Limits {
            work: 7 + remaining,
            ..Limits::default()
        })?;
        budget.work(7, crate::loan::AT)?;
        let result = LcFrequencyPrelude::new(&pi, &context, &mut budget);
        if remaining == 4 {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::Budget(_)))
            ));
        } else {
            assert!(result?.pi().same_region(&pi));
        }
        assert_eq!(budget.usage().work, if remaining == 4 { 7 } else { 12 });
    }
    for staged in [false, true] {
        for limit in [bytes - 1, bytes] {
            let mut limits = Limits::default();
            if staged {
                limits.staged_payload_bytes = limit as u64;
            } else {
                limits.output_bytes = limit as u64;
            }
            let mut budget = Budget::new(limits)?;
            let result = LcFrequencyPrelude::new(&pi, &context, &mut budget);
            if limit < bytes {
                assert!(matches!(
                    result,
                    Err(LcProductError::Loan(LoanError::Budget(_)))
                ));
            } else {
                assert!(result?.pi().same_region(&pi));
            }
            assert_eq!(budget.usage().work, 5);
        }
    }
    let mut budget = Budget::new(Limits::default())?;
    let first = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    let evidence = first.pi().evidence();
    let readings = first.factors();
    drop(first);
    let second = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    assert_eq!(budget.usage().work, 10);
    assert!(second.pi().evidence().same_region(&evidence));
    assert_eq!(second.factors(), readings);
    Ok(())
}

/// The complete cell and two final products reserve 46; 45 refuses before any load.
/// ´test:unit:loan-lc-quantity-work-boundaries´
#[test]
fn loan_lc_quantity_work_boundaries() -> TestResult {
    work::<f32>()?;
    work::<f64>()
}
fn work<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut budget = Budget::new(Limits::default())?;
    let pi = pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    for remaining in [45, 46] {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let masks = std::array::from_fn(|_| vec![0, 3, 11]);
        let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
        let references = members.references();
        let cells = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?
            .into_product([0, 1], &mut account)?;
        let spend = account.limits().work - account.usage().work - remaining;
        account.ledger.work(spend, crate::loan::AT)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let injection = crate::loan::fault::Injection::after(0);
        let result = cells.read_quantities(3, &prelude, &mut account);
        if remaining == 45 {
            assert!(matches!(
                result,
                Err(LcProductError::Loan(LoanError::Budget(_)))
            ));
        } else {
            super::oracle(&result?, [0, 1], 0)?;
        }
        assert_eq!(
            account.usage().work - before,
            if remaining == 45 { 0 } else { 46 }
        );
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            if remaining == 45 { [0, 0] } else { [15, 5] }
        );
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        drop(references);
        let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        members.finish(&mut heap, &mut scanner)?;
    }
    Ok(())
}

/// One factor phase serves multiple cells; no exact preparation or pi materialization repeats.
/// ´test:unit:loan-lc-quantity-phase-receipts´
#[test]
fn loan_lc_quantity_phase_receipts() -> TestResult {
    phases::<f32>()?;
    phases::<f64>()
}
fn phases<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = pi::<P>(&context, &mut Budget::new(Limits::default())?)?;
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish(&inventory, &axes, &mut heap, &mut account, &masks, 1)?;
    let references = members.references();
    let cells = inventory
        .readers(&heap, &references, &mut account)?
        .into_cells(&mut account)?
        .into_product([0, 1], &mut account)?;
    let (result, phases) =
        super::super::super::super::super::super::super::observation::recorded(|| {
            let prelude = LcFrequencyPrelude::new(&pi, &context, &mut account.ledger)?;
            cells.read_quantities(0, &prelude, &mut account)?;
            cells.read_quantities(11, &prelude, &mut account)?;
            Ok::<_, LcProductError>(())
        })?;
    result?;
    assert_eq!(
        phases,
        vec![
            ("LC frequency factors".to_owned(), 5),
            ("LC local quantities".to_owned(), 46),
            ("LC local quantities".to_owned(), 46)
        ]
    );
    drop(references);
    members.finish(&mut heap, &mut account)
}

/// Original sign outcomes precede final quantities; missing inputs and finite refusals stay explicit.
/// ´test:unit:loan-lc-quantity-refusals´
#[test]
fn loan_lc_quantity_refusals() -> TestResult {
    refusals::<f32>()?;
    refusals::<f64>()
}
fn refusals<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut budget = Budget::new(Limits::default())?;
    let pi = pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    for case in 0..7 {
        let (mut heap, mut account, inventory, axes) = setup::<P>()?;
        let mut masks = std::array::from_fn(|_| vec![0, 3, 11]);
        if case == 0 {
            masks[4] = vec![11];
        }
        let values = values::<P>(case);
        let members = publish_values(
            &inventory,
            &axes,
            &mut heap,
            &mut account,
            &masks,
            1,
            &values,
        )?;
        let references = members.references();
        let cells = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?
            .into_product([0, 1], &mut account)?;
        let mut other = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        let before = [account.usage().work, other.usage().work];
        crate::loan::fault::reset_reads();
        crate::loan::fault::reset_scalars();
        let injection = crate::loan::fault::Injection::after(0);
        let result = if case == 2 {
            cells.read_quantities(3, &prelude, &mut other)
        } else {
            cells.read_quantities(if case == 1 { 12 } else { 3 }, &prelude, &mut account)
        };
        outcome(case, result)?;
        assert_eq!(
            account.usage().work - before[0],
            if case == 2 { 0 } else { 46 }
        );
        assert_eq!(other.usage().work, before[1]);
        assert_eq!(
            [crate::loan::fault::reads(), crate::loan::fault::scalars()],
            match case {
                0..3 => [0, 0],
                3 | 4 => [15, 0],
                _ => [15, 1],
            }
        );
        super::oracle(
            &cells.read_quantities(11, &prelude, &mut account)?,
            [0, 1],
            1,
        )?;
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        drop(references);
        members.finish(&mut heap, &mut account)?;
    }
    Ok(())
}
fn values<P: Scalar>(case: usize) -> [[[P; 3]; 2]; 5] {
    let mut values = VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
    match case {
        3 => {
            values[0][0] = [-P::max_finite(), P::ZERO, P::ZERO];
            values[1][0] = [P::from_u8(2), P::ZERO, P::ZERO];
        }
        4 => values[1][0] = [P::ONE, P::ONE, P::ZERO],
        5 => {
            for role in [0, 1] {
                values[role][0] = [P::ONE, P::from_u8(3) / P::from_u8(4), P::ZERO];
            }
        }
        6 => {
            values[0][0] = [P::max_finite(), P::ZERO, P::ZERO];
            values[1][0] = [P::from_u8(2), P::ZERO, P::ZERO];
        }
        _ => (),
    }
    values
}
fn outcome<P: Scalar>(
    case: usize,
    result: Result<crate::LcQuantityCell<'_, '_, P, 5>, LcProductError>,
) -> TestResult {
    match case {
        0 => assert!(matches!(
            result,
            Err(LcProductError::Loan(LoanError::CoverageMismatch))
        )),
        1 => assert!(matches!(
            result,
            Err(LcProductError::Loan(LoanError::LayoutMismatch))
        )),
        2 => assert!(matches!(
            result,
            Err(LcProductError::Loan(LoanError::ForeignAccount))
        )),
        6 => assert!(matches!(
            result,
            Err(LcProductError::Kernel(crate::KernelError::BinaryDomain {
                entry: 3,
                reason: crate::DomainReason::UnboundedImage,
                ..
            }))
        )),
        _ => {
            let result = result?;
            assert!(result.values().is_none());
            assert_eq!(result.frequency().inputs().cell(), 3);
            match case {
                3 => assert!(matches!(
                    result.frequency().state(),
                    LcFrequencyState::Invalid(LcFrequencyTarget::Inductance)
                )),
                4 => assert!(matches!(
                    result.frequency().state(),
                    LcFrequencyState::Undecided(LcFrequencyTarget::Capacitance)
                )),
                _ => assert!(matches!(
                    result.frequency().state(),
                    LcFrequencyState::Undecided(LcFrequencyTarget::Product)
                )),
            }
        }
    }
    Ok(())
}
