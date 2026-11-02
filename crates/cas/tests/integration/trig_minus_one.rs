// SPDX-License-Identifier: AGPL-3.0-only
//! Complete recipe replay exposes the totality discharge and every ordered expansion link.

use crate::testing::answer::{index, records};
use crate::testing::common::{TestResult, default_budget};
use crate::testing::trig_minus_one::{fixture, premises};
use nrz_cas::proof::{Primitive, TrigRecipeOutcome, VerifiedWitness};
use nrz_cas::{Cas, FaultKind};

fn produced(kind: usize) -> TestResult<(Cas, VerifiedWitness)> {
    let (mut f, function) = fixture(kind, false)?;
    let [guard, equality] = premises(&mut f)?;
    let TrigRecipeOutcome::Proved(witness) =
        f.cas
            .trig_atan_minus_one(&function, &guard, &equality, &mut default_budget()?)?
    else {
        return Err("recipe unresolved".into());
    };
    assert_eq!(witness.judgment(), &f.expected);
    Ok((f.cas, *witness))
}

fn discharge(cas: &Cas, witness: &VerifiedWitness) -> TestResult {
    let citations = cas.proof_citations(witness.proof(), &mut default_budget()?)?;
    let close = citations
        .iter()
        .find(|node| {
            node.primitive == Primitive::ContextClose
                && node.premises.first().is_some_and(|first| {
                    citations.iter().any(|child| {
                        child.proof == first.proof && child.primitive == Primitive::Transitivity
                    })
                })
        })
        .ok_or("scalar chain close")?;
    let child = citations
        .iter()
        .find(|node| node.proof == close.premises[0].proof)
        .ok_or("chain")?;
    let local: Vec<_> = child
        .hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.context != witness.judgment().context)
        .collect();
    assert_ne!(local, [] as [&nrz_cas::proof::QualifiedHypothesis; 0]);
    assert_eq!(close.premises.len(), local.len() + 2);
    assert!(
        close
            .hypotheses
            .iter()
            .all(|hypothesis| hypothesis.context == witness.judgment().context)
    );
    if witness.judgment().residuals.is_empty() {
        assert_eq!(close.residuals, []);
    } else {
        for step in [child, close] {
            assert_ne!(step.residuals, []);
            assert!(
                step.residuals
                    .iter()
                    .all(|residual| *residual == nrz_cas::proof::HypothesisKind::NonZero)
            );
        }
    }
    for fact in close.premises.iter().skip(2) {
        let restriction = citations
            .iter()
            .find(|node| node.proof == fact.proof)
            .ok_or("restricted fact")?;
        assert_eq!(restriction.primitive, Primitive::ContextClose);
        let weakened = citations
            .iter()
            .find(|node| node.proof == restriction.premises[0].proof)
            .ok_or("weakened fact")?;
        assert_eq!(weakened.primitive, Primitive::ContextWeaken);
        let totality = citations
            .iter()
            .find(|node| node.proof == weakened.premises[0].proof)
            .ok_or("totality fact")?;
        assert_eq!(totality.primitive, Primitive::TrigDefined);
    }
    Ok(())
}

/// Independently built expected endpoints retain the close's discharge in either allocation order.
/// ´test:integration:trig-minus-one-fresh-replay´
#[test]
fn trig_minus_one_fresh_replay() -> TestResult {
    for kind in [0, 1, 2, 3, 4] {
        let (source, witness) = produced(kind)?;
        discharge(&source, &witness)?;
        let bytes = source.export_witness(&witness, &mut default_budget()?)?;
        for reordered in [false, true] {
            let (mut receiver, _) = fixture(kind, reordered)?;
            let replayed =
                receiver
                    .cas
                    .replay(&bytes, &receiver.expected, &mut default_budget()?)?;
            discharge(&receiver.cas, &replayed)?;
            let mut changed = receiver.expected.clone();
            let nrz_cas::proof::Statement::Equality { left, right, .. } = &mut changed.statement
            else {
                return Err("expected equality".into());
            };
            *right = left.clone();
            let before = receiver.cas.canonical_snapshot(&mut default_budget()?)?;
            assert!(
                receiver
                    .cas
                    .replay(&bytes, &changed, &mut default_budget()?)
                    .is_err()
            );
            assert_eq!(
                receiver.cas.canonical_snapshot(&mut default_budget()?)?,
                before
            );
        }
    }
    Ok(())
}

