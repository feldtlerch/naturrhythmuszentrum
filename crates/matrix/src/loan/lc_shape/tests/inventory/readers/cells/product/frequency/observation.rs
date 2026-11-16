// SPDX-License-Identifier: AGPL-3.0-only
//! Joined attempts preserve native outcomes, resource neighbours and original attachment identity.

mod companion;
mod evidence;
mod identity;
mod subscriber;
use super::{TestResult, VALUES, publish_values, setup};
use crate::{
    ExecutionAccount, LcFrequencyPrelude, LcInputProduct, LcQuantityDemand, LcRequestedState,
};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Comparable results retain actual values while errors keep their original typed formatting.
#[derive(Debug, PartialEq)]
struct Image<P: Scalar> {
    /// Local sign outcome and original target, without formatting Ready payloads.
    state: String,
    /// Actual common root dependency, absent when the local path stops before Ready.
    tau: Option<[P; 3]>,
    /// Actual reciprocal dependency, absent for period-only demand.
    rate: Option<[P; 3]>,
    /// Requested omega/f/T slots remain absent when the demand excludes them.
    values: Option<[Option<[P; 3]>; 3]>,
}
/// Numerical effects are measured independently of the tracing subscriber.
#[derive(Debug, PartialEq)]
struct Snapshot<P: Scalar> {
    /// Original public result, reduced only to comparable metadata and native values.
    image: Result<Image<P>, String>,
    /// Actual account delta.
    work: u64,
    /// Actual companion loads and scalar kernel entries.
    native: [usize; 2],
}

/// Every demand retains joined Ready/Invalid/Undecided/refused attempts and literal tariffs.
/// Disabled observation preserves values, errors, all budget dimensions and native activity.
/// ´test:unit:loan-lc-sign-attempt-outcomes´
#[test]
fn loan_lc_sign_attempt_outcomes() -> TestResult {
    outcomes::<f32>()?;
    outcomes::<f64>()
}
fn outcomes<P: Scalar + 'static>() -> TestResult {
    for demand in DEMANDS {
        for case in 0..11 {
            if case == 10 && demand == LcQuantityDemand::Omega {
                continue;
            }
            case_run::<P>(case, demand)?;
        }
    }
    Ok(())
}
fn case_run<P: Scalar + 'static>(case: usize, demand: LcQuantityDemand) -> TestResult {
    let (mut heap, mut account, inventory, axes) = setup::<P>()?;
    let mut budget = Budget::new(Limits::default())?;
    let context = ArithmeticContext::new()?;
    let pi = super::quantities::pi::<P>(&context, &mut budget)?;
    let prelude = LcFrequencyPrelude::new(&pi, &context, &mut budget)?;
    let prepared = budget.usage();
    let mut masks = std::array::from_fn(|_| vec![0, 3, 11]);
    if case == 6 {
        masks[4] = vec![0, 11];
    }
    let members = publish_values(
        &inventory,
        &axes,
        &mut heap,
        &mut account,
        &masks,
        1,
        &values::<P>(case),
    )?;
    let references = members.references();
    let cells = inventory
        .readers(&heap, &references, &mut account)?
        .into_cells(&mut account)?
        .into_product([0, 1], &mut account)?;
    let mut other = ExecutionAccount::new(Budget::new(Limits::default())?)?;
    let original = account.usage();
    let selected = if case == 8 { &mut other } else { &mut account };
    let (required, _, _) = spec(demand);
    if case == 9 {
        let spend = selected.limits().work - selected.usage().work - (required - 1);
        selected.ledger.work(spend, crate::loan::AT)?;
    }
    let mut previous = None;
    let mut attempts = Vec::new();
    for enabled in [true, false, true] {
        let injection = crate::loan::fault::Injection::after(0);
        let (snapshot, records) = subscriber::measured(enabled, || {
            sample(
                &cells,
                if case == 10 { None } else { Some(&prelude) },
                selected,
                if case == 7 { 12 } else { 3 },
                demand,
            )
        });
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        let records = records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if enabled {
            attempts.push(evidence::check(
                &records,
                case,
                demand,
                &snapshot,
                &references,
                &inventory,
            )?);
        } else {
            assert!(records.spans.is_empty());
            assert!(records.events.is_empty());
        }
        drop(records);
        if let Some(previous) = &previous {
            assert_eq!(&snapshot, previous);
        }
        previous = Some(snapshot);
    }
    assert_ne!(attempts[0], attempts[1]);
    if case == 8 {
        assert_eq!(account.usage(), original);
    }
    assert_eq!(budget.usage(), prepared);
    drop(references);
    let mut scanner = ExecutionAccount::new(Budget::new(Limits::default())?)?;
    members.finish(&mut heap, &mut scanner)
}

