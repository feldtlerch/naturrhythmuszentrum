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

/// The inventory preflight is one count plus five original roles, once before all declarations.
/// Per-member role and application spans remain individually visible with the original tariffs.
/// ´test:unit:loan-lc-input-inventory-phase-receipts´
#[test]
fn loan_lc_input_inventory_phase_receipts() -> TestResult {
    inventory_phases::<f32>()?;
    inventory_phases::<f64>()
}
fn inventory_phases<P: nrz_scalar::Scalar + 'static>() -> TestResult {
    let mut budget = nrz_bindings::Budget::new(nrz_bindings::Limits::default())?;
    let question = fixture::inventory_question(true, true, &mut budget)?;
    let mut account = crate::ExecutionAccount::new(budget)?;
    let phases = Arc::new(Mutex::new(Vec::<Phase>::new()));
    let inventory = tracing::subscriber::with_default(Observer(Arc::clone(&phases)), || {
        crate::LcInputInventory::<P, 5>::declare(&question, [2, 3, 2], &mut account)
    })?;
    {
        let phases = phases
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let inventory: Vec<_> = phases
            .iter()
            .filter(|phase| phase.operation == "LC input inventory")
            .collect();
        assert_eq!(inventory.len(), 1);
        assert_eq!(
            inventory[0].last - inventory[0].first.ok_or("initial work")?,
            6
        );
        assert_eq!(
            phases
                .iter()
                .filter(|phase| phase.operation == "LC shape schema")
                .count(),
            5
        );
    }
    let mut heap = crate::Heap::new(crate::HeapLimits::default())?;
    let axes = fixture::axes(&heap, &mut account)?;
    for index in 0..5 {
        let schema = inventory.input(index).ok_or("original input")?;
        let input = source(schema, &axes, &mut heap, &mut account, 0)?;
        let candidate = request(
            schema,
            &axes,
            input.into_pin(),
            &[0, 3, 5],
            &mut heap,
            &mut account,
        )?;
        phases
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        tracing::subscriber::with_default(Observer(Arc::clone(&phases)), || {
            schema.check(&heap, &axes, &candidate, &mut account)
        })?;
        {
            let phases = phases
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert_eq!(phases.len(), 2);
            for (phase, (name, work)) in phases
                .iter()
                .zip([("LC shape roles", 7), ("LC shape application", 9)])
            {
                assert_eq!(phase.operation, name);
                assert_eq!(phase.last - phase.first.ok_or("initial work")?, work);
            }
        }
        let demand = candidate.demand;
        drop(candidate);
        fixture::retire(&mut heap, &mut account, demand)?;
    }
    Ok(())
}
