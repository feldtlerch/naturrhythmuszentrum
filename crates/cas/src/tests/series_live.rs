// SPDX-License-Identifier: AGPL-3.0-only
//! Owned premise inventories and typed analytic evidence require kernel-visible inspection.

use crate::budget::{Budget, BudgetKind, Limits};
use crate::error::{FaultKind, InvalidCode};
use crate::graph::{Draft, Handle, Reference};
use crate::proof_model::{ApplicationDraft, VerifiedWitness};
use crate::series::SeriesOperation as O;
use crate::testing::common::TestResult;
use crate::testing::native::{Fixture as Base, admit, budget as default_budget, term};

struct Fixture {
    base: Base,
    field: Handle,
}

fn fixture(reordered: bool) -> TestResult<Fixture> {
    let mut base = Base::new()?;
    for value in if reordered { ["3", "2"] } else { ["2", "3"] } {
        term(
            &mut base.cas,
            &base.header,
            crate::model::Operator::Rational(crate::Rational::parse(
                value,
                &mut default_budget()?,
            )?),
            &[],
        )?;
    }
    let field = admit(
        &mut base.cas,
        Draft::Solve(crate::solving::SolveDraft::CoefficientDomain(
            crate::solving::CoefficientDomainData::Rational,
        )),
    )?;
    Ok(Fixture { base, field })
}

fn object(base: &mut Base, field: &Handle, values: &[&str]) -> TestResult<Handle> {
    let mut coefficients = Vec::new();
    for text in values {
        let value = crate::Rational::parse(text, &mut default_budget()?)?;
        coefficients.push(Reference::Existing(term(
            &mut base.cas,
            &base.header,
            crate::model::Operator::Rational(value),
            &[],
        )?));
    }
    admit(
        &mut base.cas,
        Draft::SeriesObject(crate::series::SeriesObject {
            field: Reference::Existing(field.clone()),
            centre: Reference::Existing(base.zero.clone()),
            variable: Reference::Existing(base.x.clone()),
            scope: Vec::new(),
            context: Reference::Existing(base.context.clone()),
            order: u32::try_from(values.len())?,
            coefficients,
        }),
    )
}

fn index(bytes: &[u8], offset: &mut usize) -> TestResult<u32> {
    let value = u32::from_le_bytes(bytes.get(*offset..*offset + 4).ok_or("index")?.try_into()?);
    *offset += 4;
    Ok(value)
}

fn records(bytes: &[u8]) -> TestResult<Vec<(u8, std::ops::Range<usize>)>> {
    assert_eq!(&bytes[..8], b"NRZCAS05");
    let mut at = 8;
    let count = index(bytes, &mut at)?;
    for _ in 0..count {
        let len = index(bytes, &mut at)?;
        at += usize::try_from(len)? + 4;
    }
    let count = index(bytes, &mut at)?;
    let mut result = Vec::new();
    for _ in 0..count {
        let tag = *bytes.get(at).ok_or("tag")?;
        at += 1;
        let len = usize::try_from(index(bytes, &mut at)?)?;
        result.push((tag, at..at + len));
        at += len;
    }
    assert_eq!(at + 4, bytes.len());
    Ok(result)
}

/// A complete valid product retains many large rational recurrence rows.
fn large() -> TestResult<(Fixture, VerifiedWitness)> {
    let mut f = fixture(false)?;
    let text = "123456789012345678901234567890123456789012345678901234567890123";
    let input = object(&mut f.base, &f.field, &[text; 8])?;
    let witness = f.base.cas.compute_series(
        &O::Mul {
            left: input.clone(),
            right: input,
            order: None,
        },
        &[],
        &mut default_budget()?,
    )?;
    Ok((f, witness))
}

