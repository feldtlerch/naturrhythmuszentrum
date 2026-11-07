// SPDX-License-Identifier: AGPL-3.0-only
//! Disabled instrumentation prepares no fields; the ignored probe records its runtime cost.

use std::cell::Cell;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use crate::budget::{Budget, Limits};
use crate::error::{Fault, Location};
use crate::testing::common::TestResult;
use crate::testing::work_trace::without_observation;
use crate::work_observation::{Phase, Scope, observe};

/// A representative helper consumes one deterministic unit.
#[inline]
fn plain(budget: &mut Budget) -> Result<(), Fault> {
    budget.work(1, Location::new("disabled probe"))
}

/// The same helper includes the ordinary production boundary macro.
#[inline]
fn instrumented(budget: &mut Budget) -> Result<(), Fault> {
    observe!(budget, Location::new("disabled probe"), { plain(budget) })
}

/// Disabled boundary fields, identities and snapshots remain unprepared.
/// ´test:crate:work-tracing-disabled-fields´
#[test]
fn work_tracing_disabled_fields() -> TestResult {
    without_observation(|| -> TestResult {
        let fields = Cell::new(0);
        let _scope = Scope::rule(|| {
            fields.set(fields.get() + 1);
            "disabled rule"
        });
        let mut budget = Budget::new(Limits::default())?;
        let _registration = Phase::new(&budget, || {
            fields.set(fields.get() + 1);
            "disabled registration"
        });
        observe!(
            budget,
            {
                fields.set(fields.get() + 1);
                Location::new("lazy operation")
            },
            { budget.work(1, Location::new("ordinary operation")) }
        )?;
        assert_eq!(fields.get(), 0);
        assert_eq!(budget.usage().work, 1);
        assert!(!budget.observation.assigned());
        assert_eq!(budget.observation.cold_entries(), 0);
        Ok(())
    })
}

/// A live subscriber can reject the phase callsite without preparing its fields.
/// ´test:crate:work-tracing-filtered-fields´
#[test]
fn work_tracing_filtered_fields() -> TestResult {
    let fields = Cell::new(0);
    let mut budget = Budget::new(Limits::default())?;
    let (result, report) = crate::testing::work_trace::filtered(8, false, || {
        let _scope = Scope::rule(|| {
            fields.set(fields.get() + 1);
            "filtered rule"
        });
        observe!(
            budget,
            {
                fields.set(fields.get() + 1);
                Location::new("filtered operation")
            },
            { budget.work(1, Location::new("ordinary operation")) }
        )
    });
    result?;
    assert!(report.complete);
    assert_eq!(report.rows, [] as [crate::testing::work_trace::Row; 0]);
    assert_eq!(fields.get(), 0);
    assert_eq!(budget.usage().work, 1);
    assert!(!budget.observation.assigned());
    assert_eq!(budget.observation.cold_entries(), 0);
    Ok(())
}

/// Measure a fixed population with the same opaque call and ledger access on each route.
fn elapsed(run: fn(&mut Budget) -> Result<(), Fault>, count: u32) -> TestResult<f64> {
    let mut budget = Budget::new(Limits {
        work: u64::from(count),
        ..Limits::default()
    })?;
    let start = Instant::now();
    for _ in 0..count {
        black_box(run)(black_box(&mut budget))?;
    }
    let seconds = start.elapsed().as_secs_f64();
    assert_eq!(budget.usage().work, u64::from(count));
    assert_eq!(budget.observation.cold_entries(), 0);
    Ok(seconds * 1_000_000_000.0 / f64::from(count))
}

/// Paired release timings report per-helper overhead without imposing a timing pass band.
/// ´test:crate:work-tracing-disabled-overhead´
#[test]
#[ignore = "single-core release measurement on the recorded compute environment"]
fn work_tracing_disabled_overhead() -> TestResult {
    without_observation(|| -> TestResult {
        let mut registration = Budget::new(Limits::default())?;
        instrumented(&mut registration)?;
        assert!(!registration.observation.assigned());
        let count = 10_000_000;
        let mut output = std::io::stdout().lock();
        writeln!(
            output,
            "disabled-work-tracing max_level={}",
            tracing::level_filters::LevelFilter::current()
        )?;
        let _plain_warmup = elapsed(plain, count)?;
        let _instrumented_warmup = elapsed(instrumented, count)?;
        let mut differences = Vec::new();
        for run in 0..9 {
            let (plain_ns, instrumented_ns) = if run % 2 == 0 {
                (elapsed(plain, count)?, elapsed(instrumented, count)?)
            } else {
                let instrumented_ns = elapsed(instrumented, count)?;
                (elapsed(plain, count)?, instrumented_ns)
            };
            differences.push(instrumented_ns - plain_ns);
            writeln!(
                output,
                "disabled-work-tracing run={run} count={count} plain_ns={plain_ns:.6} instrumented_ns={instrumented_ns:.6}"
            )?;
        }
        differences.sort_by(f64::total_cmp);
        let median = differences.get(4).ok_or("paired median")?;
        writeln!(output, "disabled-work-tracing median_added_ns={median:.6}")?;
        Ok(())
    })
}
