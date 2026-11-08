// SPDX-License-Identifier: AGPL-3.0-only
//! Ledger snapshots at ordinary helper boundaries; attribution belongs to subscribers.

mod callsite;

use std::sync::atomic::{AtomicU64, Ordering};

use tracing::span::EnteredSpan;
use tracing::{Level, Span};

use crate::budget::Budget;

use callsite::{Callsite, callsite};

/// The gate and span construction share this exact metadata and cached interest.
static PHASE_CALLSITE: Callsite = Callsite::new(&PHASE_METADATA);

/// A fixed field inventory also determines the length of every initial value set.
const PHASE_FIELDS: &[&str] = &[
    "family",
    "revision",
    "owner",
    "operation",
    "ledger",
    "work",
    "outcome",
    "reason",
];

/// One production callsite describes charged helper intervals; classification is field data.
static PHASE_METADATA: tracing::Metadata<'static> = tracing::metadata! {
    name: "nrz.work.phase",
    target: "nrz::work",
    level: Level::TRACE,
    fields: PHASE_FIELDS,
    callsite: &PHASE_CALLSITE,
    kind: tracing::metadata::Kind::SPAN,
};

/// A cached rejection or level-off state reaches the body without registration or dispatch calls.
#[inline]
pub(crate) fn potentially_enabled() -> bool {
    PHASE_CALLSITE.potentially_enabled()
}

/// Read the actual aggregate cache for deterministic dispatcher controls.
#[cfg(test)]
pub(crate) fn phase_interest() -> tracing::subscriber::Interest {
    PHASE_CALLSITE.interest()
}

/// A distinct site lets the concurrency control exercise first registration without cache mutation.
#[cfg(test)]
pub(crate) fn fresh_callsite_enabled() -> (bool, tracing::subscriber::Interest) {
    let callsite = callsite!(
        "nrz.work.phase",
        tracing::metadata::Kind::SPAN,
        "family",
        "revision",
        "owner",
        "operation",
        "ledger",
        "work",
        "outcome",
        "reason"
    );
    (callsite.enabled(), callsite.interest())
}

/// Process-local generations never wrap or depend on a ledger's storage address.
static NEXT_LEDGER: AtomicU64 = AtomicU64::new(1);

/// Lazy identity keeps disabled instrumentation out of ledger construction.
#[derive(Debug, Default)]
pub(crate) struct Identity {
    /// Zero until an enabled boundary observes this incarnation.
    generation: AtomicU64,
    /// Cached never-interest bypasses the cold function; foreign positive interest can enter it.
    #[cfg(test)]
    cold_entries: AtomicU64,
}

impl Identity {
    /// The disabled-field control checks that no incarnation was prepared.
    #[cfg(test)]
    pub(crate) fn assigned(&self) -> bool {
        self.generation.load(Ordering::Relaxed) != 0
    }

    /// Count the outlined decision path, including current-dispatcher rejection, never a Work charge.
    #[cfg(test)]
    fn note_cold_entry(&self) {
        let _previous =
            self.cold_entries
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                });
    }

    /// A zero count proves that the disabled probe bypassed observer setup entirely.
    #[cfg(test)]
    pub(crate) fn cold_entries(&self) -> u64 {
        self.cold_entries.load(Ordering::Relaxed)
    }

    /// Assign an owner-qualified incarnation only when a boundary is observed.
    pub(crate) fn get(&self) -> u64 {
        let known = self.generation.load(Ordering::Relaxed);
        if known != 0 {
            return known;
        }
        let Ok(next) = NEXT_LEDGER.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        }) else {
            tracing::event!(name: "nrz.observer.loss", target: "nrz::work", Level::TRACE,
                reason = "ledger identity exhausted");
            return 0;
        };
        match self
            .generation
            .compare_exchange(0, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => next,
            Err(existing) => existing,
        }
    }
}