/// Wrong arity stops before proof sizing; correct arity reserves all owned copies together.
/// ´test:crate:series-live-premise-payload´
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Both allocation orders keep exact aggregate arithmetic beside arity, rollback and valid construction controls"
)]
fn series_live_premise_payload() -> TestResult {
    let (source, original) = large()?;
    let bytes = source
        .base
        .cas
        .export_witness(&original, &mut default_budget()?)?;
    for reordered in [false, true] {
        let mut f = fixture(reordered)?;
        let witness = f.base.cas.inspect(&bytes, &mut default_budget()?)?;
        let proof = witness.proof().clone();
        // The operation's small coefficients keep its workspace independent of the
        // unrelated large proofs supplied in its two typed restriction slots.
        let input = object(&mut f.base, &f.field, &["1", "2"])?;
        let restriction = f.base.cas.compute_series(
            &O::Truncate {
                input: input.clone(),
                order: 1,
            },
            &[],
            &mut default_budget()?,
        )?;
        let payload = crate::proof_view::application_bytes(
            &f.base.cas,
            proof.logical_id(),
            &mut default_budget()?,
        )?;
        assert_eq!(payload, 30_612);
        let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
        let mut budget = Budget::new(Limits {
            work: 257,
            ..Limits::default()
        })?;
        assert_eq!(
            f.base
                .cas
                .compute_series(
                    &O::Coefficient {
                        input: input.clone(),
                        index: 0
                    },
                    &[proof.clone(), proof.clone()],
                    &mut budget,
                )
                .expect_err("zero typed slots")
                .kind,
            FaultKind::Invalid(InvalidCode::Rule)
        );
        assert_eq!(budget.usage().work, 257);
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );

        // Measure the enclosing checkpoint on this exact admitted store.
        let mut probe = default_budget()?;
        let outer = probe.limits().staged_payload_bytes;
        let effective = f.base.cas.transaction(&mut probe, |_, budget| {
            Ok(budget.limits().staged_payload_bytes)
        })?;
        let checkpoint = outer - effective;
        assert_eq!(checkpoint, 640);
        // Order one, two empty scopes (eight encoded bytes), maximum component
        // width two: W = 2 * ((2 * 4 + 8) * 16 + 512) + 8 * 4.
        let workspace = 1_568;
        let needed = workspace + payload * 2;
        let effective = needed - 1;
        let allowance = checkpoint + effective;
        assert_eq!((allowance, effective, needed), (63_431, 62_791, 62_792));
        assert!(workspace + payload <= effective);
        assert!(effective < workspace + payload * 2);
        let operation = O::Add {
            left: input.clone(),
            right: input,
            order: Some(1),
        };
        let mut budget = Budget::new(Limits {
            staged_payload_bytes: allowance,
            ..default_budget()?.limits().clone()
        })?;
        let fault = f
            .base
            .cas
            .compute_series(&operation, &[proof.clone(), proof.clone()], &mut budget)
            .expect_err("the second live premise exceeds the combined allowance");
        assert_eq!(
            fault,
            crate::Fault::exhausted(
                BudgetKind::StagedPayloadBytes,
                effective,
                needed,
                crate::error::Location::new("formal series arithmetic"),
            )
        );
        assert_eq!(budget.usage().work, 179_311);
        assert_eq!(budget.limits().staged_payload_bytes, allowance);
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
        assert_eq!(
            f.base
                .cas
                .compute_series(&operation, &[proof.clone(), proof], &mut default_budget()?,)
                .expect_err("valid unrelated product is not a truncation")
                .kind,
            FaultKind::Invalid(InvalidCode::Rule)
        );
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
        let positive = f
            .base
            .cas
            .replay(&bytes, witness.judgment(), &mut default_budget()?)?;
        assert_eq!(
            f.base
                .cas
                .export_witness(&positive, &mut default_budget()?)?,
            bytes
        );
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
        // The same small Add has a valid checked construction with its actual restrictions.
        let positive = f.base.cas.compute_series(
            &operation,
            &[restriction.proof().clone(), restriction.proof().clone()],
            &mut default_budget()?,
        )?;
        let bytes = f
            .base
            .cas
            .export_witness(&positive, &mut default_budget()?)?;
        f.base
            .cas
            .replay(&bytes, positive.judgment(), &mut default_budget()?)?;
    }
    Ok(())
}

/// An isolated record-40 invocation retains the same checked earlier records as a full replay.
struct PortableCertificate {
    bytes: Vec<u8>,
    records: Vec<crate::wire_decode::Decoded>,
    manifest: Vec<crate::rule::Primitive>,
}

