// SPDX-License-Identifier: AGPL-3.0-only
//! Exact integer recognition preserves original domains and refuses forged payloads atomically.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::half_pi::{Fixture, prepared, term};
use nrz_cas::proof::{Certificate, Statement};
use nrz_cas::syntax::Operator;
use nrz_cas::{FaultKind, InvalidCode, Rational};

/// A checked polynomial transformation supplies the canonical coefficient operand.
/// ´test:integration:half-pi-transformed-equality´
#[test]
fn half_pi_transformed_equality() -> TestResult {
    let mut f = prepared("3", "3/2", false, false)?;
    let pi = term(
        &mut f.cas,
        &f.header,
        Operator::Constant(nrz_cas::syntax::Constant::Pi),
        &[],
    )?;
    let three = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::from_ratio(3, 1, &mut default_budget()?)?),
        &[],
    )?;
    let two = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::from_ratio(2, 1, &mut default_budget()?)?),
        &[],
    )?;
    let product = term(&mut f.cas, &f.header, Operator::Mul, &[three, pi])?;
    let u = term(&mut f.cas, &f.header, Operator::Div, &[product, two])?;
    crate::testing::half_pi::original(&mut f, u.clone())?;
    let equality = f.steps.get_mut(1).ok_or("equality")?;
    equality.rule = nrz_cas::proof::Primitive::Jets;
    let Statement::Equality { left, right, .. } = &equality.conclusion else {
        return Err("equality conclusion".into());
    };
    assert_eq!(left, &u);
    assert_ne!(left, right);
    let witness = f
        .cas
        .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
    assert_eq!(witness.judgment(), &f.expected);
    let Statement::HalfPiWitness { u: retained, .. } = &witness.judgment().statement else {
        return Err("recognition conclusion".into());
    };
    assert_eq!(retained, &u);
    Ok(())
}

/// Independently transcribed coefficients span signed classes without a residue implementation.
/// ´test:integration:half-pi-integers´
#[test]
fn half_pi_integers() -> TestResult {
    for (k, q) in [
        ("-3", "-3/2"),
        ("-2", "-1"),
        ("-1", "-1/2"),
        ("0", "0"),
        ("1", "1/2"),
        ("2", "1"),
        ("3", "3/2"),
        ("4", "2"),
        ("18446744073709551617", "18446744073709551617/2"),
        ("-18446744073709551617", "-18446744073709551617/2"),
    ] {
        let mut f = prepared(k, q, false, false)?;
        let witness = f
            .cas
            .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
        assert_eq!(witness.judgment(), &f.expected);
        assert_eq!(witness.hypotheses().len(), 1);
        assert!(!witness.is_unconditional());
    }
    let mut f = prepared("0", "0", true, false)?;
    let witness = f
        .cas
        .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
    assert_eq!(witness.judgment(), &f.expected);
    assert_eq!(
        f.steps.get(1).ok_or("zero law")?.rule,
        nrz_cas::proof::Primitive::MulZero
    );
    assert_eq!(
        f.steps.get(2).ok_or("symmetry")?.rule,
        nrz_cas::proof::Primitive::Symmetry
    );
    let Statement::HalfPiWitness { u, .. } = &f.expected.statement else {
        return Err("recognition".into());
    };
    let zero = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::zero()),
        &[],
    )?;
    assert_eq!(u, &zero);
    Ok(())
}

fn refused(f: &mut Fixture, code: InvalidCode) -> TestResult {
    let counts = f.cas.counts();
    let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
    let fault = f
        .cas
        .check_proof(&f.context, &f.steps, &mut default_budget()?)
        .err()
        .ok_or("forged half-Pi witness accepted")?;
    assert_eq!(fault.kind, FaultKind::Invalid(code));
    assert_eq!(f.cas.counts(), counts);
    assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
    Ok(())
}

