// SPDX-License-Identifier: AGPL-3.0-only
//! Complete mixed ordinary and series witnesses in fresh receiving stores.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_proof::fixture;

/// The producer admits missing output syntax, then both stores check the hand-built expectation.
/// ´test:integration:series-output-absent-production´
#[test]
fn series_output_absent_production() -> TestResult {
    for kind in 0..12 {
        let mut source = crate::testing::series_proof::production(kind)?;
        let witness = source.base.cas.compute_series(
            &source.operation,
            &source.proofs,
            &mut default_budget()?,
        )?;
        let bytes = source
            .base
            .cas
            .export_witness(&witness, &mut default_budget()?)?;
        for reordered in [false, true] {
            let mut expected = fixture(kind, reordered)?;
            let checked =
                expected
                    .base
                    .cas
                    .replay(&bytes, &expected.expected, &mut default_budget()?)?;
            assert!(checked.is_discharged());
        }
        let hit = source.base.cas.compute_series(
            &source.operation,
            &source.proofs,
            &mut default_budget()?,
        )?;
        assert_eq!(
            source
                .base
                .cas
                .export_witness(&hit, &mut default_budget()?)?,
            bytes
        );
    }
    Ok(())
}

/// Independently built expectations accept in both allocation orders without producer state.
/// ´test:integration:series-fresh-expected-replay´
#[test]
fn series_fresh_expected_replay() -> TestResult {
    for kind in 0..12 {
        let mut sender = fixture(kind, false)?;
        let witness = sender.base.cas.check_proof(
            &sender.base.context,
            &sender.steps,
            &mut default_budget()?,
        )?;
        let bytes = sender
            .base
            .cas
            .export_witness(&witness, &mut default_budget()?)?;
        for reordered in [false, true] {
            let mut receiver = fixture(kind, reordered)?;
            let replay =
                receiver
                    .base
                    .cas
                    .replay(&bytes, &receiver.expected, &mut default_budget()?)?;
            assert_eq!(replay.judgment(), &receiver.expected);
            assert_eq!(replay.hypotheses(), []);
            assert_eq!(
                receiver
                    .base
                    .cas
                    .export_witness(&replay, &mut default_budget()?)?,
                bytes
            );
        }
    }
    Ok(())
}

/// Mixed ordinary premises retain qualified assumptions and closed residuals in fresh replay.
/// ´test:integration:series-scale-condition-replay´
#[test]
fn series_scale_condition_replay() -> TestResult {
    for (integral, residual) in [(false, false), (false, true), (true, false), (true, true)] {
        let make = if integral {
            crate::testing::series_proof::conditioned_integral
        } else {
            crate::testing::series_proof::conditioned
        };
        let mut source = make(residual, false)?;
        let witness = source.base.cas.check_proof(
            &source.base.context,
            &source.steps,
            &mut default_budget()?,
        )?;
        assert_eq!(witness.judgment(), &source.expected);
        assert_eq!(witness.hypotheses().len(), usize::from(!residual));
        assert_eq!(witness.judgment().residuals.len(), usize::from(residual));
        let bytes = source
            .base
            .cas
            .export_witness(&witness, &mut default_budget()?)?;
        for reordered in [false, true] {
            let mut receiver = make(residual, reordered)?;
            if residual {
                let mut changed = receiver.expected.clone();
                changed.residuals.clear();
                let before = receiver
                    .base
                    .cas
                    .canonical_snapshot(&mut default_budget()?)?;
                assert!(
                    receiver
                        .base
                        .cas
                        .replay(&bytes, &changed, &mut default_budget()?)
                        .is_err()
                );
                assert_eq!(
                    receiver
                        .base
                        .cas
                        .canonical_snapshot(&mut default_budget()?)?,
                    before
                );
            }
            let replay =
                receiver
                    .base
                    .cas
                    .replay(&bytes, &receiver.expected, &mut default_budget()?)?;
            assert_eq!(replay.hypotheses().len(), usize::from(!residual));
            assert_eq!(
                receiver
                    .base
                    .cas
                    .export_witness(&replay, &mut default_budget()?)?,
                bytes
            );
        }
    }
    Ok(())
}
