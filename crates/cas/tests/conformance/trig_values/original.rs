// SPDX-License-Identifier: AGPL-3.0-only
//! Recognition does not erase original syntax or acquire authority from approximate constants.

use crate::testing::common::{TestResult, default_budget, real};
use crate::testing::half_pi::{original, prepared, term};
use nrz_cas::domain::{AmbientDomain, HeaderDraft, RegionDraft, RegionExpr};
use nrz_cas::proof::{ApplicationDraft, Certificate, DomainContract, Primitive, Statement};
use nrz_cas::syntax::{
    AtomicSort, Constant, Declaration, Draft, Environment, Field, Operator, Sort,
};
use nrz_cas::{Rational, Reference};

/// Both arctangent endpoints preserve a nonliteral subject and its original divisor requirements.
/// ´test:integration:trig-values-atan-original-guards´
#[test]
fn trig_values_atan_original_guards() -> TestResult {
    for is_one in [false, true] {
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
        let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
        let u = if is_one {
            term(&mut f.cas, &f.header, Operator::Div, &[x.clone(), x])?
        } else {
            let reciprocal = term(&mut f.cas, &f.header, Operator::Div, &[one.clone(), x])?;
            term(
                &mut f.cas,
                &f.header,
                Operator::Mul,
                &[reciprocal, zero.clone()],
            )?
        };
        original(&mut f, u.clone())?;
        let guard = f.cas.check_proof(
            &f.context,
            f.steps.get(..1).ok_or("guard")?,
            &mut default_budget()?,
        )?;
        let equality = f.cas.check_proof(
            &f.context,
            &[ApplicationDraft {
                rule: if is_one {
                    Primitive::Cancel
                } else {
                    Primitive::MulZero
                },
                context: f.context.clone(),
                conclusion: Statement::Equality {
                    left: u.clone(),
                    right: if is_one { one } else { zero.clone() },
                    domain: f.domain.clone(),
                    domain_contract: DomainContract::DefinedAndEqual,
                },
                premises: Vec::new(),
                substitutions: Vec::new(),
                certificate: Certificate::None,
            }],
            &mut default_budget()?,
        )?;
        assert_ne!(
            equality.judgment().residuals,
            [] as [nrz_cas::proof::GuardedObligation; 0]
        );
        let function = term(&mut f.cas, &f.header, Operator::Arctangent, &[u])?;
        let right = if is_one {
            let pi = term(&mut f.cas, &f.header, Operator::Constant(Constant::Pi), &[])?;
            let quarter = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::parse("1/4", &mut default_budget()?)?),
                &[],
            )?;
            term(&mut f.cas, &f.header, Operator::Mul, &[pi, quarter])?
        } else {
            zero
        };
        let value = f
            .cas
            .trig_special_value(
                &function,
                &[guard.proof().clone(), equality.proof().clone()],
                &mut default_budget()?,
            )?
            .ok_or("arctangent endpoint")?;
        assert_eq!(
            value.judgment().statement,
            Statement::Equality {
                left: function,
                right,
                domain: f.domain.clone(),
                domain_contract: DomainContract::DefinedAndEqual,
            }
        );
        assert_eq!(value.judgment().residuals, equality.judgment().residuals);
        assert_eq!(value.hypotheses(), guard.hypotheses());
    }
    Ok(())
}

/// All original divisor requirements survive an exact zero value, including hidden factors.
/// ´test:integration:trig-values-original-guards´
#[test]
fn trig_values_original_guards() -> TestResult {
    for cosine in [false, true] {
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
        let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
        let reciprocal = term(&mut f.cas, &f.header, Operator::Div, &[one.clone(), x])?;
        let u = term(
            &mut f.cas,
            &f.header,
            Operator::Mul,
            &[reciprocal, zero.clone()],
        )?;
        original(&mut f, u.clone())?;
        let recognition = f
            .cas
            .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
        assert_ne!(
            recognition.judgment().residuals,
            [] as [nrz_cas::proof::GuardedObligation; 0]
        );
        let function = term(
            &mut f.cas,
            &f.header,
            if cosine {
                Operator::Cosine
            } else {
                Operator::Sine
            },
            &[u],
        )?;
        let expected = Statement::Equality {
            left: function.clone(),
            right: if cosine { one } else { zero },
            domain: f.domain.clone(),
            domain_contract: DomainContract::DefinedAndEqual,
        };
        let value = f
            .cas
            .trig_special_value(
                &function,
                &[recognition.proof().clone()],
                &mut default_budget()?,
            )?
            .ok_or("recognized zero")?;
        assert_eq!(value.judgment().statement, expected);
        assert_eq!(value.judgment().residuals, recognition.judgment().residuals);
        assert_eq!(value.hypotheses(), recognition.hypotheses());
    }
    Ok(())
}

