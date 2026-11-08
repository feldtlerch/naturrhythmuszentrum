// SPDX-License-Identifier: AGPL-3.0-only
//! Independent finite source vectors and ordinary original-domain dependencies.

use super::common::{TestResult, default_budget};
use super::series_proof::{defined, fixture, rational};
use crate::test_api::proof::{Judgment, Statement};
use crate::test_api::series::{
    SeriesObject, SeriesStatement, SourceCertificate, SourceClause, SourceRequest, SourceStatement,
    SourceUnitRow,
};
use crate::test_api::syntax::{Draft, Operator};
use crate::test_api::{Handle, Reference};

/// Complete source setup; expected output syntax may deliberately be absent.
pub(crate) struct Fixture {
    /// Original semantic frame.
    pub(crate) base: super::cases::Fixture,
    /// Complete source request independent of output admission.
    pub(crate) request: SourceRequest,
    /// Independent hand arithmetic certificate.
    pub(crate) certificate: SourceCertificate,
    /// Exact ordered live proof slots.
    pub(crate) proofs: Vec<Handle>,
    /// Optional independently admitted expected judgment.
    pub(crate) expected: Option<Judgment>,
}

/// Supply a visible original-source hypothesis to isolate capability from missing definedness.
pub(crate) fn assume_source(f: &mut Fixture) -> TestResult {
    use crate::test_api::domain::{AssumptionDraft, ContextDraft, HypothesisRef};
    use crate::test_api::proof::{ApplicationDraft, Certificate, Primitive};
    let predicate = f.base.predicate(
        crate::test_api::syntax::PredicateKind::Defined,
        std::slice::from_ref(&f.request.source),
    )?;
    let context = f
        .base
        .cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(f.base.header.clone()),
                admissible: Reference::Existing(f.base.domain.clone()),
                assumptions: vec![AssumptionDraft {
                    key: "original-source-defined".into(),
                    condition: Reference::Existing(predicate.clone()),
                }],
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("context")?;
    f.base.context = context.clone();
    f.request.context = context.clone();
    let draft = ApplicationDraft {
        rule: Primitive::ContextHypothesis,
        context: context.clone(),
        conclusion: Statement::PredicateFact {
            region: f.base.domain.clone(),
            predicate,
        },
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: Certificate::Hypothesis(HypothesisRef { context, slot: 0 }),
    };
    f.proofs = vec![
        f.base
            .cas
            .check_proof(&f.base.context, &[draft], &mut default_budget()?)?
            .proof()
            .clone(),
    ];
    f.expected = None;
    Ok(())
}

/// A constant seed consumes an independently closed or qualified ordinary source fact.
pub(crate) fn conditioned(residual: bool, reordered: bool) -> TestResult<Fixture> {
    let mut old = super::series_proof::conditioned(residual, reordered)?;
    let root = old.steps.pop().ok_or("scale")?;
    let Statement::Series(value) = root.conclusion else {
        return Err("series".into());
    };
    let SeriesStatement::Scale { scalar, .. } = *value else {
        return Err("scale".into());
    };
    let ordinary =
        old.base
            .cas
            .check_proof(&old.base.context, &old.steps, &mut default_budget()?)?;
    let request = SourceRequest {
        clause: SourceClause::PolynomialSeed,
        source: scalar,
        domain: old.base.domain.clone(),
        field: old.field,
        centre: old.base.zero.clone(),
        variable: old.base.x.clone(),
        scope: Vec::new(),
        context: old.base.context.clone(),
        order: 3,
    };
    let mut f = Fixture {
        base: old.base,
        request,
        certificate: SourceCertificate {
            clause: SourceClause::PolynomialSeed,
            coefficients: vec![rational("-1/2")?],
            companion: Vec::new(),
            units: Vec::new(),
        },
        proofs: vec![ordinary.proof().clone()],
        expected: None,
    };
    admit_expected(&mut f)?;
    f.expected.as_mut().ok_or("expected")?.residuals = old.expected.residuals;
    Ok(f)
}

