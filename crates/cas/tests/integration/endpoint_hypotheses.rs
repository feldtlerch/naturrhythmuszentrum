// SPDX-License-Identifier: AGPL-3.0-only
//! Qualified scalar endpoint provenance across closure and portable replay.

use nrz_cas::syntax::PredicateKind;

use crate::testing::budgets::proof_budget as budget;
use crate::testing::common::TestResult;
use crate::testing::proofs::cancellation;

/// Root assumption provenance preserves the existing portable cancellation witness.
/// ´test:integration:endpoint-root-witness-bytes´
#[test]
fn endpoint_root_witness_bytes() -> TestResult {
    let (mut fixture, _, mut step) = cancellation()?;
    fixture.assume("positive-x", PredicateKind::Positive, &fixture.x.clone())?;
    step.context = fixture.context.clone();
    let witness = fixture
        .cas
        .check_proof(&fixture.context, &[step], &mut budget()?)?;
    let bytes = fixture.cas.export_witness(&witness, &mut budget()?)?;
    assert_eq!(bytes, include_bytes!("endpoint_root.bin"));
    Ok(())
}

/// Exact local endpoint slots close with evidence and remain residual without it.
/// ´test:integration:endpoint-qualified-close-replay´
#[test]
fn endpoint_qualified_close_replay() -> TestResult {
    for consume in [false, true] {
        let mut source = crate::testing::endpoint_hypotheses::chain(false, consume)?;
        let witness = source.fixture.cas.check_proof(
            &source.fixture.context,
            &source.steps,
            &mut budget()?,
        )?;
        assert_eq!(witness.judgment(), &source.expected);
        assert_eq!(witness.hypotheses(), []);
        let bytes = source
            .fixture
            .cas
            .export_witness(&witness, &mut budget()?)?;
        for reordered in [false, true] {
            let mut receiver = crate::testing::endpoint_hypotheses::chain(reordered, consume)?;
            let replayed =
                receiver
                    .fixture
                    .cas
                    .replay(&bytes, &receiver.expected, &mut budget()?)?;
            assert_eq!(
                receiver
                    .fixture
                    .cas
                    .export_witness(&replayed, &mut budget()?)?,
                bytes
            );
            if !consume {
                let mut stripped = receiver.expected.clone();
                stripped.residuals.clear();
                let before = receiver.fixture.cas.canonical_snapshot(&mut budget()?)?;
                assert!(
                    receiver
                        .fixture
                        .cas
                        .replay(&bytes, &stripped, &mut budget()?)
                        .is_err()
                );
                assert_eq!(
                    receiver.fixture.cas.canonical_snapshot(&mut budget()?)?,
                    before
                );
            }
        }
    }
    Ok(())
}
