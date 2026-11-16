// SPDX-License-Identifier: AGPL-3.0-only
//! A repaired error companion changes only one original role's attachment closure.

use super::{TestResult, identity, publish_values, setup, values};
use crate::{
    ExecutionAccount, HandbackOutcome, Heap, HostChannel, LcFrequencyPrelude, LcInputShape,
    LcShapeAxes, LoanError, PublishedTile,
};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Keep four publications while a repaired fifth role changes only its error payload.
/// ´test:unit:loan-lc-sign-companion-only-change´
#[test]
fn loan_lc_sign_companion_only_change() -> TestResult {
    changed::<f32>()?;
    changed::<f64>()
}
fn changed<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let mut budget = Budget::new(Limits::default())?;
    let context = ArithmeticContext::new()?;
    let pi = super::super::quantities::pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    let masks = std::array::from_fn(|_| vec![0, 3, 11]);
    let mut members = publish_values(
        &inventory,
        &axes,
        &mut heap,
        &mut account,
        &masks,
        1,
        &values::<P>(0),
    )?;
    let before = identity::observe(&heap, &mut account, &inventory, &prelude, &members, [0, 1])?;
    payload(&members.tiles[4], &mut account, P::ONE / P::from_u8(32))?;
    let schema = inventory.input(4).ok_or("repeated original role")?;
    let input = repaired(schema, &axes, &mut heap, &mut account)?;
    let (tile, layout) = super::super::super::super::mapped(
        schema,
        &axes,
        &mut heap,
        &mut account,
        &masks[4],
        input,
    )?;
    members.tiles[4] = tile;
    members.layouts.push(layout);
    payload(&members.tiles[4], &mut account, P::ONE / P::from_u8(64))?;
    let after = identity::observe(&heap, &mut account, &inventory, &prelude, &members, [0, 1])?;
    assert_eq!(before.schema, after.schema);
    assert_eq!(before.snapshot, after.snapshot);
    assert_ne!(before.attempt, after.attempt);
    for (index, (mut before, mut after)) in before.claims.into_iter().zip(after.claims).enumerate()
    {
        before.remove("attempt");
        after.remove("attempt");
        if index == 4 {
            assert_ne!(before["companions"], after["companions"]);
            assert_ne!(before["source_companions"], after["source_companions"]);
            for field in ["schema", "position", "account", "error_scope"] {
                assert_eq!(before[field], after[field]);
            }
        } else {
            assert_eq!(before, after);
        }
    }
    members.finish(&mut heap, &mut account)
}
fn payload<P: Scalar + 'static>(
    tile: &crate::LcShapeTile<'_, P>,
    account: &mut ExecutionAccount,
    error: P,
) -> TestResult {
    let reader = tile.reader(account)?;
    for (cell, center, expected_error) in [(3, 20, error), (11, 21, P::ZERO)] {
        assert_eq!(
            reader.read_component(HostChannel::Center, cell, 0, account)?,
            P::from_u8(center) / P::from_u8(32)
        );
        assert_eq!(
            reader.read_component(HostChannel::Total, cell, 0, account)?,
            P::ONE / P::from_u8(32)
        );
        assert_eq!(
            reader.read_component(HostChannel::Error, cell, 0, account)?,
            expected_error
        );
    }
    Ok(())
}
fn repaired<P: Scalar + 'static>(
    schema: &LcInputShape<P>,
    axes: &LcShapeAxes,
    heap: &mut Heap,
    account: &mut ExecutionAccount,
) -> TestResult<PublishedTile<P>> {
    let mut loan = super::super::super::super::fixture::granted(
        heap.request_loan(schema.source(axes, account)?, account),
    )?;
    for (cell, row) in values::<P>(0)[4].into_iter().enumerate() {
        for (channel, value) in HostChannel::ALL.into_iter().zip(row) {
            loan.write_component(channel, cell, 0, value, account)?;
        }
    }
    let ticket = loan.ticket();
    loan.write_component(HostChannel::Error, 0, 0, -P::ONE, account)?;
    let mut loan = match heap.handback(loan, account) {
        HandbackOutcome::Repair(
            LoanError::InvalidPayload {
                channel: HostChannel::Error,
                ..
            },
            loan,
        ) => loan,
        other => return Err(format!("{other:?}").into()),
    };
    assert_eq!(loan.ticket(), ticket);
    loan.write_component(HostChannel::Error, 0, 0, P::ONE / P::from_u8(64), account)?;
    match heap.handback(loan, account) {
        HandbackOutcome::Published(tile, _) => Ok(tile),
        other => Err(format!("{other:?}").into()),
    }
}
