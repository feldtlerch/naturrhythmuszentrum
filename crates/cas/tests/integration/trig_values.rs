// SPDX-License-Identifier: AGPL-3.0-only
//! Complete special-value witnesses meet independently built receiving-store claims.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::half_pi::{Fixture, original, prepared, term};
use crate::testing::trig_values::{atan, circular};
use nrz_cas::{FaultKind, InvalidCode};

mod mutations;

#[expect(
    clippy::too_many_lines,
    reason = "four guarded fixtures spell independently constructed original claims"
)]
fn guarded(kind: usize, reordered: bool) -> TestResult<Fixture> {
    use nrz_cas::domain::Layer;
    use nrz_cas::proof::{
        ApplicationDraft, Certificate, DomainContract, GuardedObligation, Primitive, Statement,
    };
    use nrz_cas::syntax::{Draft, Operator, PredicateKind};
    use nrz_cas::{Rational, Reference};
    let mut f = prepared("0", "0", true, reordered)?;
    let one = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::one()),
        &[],
    )?;
    let zero = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::zero()),
        &[],
    )?;
    let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
    let u = if kind == 3 {
        term(
            &mut f.cas,
            &f.header,
            Operator::Div,
            &[x.clone(), x.clone()],
        )?
    } else {
        let reciprocal = term(
            &mut f.cas,
            &f.header,
            Operator::Div,
            &[one.clone(), x.clone()],
        )?;
        term(
            &mut f.cas,
            &f.header,
            Operator::Mul,
            &[reciprocal, zero.clone()],
        )?
    };
    original(&mut f, u.clone())?;
    if kind >= 2 {
        f.steps.truncate(1);
        f.steps.push(ApplicationDraft {
            rule: if kind == 3 {
                Primitive::Cancel
            } else {
                Primitive::MulZero
            },
            context: f.context.clone(),
            conclusion: Statement::Equality {
                left: u.clone(),
                right: if kind == 3 { one.clone() } else { zero.clone() },
                domain: f.domain.clone(),
                domain_contract: DomainContract::DefinedAndEqual,
            },
            premises: Vec::new(),
            substitutions: Vec::new(),
            certificate: Certificate::None,
        });
    }
    let operator = match kind {
        0 => Operator::Sine,
        1 => Operator::Cosine,
        _ => Operator::Arctangent,
    };
    let left = term(&mut f.cas, &f.header, operator, &[u])?;
    let right = match kind {
        1 => one,
        3 => quarter_pi(&mut f)?,
        _ => zero,
    };
    f.expected.statement = Statement::Equality {
        left,
        right,
        domain: f.domain.clone(),
        domain_contract: DomainContract::DefinedAndEqual,
    };
    f.steps.push(ApplicationDraft {
        rule: match kind {
            0 => Primitive::SinHalfPi,
            1 => Primitive::CosHalfPi,
            2 => Primitive::AtanZero,
            _ => Primitive::AtanOne,
        },
        context: f.context.clone(),
        conclusion: f.expected.statement.clone(),
        premises: if kind < 2 {
            vec![u32::try_from(f.steps.len() - 1)?]
        } else {
            vec![0, 1]
        },
        substitutions: Vec::new(),
        certificate: Certificate::None,
    });
    let predicate = f
        .cas
        .admit(
            &[Draft::Condition {
                header: Reference::Existing(f.header.clone()),
                scope: Vec::new(),
                kind: PredicateKind::NonZero,
                arguments: vec![Reference::Existing(x)],
            }],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("nonzero predicate")?;
    f.expected.residuals = vec![GuardedObligation {
        home: f.context.clone(),
        scope: Vec::new(),
        layer: Layer::Pointwise,
        space: None,
        region: f.domain.clone(),
        predicate,
    }];
    Ok(f)
}

fn quarter_pi(f: &mut Fixture) -> TestResult<nrz_cas::Handle> {
    use nrz_cas::Rational;
    use nrz_cas::syntax::{Constant, Operator};
    let pi = term(&mut f.cas, &f.header, Operator::Constant(Constant::Pi), &[])?;
    let quarter = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::parse("1/4", &mut default_budget()?)?),
        &[],
    )?;
    term(&mut f.cas, &f.header, Operator::Mul, &[pi, quarter])
}

/// Independently built guarded claims retain their residual and used hypothesis on fresh replay.
/// ´test:integration:trig-values-guarded-replay´
#[test]
fn trig_values_guarded_replay() -> TestResult {
    for kind in 0..4 {
        let bytes = encoded(guarded(kind, false)?)?;
        for reordered in [false, true] {
            for (name, changed) in mutations::dropped_conditions(&bytes)? {
                let mut receiver = guarded(kind, reordered)?;
                refused(
                    &mut receiver,
                    &changed,
                    Some(InvalidCode::ConditionMismatch),
                    name,
                )?;
                replay(receiver, &bytes)?;
            }
        }
    }
    Ok(())
}

