// SPDX-License-Identifier: AGPL-3.0-only
//! Original identities and literal binding Work refuse before any numerical observation.

use super::{fixture, source};
use crate::{LcInputShape, LcShapeRequest, LoanError};
use fixture::{TestResult, request, setup};
use nrz_scalar::Scalar;

/// Wrong tuple factors, substituted original targets and changed specialization cannot bind.
/// ´test:unit:loan-lc-shape-original-identity-refusals´
#[test]
fn loan_lc_shape_original_identity_refusals() -> TestResult {
    identities::<f32>()?;
    identities::<f64>()
}
fn identities<P: Scalar + 'static>() -> TestResult {
    for case in 0..6 {
        let (mut heap, mut account, schema, mut axes) = setup::<P>()?;
        let input = source(&schema, &axes, &mut heap, &mut account, 0)?;
        let mut candidate = request(
            &schema,
            &axes,
            input.into_pin(),
            &[0, 3, 5],
            &mut heap,
            &mut account,
        )?;
        let other = LcInputShape::declare(schema.question().clone(), 0, [2, 3, 2], &mut account)?;
        assert_ne!(schema.specialization_key(), other.specialization_key());
        let mut expected_target = false;
        match case {
            0 => axes.instances = heap.create_cell_axis(2, &mut account.ledger)?,
            1 => axes.samples = heap.create_cell_axis(3, &mut account.ledger)?,
            2 => axes.branches = heap.create_cell_axis(2, &mut account.ledger)?,
            3 => {
                axes.sample_branches =
                    heap.create_cell_product(axes.samples, axes.branches, &mut account.ledger)?;
            }
            4 => {
                candidate.destination = other.destination(&axes, &mut account)?;
                expected_target = true;
            }
            5 => {
                candidate.input = source(&other, &axes, &mut heap, &mut account, 0)?.into_pin();
                expected_target = true;
            }
            _ => return Err("fixture case".into()),
        }
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let refused = match schema.request(&mut heap, &axes, candidate, &mut account) {
            LcShapeRequest::Refused(crate::ApplicationError::Loan(error), request) => {
                if expected_target {
                    assert!(matches!(error, LoanError::TargetMismatch));
                } else {
                    assert!(matches!(error, LoanError::LayoutMismatch));
                }
                request
            }
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        let demand = refused.demand;
        drop(refused);
        fixture::retire(&mut heap, &mut account, demand)?;
    }
    Ok(())
}

/// Design inventory: seven role units and nine fixed application checks; schema wrappers allocate zero.
/// Both 15 and 16 remaining Work units use the original ledger after setup consumption.
/// ´test:unit:loan-lc-shape-binding-work-boundaries´
#[test]
fn loan_lc_shape_binding_work_boundaries() -> TestResult {
    work::<f32>()?;
    work::<f64>()
}
fn work<P: Scalar + 'static>() -> TestResult {
    for remaining in [15, 16] {
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
            assert!(matches!(result, Err(LoanError::Budget(_))));
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
        let mut scanner = crate::ExecutionAccount::new(nrz_bindings::Budget::new(
            nrz_bindings::Limits::default(),
        )?)?;
        fixture::retire(&mut heap, &mut scanner, demand)?;
    }
    Ok(())
}