fn portable_certificate(
    cas: &mut crate::Cas,
    witness: &VerifiedWitness,
) -> TestResult<PortableCertificate> {
    use crate::domain_model::Layer;
    use crate::series::portable::{decode, describe};
    use crate::wire_build::Entity;
    use crate::wire_decode::Decoded;
    use crate::wire_read::Reader;
    let id = witness.proof().logical_id();
    let application = crate::proof_view::application(cas, id, &mut default_budget()?)?;
    let mut manifest = vec![application.rule];
    for citation in cas.proof_citations(witness.proof(), &mut default_budget()?)? {
        if !manifest.contains(&citation.primitive) {
            manifest.push(citation.primitive);
        }
    }
    let mut claim = describe(cas, &Entity::SeriesStatement(id), &mut default_budget()?)?;
    let mut records = Vec::new();
    for (offset, entity) in claim.references {
        let decoded = match entity {
            Entity::Object(id) => Decoded::Object(cas.handle(id)),
            Entity::Guard(guard) => Decoded::Guard(*guard),
            _ => return Err("claim dependency".into()),
        };
        assert_eq!(&claim.payload[offset..offset + 4], &0_u32.to_le_bytes());
        claim.payload[offset..offset + 4]
            .copy_from_slice(&u32::try_from(records.len())?.to_le_bytes());
        records.push(decoded);
    }
    let mut budget = default_budget()?;
    let mut reader = Reader::new(&claim.payload, &mut budget)?;
    let claim = decode(
        cas,
        39,
        Layer::Logic,
        None,
        &mut reader,
        &records,
        &manifest,
    )?;
    reader.finished()?;
    let mut records = vec![claim];
    let mut certificate = describe(cas, &Entity::SeriesCertificate(id), &mut default_budget()?)?;
    for (offset, entity) in certificate.references {
        let index = match entity {
            Entity::SeriesStatement(proof) if proof == id => 0,
            Entity::Object(proof) => {
                let index = u32::try_from(records.len())?;
                records.push(Decoded::Application(cas.handle(proof)));
                index
            }
            _ => return Err("certificate dependency".into()),
        };
        assert_eq!(
            &certificate.payload[offset..offset + 4],
            &0_u32.to_le_bytes()
        );
        certificate.payload[offset..offset + 4].copy_from_slice(&index.to_le_bytes());
    }
    Ok(PortableCertificate {
        bytes: certificate.payload,
        records,
        manifest,
    })
}

fn replay_certificate(
    cas: &mut crate::Cas,
    certificate: &PortableCertificate,
    budget: &mut Budget,
) -> Result<(), crate::Fault> {
    let mut reader = crate::wire_read::Reader::new(&certificate.bytes, budget)?;
    crate::series::portable::decode(
        cas,
        40,
        crate::domain_model::Layer::Logic,
        None,
        &mut reader,
        &certificate.records,
        &certificate.manifest,
    )?;
    reader.finished()
}

/// Earlier valid large proofs cannot be repeated into a certificate with no typed slots.
/// ´test:crate:series-repeated-portable-premise-payload´
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The full witness mutation and isolated consumed prefix retain their shared positive and rollback controls"
)]
fn series_repeated_portable_premise_payload() -> TestResult {
    let (source, witness) = large()?;
    let payload = crate::proof_view::application_bytes(
        &source.base.cas,
        witness.proof().logical_id(),
        &mut default_budget()?,
    )?;
    let bytes = source
        .base
        .cas
        .export_witness(&witness, &mut default_budget()?)?;
    let records = records(&bytes)?;
    let (_, certificate) = records
        .iter()
        .find(|(tag, _)| *tag == 40)
        .ok_or("certificate")?;
    assert_eq!(
        &bytes[certificate.start + 10..certificate.start + 15],
        &[3, 0, 0, 0, 0]
    );
    let root = index(&bytes, &mut (bytes.len() - 4))?;
    assert_eq!(records[usize::try_from(root)?].0, 41);
    let count_at = records.first().ok_or("first record")?.1.start - 9;
    assert_eq!(
        index(&bytes, &mut count_at.clone())?,
        u32::try_from(records.len())?
    );
    let mut added = bytes[certificate.clone()].to_vec();
    added[11..15].copy_from_slice(&64_u32.to_le_bytes());
    added.splice(15..15, (0..64).flat_map(|_| root.to_le_bytes()));
    let mut changed = bytes[..bytes.len() - 4].to_vec();
    changed[count_at..count_at + 4]
        .copy_from_slice(&(u32::try_from(records.len())? + 1).to_le_bytes());
    changed.push(40);
    changed.extend_from_slice(&u32::try_from(added.len())?.to_le_bytes());
    changed.extend_from_slice(&added);
    changed.extend_from_slice(&u32::try_from(records.len())?.to_le_bytes());
    for reordered in [false, true] {
        let mut receiver = fixture(reordered)?;
        let expected = receiver
            .base
            .cas
            .inspect(&bytes, &mut default_budget()?)?
            .judgment()
            .clone();
        let before = receiver
            .base
            .cas
            .canonical_snapshot(&mut default_budget()?)?;
        let allowance = payload * 8;
        assert!(512 * 64 < allowance && payload * 64 > allowance);
        let mut budget = Budget::new(Limits {
            staged_payload_bytes: allowance,
            ..default_budget()?.limits().clone()
        })?;
        assert_eq!(
            receiver
                .base
                .cas
                .replay(&changed, &expected, &mut budget)
                .expect_err("repeated references exceed typed arity")
                .kind,
            FaultKind::Invalid(InvalidCode::Rule)
        );
        assert_eq!(
            receiver
                .base
                .cas
                .canonical_snapshot(&mut default_budget()?)?,
            before
        );
        // Isolate the same record-40 boundary from the already checked prefix.
        // Only the reference, mode, count and fixed slot inventory may be charged.
        let positive = receiver.base.cas.inspect(&bytes, &mut default_budget()?)?;
        let mut isolated = portable_certificate(&mut receiver.base.cas, &positive)?;
        assert_eq!(&isolated.bytes[..9], &[0, 0, 0, 0, 3, 0, 0, 0, 0]);
        let reference = u32::try_from(isolated.records.len())?;
        isolated
            .records
            .push(crate::wire_decode::Decoded::Application(
                positive.proof().clone(),
            ));
        isolated.bytes[5..9].copy_from_slice(&64_u32.to_le_bytes());
        isolated
            .bytes
            .splice(9..9, (0..64).flat_map(|_| reference.to_le_bytes()));
        let mut budget = Budget::new(Limits {
            work: 32_814,
            staged_payload_bytes: allowance,
            ..default_budget()?.limits().clone()
        })?;
        let at = crate::error::Location::new("portable arity control");
        budget.work(37, at)?;
        let fault = replay_certificate(&mut receiver.base.cas, &isolated, &mut budget)
            .expect_err("arity precedes every dynamic premise inspection");
        assert_eq!(fault.kind, FaultKind::Invalid(InvalidCode::Rule));
        // Measured 37 prior Work + 9 header bytes + 32,768 fixed slot bytes.
        // No Work remains even to read the first premise reference, let alone size it.
        assert_eq!(budget.usage().work, 32_814);
        assert_eq!(budget.limits().staged_payload_bytes, allowance);
        assert_eq!(
            receiver
                .base
                .cas
                .canonical_snapshot(&mut default_budget()?)?,
            before
        );

        let mut budget = Budget::new(Limits {
            staged_payload_bytes: allowance,
            ..default_budget()?.limits().clone()
        })?;
        let positive = receiver.base.cas.replay(&bytes, &expected, &mut budget)?;
        assert_eq!(
            receiver
                .base
                .cas
                .export_witness(&positive, &mut default_budget()?)?,
            bytes
        );
    }
    Ok(())
}

