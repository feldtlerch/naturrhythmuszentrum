// SPDX-License-Identifier: AGPL-3.0-only
//! Kind and original-axis substitutions cannot acquire a pending queried-output capability.

use super::{TestResult, demand, request, setup};
use crate::{LcOmegaOutput, LoanError};
use nrz_bindings::{
    Descriptor, ExponentPolicy, Kind, Limits, Measure, ReferenceDefinition, Registry, Role, Scale,
    readings::EntryMeaning,
};
use nrz_scalar::Scalar;

/// Exact one-radian and ordered kind semantics are checked independently of equal dimensions.
/// ´test:unit:loan-lc-output-meaning-refusals´
#[test]
fn loan_lc_output_meaning_refusals() -> TestResult {
    meanings::<f32>()?;
    meanings::<f64>()
}
fn meanings<P: Scalar + 'static>() -> TestResult {
    let mut s = setup::<P>(Limits::default(), [2, 3, 2])?;
    for (kind, magnitude) in [(Kind::rate()?, "1"), (Kind::Angle, "2")] {
        let descriptor = Descriptor::new(kind, Measure::coherent(), Role::Difference)?;
        let value =
            nrz_bindings::readings::algebra::Rational::parse(magnitude, &mut s.account.ledger)?;
        let meaning = EntryMeaning::new(
            descriptor.clone(),
            ReferenceDefinition::new("wrong operand", &descriptor, Scale::Exact(value))?,
            None,
            &mut s.account.ledger,
        )?;
        let proposed = request(&s.registry, &meaning);
        let injection = crate::loan::fault::Injection::after(0);
        assert!(matches!(
            LcOmegaOutput::declare(&s.inventory, proposed, &mut s.account),
            Err(crate::LcShapeError::Loan(LoanError::TargetMismatch))
        ));
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
    }
    let registry = Registry::new(ExponentPolicy::Rational);
    let injection = crate::loan::fault::Injection::after(0);
    assert!(
        LcOmegaOutput::declare(&s.inventory, request(&registry, &s.radian), &mut s.account)
            .is_err()
    );
    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
    drop(injection);
    Ok(())
}

/// Equal-capacity foreign axes and extra components refuse before any native reservation.
/// ´test:unit:loan-lc-output-demand-refusals´
#[test]
fn loan_lc_output_demand_refusals() -> TestResult {
    masks::<f32>()?;
    masks::<f64>()
}
fn masks<P: Scalar + 'static>() -> TestResult {
    for wrong_axis in [false, true] {
        let mut s = setup::<P>(Limits::default(), [2, 3, 2])?;
        let schema = LcOmegaOutput::declare(
            &s.inventory,
            request(&s.registry, &s.radian),
            &mut s.account,
        )?;
        let axis = if wrong_axis {
            s.heap.create_cell_axis(12, &mut s.account.ledger)?
        } else {
            s.axes.combined.axis()
        };
        let components = if wrong_axis { 1 } else { 2 };
        let layout = s.heap.create_cell_layout(
            axis,
            crate::Layout::contiguous(crate::Extent::new(12, components)?)?,
            &mut s.account.ledger,
        )?;
        let mask = s.heap.create_mask(layout, &[3], &mut s.account.ledger)?;
        let injection = crate::loan::fault::Injection::after(0);
        assert!(matches!(
            schema.reserve(&mut s.heap, &s.axes, mask, &mut s.account),
            Err(LoanError::LayoutMismatch)
        ));
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        assert_eq!(s.heap.loans.elements()?, [0, 0]);
        drop(injection);
        super::fixture::retire(&mut s.heap, &mut s.account, mask)?;
    }
    Ok(())
}

/// Abandonment uses ordinary local producer delivery and releases all demand and native owners.
/// ´test:unit:loan-lc-output-abandonment´
#[test]
fn loan_lc_output_abandonment() -> TestResult {
    abandonment::<f32>()?;
    abandonment::<f64>()
}
fn abandonment<P: Scalar + 'static>() -> TestResult {
    let mut s = setup::<P>(Limits::default(), [2, 3, 2])?;
    let schema = LcOmegaOutput::declare(
        &s.inventory,
        request(&s.registry, &s.radian),
        &mut s.account,
    )?;
    let demand = demand(&mut s.heap, &s.axes, &[3], &mut s.account)?;
    let pending = schema.reserve(&mut s.heap, &s.axes, demand, &mut s.account)?;
    let ticket = pending.ticket()?;
    drop(pending);
    assert!(matches!(
        s.heap.residence(&mut s.account, 100_000)?,
        crate::Residence::Pending {
            elements: Some([0, 0]),
            ..
        }
    ));
    assert!(
        matches!(s.heap.take_producer_outcome(ticket, &mut s.account)?, crate::ProducerOutcome::Absent(receipt) if receipt.reason() == crate::AbsenceReason::Abandoned)
    );
    assert!(matches!(
        s.heap.take_producer_outcome(ticket, &mut s.account)?,
        crate::ProducerOutcome::AlreadyDelivered
    ));
    super::fixture::retire(&mut s.heap, &mut s.account, demand)
}
