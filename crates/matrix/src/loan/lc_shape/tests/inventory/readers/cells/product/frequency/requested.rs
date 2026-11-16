// SPDX-License-Identifier: AGPL-3.0-only
//! Every requested subset has independent enclosure, dependency and literal cost evidence.

mod oracle;
use super::{
    TestResult, VALUES,
    observation::{DEMANDS, spec},
    publish_values, setup,
};
use crate::{ExecutionAccount, LcFrequencyPrelude, LcQuantityDemand, LcRequestedState};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Both formats and every subset preserve original sparse tuples and skip unneeded kernels.
/// ´test:unit:loan-lc-demand-subsets-rational-images´
#[test]
fn loan_lc_demand_subsets_rational_images() -> TestResult {
    images::<f32>()?;
    images::<f64>()
}
fn images<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut budget = Budget::new(Limits::default())?;
    let pi = super::quantities::pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    let prepared = budget.usage();
    for mode in 0..3 {
        image(&prelude, mode)?;
    }
    assert_eq!(budget.usage(), prepared);
    Ok(())
}
fn image<P: Scalar + 'static>(prelude: &LcFrequencyPrelude<P>, mode: usize) -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let mut values = VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
    let pair = if mode == 1 {
        [P::min_positive_normal(), P::ONE / P::from_u8(2)]
    } else {
        [P::max_finite() / P::from_u8(16), P::from_u8(2)]
    };
    if mode != 0 {
        for (role, value) in pair.into_iter().enumerate() {
            values[role][0] = [value, P::ZERO, P::ZERO];
        }
    }
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
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
    for positions in [[0, 1], [4, 1]] {
        let cells = inventory
            .readers(&heap, &references, &mut account)?
            .into_cells(&mut account)?
            .into_product(positions, &mut account)?;
        for demand in DEMANDS {
            for cell in [0, 3, 11] {
                crate::loan::fault::reset_reads();
                crate::loan::fault::reset_scalars();
                let injection = crate::loan::fault::Injection::after(0);
                let before = account.usage().work;
                let result = cells.read_requested(cell, demand, Some(prelude), &mut account)?;
                let (work, kernels, _) = spec(demand);
                assert_eq!(account.usage().work - before, work);
                assert_eq!(
                    [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                    [15, kernels]
                );
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                drop(injection);
                assert_eq!(result.demand(), demand);
                assert_eq!(result.positions(), positions);
                assert_eq!(result.inputs().axes(), axes);
                assert_eq!(result.inputs().cell(), cell);
                assert!(std::ptr::eq(
                    result.inputs().inventory(),
                    &raw const inventory
                ));
                if demand == LcQuantityDemand::Omega {
                    assert!(result.prelude().is_none());
                } else {
                    assert!(std::ptr::eq(result.prelude().ok_or("prelude")?, prelude));
                }
                for (reading, original) in result.inputs().readings().iter().zip(values) {
                    assert_eq!(reading.values(), original[usize::from(cell == 11)]);
                }
                oracle::check(&result)?;
            }
        }
    }
    drop(references);
    members.finish(&mut heap, &mut account)
}

/// Every literal exact Work allowance succeeds on the original already-used account.
/// ´test:unit:loan-lc-demand-literal-work-boundaries´
#[test]
fn loan_lc_demand_literal_work_boundaries() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}
fn boundaries<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut budget = Budget::new(Limits::default())?;
    let pi = super::quantities::pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    for demand in DEMANDS {
        for enough in [false, true] {
            let (mut heap, mut account, inventory, axes) = setup::<P>()?;
            let masks = std::array::from_fn(|_| vec![0, 3, 11]);
            let values =
                VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
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
            let (work, kernels, _) = spec(demand);
            let allowance = if enough { work } else { work - 1 };
            let spend = account.limits().work - account.usage().work - allowance;
            account.ledger.work(spend, crate::loan::AT)?;
            let before = account.usage().work;
            crate::loan::fault::reset_reads();
            crate::loan::fault::reset_scalars();
            let result = cells.read_requested(
                3,
                demand,
                if demand == LcQuantityDemand::Omega {
                    None
                } else {
                    Some(&prelude)
                },
                &mut account,
            );
            if enough {
                assert!(matches!(result?.state(), LcRequestedState::Ready { .. }));
                assert_eq!(account.usage().work - before, work);
                assert_eq!(
                    [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                    [15, kernels]
                );
            } else {
                assert!(matches!(
                    result,
                    Err(crate::LcProductError::Loan(crate::LoanError::Budget(_)))
                ));
                assert_eq!(account.usage().work, before);
                assert_eq!(
                    [crate::loan::fault::reads(), crate::loan::fault::scalars()],
                    [0, 0]
                );
            }
            drop(references);
            let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
            members.finish(&mut heap, &mut scanner)?;
        }
    }
    Ok(())
}