fn fixture(kind: usize, reordered: bool) -> TestResult<Fixture> {
    match kind {
        0 => circular("3", "3/2", false, -1, false, reordered),
        1 => circular("3", "3/2", true, 0, false, reordered),
        2 => atan("0", false, reordered),
        _ => atan("1", true, reordered),
    }
}

fn encoded(mut f: Fixture) -> TestResult<Vec<u8>> {
    let proof = f
        .cas
        .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
    assert_eq!(proof.judgment(), &f.expected);
    Ok(f.cas.export_witness(&proof, &mut default_budget()?)?)
}

fn replay(mut receiver: Fixture, bytes: &[u8]) -> TestResult {
    let witness = receiver
        .cas
        .replay(bytes, &receiver.expected, &mut default_budget()?)?;
    assert_eq!(witness.judgment(), &receiver.expected);
    assert_eq!(witness.hypotheses().len(), 1);
    assert_eq!(
        receiver
            .cas
            .export_witness(&witness, &mut default_budget()?)?,
        bytes
    );
    Ok(())
}

/// Each destination builds its own syntax and expected judgment without invoking a checker.
/// ´test:integration:trig-values-fresh-replay´
#[test]
fn trig_values_fresh_replay() -> TestResult {
    for kind in 0..4 {
        let bytes = encoded(fixture(kind, false)?)?;
        for reordered in [false, true] {
            replay(fixture(kind, reordered)?, &bytes)?;
        }
    }
    for (k, q, sine, cosine, zero) in [
        ("0", "0", 0, 1, true),
        ("-4", "-2", 0, 1, false),
        ("-3", "-3/2", 1, 0, false),
        ("-2", "-1", 0, -1, false),
        ("-1", "-1/2", -1, 0, false),
        (
            "18446744073709551617",
            "18446744073709551617/2",
            1,
            0,
            false,
        ),
        (
            "-18446744073709551617",
            "-18446744073709551617/2",
            -1,
            0,
            false,
        ),
    ] {
        for (cos, value) in [(false, sine), (true, cosine)] {
            let bytes = encoded(circular(k, q, cos, value, zero, false)?)?;
            for reordered in [false, true] {
                replay(circular(k, q, cos, value, zero, reordered)?, &bytes)?;
            }
        }
    }
    Ok(())
}

/// Every changed complete witness is paired with the unchanged accepted control in both stores.
/// ´test:integration:trig-values-replay-mutations´
#[test]
fn trig_values_replay_mutations() -> TestResult {
    for kind in 0..4 {
        let bytes = encoded(fixture(kind, false)?)?;
        let changes = mutations::changed(&bytes, kind)?;
        for reordered in [false, true] {
            let mut destination = fixture(kind, reordered)?;
            for (name, changed, expected) in &changes {
                refused(&mut destination, changed, *expected, name)?;
            }
            replay(destination, &bytes)?;
        }
    }
    Ok(())
}

/// A different valid theorem remains valid for itself but cannot satisfy the original claim.
/// ´test:integration:trig-values-replay-claim-boundary´
#[test]
fn trig_values_replay_claim_boundary() -> TestResult {
    for kind in 0..4 {
        let alternative = encoded(fixture(kind ^ 1, false)?)?;
        replay(fixture(kind ^ 1, true)?, &alternative)?;
        let original = encoded(fixture(kind, false)?)?;
        for reordered in [false, true] {
            let mut receiver = fixture(kind, reordered)?;
            refused(
                &mut receiver,
                &alternative,
                Some(InvalidCode::EndpointMismatch),
                "different valid claim",
            )?;
            replay(receiver, &original)?;
        }
    }
    Ok(())
}

fn refused(f: &mut Fixture, bytes: &[u8], code: Option<InvalidCode>, name: &str) -> TestResult {
    let counts = f.cas.counts();
    let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
    let fault = f
        .cas
        .replay(bytes, &f.expected, &mut default_budget()?)
        .err()
        .ok_or("changed witness accepted")?;
    assert!(
        matches!(fault.kind, FaultKind::Invalid(_)),
        "{name}: {fault:?}"
    );
    if let Some(code) = code {
        assert_eq!(fault.kind, FaultKind::Invalid(code), "{name}");
    }
    assert_eq!(f.cas.counts(), counts);
    assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
    Ok(())
}