/// Independently proved transformed operands remain the actual arguments of both functions.
/// ´test:integration:trig-values-original-transformation´
#[test]
fn trig_values_original_transformation() -> TestResult {
    let mut f = prepared("3", "3/2", false, false)?;
    let pi = term(&mut f.cas, &f.header, Operator::Constant(Constant::Pi), &[])?;
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
    original(&mut f, u.clone())?;
    f.steps.get_mut(1).ok_or("equality")?.rule = Primitive::Jets;
    let recognition = f
        .cas
        .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
    for (operator, expected) in [(Operator::Sine, -1), (Operator::Cosine, 0)] {
        let function = term(&mut f.cas, &f.header, operator, std::slice::from_ref(&u))?;
        let right = term(
            &mut f.cas,
            &f.header,
            Operator::Rational(Rational::from_ratio(expected, 1, &mut default_budget()?)?),
            &[],
        )?;
        let value = f
            .cas
            .trig_special_value(
                &function,
                &[recognition.proof().clone()],
                &mut default_budget()?,
            )?
            .ok_or("recognized operand")?;
        assert_eq!(
            value.judgment().statement,
            Statement::Equality {
                left: function,
                right,
                domain: f.domain.clone(),
                domain_contract: DomainContract::DefinedAndEqual
            }
        );
    }
    Ok(())
}

/// Two ordinary proofs grant no consumer authority, even for a recognized half-Pi input.
/// Prerequisite recognition controls establish impostor rejection independently of this arity.
/// ´test:integration:trig-values-no-authority-inputs´
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "parallel no-authority fixtures retain their full source construction"
)]
fn trig_values_no_authority_inputs() -> TestResult {
    for impostor in 0..4 {
        let mut f = prepared("1", if impostor == 3 { "1/2" } else { "1/3" }, false, false)?;
        if impostor == 3 {
            f.cas
                .check_proof(&f.context, &f.steps, &mut default_budget()?)?;
        }
        if impostor == 1 || impostor == 2 {
            let u = if impostor == 1 {
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
                f.header = f
                    .cas
                    .admit(
                        &[Draft::Header(HeaderDraft::Core(Environment {
                            declarations: vec![
                                Declaration {
                                    key: "x".into(),
                                    display_name: "x".into(),
                                    sort: real(),
                                },
                                Declaration {
                                    key: "Pi".into(),
                                    display_name: "Pi".into(),
                                    sort: Sort::Function {
                                        inputs: Vec::new(),
                                        output: AtomicSort::Scalar(Field::Real),
                                    },
                                },
                            ],
                            ..Environment::default()
                        }))],
                        &mut default_budget()?,
                    )?
                    .pop()
                    .ok_or("header")?;
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
                    .ok_or("domain")?;
                let opaque = term(&mut f.cas, &f.header, Operator::Call("Pi".into()), &[])?;
                let half = term(
                    &mut f.cas,
                    &f.header,
                    Operator::Rational(Rational::parse("1/2", &mut default_budget()?)?),
                    &[],
                )?;
                term(&mut f.cas, &f.header, Operator::Mul, &[opaque, half])?
            };
            original(&mut f, u.clone())?;
            let equality = f.steps.get_mut(1).ok_or("equality")?;
            equality.rule = Primitive::Reflexivity;
            let Statement::Equality { right, .. } = &mut equality.conclusion else {
                return Err("equality".into());
            };
            *right = u;
        }
        let Statement::HalfPiWitness { u, .. } = &f.expected.statement else {
            return Err("witness".into());
        };
        let u = u.clone();
        let mut proofs = Vec::new();
        for step in f.steps.get(..2).ok_or("premises")? {
            proofs.push(
                f.cas
                    .check_proof(
                        &f.context,
                        std::slice::from_ref(step),
                        &mut default_budget()?,
                    )?
                    .proof()
                    .clone(),
            );
        }
        for operator in [Operator::Sine, Operator::Cosine] {
            let function = term(&mut f.cas, &f.header, operator, std::slice::from_ref(&u))?;
            let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
            assert!(
                f.cas
                    .trig_special_value(&function, &proofs, &mut default_budget()?)?
                    .is_none()
            );
            assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
        }
    }
    Ok(())
}
