// SPDX-License-Identifier: AGPL-3.0-only
//! The LC facade borrows its schema and adds zero allocation beyond its native protected pin.

use super::{fixture, source};
use crate::{ExecutionAccount, LcShapeRequest};
use fixture::{TestResult, request, setup};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Design pin inventory: shared pin 16+56=72 bytes. Reclaimed input leaves a vacant
/// weak-index row and debit row, so neither table grows: retained and staged are both 72.
/// ´test:unit:loan-lc-shape-outside-byte-boundaries´
#[test]
fn loan_lc_shape_outside_byte_boundaries() -> TestResult {
    bytes::<f32>()?;
    bytes::<f64>()
}
fn bytes<P: Scalar + 'static>() -> TestResult {
    for staged in [false, true] {
        let required = 72;
        for limit in [required - 1, required] {
            let (mut heap, mut account, schema, axes) = setup::<P>()?;
            let input = source(&schema, &axes, &mut heap, &mut account, 0)?;
            let candidate = request(
                &schema,
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

/// Mapped-movement inventory: 3*((3 loads+3 stores+6 law units)*1+2 completion)+3 binds+2 words+1 entry=48.
/// The LC continuation adds no traversal charge; both neighbours preserve the original ledger.
/// ´test:unit:loan-lc-shape-traversal-work-boundaries´
#[test]
fn loan_lc_shape_traversal_work_boundaries() -> TestResult {
    traversal::<f32>()?;
    traversal::<f64>()
}
fn traversal<P: Scalar + 'static>() -> TestResult {
    for remaining in [47, 48] {
        let (mut heap, mut account, schema, axes) = setup::<P>()?;
        let input = source(&schema, &axes, &mut heap, &mut account, 0)?;
        let candidate = request(
            &schema,
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
        let spend = account.limits().work - account.usage().work - remaining;
        account.ledger.work(spend, crate::loan::AT)?;
        let before = account.usage().work;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = application.evaluate(&mut account);
        if remaining == 48 {
            result?;
            assert_eq!(account.usage().work - before, 48);
            assert_eq!(crate::loan::fault::reads(), 9);
        } else {
            assert!(matches!(
                result,
                Err(crate::ApplicationError::Loan(crate::LoanError::Budget(_)))
            ));
            assert_eq!(crate::loan::fault::reads(), 0);
        }
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        let used = account.usage().work;
        assert_eq!(
            application.cancel(&mut heap, &mut account)?.reason(),
            crate::AbsenceReason::Cancelled
        );
        assert_eq!(account.usage().work, used);
        drop(application);
        let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        fixture::retire(&mut heap, &mut scanner, demand)?;
    }
    Ok(())
}
