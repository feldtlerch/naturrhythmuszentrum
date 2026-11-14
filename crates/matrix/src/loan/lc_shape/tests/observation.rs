// SPDX-License-Identifier: AGPL-3.0-only
//! Binding spans expose their own immutable original-ledger Work deltas.

use super::{fixture, source};
use fixture::{TestResult, request, setup};
use std::sync::{Arc, Mutex};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

/// One record is indexed by its tracing span identifier.
#[derive(Default)]
struct Phase {
    /// Stable native phase name.
    operation: String,
    /// First cumulative ledger snapshot.
    first: Option<u64>,
    /// Most recent cumulative ledger snapshot.
    last: u64,
}
impl Visit for Phase {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "operation" {
            self.operation = value.to_owned();
        }
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "work" {
            self.first.get_or_insert(value);
            self.last = value;
        }
    }
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}
/// Each test owns one synchronous subscriber and its completed receipts.
struct Observer(Arc<Mutex<Vec<Phase>>>);
impl Subscriber for Observer {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target() == "nrz::work"
    }
    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut phases = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut phase = Phase::default();
        attributes.record(&mut phase);
        phases.push(phase);
        Id::from_u64(u64::try_from(phases.len()).unwrap_or(u64::MAX).max(1))
    }
    fn record(&self, id: &Id, values: &Record<'_>) {
        let mut phases = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let index = usize::try_from(id.into_u64())
            .unwrap_or(0)
            .saturating_sub(1);
        if let Some(phase) = phases.get_mut(index) {
            values.record(phase);
        }
    }
    fn record_follows_from(&self, _id: &Id, _follows: &Id) {}
    fn event(&self, _event: &Event<'_>) {}
    fn enter(&self, _id: &Id) {}
    fn exit(&self, _id: &Id) {}
}

/// The design's six schema-prefix, seven role and nine application units are separately visible as span deltas.
/// ´test:unit:loan-lc-shape-work-phase-receipts´
#[test]
fn loan_lc_shape_work_phase_receipts() -> TestResult {
    phases::<f32>()?;
    phases::<f64>()
}
fn phases<P: nrz_scalar::Scalar + 'static>() -> TestResult {
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
    let phases = Arc::new(Mutex::new(Vec::<Phase>::new()));
    tracing::subscriber::with_default(Observer(Arc::clone(&phases)), || {
        assert!(matches!(
            crate::LcInputShape::<P>::declare(
                schema.question().clone(),
                4,
                [2, 3, 2],
                &mut account
            ),
            Err(crate::LcShapeError::Loan(
                crate::LoanError::UnsupportedContract
            ))
        ));
        schema.check(&heap, &axes, &candidate, &mut account)
    })?;
    let phases = phases
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(phases.len(), 3);
    for (phase, (name, work)) in phases.iter().zip([
        ("LC shape schema", 6),
        ("LC shape roles", 7),
        ("LC shape application", 9),
    ]) {
        assert_eq!(phase.operation, name);
        assert_eq!(phase.last - phase.first.ok_or("initial work")?, work);
    }
    drop(phases);
    let demand = candidate.demand;
    drop(candidate);
    fixture::retire(&mut heap, &mut account, demand)
}