/// Changed integer, fractional integer, missing slots and mismatched repeated fields refuse.
/// ´test:integration:half-pi-payload-refusals´
#[test]
fn half_pi_payload_refusals() -> TestResult {
    for (k, q) in [("3", "1"), ("1/2", "1/4")] {
        refused(
            &mut prepared(k, q, false, false)?,
            InvalidCode::EndpointMismatch,
        )?;
    }
    let mut f = prepared("3", "3/2", false, false)?;
    let original = f.steps.clone();
    for changed in 0..4 {
        f.steps.clone_from(&original);
        let step = f.steps.last_mut().ok_or("recognition")?;
        let Certificate::HalfPiWitness {
            k,
            defined,
            equality,
            u,
            ..
        } = &mut step.certificate
        else {
            return Err("certificate".into());
        };
        match changed {
            0 => **k = Rational::one(),
            1 => *defined = 1,
            2 => *equality = 0,
            _ => {
                *u = term(
                    &mut f.cas,
                    &f.header,
                    Operator::Rational(Rational::zero()),
                    &[],
                )?;
            }
        }
        refused(&mut f, InvalidCode::EndpointMismatch)?;
    }
    Ok(())
}

/// A rational approximation or Pi/3 cannot become an integer half-Pi witness.
/// ´test:integration:half-pi-impostors´
#[test]
fn half_pi_impostors() -> TestResult {
    refused(
        &mut prepared("1", "1/3", false, false)?,
        InvalidCode::EndpointMismatch,
    )?;
    let mut f = prepared("1", "1/2", false, false)?;
    let number = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::parse(
            "15707963267948966/10000000000000000",
            &mut default_budget()?,
        )?),
        &[],
    )?;
    if let Statement::Equality { right, .. } = &mut f.steps.get_mut(1).ok_or("equality")?.conclusion
    {
        *right = number;
    }
    refused(&mut f, InvalidCode::Rule)?;
    Ok(())
}

/// Original division remains checked even when multiplying by zero erases the entire expression.
/// ´test:integration:half-pi-original-divisor´
#[test]
fn half_pi_original_divisor() -> TestResult {
    for undefined in [false, true] {
        let mut f = prepared("0", "0", true, false)?;
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
        let divisor = if undefined {
            zero.clone()
        } else {
            term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?
        };
        let quotient = term(&mut f.cas, &f.header, Operator::Div, &[one, divisor])?;
        let u = term(&mut f.cas, &f.header, Operator::Mul, &[quotient, zero])?;
        crate::testing::half_pi::original(&mut f, u)?;
        if undefined {
            refused(&mut f, InvalidCode::RefutedDomain)?;
        } else {
            let witness = f
                .cas
                .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
            assert_ne!(witness.judgment().residuals.len(), 0);
            let bytes = f.cas.export_witness(&witness, &mut default_budget()?)?;
            let counts = f.cas.counts();
            let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
            let fault = f
                .cas
                .replay(&bytes, &f.expected, &mut default_budget()?)
                .err()
                .ok_or("dropped divisor guard")?;
            assert_eq!(
                fault.kind,
                FaultKind::Invalid(InvalidCode::EndpointMismatch)
            );
            assert_eq!(f.cas.counts(), counts);
            assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
            f.cas
                .replay(&bytes, witness.judgment(), &mut default_budget()?)?;
        }
    }
    Ok(())
}

