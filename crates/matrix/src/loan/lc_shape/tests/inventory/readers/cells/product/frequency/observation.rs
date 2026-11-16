// SPDX-License-Identifier: AGPL-3.0-only
//! Joined attempts preserve native outcomes, resource neighbours and original attachment identity.

mod companion;
mod evidence;
mod identity;
mod subscriber;
use super::{TestResult, VALUES, publish_values, setup};
use crate::{ExecutionAccount, LcFrequencyPrelude, LcFrequencyState, LcInputProduct};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Comparable results retain actual values while errors keep their original typed formatting.
#[derive(Debug, PartialEq)]
struct Image<P: Scalar> {
    /// Local sign outcome and original target, without formatting Ready payloads.
    state: String,
    /// Frequency uses tau/rate; quantities use omega/f/T.
    values: Option<[[P; 3]; 3]>,
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

/// Both public routes retain joined Ready/Invalid/Undecided/refused attempts and exact old tariffs.
/// Disabled observation preserves values, errors, all budget dimensions and native activity.
/// ´test:unit:loan-lc-sign-attempt-outcomes´
#[test]
fn loan_lc_sign_attempt_outcomes() -> TestResult {
    outcomes::<f32>()?;
    outcomes::<f64>()
}
fn outcomes<P: Scalar + 'static>() -> TestResult {
    for quantities in [false, true] {
        for case in 0..10 {
            case_run::<P>(case, quantities)?;
        }
    }
    Ok(())
}
fn case_run<P: Scalar + 'static>(case: usize, quantities: bool) -> TestResult {
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
    let required = if quantities { 46 } else { 43 };
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
                &prelude,
                selected,
                if case == 7 { 12 } else { 3 },
                quantities,
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
                quantities,
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
    prelude: &LcFrequencyPrelude<P>,
    account: &mut ExecutionAccount,
    cell: usize,
    quantities: bool,
) -> Snapshot<P> {
    crate::loan::fault::reset_reads();
    crate::loan::fault::reset_scalars();
    let mut before = account.usage();
    let image = if quantities {
        cells
            .read_quantities(cell, prelude, account)
            .map(|result| Image {
                state: state(result.frequency().state()),
                values: result
                    .values()
                    .map(|values| [values.omega, values.frequency, values.period]),
            })
    } else {
        cells.read_frequency(cell, account).map(|result| Image {
            state: state(result.state()),
            values: if let LcFrequencyState::Ready { tau, rate } = result.state() {
                Some([tau, rate, [P::ZERO; 3]])
            } else {
                None
            },
        })
    }
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
fn state<P: Scalar>(state: LcFrequencyState<P>) -> String {
    match state {
        LcFrequencyState::Ready { .. } => "Ready".to_owned(),
        LcFrequencyState::Invalid(target) => format!("Invalid({target:?})"),
        LcFrequencyState::Undecided(target) => format!("Undecided({target:?})"),
    }
}