fn values<P: Scalar>(case: usize) -> [[[P; 3]; 2]; 5] {
    let mut values = VALUES.map(|rows| rows.map(|row| row.map(|n| P::from_u8(n) / P::from_u8(32))));
    match case {
        1 => values[0][0] = [P::ZERO; 3],
        2 => values[1][0] = [P::ONE, P::ONE, P::ZERO],
        3 => {
            for role in [0, 1] {
                values[role][0] = [P::ONE, P::from_u8(3) / P::from_u8(4), P::ZERO];
            }
        }
        4 => values[0][0] = [-P::ONE, P::ZERO, P::ZERO],
        5 => {
            values[0][0] = [P::max_finite(), P::ZERO, P::ZERO];
            values[1][0] = [P::from_u8(2), P::ZERO, P::ZERO];
        }
        _ => (),
    }
    values
}
fn sample<P: Scalar + 'static>(
    cells: &LcInputProduct<'_, P, 5>,
    prelude: Option<&LcFrequencyPrelude<P>>,
    account: &mut ExecutionAccount,
    cell: usize,
    demand: LcQuantityDemand,
) -> Snapshot<P> {
    crate::loan::fault::reset_reads();
    crate::loan::fault::reset_scalars();
    let mut before = account.usage();
    let image = cells
        .read_requested(cell, demand, prelude, account)
        .map(|result| {
            let (tau, rate) = match result.state() {
                LcRequestedState::Ready { tau, rate, .. } => (Some(tau), rate),
                LcRequestedState::Undecided(_) | LcRequestedState::Invalid(_) => (None, None),
            };
            Image {
                state: state(result.state()),
                tau,
                rate,
                values: result
                    .values()
                    .map(|values| [values.omega, values.frequency, values.period]),
            }
        })
        .map_err(|error| format!("{error:?}"));
    let after = account.usage();
    let work = after.work - before.work;
    before.work = after.work;
    assert_eq!(before, after);
    Snapshot {
        image,
        work,
        native: [crate::loan::fault::reads(), crate::loan::fault::scalars()],
    }
}
fn state<P: Scalar>(state: LcRequestedState<P>) -> String {
    match state {
        LcRequestedState::Ready { .. } => "Ready".to_owned(),
        LcRequestedState::Invalid(target) => format!("Invalid({target:?})"),
        LcRequestedState::Undecided(target) => format!("Undecided({target:?})"),
    }
}

/// Literal independent request inventory and complete-call expectations.
pub(super) const DEMANDS: [LcQuantityDemand; 7] = [
    LcQuantityDemand::Omega,
    LcQuantityDemand::Frequency,
    LcQuantityDemand::Period,
    LcQuantityDemand::OmegaFrequency,
    LcQuantityDemand::OmegaPeriod,
    LcQuantityDemand::FrequencyPeriod,
    LcQuantityDemand::All,
];
/// Literal tariffs and scalar entries do not use the production dependency calculation.
pub(super) const fn spec(demand: LcQuantityDemand) -> (u64, usize, &'static str) {
    match demand {
        LcQuantityDemand::Omega => (43, 3, "omega"),
        LcQuantityDemand::Frequency => (45, 4, "frequency"),
        LcQuantityDemand::Period => (44, 3, "period"),
        LcQuantityDemand::OmegaFrequency => (45, 4, "omega-frequency"),
        LcQuantityDemand::OmegaPeriod => (45, 4, "omega-period"),
        LcQuantityDemand::FrequencyPeriod => (46, 5, "frequency-period"),
        LcQuantityDemand::All => (46, 5, "omega-frequency-period"),
    }
}