/// The payload's context and domain cannot replace the frame of either premise.
/// ´test:integration:half-pi-frame-refusals´
#[test]
fn half_pi_frame_refusals() -> TestResult {
    use nrz_cas::Reference;
    use nrz_cas::domain::{AmbientDomain, ContextDraft, RegionDraft, RegionExpr};
    use nrz_cas::syntax::Draft;
    let mut f = prepared("3", "3/2", false, false)?;
    let other_domain = f
        .cas
        .admit(
            &[Draft::Region(RegionDraft {
                header: Reference::Existing(f.header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::Empty,
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("region")?;
    let other_context = f
        .cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(f.header.clone()),
                admissible: Reference::Existing(f.domain.clone()),
                assumptions: Vec::new(),
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("context")?;
    let original = f.steps.clone();
    for context_change in [false, true] {
        f.steps.clone_from(&original);
        let step = f.steps.last_mut().ok_or("recognition")?;
        let Statement::HalfPiWitness {
            context, domain, ..
        } = &mut step.conclusion
        else {
            return Err("statement".into());
        };
        let Certificate::HalfPiWitness {
            context: home,
            domain: region,
            ..
        } = &mut step.certificate
        else {
            return Err("certificate".into());
        };
        if context_change {
            *context = other_context.clone();
            *home = other_context.clone();
        } else {
            *domain = other_domain.clone();
            *region = other_domain.clone();
        }
        refused(&mut f, InvalidCode::EndpointMismatch)?;
    }
    Ok(())
}

/// Valid equality premises cannot turn rational or opaque names into the exact named Pi.
/// ´test:integration:half-pi-recognizer-impostors´
#[test]
fn half_pi_recognizer_impostors() -> TestResult {
    use nrz_cas::Reference;
    use nrz_cas::domain::{AmbientDomain, HeaderDraft, RegionDraft, RegionExpr};
    use nrz_cas::syntax::{AtomicSort, Constant, Declaration, Draft, Environment, Field, Sort};
    for impostor in 0..3 {
        let mut f = prepared("1", "1/2", false, false)?;
        let u = if impostor == 0 {
            term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::parse(
                    "15707963267948966/10000000000000000",
                    &mut default_budget()?,
                )?),
                &[],
            )?
        } else {
            let operator = if impostor == 1 {
                let real = AtomicSort::Scalar(Field::Real);
                f.header = f
                    .cas
                    .admit(
                        &[Draft::Header(HeaderDraft::Core(Environment {
                            declarations: vec![
                                Declaration {
                                    key: "x".into(),
                                    display_name: "x".into(),
                                    sort: Sort::Value(real.clone()),
                                },
                                Declaration {
                                    key: "Pi".into(),
                                    display_name: "Pi".into(),
                                    sort: Sort::Function {
                                        inputs: Vec::new(),
                                        output: real,
                                    },
                                },
                            ],
                            ..Environment::default()
                        }))],
                        &mut default_budget()?,
                    )?
                    .pop()
                    .ok_or("opaque Pi declaration")?;
                f.domain = f
                    .cas
                    .admit(
                        &[Draft::Region(RegionDraft {
                            header: Reference::Existing(f.header.clone()),
                            scope: Vec::new(),
                            ambient: AmbientDomain::Real("x".into()),
                            expression: RegionExpr::Ambient,
                        })],
                        &mut default_budget()?,
                    )?
                    .pop()
                    .ok_or("opaque Pi domain")?;
                Operator::Call("Pi".into())
            } else {
                Operator::Constant(Constant::E)
            };
            let pi = term(&mut f.cas, &f.header, operator, &[])?;
            let half = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::parse("1/2", &mut default_budget()?)?),
                &[],
            )?;
            term(&mut f.cas, &f.header, Operator::Mul, &[pi, half])?
        };
        crate::testing::half_pi::original(&mut f, u.clone())?;
        let equality = f.steps.get_mut(1).ok_or("equality")?;
        equality.rule = nrz_cas::proof::Primitive::Reflexivity;
        let Statement::Equality { right, .. } = &mut equality.conclusion else {
            return Err("equality conclusion".into());
        };
        *right = u;
        recognizer_refused(
            &mut f,
            if impostor == 0 {
                InvalidCode::Rule
            } else {
                InvalidCode::EndpointMismatch
            },
        )?;
    }
    Ok(())
}

/// A normalized zero still needs checked equality to the literal Pi product.
/// ´test:integration:half-pi-zero-requires-product´
#[test]
fn half_pi_zero_requires_product() -> TestResult {
    let mut f = prepared("0", "0", true, false)?;
    let equality = f.steps.get_mut(2).ok_or("symmetry")?;
    equality.rule = nrz_cas::proof::Primitive::Reflexivity;
    equality.premises.clear();
    let Statement::Equality { left, right, .. } = &mut equality.conclusion else {
        return Err("equality conclusion".into());
    };
    *right = left.clone();
    let equality = equality.clone();
    let guard = f.steps.first().ok_or("guard")?.clone();
    let mut recognition = f.steps.last().ok_or("recognition")?.clone();
    recognition.premises = vec![0, 1];
    f.steps = vec![guard, equality, recognition];
    recognizer_refused(&mut f, InvalidCode::Rule)
}

/// Authenticate both premise roots independently before testing the complete recognition DAG.
fn recognizer_refused(f: &mut Fixture, code: InvalidCode) -> TestResult {
    for index in [0, 1] {
        let premise = f.steps.get(index).ok_or("premise")?;
        let witness = f.cas.check_proof(
            &f.context,
            std::slice::from_ref(premise),
            &mut default_budget()?,
        )?;
        assert_eq!(witness.judgment().statement, premise.conclusion);
    }
    refused(f, code)
}