/// Correct typed arity still requires the simultaneous payloads of both restriction proofs.
/// ´test:crate:series-portable-aggregate-premise-payload´
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Both allocation orders keep individual-copy controls beside aggregate exhaustion and unchanged full replay"
)]
fn series_portable_aggregate_premise_payload() -> TestResult {
    let mut source = fixture(false)?;
    let text = "123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
    let input = object(&mut source.base, &source.field, &[text; 9])?;
    let restriction = source.base.cas.compute_series(
        &O::Truncate {
            input: input.clone(),
            order: 8,
        },
        &[],
        &mut default_budget()?,
    )?;
    let witness = source.base.cas.compute_series(
        &O::Add {
            left: input.clone(),
            right: input,
            order: Some(8),
        },
        &[restriction.proof().clone(), restriction.proof().clone()],
        &mut default_budget()?,
    )?;
    let bytes = source
        .base
        .cas
        .export_witness(&witness, &mut default_budget()?)?;
    for reordered in [false, true] {
        let mut receiver = fixture(reordered)?;
        let positive = receiver.base.cas.inspect(&bytes, &mut default_budget()?)?;
        let certificate = portable_certificate(&mut receiver.base.cas, &positive)?;
        assert_eq!(&certificate.bytes[..9], &[0, 0, 0, 0, 1, 2, 0, 0, 0]);
        let application = crate::proof_view::application(
            &receiver.base.cas,
            positive.proof().logical_id(),
            &mut default_budget()?,
        )?;
        assert_eq!(application.premises.len(), 2);
        assert_eq!(application.premises[0], application.premises[1]);
        let mut aggregate = 0;
        for proof in &application.premises {
            aggregate += crate::proof_view::application_bytes(
                &receiver.base.cas,
                *proof,
                &mut default_budget()?,
            )?;
        }
        assert_eq!(aggregate, 8_504);
        let allowance = aggregate - 1;
        // Each copy actually succeeds in isolation under the same reduced allowance.
        for proof in &application.premises {
            let mut budget = Budget::new(Limits {
                staged_payload_bytes: allowance,
                ..default_budget()?.limits().clone()
            })?;
            crate::proof_view::application(&receiver.base.cas, *proof, &mut budget)?;
        }
        let before = receiver
            .base
            .cas
            .canonical_snapshot(&mut default_budget()?)?;
        let mut budget = Budget::new(Limits {
            staged_payload_bytes: allowance + 113,
            ..default_budget()?.limits().clone()
        })?;
        let at = crate::error::Location::new("portable premise control");
        budget.work(37, at)?;
        let previous = budget.reserve_temporary(113, at)?;
        let fault = replay_certificate(&mut receiver.base.cas, &certificate, &mut budget)
            .expect_err("both live restriction payloads exceed the allowance");
        assert_eq!(
            fault,
            crate::Fault::exhausted(
                BudgetKind::StagedPayloadBytes,
                allowance,
                aggregate,
                crate::error::Location::new("formal series arithmetic"),
            )
        );
        assert_eq!(budget.usage().work, 197_319);
        assert_eq!(budget.limits().staged_payload_bytes, allowance);
        budget.restore_limits(previous);
        assert_eq!(
            receiver
                .base
                .cas
                .canonical_snapshot(&mut default_budget()?)?,
            before
        );
        replay_certificate(&mut receiver.base.cas, &certificate, &mut default_budget()?)?;
        let unchanged =
            receiver
                .base
                .cas
                .replay(&bytes, positive.judgment(), &mut default_budget()?)?;
        assert_eq!(
            receiver
                .base
                .cas
                .export_witness(&unchanged, &mut default_budget()?)?,
            bytes
        );
        assert_eq!(
            receiver
                .base
                .cas
                .canonical_snapshot(&mut default_budget()?)?,
            before
        );
    }
    Ok(())
}

