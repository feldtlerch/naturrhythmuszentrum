// SPDX-License-Identifier: AGPL-3.0-only
//! Both source routes and scalar formats retain consumed prefixes at literal resource boundaries.

#[path = "costs/fixture.rs"]
mod fixture;
#[path = "costs/payload.rs"]
mod payload;
#[path = "costs/pins.rs"]
mod pins;
#[path = "costs/reads.rs"]
mod reads;
use super::{TestResult, ledger};
use fixture::{AT, PRIOR, Pin, Source, work};
use nrz_scalar::Scalar;

/// Every outcome has literal cumulative preparation and final publication Work boundaries.
/// ´test:integration:lc-axis-phase-cumulative-work´
#[test]
fn lc_axis_phase_cumulative_work() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

/// Both original source routes keep the same caller prefix through each stage.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for block in [false, true] {
        for (index, pin) in pins::pins::<P>(block).into_iter().enumerate() {
            for short in [0, 1] {
                preparation::<P>(index, block, pin, short)?;
                publication::<P>(index, block, pin, short)?;
            }
        }
    }
    Ok(())
}

/// A refused attempt consumes its admitted prefix without creating heap or live output state.
fn preparation<P: Scalar + 'static>(index: usize, block: bool, pin: Pin, short: u64) -> TestResult {
    let source = Source::<P>::new(index, block)?;
    let usage = source.heap().provenance_usage();
    let mut budget = work(PRIOR + pin.work[0] - short)?;
    budget.work(PRIOR, AT)?;
    let result = source.prepare(index, &mut budget);
    assert_eq!(result.is_ok(), short == 0, "{result:?}");
    assert!(budget.usage().work > PRIOR);
    assert_eq!(source.heap().provenance_usage(), usage);
    if let Ok(prepared) = result {
        assert_eq!(budget.usage().work, PRIOR + pin.work[0]);
        let published = source.heap().publish_axis_phase(prepared, &mut ledger()?)?;
        let attempt = published.current(source.heap(), source.context(), &mut ledger()?)?;
        assert_eq!(attempt.usage()[1].work, PRIOR + pin.work[0]);
        assert!(attempt.usage()[0].work > PRIOR);
        assert_eq!(published.retained_payload_bytes(), pin.bytes[2]);
        assert_eq!(published.inputs().retained_payload_bytes(), pin.bytes[3]);
    }
    Ok(())
}

/// Publication consumes the same actual ledger as capture, lowering and the original sign attempt.
fn publication<P: Scalar + 'static>(index: usize, block: bool, pin: Pin, short: u64) -> TestResult {
    let source = Source::<P>::new(index, block)?;
    let usage = source.heap().provenance_usage();
    let needed = PRIOR + pin.work[0] + pin.work[1];
    let mut budget = work(needed - short)?;
    budget.work(PRIOR, AT)?;
    let prepared = source.prepare(index, &mut budget)?;
    let prefix = budget.usage().work;
    assert_eq!(prefix, PRIOR + pin.work[0]);
    let result = source.heap().publish_axis_phase(prepared, &mut budget);
    assert_eq!(result.is_ok(), short == 0, "{result:?}");
    assert!(budget.usage().work > prefix);
    assert_eq!(source.heap().provenance_usage(), usage);
    if result.is_ok() {
        assert_eq!(budget.usage().work, needed);
    }
    Ok(())
}