/// A strict original subregion is fixed before ordinary totality and the source theorem.
pub(crate) fn restricted(reordered: bool) -> TestResult<(Fixture, Handle)> {
    use crate::test_api::domain::ContextDraft;
    let mut f = setup(0, 4, reordered, false)?;
    let ambient = f.base.domain.clone();
    let predicate = f.base.predicate(
        crate::test_api::syntax::PredicateKind::Positive,
        &[f.base.x.clone()],
    )?;
    f.base.domain = f.base.region(&predicate, true)?;
    f.base.context = f
        .base
        .cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(f.base.header.clone()),
                admissible: Reference::Existing(f.base.domain.clone()),
                assumptions: Vec::new(),
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("context")?;
    f.request.domain = f.base.domain.clone();
    f.request.context = f.base.context.clone();
    let request = f.base.request(
        crate::test_api::request::Task::Coverage {
            domain: f.base.domain.clone(),
        },
        &[f.base.x.clone()],
    )?;
    let coordinate = f.base.proved(&request)?.witness;
    let guard = f.base.cas.trig_defined(
        &f.request.source,
        coordinate.proof(),
        &mut default_budget()?,
    )?;
    f.proofs = vec![guard.proof().clone()];
    admit_expected(&mut f)?;
    Ok((f, ambient))
}

/// Factorial formulas are an independent oracle, not the paired checker recurrence.
pub(crate) fn vectors(
    order: u32,
) -> TestResult<(
    Vec<crate::test_api::Rational>,
    Vec<crate::test_api::Rational>,
)> {
    let mut sine = Vec::new();
    let mut cosine = Vec::new();
    for degree in 0..order {
        let factorial = (1..=u64::from(degree)).product::<u64>();
        let sign = if (degree / 2) % 2 == 0 { 1 } else { -1 };
        let coefficient =
            crate::test_api::Rational::from_ratio(sign, factorial, &mut default_budget()?)?;
        sine.push(if degree % 2 == 1 {
            coefficient.clone()
        } else {
            rational("0")?
        });
        cosine.push(if degree % 2 == 0 {
            coefficient
        } else {
            rational("0")?
        });
    }
    Ok((sine, cosine))
}

/// Build origins at orders one, four and six, or the full nonzero-centre polynomial seed.
pub(crate) fn setup(
    kind: usize,
    order: u32,
    reordered: bool,
    admit_output: bool,
) -> TestResult<Fixture> {
    setup_inner(kind, order, reordered, admit_output, true)
}

/// Construct independent syntax, optionally authenticating the sender's ordinary premise.
#[expect(
    clippy::too_many_lines,
    reason = "Independent rows and optional ordinary closure are explicit"
)]
fn setup_inner(
    kind: usize,
    order: u32,
    reordered: bool,
    admit_output: bool,
    with_proofs: bool,
) -> TestResult<Fixture> {
    let old = fixture(0, reordered)?;
    let mut base = old.base;
    let clause = match kind {
        0 => SourceClause::SineOrigin,
        1 => SourceClause::CosineOrigin,
        2 => SourceClause::ArctangentOrigin,
        _ => SourceClause::PolynomialSeed,
    };
    let source = match kind {
        0 => base.term(Operator::Sine, &[base.x.clone()])?,
        1 => base.term(Operator::Cosine, &[base.x.clone()])?,
        2 => base.term(Operator::Arctangent, &[base.x.clone()])?,
        _ => {
            let square = base.term(Operator::IntegerPower(2), &[base.x.clone()])?;
            let two = base.number("2")?;
            let twice = base.term(Operator::Mul, &[two, base.x.clone()])?;
            let sum = base.term(Operator::Add, &[square, twice])?;
            base.term(Operator::Add, &[sum, base.one.clone()])?
        }
    };
    let proofs = if with_proofs {
        let guard_source = if kind < 3 {
            base.x.clone()
        } else {
            source.clone()
        };
        let guard = if kind < 3 {
            let guard = defined(&mut base, &guard_source)?;
            base.cas
                .check_proof(&base.context, &[guard], &mut default_budget()?)?
        } else {
            let request = base.request(
                crate::test_api::request::Task::Coverage {
                    domain: base.domain.clone(),
                },
                &[guard_source],
            )?;
            base.proved(&request)?.witness
        };
        let guard = if kind < 3 {
            base.cas
                .trig_defined(&source, guard.proof(), &mut default_budget()?)?
        } else {
            guard
        };
        vec![guard.proof().clone()]
    } else {
        Vec::new()
    };
    let centre = if kind < 3 {
        base.zero.clone()
    } else {
        base.number("2")?
    };
    let request = SourceRequest {
        clause,
        source,
        domain: base.domain.clone(),
        field: old.field,
        centre,
        variable: base.x.clone(),
        scope: Vec::new(),
        context: base.context.clone(),
        order,
    };
    let (coefficients, companion) = match kind {
        0 | 1 => vectors(order)?,
        2 => (
            ["0", "1", "0", "-1/3", "0", "1/5"]
                .iter()
                .take(usize::try_from(order)?)
                .map(|s| rational(s))
                .collect::<TestResult<Vec<_>>>()?,
            Vec::new(),
        ),
        _ => (
            ["9", "6", "1"]
                .iter()
                .map(|s| rational(s))
                .collect::<TestResult<Vec<_>>>()?,
            Vec::new(),
        ),
    };
    let mut units = Vec::new();
    if kind < 3 {
        for index in 0..order {
            let integer = if kind == 2 {
                if index % 2 == 0 {
                    continue;
                }
                index
            } else {
                if index + 1 == order {
                    continue;
                }
                index + 1
            };
            units.push(SourceUnitRow {
                index,
                integer: rational(&integer.to_string())?,
                inverse: rational(&format!("1/{integer}"))?,
            });
        }
    }
    let certificate = SourceCertificate {
        clause,
        coefficients,
        companion,
        units,
    };
    let mut f = Fixture {
        base,
        request,
        certificate,
        proofs,
        expected: None,
    };
    if admit_output {
        admit_expected(&mut f)?;
    }
    Ok(f)
}