impl Clone for Identity {
    /// A copied ledger owns a fresh incarnation even when its counters agree.
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// Ledgerless rule and purpose scopes preserve the existing attribution vocabulary.
pub(crate) struct Scope;

impl Scope {
    /// Enter a checked primitive's validation scope.
    pub(crate) fn rule(rule: impl FnOnce() -> &'static str) -> EnteredSpan {
        let callsite = callsite!(
            "nrz.work.phase",
            tracing::metadata::Kind::SPAN,
            "family",
            "revision",
            "owner",
            "rule",
            "phase"
        );
        if !callsite.enabled() {
            return Span::none().entered();
        }
        let rule = rule();
        let values: [Option<&dyn tracing::field::Value>; 5] = [
            Some(&"cas"),
            Some(&1_u64),
            Some(&"nrz-cas"),
            Some(&rule),
            Some(&"validation"),
        ];
        Span::new(
            callsite.metadata(),
            &callsite.metadata().fields().value_set_all(&values),
        )
        .entered()
    }

    /// Refine the mathematical purpose while inheriting the active rule.
    pub(crate) fn phase(phase: &'static str) -> EnteredSpan {
        let callsite = callsite!(
            "nrz.work.phase",
            tracing::metadata::Kind::SPAN,
            "family",
            "revision",
            "owner",
            "phase"
        );
        if !callsite.enabled() {
            return Span::none().entered();
        }
        let values: [Option<&dyn tracing::field::Value>; 4] =
            [Some(&"cas"), Some(&1_u64), Some(&"nrz-cas"), Some(&phase)];
        Span::new(
            callsite.metadata(),
            &callsite.metadata().fields().value_set_all(&values),
        )
        .entered()
    }
}

/// One causal phase can have several disjoint entered intervals.
#[derive(Debug)]
pub(crate) struct Phase(Span);

impl Phase {
    /// The span callsite gates operation, identity and snapshot preparation together.
    #[inline]
    pub(crate) fn new(budget: &Budget, operation: impl FnOnce() -> &'static str) -> Self {
        Self::family(budget, "cas", operation)
    }

    /// Consumer families share the original callsite and ledger owner namespace.
    fn family(
        budget: &Budget,
        family: &'static str,
        operation: impl FnOnce() -> &'static str,
    ) -> Self {
        if !PHASE_CALLSITE.enabled() {
            return Self(Span::none());
        }
        let operation = operation();
        let ledger = budget.observation.get();
        let work = budget.usage().work;
        let values: [Option<&dyn tracing::field::Value>; PHASE_FIELDS.len()] = [
            Some(&family),
            Some(&1_u64),
            Some(&"nrz-cas"),
            Some(&operation),
            Some(&ledger),
            Some(&work),
            None,
            None,
        ];
        Self(Span::new(
            &PHASE_METADATA,
            &PHASE_METADATA.fields().value_set_all(&values),
        ))
    }

    /// The disabled helper path performs no field preparation or ledger access.
    #[inline]
    pub(crate) fn is_disabled(&self) -> bool {
        self.0.is_disabled()
    }

