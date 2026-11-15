// SPDX-License-Identifier: AGPL-3.0-only
//! Reconstructing both declarations must not change the schema's original coverage assumption.

use super::{fixture, source};
use crate::{Coverage, HostChannel, HostTileDeclaration, LcInputShape, LcShapeRequest, LoanError};
use fixture::{TestResult, request, setup};
use nrz_scalar::Scalar;

/// Original symbols with a changed coverage assumption or derived producer still refuse.
/// The literal 7+9 binding Work detects either guard half bypassing to the downstream gate.
/// ´test:unit:loan-lc-shape-original-scope-refusals´
#[test]
fn loan_lc_shape_original_scope_refusals() -> TestResult {
    scope::<f32>()?;
    scope::<f64>()
}
fn scope<P: Scalar + 'static>() -> TestResult {
    for case in 0..4 {
        let derived = case == 3;
        let (mut heap, mut account, schema, axes) = setup::<P>()?;
        let input = if derived {
            derived_source(&schema, &axes, &mut heap, &mut account)?
        } else if case == 1 {
            source(&schema, &axes, &mut heap, &mut account, 0)?
        } else {
            declared_source(&schema, &axes, &mut heap, &mut account)?
        };
        let mut candidate = request(
            &schema,
            &axes,
            input.into_pin(),
            &[0, 3, 5],
            &mut heap,
            &mut account,
        )?;
        if matches!(case, 1 | 2) {
            candidate.destination = HostTileDeclaration::check(
                schema.symbols[1].clone(),
                axes.combined.axis(),
                Coverage::DeclaredBounds,
                &mut account,
            )?;
        }
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let before = account.usage().work;
        let candidate = match schema.request(&mut heap, &axes, candidate, &mut account) {
            LcShapeRequest::Refused(crate::ApplicationError::Loan(error), candidate) => {
                if derived {
                    assert!(matches!(error, LoanError::UnsupportedContract));
                } else {
                    assert!(matches!(error, LoanError::CoverageMismatch));
                }
                candidate
            }
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(account.usage().work - before, 16);
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
        fixture::retire(&mut heap, &mut account, demand)?;
    }
    Ok(())
}
/// Public host declarations can retain identical original symbols under another assumption.
fn declared_source<P: Scalar + 'static>(
    schema: &LcInputShape<P>,
    axes: &crate::LcShapeAxes,
    heap: &mut crate::Heap,
    account: &mut crate::ExecutionAccount,
) -> TestResult<crate::PublishedTile<P>> {
    let declaration = HostTileDeclaration::check(
        schema.symbols[0].clone(),
        axes.instances,
        Coverage::DeclaredBounds,
        account,
    )?;
    let mut loan = fixture::granted(heap.request_loan(declaration, account))?;
    for cell in 0..2 {
        for channel in HostChannel::ALL {
            loan.write_component(channel, cell, 0, P::ONE, account)?;
        }
    }
    match heap.handback(loan, account) {
        crate::HandbackOutcome::Published(tile, _) => Ok(tile),
        other => Err(format!("{other:?}").into()),
    }
}
/// An ordinary native copy can target the same symbols while retaining a derived producer.
fn derived_source<P: Scalar + 'static>(
    schema: &LcInputShape<P>,
    axes: &crate::LcShapeAxes,
    heap: &mut crate::Heap,
    account: &mut crate::ExecutionAccount,
) -> TestResult<crate::PublishedTile<P>> {
    let other = LcInputShape::declare(schema.question().clone(), 0, [2, 3, 2], account)?;
    let input = source(&other, axes, heap, account, 0)?;
    let layout = input.pin().ticket()?.identity.layout;
    let demand = heap.create_mask(layout, &[0, 1], &mut account.ledger)?;
    let map = heap.cell_entry_map(crate::Extent::new(2, 1)?, demand, None, &mut account.ledger)?;
    let owner = crate::conversion_testing::owner()?;
    let shape = crate::conversion_testing::shape(
        &owner,
        &nrz_bindings::LcQuantity::Inductance.descriptor()?,
        2,
    )?;
    let operation = crate::OperationDraft::map_cell_copy(shape.clone(), shape, map)?
        .check(crate::ExecutionLimits::default())?;
    let candidate = crate::PointwiseRequest {
        operation,
        input: input.into_pin(),
        destination: schema.source(axes, account)?,
        demand,
        limits: crate::ExecutionLimits::default(),
    };
    let application = match heap.request_mapped_copy(candidate, account) {
        crate::ApplicationRequest::ApplicationReady(owner) => owner,
        other @ crate::ApplicationRequest::ApplicationRefused(..) => {
            return Err(format!("{other:?}").into());
        }
    };
    let application = match application.evaluate(account) {
        crate::ApplicationEvaluation::ApplicationCandidate(owner) => owner,
        other => return Err(format!("{other:?}").into()),
    };
    let result = match heap.publish_application(application, account) {
        crate::ApplicationPublication::ApplicationPublished(tile, _) => tile,
        other => return Err(format!("{other:?}").into()),
    };
    heap.retire_mask(demand, &mut account.ledger)?;
    Ok(result)
}

/// A changed captured sample cardinality gets a distinct key and cannot accept the old application.
/// ´test:unit:loan-lc-shape-specialization-refusal´
#[test]
fn loan_lc_shape_specialization_refusal() -> TestResult {
    specialization::<f32>()?;
    specialization::<f64>()
}
fn specialization<P: Scalar + 'static>() -> TestResult {
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
    let changed = LcInputShape::declare(schema.question().clone(), 0, [2, 2, 2], &mut account)?;
    assert_ne!(schema.specialization_key(), changed.specialization_key());
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    let candidate = match changed.request(&mut heap, &axes, candidate, &mut account) {
        LcShapeRequest::Refused(
            crate::ApplicationError::Loan(LoanError::LayoutMismatch),
            candidate,
        ) => candidate,
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
    let demand = candidate.demand;
    drop(candidate);
    fixture::retire(&mut heap, &mut account, demand)
}
