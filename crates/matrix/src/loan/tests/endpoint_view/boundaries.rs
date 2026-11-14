// SPDX-License-Identifier: AGPL-3.0-only
//! Literal inventories pin conversion and inherited pin-retention resource neighbours.

use super::{TestResult, publish};
use crate::{EndpointReadError, ExecutionAccount, LoanError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::Scalar;

/// Three binding role visits and one environment check, then 2 map + 1 address + 3 loads + 28 scalar units.
/// Both neighbours use the original immutable ledger after prior setup consumption.
/// ´test:unit:loan-endpoint-view-work-boundaries´
#[test]
fn loan_endpoint_view_work_boundaries() -> TestResult {
    work::<f32>()?;
    work::<f64>()
}
fn work<P: Scalar + 'static>() -> TestResult {
    for binding in [true, false] {
        let required = if binding { 4 } else { 34 };
        for remaining in [required - 1, required] {
            let (mut heap, mut account, tile, mask) = publish(&[[P::ONE, P::ONE, P::ONE]])?;
            let source = tile.reader(&heap, &mut account)?;
            let view = source.masked_view(heap.mask(mask, mask.layout())?, &mut account)?;
            let mut bound = None;
            let mut unbound = Some(view);
            if !binding {
                bound = Some(
                    unbound
                        .take()
                        .ok_or("view")?
                        .into_endpoint_view(&mut account)?,
                );
            }
            let spend = account.limits().work - account.usage().work - remaining;
            account.ledger.work(spend, crate::loan::AT)?;
            let before = account.usage().work;
            crate::loan::fault::reset_reads();
            let injection = crate::loan::fault::Injection::after(0);
            let result = if let Some(view) = unbound {
                view.into_endpoint_view(&mut account).map(|_| ())
            } else {
                bound
                    .as_ref()
                    .ok_or("bound")?
                    .read_component(0, 0, &mut account)
                    .map(|_| ())
            };
            if remaining == required {
                result?;
                assert_eq!(account.usage().work - before, required);
                assert_eq!(crate::loan::fault::reads(), if binding { 0 } else { 3 });
            } else {
                assert!(matches!(
                    result,
                    Err(EndpointReadError::Loan(LoanError::Budget(_)))
                ));
                assert_eq!(crate::loan::fault::reads(), 0);
            }
            assert_eq!(crate::loan::fault::Injection::attempts(), 0);
            drop(injection);
            drop(tile);
            let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
            super::empty(&mut heap, &mut scanner)?;
        }
    }
    Ok(())
}

/// Design inventory: shared pin 16+56=72, weak-index row 40; retained 112.
/// Replace one old row: staged 112+2*40=192. Publication has a vacant debit row.
/// The endpoint view adds no allocations; both formats pin 111/112 and 191/192.
/// ´test:unit:loan-endpoint-view-retained-byte-boundaries´
#[test]
fn loan_endpoint_view_retained_byte_boundaries() -> TestResult {
    bytes::<f32>()?;
    bytes::<f64>()
}
fn bytes<P: Scalar + 'static>() -> TestResult {
    for staged in [false, true] {
        let required = if staged { 192 } else { 112 };
        for limit in [required - 1, required] {
            let (mut heap, mut account, tile, mask) = publish(&[[P::ONE, P::ONE, P::ONE]])?;
            let ticket = tile.pin().ticket()?;
            let mut limits = Limits::default();
            if staged {
                limits.staged_payload_bytes = limit;
            } else {
                limits.output_bytes = limit;
            }
            let mut reader_account = ExecutionAccount::new(Budget::new(limits)?)?;
            crate::loan::fault::reset_reads();
            let refusal_injection =
                (limit < required).then(|| crate::loan::fault::Injection::after(0));
            let retained = heap.acquire_pin::<P>(ticket, &mut reader_account);
            if limit == required {
                let pin = retained?;
                drop(tile);
                assert!(matches!(
                    heap.residence(&mut account, 100_000)?,
                    crate::Residence::Pending { .. }
                ));
                let source = pin.historical_reader(&mut reader_account)?;
                let view =
                    source.masked_view(heap.mask(mask, mask.layout())?, &mut reader_account)?;
                let injection = crate::loan::fault::Injection::after(0);
                let reader = view.into_endpoint_view(&mut reader_account)?;
                super::oracle(
                    &reader.read_component(0, 0, &mut reader_account)?,
                    [P::ONE; 3],
                )?;
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                drop(injection);
                drop(pin);
            } else {
                let Err(LoanError::Heap(crate::HeapError::ResourceBytes {
                    staged: actual,
                    used,
                    limit: ceiling,
                })) = retained
                else {
                    return Err("expected byte refusal".into());
                };
                assert_eq!((actual, used, ceiling), (staged, required, limit));
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                assert_eq!(crate::loan::fault::reads(), 0);
                drop(tile);
            }
            drop(refusal_injection);
            super::empty(&mut heap, &mut account)?;
        }
    }
    Ok(())
}
