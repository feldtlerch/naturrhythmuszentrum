// SPDX-License-Identifier: AGPL-3.0-only
//! Aggregate interest never substitutes for the current thread's actual dispatcher.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};

use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};

use crate::budget::{Budget, Limits};
use crate::error::Location;
use crate::testing::common::TestResult;
use crate::testing::work_trace::{Row, dispatcher_guard, uncoordinated};
use crate::work_observation::{Phase, Scope, fresh_callsite_enabled, observe, phase_interest};

/// Enablement calls and delivered observations are independent of field-expression counters.
#[derive(Debug, Default)]
struct Calls {
    /// Queries using the actual operation phase metadata.
    phases: AtomicU64,
    /// Span creation and event delivery must remain absent for a rejecting dispatcher.
    delivered: AtomicU64,
}

/// A configurable cache response separates aggregate interest from dynamic enablement.
#[derive(Debug)]
struct Filter {
    /// Returned by registration without evaluating dynamic enablement.
    interest: Interest,
    /// The thread's current dispatcher decides this independently of cached interest.
    enabled: bool,
    /// Shared counters survive subscriber destruction.
    calls: Arc<Calls>,
}

impl Subscriber for Filter {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if metadata.target() == "nrz::work" {
            self.interest.clone()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if metadata.fields().field("operation").is_some() {
            assert_eq!(metadata.name(), "nrz.work.phase");
            assert_eq!(metadata.target(), "nrz::work");
            assert!(metadata.is_span());
            let fields: Vec<_> = metadata.fields().iter().map(|field| field.name()).collect();
            assert_eq!(
                fields,
                [
                    "family",
                    "revision",
                    "owner",
                    "operation",
                    "ledger",
                    "work",
                    "outcome",
                    "reason"
                ]
            );
            self.calls.phases.fetch_add(1, Ordering::Relaxed);
        }
        self.enabled
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        self.calls.delivered.fetch_add(1, Ordering::Relaxed);
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.calls.delivered.fetch_add(1, Ordering::Relaxed);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

/// The context form must defer its ledger access and operation fields just like the direct form.
struct Context<'a> {
    /// An ordinary caller-owned ledger.
    budget: &'a mut Budget,
}

/// Exercise lifecycle, rule, direct phase and both production helper forms under rejection.
fn rejected() -> TestResult<u64> {
    let fields = Cell::new(0);
    let mut budget = Budget::new(Limits::default())?;
    assert!(!budget.observation.assigned());
    let _scope = Scope::rule(|| {
        fields.set(fields.get() + 1);
        "rejected rule"
    });
    let _purpose = Scope::phase("rejected purpose");
    let _phase = Phase::new(&budget, || {
        fields.set(fields.get() + 1);
        "rejected phase"
    });
    observe!(
        budget,
        {
            fields.set(fields.get() + 1);
            Location::new("rejected helper")
        },
        { budget.work(1, Location::new("body")) }
    )?;
    let context = &mut Context {
        budget: &mut budget,
    };
    observe!(
        context.budget,
        {
            fields.set(fields.get() + 1);
            Location::new("rejected context")
        },
        { context.budget.work(1, Location::new("body")) }
    )?;
    assert_eq!(fields.get(), 0);
    assert_eq!(budget.usage().work, 2);
    assert!(!budget.observation.assigned());
    Ok(budget.observation.cold_entries())
}

/// Register the tested callsites before another thread becomes the only registered subscriber.
fn prime() -> TestResult {
    tracing::subscriber::with_default(
        Filter {
            interest: Interest::sometimes(),
            enabled: false,
            calls: Arc::default(),
        },
        rejected,
    )?;
    Ok(())
}

/// Sometimes-interest checks the current dispatcher on the exact phase field inventory.
/// ´test:crate:work-tracing-sometimes-dispatcher´
#[test]
fn work_tracing_sometimes_dispatcher() -> TestResult {
    let _guard = dispatcher_guard();
    let calls = Arc::<Calls>::default();
    tracing::subscriber::with_default(
        Filter {
            interest: Interest::sometimes(),
            enabled: false,
            calls: Arc::clone(&calls),
        },
        || -> TestResult {
            assert_eq!(rejected()?, 2);
            assert!(phase_interest().is_sometimes());
            Ok(())
        },
    )?;
    assert_eq!(calls.phases.load(Ordering::Relaxed), 3);
    assert_eq!(calls.delivered.load(Ordering::Relaxed), 0);
    Ok(())
}

/// Another thread's always-interest still requires rejection by the current `NoSubscriber`.
/// ´test:crate:work-tracing-foreign-always-dispatcher´
#[test]
fn work_tracing_foreign_always_dispatcher() -> TestResult {
    let _guard = dispatcher_guard();
    prime()?;
    std::thread::scope(|threads| -> TestResult {
        let (ready, started) = mpsc::sync_channel(0);
        let (release, finished) = mpsc::channel::<()>();
        let foreign = threads.spawn(move || {
            tracing::subscriber::with_default(
                Filter {
                    interest: Interest::always(),
                    enabled: true,
                    calls: Arc::default(),
                },
                || {
                    let _ready = ready.send(());
                    let _finished = finished.recv();
                },
            );
        });
        started.recv()?;
        let result =
            tracing::dispatcher::with_default(&tracing::Dispatch::none(), || -> TestResult {
                assert!(phase_interest().is_always());
                assert_eq!(rejected()?, 2);
                assert!(phase_interest().is_always());
                Ok(())
            });
        drop(release);
        foreign.join().map_err(|_| "foreign subscriber thread")?;
        result
    })
}

/// Disabled field assertions overlap repeated subscriber installation and destruction.
/// ´test:crate:work-tracing-concurrent-dispatchers´
#[test]
fn work_tracing_concurrent_dispatchers() -> TestResult {
    let _guard = dispatcher_guard();
    prime()?;
    std::thread::scope(|threads| -> TestResult {
        let (ready, started) = mpsc::sync_channel(0);
        let (release, finished) = mpsc::sync_channel(0);
        let foreign = threads.spawn(move || -> Result<(), &'static str> {
            for _ in 0..256 {
                let (result, report) = uncoordinated(16, true, || -> Result<(), &'static str> {
                    let mut budget =
                        Budget::new(Limits::default()).map_err(|_| "foreign budget")?;
                    let _scope = Scope::rule(|| "concurrent rule");
                    observe!(budget, Location::new("concurrent helper"), {
                        budget.work(1, Location::new("concurrent body"))
                    })
                    .map_err(|_| "foreign body")?;
                    ready.send(()).map_err(|_| "disabled thread ended")?;
                    finished.recv().map_err(|_| "disabled thread ended")
                });
                result?;
                assert!(report.complete);
                assert_eq!(
                    report.rows,
                    [Row {
                        rule: "concurrent rule".into(),
                        phase: "validation".into(),
                        location: "concurrent helper".into(),
                        work: 1,
                    }]
                );
            }
            Ok(())
        });
        let result =
            tracing::dispatcher::with_default(&tracing::Dispatch::none(), || -> TestResult {
                for _ in 0..256 {
                    started.recv()?;
                    assert_eq!(rejected()?, 2);
                    release.send(())?;
                    rejected()?;
                }
                Ok(())
            });
        drop(release);
        drop(started);
        foreign.join().map_err(|_| "subscriber churn thread")??;
        result
    })
}

/// First use on an unsubscribed thread cannot hide a subscriber that is live on another thread.
/// ´test:crate:work-tracing-foreign-registration´
#[test]
fn work_tracing_foreign_registration() -> TestResult {
    let _guard = dispatcher_guard();
    std::thread::scope(|threads| -> TestResult {
        let (ready, started) = mpsc::sync_channel(0);
        let (release, finished) = mpsc::channel::<()>();
        let foreign = threads.spawn(move || {
            tracing::subscriber::with_default(
                Filter {
                    interest: Interest::always(),
                    enabled: true,
                    calls: Arc::default(),
                },
                || {
                    let _ready = ready.send(());
                    let _finished = finished.recv();
                    fresh_callsite_enabled().0
                },
            )
        });
        started.recv()?;
        tracing::dispatcher::with_default(&tracing::Dispatch::none(), || {
            let (enabled, interest) = fresh_callsite_enabled();
            assert!(!enabled);
            assert!(!interest.is_never());
        });
        drop(release);
        assert!(foreign.join().map_err(|_| "first-use subscriber thread")?);
        Ok(())
    })
}