    /// Snapshot on each resume; causal parentage does not choose the ledger parent.
    pub(crate) fn enter(&self, budget: &Budget) -> EnteredSpan {
        if !self.is_disabled() {
            self.0.record("ledger", budget.observation.get());
            self.0.record("work", budget.usage().work);
        }
        self.0.clone().entered()
    }
}

/// Close before leaving the entered stack, preserving a refused attempt's spent prefix.
pub(crate) fn finish<T, E: std::fmt::Debug>(
    entered: EnteredSpan,
    budget: &Budget,
    result: &Result<T, E>,
) {
    entered.record("work", budget.usage().work);
    match result {
        Ok(_) => {
            entered.record("outcome", "completed");
        }
        Err(fault) => {
            entered.record("outcome", "refused");
            entered.record("reason", tracing::field::debug(fault));
        }
    }
    drop(entered);
}

/// Observe creation or destruction without introducing an accounting callback.
#[inline]
pub(crate) fn ledger(budget: &Budget, action: &'static str) {
    let callsite = callsite!(
        "nrz.work.ledger",
        tracing::metadata::Kind::EVENT,
        "owner",
        "ledger",
        "action",
        "work"
    );
    if !callsite.enabled() {
        return;
    }
    let ledger = budget.observation.get();
    let work = budget.usage().work;
    let values: [Option<&dyn tracing::field::Value>; 4] =
        [Some(&"nrz-cas"), Some(&ledger), Some(&action), Some(&work)];
    tracing::Event::dispatch(
        callsite.metadata(),
        &callsite.metadata().fields().value_set_all(&values),
    );
}

/// Run the body on every cold branch so the inline path needs no call-preserved registers.
#[cold]
#[inline(never)]
pub(crate) fn observed_context<C, T, E: std::fmt::Debug>(
    context: &mut C,
    ledger: impl Fn(&C) -> &Budget,
    operation: impl FnOnce(&C) -> &'static str,
    run: impl FnOnce(&mut C) -> Result<T, E>,
) -> Result<T, E> {
    observed_family(context, ledger, "cas", operation, run)
}

/// Public consumers enter the same cold helper without creating another ledger or callsite.
#[cold]
#[inline(never)]
fn observed_family<C, T, E: std::fmt::Debug>(
    context: &mut C,
    ledger: impl Fn(&C) -> &Budget,
    family: &'static str,
    operation: impl FnOnce(&C) -> &'static str,
    run: impl FnOnce(&mut C) -> Result<T, E>,
) -> Result<T, E> {
    #[cfg(test)]
    ledger(context).observation.note_cold_entry();
    let phase = if family == "cas" {
        Phase::new(ledger(context), || operation(context))
    } else {
        Phase::family(ledger(context), family, || operation(context))
    };
    if phase.is_disabled() {
        return run(context);
    }
    let entered = phase.enter(ledger(context));
    let result = run(context);
    finish(entered, ledger(context), &result);
    result
}

impl Budget {
    /// Observe a cross-crate boundary on this shared Work ledger.
    ///
    /// Static consumer `family` and `operation` names use the existing boundary
    /// receipts and original ledger incarnation under (´def:cas:work-observation-protocol´).
    /// The tiles-and-streams design governs shared-ledger attribution. The `owner`
    /// field remains the ledger's `nrz-cas` namespace. Observation adds no tariff: only `run` consumes Work.
    /// Disabled observation calls `run` without preparing fields or ledger identity.
    /// # Errors
    /// Preserves the body's error and its successfully consumed Work.
    #[inline]
    pub fn with_work_phase<T, E: std::fmt::Debug>(
        &mut self,
        family: &'static str,
        operation: &'static str,
        run: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        if potentially_enabled() {
            observed_family(self, |budget| budget, family, |_| operation, run)
        } else {
            run(self)
        }
    }

    /// Reborrow an ordinary ledger into the separate cold observation path.
    #[inline]
    pub(crate) fn observed_phase<T, E: std::fmt::Debug>(
        &mut self,
        operation: impl FnOnce() -> &'static str,
        run: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        observed_context(self, |budget| budget, |_| operation(), run)
    }
}

/// Wrap a helper, never an individual charge, with enabled-only snapshots.
macro_rules! observe {
    ($context:ident . $ledger:ident, $at:expr, $body:block) => {{
        if crate::work_observation::potentially_enabled() {
            crate::work_observation::observed_context(
                $context,
                |context| &*context.$ledger,
                |_| $at.operation,
                |$context| $body,
            )
        } else {
            $body
        }
    }};
    ($budget:ident, $at:expr, $body:block) => {{
        if crate::work_observation::potentially_enabled() {
            $budget.observed_phase(|| $at.operation, |$budget| $body)
        } else {
            $body
        }
    }};
}

pub(crate) use observe;