/// Every removed premise is a full-record mutation, including each totality discharge link.
/// ´test:integration:trig-minus-one-severed-links´
#[test]
fn trig_minus_one_severed_links() -> TestResult {
    for kind in [0, 1, 2, 3, 4] {
        let (source, witness) = produced(kind)?;
        let bytes = source.export_witness(&witness, &mut default_budget()?)?;
        let inventory = records(&bytes)?;
        let mut mutations = 0;
        for (_, application) in inventory.iter().filter(|(tag, _)| *tag == 11) {
            assert_eq!(
                bytes.get(application.start..application.start + 4),
                Some(1_u32.to_le_bytes().as_slice())
            );
            assert!(matches!(bytes.get(application.start + 4), Some(0 | 1)));
            assert_eq!(bytes.get(application.start + 5), Some(&0));
            let count = index(&bytes, &mut (application.start + 14))?;
            assert_eq!(
                index(&bytes, &mut (application.start - 4))?,
                u32::try_from(application.len())?
            );
            for slot in 0..count {
                let offset = application.start + 18 + 4 * usize::try_from(slot)?;
                let mut cursor = offset;
                let premise = usize::try_from(index(&bytes, &mut cursor)?)?;
                assert_eq!(inventory.get(premise).ok_or("proof record")?.0, 11);
                let mut changed = bytes.clone();
                changed
                    .get_mut(application.start + 14..application.start + 18)
                    .ok_or("count")?
                    .copy_from_slice(&(count - 1).to_le_bytes());
                changed.drain(offset..offset + 4);
                changed
                    .get_mut(application.start - 4..application.start)
                    .ok_or("length")?
                    .copy_from_slice(&u32::try_from(application.len() - 4)?.to_le_bytes());
                assert_eq!(records(&changed)?.len(), inventory.len());
                for reordered in [false, true] {
                    let (mut receiver, _) = fixture(kind, reordered)?;
                    let before = receiver.cas.canonical_snapshot(&mut default_budget()?)?;
                    let error = receiver
                        .cas
                        .replay(&changed, &receiver.expected, &mut default_budget()?)
                        .err()
                        .ok_or("severed link accepted")?;
                    assert!(matches!(error.kind, FaultKind::Invalid(_)));
                    assert_eq!(
                        receiver.cas.canonical_snapshot(&mut default_budget()?)?,
                        before
                    );
                    receiver
                        .cas
                        .replay(&bytes, &receiver.expected, &mut default_budget()?)?;
                    mutations += 1;
                }
            }
        }
        assert!(mutations > 20);
    }
    Ok(())
}

/// Conditional replay cannot discard either retained region of the original nonzero residual.
/// ´test:integration:trig-minus-one-conditional-residual-replay´
#[test]
fn trig_minus_one_conditional_residual_replay() -> TestResult {
    let (source, witness) = produced(2)?;
    let bytes = source.export_witness(&witness, &mut default_budget()?)?;
    for reordered in [false, true] {
        for removed in 0..3 {
            let (mut receiver, _) = fixture(2, reordered)?;
            assert_eq!(receiver.expected.residuals.len(), 2);
            let mut expected = receiver.expected.clone();
            if removed == 2 {
                expected.residuals.clear();
            } else {
                expected.residuals.remove(removed);
            }
            let before = receiver.cas.canonical_snapshot(&mut default_budget()?)?;
            let fault = receiver
                .cas
                .replay(&bytes, &expected, &mut default_budget()?)
                .expect_err("dropped residual");
            assert_eq!(
                fault.kind,
                FaultKind::Invalid(nrz_cas::InvalidCode::EndpointMismatch)
            );
            assert_eq!(
                receiver.cas.canonical_snapshot(&mut default_budget()?)?,
                before
            );
            let replayed =
                receiver
                    .cas
                    .replay(&bytes, &receiver.expected, &mut default_budget()?)?;
            assert_eq!(replayed.judgment(), &receiver.expected);
            discharge(&receiver.cas, &replayed)?;
        }
    }
    Ok(())
}
