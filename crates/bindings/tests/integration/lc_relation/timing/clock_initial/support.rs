// SPDX-License-Identifier: AGPL-3.0-only
//! Exact original shared-clock fixtures and cumulative Work boundaries.

use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    Budget, LcClockPhaseParameters, LcInput, LcInterval, LcQuantity as Q, LcRelation,
    LcRelationError, LcSharedClock, Limits, LimitsOverride,
};

/// Original timestamps contain exactly the declared nominal-plus-offset image.
pub(super) fn clock(
    initial: Option<[&str; 2]>,
    offsets: [&str; 2],
    extra: Vec<LcInput>,
) -> TestResult<LcSharedClock> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if let Some(initial) = initial {
        inputs.push(input(Q::InitialPhase, initial)?);
    }
    let offset = interval(offsets)?;
    let offsets = [number(offsets[0])?, number(offsets[1])?];
    for (index, (nominal, phase)) in [("1", "2"), ("2", "4")].into_iter().enumerate() {
        let nominal = number(nominal)?;
        let time = LcInterval::new(
            Some(nominal.add(&offsets[0], &mut ledger()?)?.into()),
            Some(nominal.add(&offsets[1], &mut ledger()?)?.into()),
            &mut ledger()?,
        )?;
        let quantity = Q::Time(index);
        inputs.push(LcInput::new(
            quantity,
            &quantity.descriptor()?,
            time,
            &mut ledger()?,
        )?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    inputs.extend(extra);
    let relation = LcRelation::new(
        "unknown initial phase clock epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &offset,
        &mut ledger()?,
    )?)
}

/// Exact signed supports retain physical clock and phase meanings separately.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Original alpha and offset are explicitly chosen; physical scales remain positive and free.
pub(super) fn parameters(
    rate: &str,
    phase: &str,
    offset: &str,
    gauge: &str,
) -> TestResult<LcClockPhaseParameters> {
    Ok(LcClockPhaseParameters {
        rate: number(rate)?,
        initial_phase: number(phase)?,
        gauge: number(gauge)?,
        offset: number(offset)?,
        ratio: number("1")?,
        voltage: number("1")?,
    })
}

/// Fixed nonzero initial phase is published as its original physical value, never effective beta.
pub(super) fn fixed_nonzero() -> TestResult {
    let clock = clock(Some(["1/10", "1/10"]), ["-1/10", "1/10"], Vec::new())?;
    let cover = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let phase = cover.bounds(Q::InitialPhase, &mut ledger()?)?;
    assert_eq!(phase.lower(), Some(&number("1/10")?.into()));
    assert_eq!(phase.upper(), phase.lower());
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/20")?, number("-1/20")?]
    );
    let point =
        cover.witness_with_phase(&parameters("2", "1/10", "-1/20", "0")?, &mut ledger()?)?;
    clock.check_witness(
        point.original().values().clone(),
        number("-1/20")?,
        &mut ledger()?,
    )?;
    Ok(())
}

/// Caller Work already spent is retained; the adjacent failure must specifically exhaust Work.
pub(super) fn work_boundary<T>(
    mut operation: impl FnMut(&mut Budget) -> Result<T, LcRelationError>,
) -> TestResult {
    let mut measured = ledger()?;
    assert!(operation(&mut measured).is_ok());
    let work = measured.usage().work;
    assert!(work > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(17, nrz_cas::Location::new("clock initial phase prefix"))?;
        let outcome = operation(&mut budget);
        assert_eq!(outcome.is_ok(), succeeds);
        if !succeeds {
            assert!(matches!(
                outcome,
                Err(LcRelationError::Algebra(nrz_cas::Fault {
                    kind: nrz_cas::FaultKind::Exhausted {
                        budget: nrz_cas::limits::BudgetKind::Work,
                        ..
                    },
                    ..
                }))
            ));
        }
    }
    Ok(())
}