/// Admit only hand-chosen coefficients and reconstruct the complete expected source identity.
pub(crate) fn admit_expected(f: &mut Fixture) -> TestResult {
    let values = if f.request.clause == SourceClause::CosineOrigin {
        &f.certificate.companion
    } else {
        &f.certificate.coefficients
    };
    let mut coefficients = Vec::new();
    for index in 0..usize::try_from(f.request.order)? {
        let zero = rational("0")?;
        let value = values.get(index).unwrap_or(&zero);
        coefficients.push(Reference::Existing(
            f.base.term(Operator::Rational(value.clone()), &[])?,
        ));
    }
    let output = f
        .base
        .cas
        .admit(
            &[Draft::SeriesObject(SeriesObject {
                field: Reference::Existing(f.request.field.clone()),
                centre: Reference::Existing(f.request.centre.clone()),
                variable: Reference::Existing(f.request.variable.clone()),
                scope: f.request.scope.clone(),
                context: Reference::Existing(f.request.context.clone()),
                order: f.request.order,
                coefficients,
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("output")?;
    f.expected = Some(Judgment {
        context: f.request.context.clone(),
        layer: crate::test_api::domain::Layer::Logic,
        space: None,
        statement: Statement::Series(Box::new(SeriesStatement::Source(SourceStatement {
            clause: f.request.clause,
            source: f.request.source.clone(),
            domain: f.request.domain.clone(),
            output,
        }))),
        residuals: Vec::new(),
    });
    Ok(())
}

/// Borrow the independent source conclusion without accepting another result category.
pub(crate) fn statement(expected: &Judgment) -> TestResult<&SourceStatement> {
    let Statement::Series(value) = &expected.statement else {
        return Err("series".into());
    };
    let SeriesStatement::Source(value) = value.as_ref() else {
        return Err("source".into());
    };
    Ok(value)
}

/// Ordered finite source joins include a visible original-composite definedness hypothesis.
#[expect(
    clippy::too_many_lines,
    reason = "The independent fixture spells every original operator and ordered proof vector"
)]
pub(crate) fn lifted(kind: usize, reordered: bool) -> TestResult<Fixture> {
    use crate::test_api::domain::{AssumptionDraft, ContextDraft, HypothesisRef};
    use crate::test_api::proof::{ApplicationDraft, Certificate, Primitive};
    use crate::test_api::series::SeriesOperation as O;
    use crate::test_api::syntax::PredicateKind;
    let mut f = setup(0, 4, reordered, true)?;
    let sine = f.request.source.clone();
    let cosine = f.base.term(Operator::Cosine, &[f.base.x.clone()])?;
    let minus = f.base.number("-1")?;
    let two = f.base.number("2")?;
    let (clause, composite, values) = match kind {
        0 => (SourceClause::Truncate, sine.clone(), vec!["0", "1"]),
        1 => (
            SourceClause::Add,
            f.base
                .term(Operator::Add, &[sine.clone(), cosine.clone()])?,
            vec!["1", "1", "-1/2", "-1/6"],
        ),
        2 => (
            SourceClause::Sub,
            f.base
                .term(Operator::Sub, &[sine.clone(), cosine.clone()])?,
            vec!["-1", "1", "1/2", "-1/6"],
        ),
        3 => (
            SourceClause::Neg,
            f.base.term(Operator::Neg, std::slice::from_ref(&sine))?,
            vec!["0", "-1", "0", "1/6"],
        ),
        4 => (
            SourceClause::ScaleLeft,
            f.base.term(Operator::Mul, &[two.clone(), sine.clone()])?,
            vec!["0", "2", "0", "-1/3"],
        ),
        5 => (
            SourceClause::ScaleRight,
            f.base.term(Operator::Mul, &[sine.clone(), two.clone()])?,
            vec!["0", "2", "0", "-1/3"],
        ),
        6 => (
            SourceClause::Mul,
            f.base
                .term(Operator::Mul, &[sine.clone(), cosine.clone()])?,
            vec!["0", "1", "0", "-2/3"],
        ),
        8 => (
            SourceClause::Mul,
            f.base
                .term(Operator::Div, &[sine.clone(), cosine.clone()])?,
            vec!["0", "1", "0", "-2/3"],
        ),
        _ => (
            SourceClause::Mul,
            f.base.term(Operator::Mul, &[sine.clone(), sine.clone()])?,
            vec!["0", "0", "1", "0"],
        ),
    };
    let ordinary = if kind == 0 {
        None
    } else {
        let predicate = f
            .base
            .predicate(PredicateKind::Defined, std::slice::from_ref(&composite))?;
        let context = f
            .base
            .cas
            .admit(
                &[Draft::Context(ContextDraft::Root {
                    header: Reference::Existing(f.base.header.clone()),
                    admissible: Reference::Existing(f.base.domain.clone()),
                    assumptions: vec![AssumptionDraft {
                        key: "original-composite-defined".into(),
                        condition: Reference::Existing(predicate.clone()),
                    }],
                })],
                &mut default_budget()?,
            )?
            .pop()
            .ok_or("context")?;
        f.base.context = context.clone();
        f.request.context = context.clone();
        let hypothesis = ApplicationDraft {
            rule: Primitive::ContextHypothesis,
            context: context.clone(),
            conclusion: Statement::PredicateFact {
                region: f.base.domain.clone(),
                predicate,
            },
            premises: Vec::new(),
            substitutions: Vec::new(),
            certificate: Certificate::Hypothesis(HypothesisRef { context, slot: 0 }),
        };
        Some(
            f.base
                .cas
                .check_proof(&f.base.context, &[hypothesis], &mut default_budget()?)?
                .proof()
                .clone(),
        )
    };
    let x = f.base.x.clone();
    let guard = defined(&mut f.base, &x)?;
    let guard = f
        .base
        .cas
        .check_proof(&f.base.context, &[guard], &mut default_budget()?)?;
    let coordinate_guard = guard.proof().clone();
    let guard = f
        .base
        .cas
        .trig_defined(&sine, guard.proof(), &mut default_budget()?)?;
    f.proofs = vec![guard.proof().clone()];
    admit_expected(&mut f)?;
    let left = statement(f.expected.as_ref().ok_or("left")?)?.clone();
    let left_proof =
        f.base
            .cas
            .check_series_source(&left, &f.certificate, &f.proofs, &mut default_budget()?)?;
    let (right, right_proof) = if matches!(kind, 1 | 2 | 6 | 8) {
        f.request.source = cosine.clone();
        f.request.clause = SourceClause::CosineOrigin;
        f.certificate.clause = SourceClause::CosineOrigin;
        let guard = f
            .base
            .cas
            .trig_defined(&cosine, &coordinate_guard, &mut default_budget()?)?;
        f.proofs = vec![guard.proof().clone()];
        admit_expected(&mut f)?;
        let right = statement(f.expected.as_ref().ok_or("right")?)?.clone();
        let proof = f.base.cas.check_series_source(
            &right,
            &f.certificate,
            &f.proofs,
            &mut default_budget()?,
        )?;
        (right, proof)
    } else {
        (left.clone(), left_proof.clone())
    };
    let mut proofs = vec![left_proof.proof().clone()];
    let operation = match kind {
        0 => O::Truncate {
            input: left.output.clone(),
            order: 2,
        },
        1 | 2 => {
            proofs.push(right_proof.proof().clone());
            let right_output = if kind == 2 {
                let fact = defined(&mut f.base, &minus)?;
                let fact =
                    f.base
                        .cas
                        .check_proof(&f.base.context, &[fact], &mut default_budget()?)?;
                let scaled = f.base.cas.compute_series(
                    &O::Scale {
                        input: right.output,
                        scalar: minus.clone(),
                    },
                    &[fact.proof().clone()],
                    &mut default_budget()?,
                )?;
                proofs.push(scaled.proof().clone());
                object_output(scaled.judgment())?
            } else {
                right.output
            };
            O::Add {
                left: left.output.clone(),
                right: right_output,
                order: Some(4),
            }
        }
        3..=5 => O::Scale {
            input: left.output.clone(),
            scalar: if kind == 3 { minus.clone() } else { two },
        },
        _ => {
            proofs.push(right_proof.proof().clone());
            O::Mul {
                left: left.output.clone(),
                right: right.output,
                order: Some(4),
            }
        }
    };
    let mut object_proofs = Vec::new();
    if let O::Scale { scalar, .. } = &operation {
        let fact = defined(&mut f.base, scalar)?;
        object_proofs.push(
            f.base
                .cas
                .check_proof(&f.base.context, &[fact], &mut default_budget()?)?
                .proof()
                .clone(),
        );
    }
    let computation =
        f.base
            .cas
            .compute_series(&operation, &object_proofs, &mut default_budget()?)?;
    proofs.push(computation.proof().clone());
    if let Some(proof) = ordinary {
        proofs.push(proof);
    }
    f.request.clause = clause;
    f.request.source = composite;
    f.request.order = u32::try_from(values.len())?;
    f.certificate = SourceCertificate {
        clause,
        coefficients: values
            .iter()
            .map(|v| rational(v))
            .collect::<TestResult<_>>()?,
        companion: Vec::new(),
        units: Vec::new(),
    };
    admit_expected(&mut f)?;
    assert_eq!(
        statement(f.expected.as_ref().ok_or("expected")?)?.output,
        object_output(computation.judgment())?
    );
    f.certificate.coefficients.clear();
    f.proofs = proofs;
    Ok(f)
}

/// Object inference remains a distinct result category from the lifted conclusion.
fn object_output(judgment: &Judgment) -> TestResult<Handle> {
    let Statement::Series(value) = &judgment.statement else {
        return Err("object".into());
    };
    match value.as_ref() {
        SeriesStatement::Truncate { output, .. }
        | SeriesStatement::Scale { output, .. }
        | SeriesStatement::Add { output, .. }
        | SeriesStatement::Mul { output, .. } => Ok(output.clone()),
        _ => Err("object inference".into()),
    }
}

/// Minimal polynomial seed keeps its independent portable ordinary closure small.
pub(crate) fn linear_seed(reordered: bool) -> TestResult<Fixture> {
    let mut f = setup(0, 2, reordered, false)?;
    f.request.clause = SourceClause::PolynomialSeed;
    f.request.source = f.base.x.clone();
    f.certificate = SourceCertificate {
        clause: SourceClause::PolynomialSeed,
        coefficients: vec![rational("0")?, rational("1")?],
        companion: Vec::new(),
        units: Vec::new(),
    };
    let x = f.base.x.clone();
    let guard = defined(&mut f.base, &x)?;
    let proof = f
        .base
        .cas
        .check_proof(&f.base.context, &[guard], &mut default_budget()?)?;
    f.proofs = vec![proof.proof().clone()];
    admit_expected(&mut f)?;
    Ok(f)
}

/// Construct only syntax and the independent expectation in a fresh receiving store.
pub(crate) fn receiver(kind: usize, order: u32, reordered: bool) -> TestResult<Fixture> {
    use crate::test_api::domain::{AssumptionDraft, ContextDraft};
    use crate::test_api::syntax::PredicateKind;
    if kind < 4 {
        return setup_inner(kind, order, reordered, true, false);
    }
    let kind = kind - 4;
    let mut f = setup_inner(0, 4, reordered, false, false)?;
    let sine = f.request.source.clone();
    let cosine = f.base.term(Operator::Cosine, &[f.base.x.clone()])?;
    let two = f.base.number("2")?;
    let (clause, composite, values) = match kind {
        0 => (SourceClause::Truncate, sine, vec!["0", "1"]),
        1 => (
            SourceClause::Add,
            f.base.term(Operator::Add, &[sine, cosine])?,
            vec!["1", "1", "-1/2", "-1/6"],
        ),
        2 => (
            SourceClause::Sub,
            f.base.term(Operator::Sub, &[sine, cosine])?,
            vec!["-1", "1", "1/2", "-1/6"],
        ),
        3 => (
            SourceClause::Neg,
            f.base.term(Operator::Neg, std::slice::from_ref(&sine))?,
            vec!["0", "-1", "0", "1/6"],
        ),
        4 => (
            SourceClause::ScaleLeft,
            f.base.term(Operator::Mul, &[two, sine])?,
            vec!["0", "2", "0", "-1/3"],
        ),
        5 => (
            SourceClause::ScaleRight,
            f.base.term(Operator::Mul, &[sine, two])?,
            vec!["0", "2", "0", "-1/3"],
        ),
        6 => (
            SourceClause::Mul,
            f.base.term(Operator::Mul, &[sine, cosine])?,
            vec!["0", "1", "0", "-2/3"],
        ),
        8 => (
            SourceClause::Mul,
            f.base.term(Operator::Div, &[sine, cosine])?,
            vec!["0", "1", "0", "-2/3"],
        ),
        _ => (
            SourceClause::Mul,
            f.base.term(Operator::Mul, &[sine.clone(), sine])?,
            vec!["0", "0", "1", "0"],
        ),
    };
    if kind != 0 {
        let predicate = f
            .base
            .predicate(PredicateKind::Defined, std::slice::from_ref(&composite))?;
        let context = f
            .base
            .cas
            .admit(
                &[Draft::Context(ContextDraft::Root {
                    header: Reference::Existing(f.base.header.clone()),
                    admissible: Reference::Existing(f.base.domain.clone()),
                    assumptions: vec![AssumptionDraft {
                        key: "original-composite-defined".into(),
                        condition: Reference::Existing(predicate),
                    }],
                })],
                &mut default_budget()?,
            )?
            .pop()
            .ok_or("context")?;
        f.base.context = context.clone();
        f.request.context = context;
    }
    f.request.clause = clause;
    f.request.source = composite;
    f.request.order = u32::try_from(values.len())?;
    f.certificate = SourceCertificate {
        clause,
        coefficients: values
            .iter()
            .map(|s| rational(s))
            .collect::<TestResult<_>>()?,
        companion: Vec::new(),
        units: Vec::new(),
    };
    admit_expected(&mut f)?;
    Ok(f)
}

/// The seed golden receiver admits no ordinary or series facts.
pub(crate) fn linear_seed_receiver(reordered: bool) -> TestResult<Fixture> {
    let mut f = setup_inner(0, 2, reordered, false, false)?;
    f.request.clause = SourceClause::PolynomialSeed;
    f.request.source = f.base.x.clone();
    f.certificate = SourceCertificate {
        clause: SourceClause::PolynomialSeed,
        coefficients: vec![rational("0")?, rational("1")?],
        companion: Vec::new(),
        units: Vec::new(),
    };
    admit_expected(&mut f)?;
    Ok(f)
}