/// A checked finite congruence cannot replace an analytic side-jet premise.
/// ´test:crate:series-as-germ-premise-refused´
#[test]
fn series_as_germ_premise_refused() -> TestResult {
    let mut f = fixture(false)?;
    let input = object(&mut f.base, &f.field, &["1", "2"])?;
    let series = f.base.cas.compute_series(
        &O::Truncate { input, order: 1 },
        &[],
        &mut default_budget()?,
    )?;
    let point = crate::boundary_regions::boundary(
        &mut f.base.cas,
        &f.base.domain,
        &crate::Rational::zero(),
        &mut default_budget()?,
    )?;
    let request = crate::request_model::Request {
        task: crate::task::Task::Series(Box::new(crate::germ_model::SeriesAt {
            at: crate::germ_model::GermAt {
                coordinate: "x".into(),
                point,
                side: None,
            },
            order: 1,
            analytic: true,
        })),
        context: f.base.context.clone(),
        inputs: Vec::new(),
        premises: Vec::new(),
        limits: crate::budget::LimitsOverride::default(),
    };
    let result = crate::germ_producer::execute(
        &mut f.base.cas,
        &request,
        &[f.base.x.logical_id()],
        &crate::search::SearchPolicy::default(),
        &mut default_budget()?,
        &mut default_budget()?,
    )?;
    let application = crate::proof_view::application(
        &f.base.cas,
        result.witness.proof().logical_id(),
        &mut default_budget()?,
    )?;
    let mut premises = application
        .premises
        .iter()
        .map(|id| {
            Ok(crate::proof_view::application(
                &f.base.cas,
                *id,
                &mut default_budget()?,
            )?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let draft = ApplicationDraft {
        context: application.judgment.context,
        rule: application.rule,
        conclusion: application.judgment.statement,
        premises: (0..premises.len())
            .map(u32::try_from)
            .collect::<Result<_, _>>()?,
        substitutions: application.substitutions,
        certificate: application.certificate,
    };
    let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
    crate::scoped_check::check_application(
        &mut f.base.cas,
        &draft,
        &premises,
        &mut default_budget()?,
    )?;
    let original = premises.first().ok_or("analytic jet")?.clone();
    premises[0] = crate::proof_view::application(
        &f.base.cas,
        series.proof().logical_id(),
        &mut default_budget()?,
    )?;
    assert_eq!(
        f.base.cas.checked(&premises[0].judgment.context)?,
        f.base.cas.checked(&draft.context)?
    );
    let fault = crate::scoped_check::check_application(
        &mut f.base.cas,
        &draft,
        &premises,
        &mut default_budget()?,
    )
    .expect_err("finite congruence is not an analytic jet");
    assert_eq!(fault.kind, crate::case_syntax::invalid().kind);
    assert_eq!(
        f.base.cas.canonical_snapshot(&mut default_budget()?)?,
        before
    );
    premises[0] = original;
    crate::scoped_check::check_application(
        &mut f.base.cas,
        &draft,
        &premises,
        &mut default_budget()?,
    )?;
    Ok(())
}
