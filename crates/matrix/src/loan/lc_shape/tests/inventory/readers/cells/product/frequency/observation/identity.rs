// SPDX-License-Identifier: AGPL-3.0-only
//! Equal-valued fresh sources retain distinct original attachment evidence.

use super::{Snapshot, TestResult, publish_values, sample, setup, subscriber, values};
use crate::{LcFrequencyPrelude, LcInputInventory, LcShapeAxes};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::collections::BTreeMap;

/// Observer locators are compared independently of numerical answers.
pub(super) struct Closure<P: Scalar> {
    /// Exact original schema association.
    pub(super) schema: String,
    /// Each ordered role retains all of its emitted attachment fields.
    pub(super) claims: Vec<BTreeMap<String, String>>,
    /// Actual numerical outcome and costs are unchanged by an unused-role companion update.
    pub(super) snapshot: Snapshot<P>,
    /// Attempt identity is not recycled when a scoped subscriber starts over.
    pub(super) attempt: String,
}

/// Fresh occupations, unchanged-center companion edits and equal-valued foreign inventories
/// remain distinguishable, including an unused original role and selected repeated inductance.
/// ´test:unit:loan-lc-sign-attempt-attachments´
#[test]
fn loan_lc_sign_attempt_attachments() -> TestResult {
    identities::<f32>()?;
    identities::<f64>()
}
fn identities<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let mut budget = Budget::new(Limits::default())?;
    let context = ArithmeticContext::new()?;
    let pi = super::super::quantities::pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    let mut previous: Option<Closure<P>> = None;
    for mode in 0..2 {
        let closure = capture(&mut heap, &mut account, &inventory, &axes, &prelude, mode)?;
        if let Some(previous) = &previous {
            assert_eq!(closure.schema, previous.schema);
            assert_eq!(closure.snapshot, previous.snapshot);
            distinct(&closure, previous);
        }
        previous = Some(closure);
    }
    let previous = previous.ok_or("original closure")?;
    let (mut foreign_heap, mut foreign_account, foreign_inventory, foreign_axes) = setup::<P>()?;
    let foreign = capture(
        &mut foreign_heap,
        &mut foreign_account,
        &foreign_inventory,
        &foreign_axes,
        &prelude,
        0,
    )?;
    assert_ne!(foreign.schema, previous.schema);
    assert_eq!(foreign.snapshot, previous.snapshot);
    distinct(&foreign, &previous);
    let repeated = capture(&mut heap, &mut account, &inventory, &axes, &prelude, 3)?;
    assert_eq!(repeated.schema, previous.schema);
    assert_ne!(repeated.snapshot.image, previous.snapshot.image);
    Ok(())
}

fn distinct<P: Scalar>(left: &Closure<P>, right: &Closure<P>) {
    assert_ne!(left.attempt, right.attempt);
    for (left, right) in left.claims.iter().zip(&right.claims) {
        for field in ["occupation", "companions", "source", "source_companions"] {
            assert_ne!(left[field], right[field]);
        }
    }
}

fn capture<P: Scalar + 'static>(
    heap: &mut crate::Heap,
    account: &mut crate::ExecutionAccount,
    inventory: &LcInputInventory<P, 5>,
    axes: &LcShapeAxes,
    prelude: &LcFrequencyPrelude<P>,
    mode: usize,
) -> TestResult<Closure<P>> {
    let values = values::<P>(0);
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let members = publish_values(inventory, axes, heap, account, &masks, 1, &values)?;
    let closure = observe(
        heap,
        account,
        inventory,
        prelude,
        &members,
        if mode == 3 { [4, 1] } else { [0, 1] },
    )?;
    members.finish(heap, account)?;
    Ok(closure)
}
/// Capture one attempt over retained original publications without republishing any role.
pub(super) fn observe<P: Scalar + 'static>(
    heap: &crate::Heap,
    account: &mut crate::ExecutionAccount,
    inventory: &LcInputInventory<P, 5>,
    prelude: &LcFrequencyPrelude<P>,
    members: &super::super::super::super::Members<'_, P>,
    positions: [usize; 2],
) -> TestResult<Closure<P>> {
    let references = members.references();
    let cells = inventory
        .readers(heap, &references, account)?
        .into_cells(account)?
        .into_product(positions, account)?;
    let (snapshot, records) =
        subscriber::measured(true, || sample(&cells, prelude, account, 3, true));
    assert_eq!(snapshot.work, 46);
    assert_eq!(snapshot.native, [15, 5]);
    assert_eq!(
        snapshot.image.as_ref().map_err(|_| "ready image")?.state,
        "Ready"
    );
    let records = records
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let barrier = &records.spans[0];
    assert_eq!(barrier.fields["inductance"], positions[0].to_string());
    let closure = Closure {
        schema: barrier.fields["schema"].clone(),
        claims: records
            .events
            .iter()
            .filter(|event| event.name == "nrz.claim")
            .map(|event| event.fields.clone())
            .collect(),
        snapshot,
        attempt: barrier.fields["attempt"].clone(),
    };
    assert_eq!(closure.claims.len(), 5);
    drop(records);
    Ok(closure)
}
