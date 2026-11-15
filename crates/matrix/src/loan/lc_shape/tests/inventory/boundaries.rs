// SPDX-License-Identifier: AGPL-3.0-only
//! The LC facade borrows its schema and adds zero allocation beyond its native protected pin.

use super::setup;
use super::{fixture, source};
use crate::{ExecutionAccount, LcShapeRequest};
use fixture::{TestResult, request};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Design pin inventory: shared pin 16+56=72 bytes. Reclaimed input leaves a vacant
/// weak-index row and debit row, so neither table grows: retained and staged are both 72.
/// ´test:unit:loan-lc-input-inventory-byte-boundaries´
#[test]
fn loan_lc_input_inventory_byte_boundaries() -> TestResult {
    for index in 0..5 {
        bytes::<f32>(index)?;
        bytes::<f64>(index)?;
    }
    Ok(())
}
fn bytes<P: Scalar + 'static>(index: usize) -> TestResult {
    for staged in [false, true] {
        let required = 72;
        for limit in [required - 1, required] {
            let (mut heap, mut account, inventory, axes) = setup::<P>()?;
            let schema = inventory.input(index).ok_or("original inventory member")?;
            let input = source(schema, &axes, &mut heap, &mut account, 0)?;
            let candidate = request(
                schema,
                &axes,
                input.into_pin(),
                &[0, 3, 5],
                &mut heap,
                &mut account,
            )?;
            let demand = candidate.demand;
            let mut application = match schema.request(&mut heap, &axes, candidate, &mut account) {
                LcShapeRequest::Ready(application) => application,
                other @ LcShapeRequest::Refused(..) => return Err(format!("{other:?}").into()),
            };
            application.evaluate(&mut account)?;
            let tile = application.publish(&mut heap, &mut account)?;
            drop(application);
            assert!(matches!(
                heap.residence(&mut account, 100_000)?,
                crate::Residence::Pending { .. }
            ));
            let mut limits = Limits::default();
            if staged {
                limits.staged_payload_bytes = limit;
            } else {
                limits.output_bytes = limit;
            }
            let mut reader_account = ExecutionAccount::new(Budget::new(limits)?)?;
            crate::loan::fault::reset_reads();
            let injection = (limit < required).then(|| crate::loan::fault::Injection::after(0));
            let outside = tile.retain(&mut heap, &mut reader_account);
            if limit == required {
                let outside = outside?;
                drop(tile);
                assert_eq!(
                    outside.schema().specialization_key(),
                    schema.specialization_key()
                );
                assert!(
                    outside
                        .reader(&mut reader_account)?
                        .contains(3, &mut reader_account)?
                );
                drop(outside);
            } else {
                let Err(crate::LoanError::Heap(error)) = outside else {
                    return Err("expected native pin byte refusal".into());
                };
                assert_eq!(
                    error,
                    crate::HeapError::ResourceBytes {
                        staged,
                        used: 72,
                        limit: 71,
                    }
                );
                assert_eq!(
                    [
                        crate::loan::fault::reads(),
                        crate::loan::fault::Injection::attempts()
                    ],
                    [0; 2]
                );
                drop(injection);
                drop(tile);
            }
            fixture::retire(&mut heap, &mut account, demand)?;
        }
    }
    Ok(())
}

/// Count plus all original role visits: four inputs need 5 Work; five need 6.
/// Each neighbour is tested on the consumed original ledger, before inherited declarations.
/// ´test:unit:loan-lc-input-inventory-work-boundaries´
#[test]
fn loan_lc_input_inventory_work_boundaries() -> TestResult {
    work::<f32, 4>(false, 5)?;
    work::<f64, 4>(false, 5)?;
    work::<f32, 5>(true, 6)?;
    work::<f64, 5>(true, 6)
}
fn work<P: Scalar, const N: usize>(repeated: bool, required: u64) -> TestResult {
    for remaining in [required - 1, required] {
        let mut budget = Budget::new(Limits::default())?;
        let question = fixture::inventory_question(true, repeated, &mut budget)?;
        let mut account = ExecutionAccount::new(budget)?;
        let spend = account.limits().work - account.usage().work - remaining;
        account.ledger.work(spend, crate::loan::AT)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = crate::LcInputInventory::<P, N>::preflight(&question, &mut account);
        if remaining == required {
            result?;
            assert_eq!(account.usage().work - before, required);
        } else {
            assert!(matches!(result, Err(crate::LoanError::Budget(_))));
            assert_eq!(account.usage().work - before, remaining);
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

/// Each complete-inventory member retains exactly seven role and nine application visits.
/// ´test:unit:loan-lc-input-inventory-binding-boundaries´
#[test]
fn loan_lc_input_inventory_binding_boundaries() -> TestResult {
    binding::<f32>()?;
    binding::<f64>()
}
fn binding<P: Scalar + 'static>() -> TestResult {
    for index in 0..5 {
        for remaining in [15, 16] {
            let (mut heap, mut account, inventory, axes) = setup::<P>()?;
            let schema = inventory.input(index).ok_or("original inventory member")?;
            let input = source(schema, &axes, &mut heap, &mut account, 0)?;
            let candidate = request(
                schema,
                &axes,
                input.into_pin(),
                &[0, 3, 5],
                &mut heap,
                &mut account,
            )?;
            let spend = account.limits().work - account.usage().work - remaining;
            account.ledger.work(spend, crate::loan::AT)?;
            let before = account.usage().work;
            crate::loan::fault::reset_reads();
            let injection = crate::loan::fault::Injection::after(0);
            let result = schema.check(&heap, &axes, &candidate, &mut account);
            if remaining == 16 {
                result?;
                assert_eq!(account.usage().work - before, 16);
            } else {
                assert!(matches!(result, Err(crate::LoanError::Budget(_))));
            }
            assert_eq!(
                [
                    crate::loan::fault::reads(),
                    crate::loan::fault::Injection::attempts()
                ],
                [0; 2]
            );
            drop(injection);
            let demand = candidate.demand;
            drop(candidate);
            let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
            fixture::retire(&mut heap, &mut scanner, demand)?;
        }
    }
    Ok(())
}
