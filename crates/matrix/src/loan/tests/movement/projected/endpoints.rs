// SPDX-License-Identifier: AGPL-3.0-only
//! Endpoint conversion preserves the local numerical-error scope of an arithmetic result.

use super::{Setup, TestResult, candidate, empty, publish, square};
use crate::{ApplicationRequest, NumericalErrorScope};
use nrz_scalar::Scalar;

/// Literal square extrema and independently encoded local ulps survive endpoint conversion.
/// ´test:unit:loan-endpoint-view-application-error-scope´
#[test]
fn loan_endpoint_view_application_error_scope() -> TestResult {
    scope::<f32>()?;
    scope::<f64>()
}
fn scope<P: Scalar + 'static>() -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        demand,
    } = super::super::setup::<P>()?;
    square(&mut request)?;
    let owner = match heap.request_projected_elementwise(request, &mut account) {
        ApplicationRequest::ApplicationReady(owner) => owner,
        other @ ApplicationRequest::ApplicationRefused(..) => {
            return Err(format!("{other:?}").into());
        }
    };
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let identity = tile.pin().ticket()?.identity;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(identity.mask, identity.layout)?, &mut account)?
        .into_endpoint_view(&mut account)?;
    for (cell, center, lower, upper) in [(0, 121.0, 64.0, 196.0), (2, 196.0, 121.0, 289.0)] {
        let result = reader.read_component(cell, 0, &mut account)?;
        assert!(result.bounds().lower().get().to_f64() <= lower);
        assert!(result.bounds().upper().get().to_f64() >= upper);
        assert_eq!(
            result.input_error().to_f64(),
            super::super::arithmetic::local_error::<P>(center)
        );
        assert_eq!(
            result.numerical_error_scope(),
            NumericalErrorScope::LocalOperation
        );
    }
    drop(tile);
    heap.retire_mask(demand, &mut account.ledger)?;
    super::super::retire_copy_layout(&mut heap, &mut account, demand.layout())?;
    empty(&mut heap, &mut account)
}
