// SPDX-License-Identifier: AGPL-3.0-only
//! Closed source laws and finite inference on an identical original domain.

use super::check::{AT, compatible, defined, equal, invalid, number, unsupported};
use super::{SeriesCertificate, SeriesObject, SeriesStatement};
use crate::budget::{Budget, BudgetKind};
use crate::error::Fault;
use crate::graph::{Cas, Handle, size};
use crate::model::{AtomicSort, Field, Operator, Sort};
use crate::number::Rational;
use crate::proof_model::{
    ApplicationDraft, Certificate, CheckedApplication, ScopedConditions, Statement,
};
use crate::rule::Primitive;

/// Exactly the admitted theorem and finite inference clauses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceClause {
    /// Paired formal sine law at zero.
    SineOrigin,
    /// Paired formal cosine law at zero.
    CosineOrigin,
    /// Principal real arctangent law at zero.
    ArctangentOrigin,
    /// Complete centred rational polynomial expansion.
    PolynomialSeed,
    /// Explicit source restriction through object truncation.
    Truncate,
    /// Ordered original addition.
    Add,
    /// Ordered subtraction through negative scaling and addition.
    Sub,
    /// Original multiplication with a rational left scalar.
    ScaleLeft,
    /// Original multiplication with a rational right scalar.
    ScaleRight,
    /// Original negation through scaling by minus one.
    Neg,
    /// Ordered original product.
    Mul,
}

impl SourceClause {
    /// The registered primitive owning this clause.
    #[must_use]
    pub const fn primitive(self) -> Primitive {
        match self {
            Self::SineOrigin | Self::CosineOrigin | Self::ArctangentOrigin => {
                Primitive::SeriesSource
            }
            Self::PolynomialSeed | Self::Truncate => Primitive::SeriesTruncate,
            Self::Add | Self::Sub => Primitive::SeriesAdd,
            Self::ScaleLeft | Self::ScaleRight | Self::Neg => Primitive::SeriesScale,
            Self::Mul => Primitive::SeriesMul,
        }
    }

    /// Closed mode allocation, independent of result and graph tags.
    #[must_use]
    pub const fn mode(self) -> u8 {
        match self {
            Self::SineOrigin | Self::CosineOrigin | Self::ArctangentOrigin => 10,
            Self::PolynomialSeed | Self::Truncate => 0,
            Self::Add | Self::Sub => 1,
            Self::ScaleLeft | Self::ScaleRight | Self::Neg => 2,
            Self::Mul => 3,
        }
    }

    /// Direct theorem, lifted inference or polynomial seed form.
    #[must_use]
    pub const fn form(self) -> u8 {
        match self {
            Self::SineOrigin | Self::CosineOrigin | Self::ArctangentOrigin => 0,
            Self::PolynomialSeed => 2,
            _ => 1,
        }
    }

    /// Mode-specific closed selector; no reserved law is admitted.
    pub(super) const fn selector(self) -> u8 {
        match self {
            Self::SineOrigin => 3,
            Self::CosineOrigin => 4,
            Self::ArctangentOrigin => 5,
            Self::Sub | Self::ScaleRight => 1,
            Self::Neg => 2,
            _ => 0,
        }
    }

    /// Typed arity is known before measuring any supplied proof payload.
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::SineOrigin
            | Self::CosineOrigin
            | Self::ArctangentOrigin
            | Self::PolynomialSeed => 1,
            Self::Truncate => 2,
            Self::ScaleLeft | Self::ScaleRight | Self::Neg => 3,
            Self::Add | Self::Mul => 4,
            Self::Sub => 5,
        }
    }

    /// Refuse every unregistered mode/form/law combination.
    pub(super) const fn read(mode: u8, form: u8, selector: u8) -> Result<Self, Fault> {
        Ok(match (mode, form, selector) {
            (10, 0, 3) => Self::SineOrigin,
            (10, 0, 4) => Self::CosineOrigin,
            (10, 0, 5) => Self::ArctangentOrigin,
            (0, 2, 0) => Self::PolynomialSeed,
            (0, 1, 0) => Self::Truncate,
            (1, 1, 0) => Self::Add,
            (1, 1, 1) => Self::Sub,
            (2, 1, 0) => Self::ScaleLeft,
            (2, 1, 1) => Self::ScaleRight,
            (2, 1, 2) => Self::Neg,
            (3, 1, 0) => Self::Mul,
            _ => return Err(unsupported()),
        })
    }
}

/// Exact original graph and domain; the output retains all increment-ring metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceStatement {
    /// Registered theorem or finite inference clause.
    pub clause: SourceClause,
    /// Original expression, never its polynomial representative.
    pub source: Handle,
    /// Exact context admissible region, without transport or side restriction.
    pub domain: Handle,
    /// Referenced finite rational object with complete interpretation and order.
    pub output: Handle,
}

/// Untrusted proposed integer unit, reconstructed by the checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceUnitRow {
    /// Ascending transition or odd coefficient index.
    pub index: u32,
    /// Proposed positive integer embedding.
    pub integer: Rational,
    /// Proposed inverse, checked by multiplication to one.
    pub inverse: Rational,
}

/// Complete theorem rows, polynomial coefficients or empty lifted evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCertificate {
    /// Exact clause, agreeing with the statement.
    pub clause: SourceClause,
    /// Complete sine vector, arctangent vector or untruncated polynomial expansion.
    pub coefficients: Vec<Rational>,
    /// Complete cosine companion for either paired theorem; empty otherwise.
    pub companion: Vec<Rational>,
    /// Every required integer unit in ascending order; empty for finite lifting.
    pub units: Vec<SourceUnitRow>,
}

/// Measure borrowed source evidence before any owned candidate copy.
pub(super) fn evidence_bytes(value: &SourceCertificate, budget: &mut Budget) -> Result<u64, Fault> {
    let count = value
        .coefficients
        .len()
        .checked_add(value.companion.len())
        .and_then(|n| n.checked_add(value.units.len()))
        .ok_or_else(invalid)?;
    budget.check(BudgetKind::JetEntries, size(count, AT)?, AT)?;
    let mut out = crate::encoding::Encoder::new(budget, AT);
    encode_evidence(value, &mut out)?;
    size(out.finish().len(), AT)?
        .checked_mul(4)
        .and_then(|n| n.checked_add(size(count, AT).ok()?.checked_mul(256)?))
        .ok_or_else(invalid)
}

/// Encode the full independent vectors and units without cloning input-controlled components.
pub(super) fn encode_evidence(
    value: &SourceCertificate,
    out: &mut crate::encoding::Encoder<'_>,
) -> Result<(), Fault> {
    out.byte(value.clause.selector())?;
    crate::split_wire::numbers(&value.coefficients, out)?;
    crate::split_wire::numbers(&value.companion, out)?;
    out.count(value.units.len())?;
    for row in &value.units {
        crate::wire::index(row.index, out)?;
        out.number(&row.integer)?;
        out.number(&row.inverse)?;
    }
    Ok(())
}

/// Original source and complete requested increment-ring interpretation.
#[derive(Clone, Debug)]
pub struct SourceRequest {
    /// Exact admitted clause.
    pub clause: SourceClause,
    /// Original expression graph.
    pub source: Handle,
    /// Original context admissible region.
    pub domain: Handle,
    /// Rational coefficient interpretation.
    pub field: Handle,
    /// Rational expansion centre.
    pub centre: Handle,
    /// Declared Real coordinate.
    pub variable: Handle,
    /// Complete binder frame.
    pub scope: crate::model::Scope,
    /// Complete assumption context.
    pub context: Handle,
    /// Positive coefficient count.
    pub order: u32,
}

impl Cas {
    /// Check a supplied source candidate with aggregate ownership of every ordered proof slot.
    ///
    /// # Errors
    /// Refuses false source, domain, coefficient or premise joins and exhausted caller budgets.
    pub fn check_series_source(
        &mut self,
        statement: &SourceStatement,
        certificate: &SourceCertificate,
        proofs: &[Handle],
        budget: &mut Budget,
    ) -> Result<crate::proof_model::VerifiedWitness, Fault> {
        self.proof_limits(budget, |cas, budget| {
            cas.transaction(budget, |cas, budget| {
                if proofs.len() != statement.clause.arity() {
                    return Err(invalid());
                }
                let workspace = evidence_bytes(certificate, budget)?;
                let bytes = workspace
                    .checked_add(premise_bytes(cas, proofs, budget)?)
                    .ok_or_else(invalid)?;
                budget.with_retained_payload(bytes, AT, |budget| {
                    let draft = ApplicationDraft {
                        rule: statement.clause.primitive(),
                        context: cas.series_object(&statement.output)?.context.clone(),
                        conclusion: Statement::Series(Box::new(SeriesStatement::Source(
                            statement.clone(),
                        ))),
                        premises: Vec::new(),
                        substitutions: Vec::new(),
                        certificate: Certificate::Series(Box::new(SeriesCertificate::Source(
                            certificate.clone(),
                        ))),
                    };
                    let proof = publish(cas, &draft, proofs, budget)?;
                    cas.seal_scoped(&proof, budget)
                })
            })
        })
    }

    /// Produce an untrusted exact source candidate and independently check it before publication.
    ///
    /// # Errors
    /// Refuses unavailable sources or laws, malformed ordered premises and named resource exhaustion.
    pub fn expand_series_source(
        &mut self,
        request: &SourceRequest,
        proofs: &[Handle],
        budget: &mut Budget,
    ) -> Result<crate::proof_model::VerifiedWitness, Fault> {
        self.proof_limits(budget, |cas, budget| {
            cas.transaction(budget, |cas, budget| {
                if proofs.len() != request.clause.arity() {
                    return Err(invalid());
                }
                crate::validation::scope_limit(&request.scope, budget)?;
                let scope_bytes = if request.scope.is_empty() {
                    0
                } else {
                    let mut out = crate::encoding::Encoder::new(budget, AT);
                    crate::encoding::scope(&request.scope, &mut out)?;
                    size(out.finish().len(), AT)?
                        .checked_mul(16)
                        .and_then(|n| {
                            n.checked_add(size(request.scope.len(), AT).ok()?.checked_mul(96)?)
                        })
                        .ok_or_else(invalid)?
                };
                let degree = request.order.checked_sub(1).ok_or_else(unsupported)?;
                budget.check(BudgetKind::JetOrder, u64::from(degree), AT)?;
                let n = usize::try_from(request.order).map_err(|_| invalid())?;
                let entries = u64::from(request.order)
                    .checked_mul(4)
                    .ok_or_else(invalid)?;
                budget.check(BudgetKind::JetEntries, entries, AT)?;
                let per_entry = budget
                    .limits()
                    .component_bits
                    .checked_div(4)
                    .and_then(|n| n.checked_add(512))
                    .ok_or_else(invalid)?;
                let workspace = entries
                    .checked_mul(per_entry)
                    .and_then(|n| n.checked_add(scope_bytes))
                    .ok_or_else(invalid)?;
                let bytes = workspace
                    .checked_add(premise_bytes(cas, proofs, budget)?)
                    .ok_or_else(invalid)?;
                budget.work(workspace, AT)?;
                budget.with_retained_payload(bytes, AT, |budget| {
                    produce(cas, request, n, proofs, budget)
                })
            })
        })
    }
}

/// Each occurrence counts independently, even when both ordered slots cite the same handle.
fn premise_bytes(cas: &Cas, proofs: &[Handle], budget: &mut Budget) -> Result<u64, Fault> {
    let mut bytes = 0_u64;
    for proof in proofs {
        bytes = bytes
            .checked_add(crate::proof_view::application_bytes(
                cas,
                cas.checked(proof)?,
                budget,
            )?)
            .ok_or_else(invalid)?;
    }
    Ok(bytes)
}

/// Copy every slot only inside the caller's aggregate reservation, through publication.
fn publish(
    cas: &mut Cas,
    draft: &ApplicationDraft,
    proofs: &[Handle],
    budget: &mut Budget,
) -> Result<Handle, Fault> {
    let mut premises = Vec::new();
    premises
        .try_reserve_exact(proofs.len())
        .map_err(|_| Fault::allocation(AT))?;
    for proof in proofs {
        premises.push(crate::proof_view::application(
            cas,
            cas.checked(proof)?,
            budget,
        )?);
    }
    let mut checked = crate::scoped_check::check_application(cas, draft, &premises, budget)?;
    checked.premises = proofs.iter().map(Handle::logical_id).collect();
    cas.publish_scoped(checked, budget)
}

/// Candidate arithmetic does not grant source authority; publication always invokes the checker.
fn produce(
    cas: &mut Cas,
    request: &SourceRequest,
    n: usize,
    proofs: &[Handle],
    budget: &mut Budget,
) -> Result<crate::proof_model::VerifiedWitness, Fault> {
    let retained = cas.judgment_reservation(cas.retained_judgment_bytes(), budget, AT)?;
    let draft = budget.with_retained_payload(retained, AT, |budget| {
        candidate(cas, request, n, proofs, budget)
    })?;
    let evidence = super::codec::source_workspace(&draft.certificate, budget)?;
    budget.with_retained_payload(evidence, AT, |budget| {
        let proof = publish(cas, &draft, proofs, budget)?;
        cas.seal_scoped(&proof, budget)
    })
}

/// Construct the candidate while retained judgments remain beside arithmetic scratch.
#[expect(
    clippy::too_many_lines,
    reason = "Candidate construction retains each closed source clause together"
)]
fn candidate(
    cas: &mut Cas,
    request: &SourceRequest,
    n: usize,
    proofs: &[Handle],
    budget: &mut Budget,
) -> Result<ApplicationDraft, Fault> {
    use crate::graph::{Draft, ExpressionDraft, Reference};
    let mut evidence = SourceCertificate {
        clause: request.clause,
        coefficients: Vec::new(),
        companion: Vec::new(),
        units: Vec::new(),
    };
    let mut basis = SeriesObject {
        field: request.field.clone(),
        centre: request.centre.clone(),
        variable: request.variable.clone(),
        scope: request.scope.clone(),
        context: request.context.clone(),
        order: request.order,
        coefficients: Vec::new(),
    };
    let output = if request.clause.form() == 1 {
        let slot = match request.clause {
            SourceClause::Sub => 3,
            SourceClause::Add | SourceClause::Mul => 2,
            _ => 1,
        };
        let application = crate::proof_view::application(cas, cas.checked(&proofs[slot])?, budget)?;
        match object(&application)? {
            SeriesStatement::Truncate { output, .. }
            | SeriesStatement::Add { output, .. }
            | SeriesStatement::Scale { output, .. }
            | SeriesStatement::Mul { output, .. } => output.clone(),
            _ => return Err(invalid()),
        }
    } else {
        if request.clause == SourceClause::PolynomialSeed {
            evidence.coefficients = polynomial(cas, &request.source, &basis, 1, budget)?;
        } else {
            evidence.coefficients = zeros(n, budget)?;
            evidence
                .units
                .try_reserve_exact(n)
                .map_err(|_| Fault::allocation(AT))?;
            if request.clause == SourceClause::ArctangentOrigin {
                for index in (1..request.order).step_by(2) {
                    let integer = super::calculus::integer(index, budget)?;
                    let inverse = Rational::one().div(&integer, budget)?;
                    evidence.coefficients[usize::try_from(index).map_err(|_| invalid())?] =
                        if (index / 2) % 2 == 0 {
                            super::produce::copied(&inverse, budget)?
                        } else {
                            inverse.negated(budget)?
                        };
                    evidence.units.push(SourceUnitRow {
                        index,
                        integer,
                        inverse,
                    });
                }
            } else {
                evidence.companion = zeros(n, budget)?;
                evidence.companion[0] = Rational::one();
                for index in 0..request.order - 1 {
                    let i = usize::try_from(index).map_err(|_| invalid())?;
                    let integer = super::calculus::integer(index + 1, budget)?;
                    let inverse = Rational::one().div(&integer, budget)?;
                    evidence.coefficients[i + 1] = evidence.companion[i].mul(&inverse, budget)?;
                    evidence.companion[i + 1] = evidence.coefficients[i]
                        .negated(budget)?
                        .mul(&inverse, budget)?;
                    evidence.units.push(SourceUnitRow {
                        index,
                        integer,
                        inverse,
                    });
                }
            }
        }
        let values = if request.clause == SourceClause::CosineOrigin {
            &evidence.companion
        } else {
            &evidence.coefficients
        };
        basis
            .coefficients
            .try_reserve_exact(n)
            .map_err(|_| Fault::allocation(AT))?;
        let header = cas.handle(cas.state.context(cas.checked(&request.context)?)?.header_id);
        for index in 0..n {
            let zero = Rational::zero();
            let value = super::produce::copied(values.get(index).unwrap_or(&zero), budget)?;
            let literal = cas
                .admit(
                    &[Draft::Expression(ExpressionDraft {
                        header: Reference::Existing(header.clone()),
                        space: None,
                        operator: Operator::Rational(value),
                        sort: Sort::Value(AtomicSort::Scalar(Field::Real)),
                        scope: request.scope.clone(),
                        binding: None,
                        operands: Vec::new(),
                    })],
                    budget,
                )?
                .pop()
                .ok_or_else(invalid)?;
            basis.coefficients.push(literal);
        }
        let object = basis.resolve(&mut |handle| Ok(Reference::Existing(handle.clone())))?;
        cas.admit(&[Draft::SeriesObject(object)], budget)?
            .pop()
            .ok_or_else(invalid)?
    };
    let actual = cas.series_object(&output)?;
    compatible(&basis, actual, budget)?;
    if actual.order != request.order {
        return Err(invalid());
    }
    let statement = SourceStatement {
        clause: request.clause,
        source: request.source.clone(),
        domain: request.domain.clone(),
        output,
    };
    Ok(ApplicationDraft {
        rule: request.clause.primitive(),
        context: request.context.clone(),
        conclusion: Statement::Series(Box::new(SeriesStatement::Source(statement))),
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: Certificate::Series(Box::new(SeriesCertificate::Source(evidence))),
    })
}

/// Validate the original source and identical domain without copying the graph.
pub(super) fn roles(cas: &Cas, value: &SourceStatement, budget: &mut Budget) -> Result<(), Fault> {
    let basis = cas.series_object(&value.output)?;
    basis.preflight(budget)?;
    let source = cas.state.expression(cas.checked(&value.source)?)?;
    let context = cas.state.context(cas.checked(&basis.context)?)?;
    crate::rule_check::prepay_shape(source, budget)?;
    let header = context.header_id;
    if source.header != header
        || source.scope != basis.scope
        || source.sort != Sort::Value(AtomicSort::Scalar(Field::Real))
        || source.space.is_some()
        || cas.checked(&value.domain)? != context.admissible
    {
        return Err(invalid());
    }
    Ok(())
}

/// Source checking never promotes bare coefficient equality to source authority.
pub(super) fn check(
    cas: &Cas,
    draft: &ApplicationDraft,
    premises: &[CheckedApplication],
    budget: &mut Budget,
) -> Result<ScopedConditions, Fault> {
    let retained = cas.judgment_reservation(cas.retained_judgment_bytes(), budget, AT)?;
    let previous = budget.reserve_temporary(retained, AT)?;
    let result = check_inner(cas, draft, premises, budget);
    budget.restore_limits(previous);
    result
}

/// Keep all full polynomial intermediates reserved until their final comparisons finish.
fn check_inner(
    cas: &Cas,
    draft: &ApplicationDraft,
    premises: &[CheckedApplication],
    budget: &mut Budget,
) -> Result<ScopedConditions, Fault> {
    let (Statement::Series(statement), Certificate::Series(certificate)) =
        (&draft.conclusion, &draft.certificate)
    else {
        return Err(invalid());
    };
    let (SeriesStatement::Source(value), SeriesCertificate::Source(evidence)) =
        (statement.as_ref(), certificate.as_ref())
    else {
        return Err(invalid());
    };
    if premises.len() != value.clause.arity()
        || evidence.clause != value.clause
        || draft.rule != value.clause.primitive()
        || !draft.substitutions.is_empty()
    {
        return Err(invalid());
    }
    roles(cas, value, budget)?;
    let basis = cas.series_object(&value.output)?;
    if draft.context != basis.context
        || premises.iter().any(|p| p.judgment.context != draft.context)
    {
        return Err(invalid());
    }
    super::codec::measure(certificate, budget)?;
    match value.clause {
        SourceClause::SineOrigin | SourceClause::CosineOrigin | SourceClause::ArctangentOrigin => {
            defined(cas, &value.source, basis, premises)?;
            origin(cas, value, evidence, budget)?;
        }
        SourceClause::PolynomialSeed => {
            defined(cas, &value.source, basis, premises)?;
            if !evidence.companion.is_empty() || !evidence.units.is_empty() {
                return Err(invalid());
            }
            let expanded = polynomial(cas, &value.source, basis, 1, budget)?;
            if expanded.len() != evidence.coefficients.len() {
                return Err(invalid());
            }
            for (actual, proposed) in expanded.iter().zip(&evidence.coefficients) {
                equal(actual, proposed, budget)?;
            }
            selected(cas, basis, &expanded, true, budget)?;
        }
        _ => {
            if !evidence.coefficients.is_empty()
                || !evidence.companion.is_empty()
                || !evidence.units.is_empty()
            {
                return Err(invalid());
            }
            lifted(cas, value, premises, budget)?;
        }
    }
    let mut conditions = ScopedConditions::default();
    for premise in premises {
        crate::statement::merge(cas, &mut conditions, &premise.conditions, budget)?;
    }
    Ok(conditions)
}

/// Compare all selected coefficients; only a fully expanded polynomial permits padding.
fn selected(
    cas: &Cas,
    basis: &SeriesObject<Handle>,
    values: &[Rational],
    pad: bool,
    budget: &mut Budget,
) -> Result<(), Fault> {
    if !pad && size(values.len(), AT)? != u64::from(basis.order) {
        return Err(invalid());
    }
    for (index, output) in basis.coefficients.iter().enumerate() {
        budget.specialized(BudgetKind::JetEntries, 1, true, AT)?;
        let zero = Rational::zero();
        equal(
            number(cas, output)?,
            values.get(index).unwrap_or(&zero),
            budget,
        )?;
    }
    Ok(())
}

/// Every integer unit is checked independently, including transitions through zero.
fn unit(row: &SourceUnitRow, index: u32, integer: u32, budget: &mut Budget) -> Result<(), Fault> {
    budget.work(1, AT)?;
    if row.index != index {
        return Err(invalid());
    }
    let reconstructed = super::calculus::integer(integer, budget)?;
    equal(&row.integer, &reconstructed, budget)?;
    equal(
        &reconstructed.mul(&row.inverse, budget)?,
        &Rational::one(),
        budget,
    )
}

/// Reconstruct the closed operator, its coordinate child and every finite law row.
fn origin(
    cas: &Cas,
    value: &SourceStatement,
    evidence: &SourceCertificate,
    budget: &mut Budget,
) -> Result<(), Fault> {
    let basis = cas.series_object(&value.output)?;
    let source = cas.state.expression(cas.checked(&value.source)?)?;
    let operator = match value.clause {
        SourceClause::SineOrigin => Operator::Sine,
        SourceClause::CosineOrigin => Operator::Cosine,
        SourceClause::ArctangentOrigin => Operator::Arctangent,
        _ => return Err(invalid()),
    };
    budget.work(2, AT)?;
    if source.operator != operator
        || source.operands.as_slice() != [cas.checked(&basis.variable)?]
        || !number(cas, &basis.centre)?.is_zero()
    {
        return Err(unsupported());
    }
    let n = usize::try_from(basis.order).map_err(|_| invalid())?;
    if evidence.coefficients.len() != n {
        return Err(invalid());
    }
    let first = evidence.coefficients.first().ok_or_else(invalid)?;
    equal(first, &Rational::zero(), budget)?;
    if value.clause == SourceClause::ArctangentOrigin {
        if !evidence.companion.is_empty() || evidence.units.len() != n / 2 {
            return Err(invalid());
        }
        let mut units = evidence.units.iter();
        for (index, coefficient) in evidence.coefficients.iter().enumerate() {
            budget.specialized(BudgetKind::JetEntries, 1, true, AT)?;
            if index % 2 == 0 {
                equal(coefficient, &Rational::zero(), budget)?;
            } else {
                let row = units.next().ok_or_else(invalid)?;
                let degree = u32::try_from(index).map_err(|_| invalid())?;
                unit(row, degree, degree, budget)?;
                let expected = if (index / 2) % 2 == 0 {
                    super::produce::copied(&row.inverse, budget)?
                } else {
                    row.inverse.negated(budget)?
                };
                equal(coefficient, &expected, budget)?;
            }
        }
        return selected(cas, basis, &evidence.coefficients, false, budget);
    }
    if evidence.companion.len() != n || evidence.units.len() != n - 1 {
        return Err(invalid());
    }
    equal(
        evidence.companion.first().ok_or_else(invalid)?,
        &Rational::one(),
        budget,
    )?;
    for (index, row) in evidence.units.iter().enumerate() {
        budget.specialized(BudgetKind::JetEntries, 2, true, AT)?;
        let j = u32::try_from(index).map_err(|_| invalid())?;
        unit(row, j, j.checked_add(1).ok_or_else(invalid)?, budget)?;
        equal(
            &row.integer.mul(&evidence.coefficients[index + 1], budget)?,
            &evidence.companion[index],
            budget,
        )?;
        equal(
            &row.integer.mul(&evidence.companion[index + 1], budget)?,
            &evidence.coefficients[index].negated(budget)?,
            budget,
        )?;
    }
    selected(
        cas,
        basis,
        if value.clause == SourceClause::SineOrigin {
            &evidence.coefficients
        } else {
            &evidence.companion
        },
        false,
        budget,
    )
}

/// Read an exact source premise, retaining its complete object and identical domain.
fn source<'a>(
    cas: &Cas,
    value: &SourceStatement,
    premise: &'a CheckedApplication,
    budget: &mut Budget,
) -> Result<&'a SourceStatement, Fault> {
    budget.work(1, AT)?;
    let Statement::Series(statement) = &premise.judgment.statement else {
        return Err(invalid());
    };
    let SeriesStatement::Source(previous) = statement.as_ref() else {
        return Err(invalid());
    };
    if previous.domain != value.domain || premise.rule != previous.clause.primitive() {
        return Err(invalid());
    }
    compatible(
        cas.series_object(&previous.output)?,
        cas.series_object(&value.output)?,
        budget,
    )?;
    Ok(previous)
}

/// Borrow a matching object computation; a source judgment cannot fill this slot.
fn object(premise: &CheckedApplication) -> Result<&SeriesStatement, Fault> {
    let Statement::Series(statement) = &premise.judgment.statement else {
        return Err(invalid());
    };
    if matches!(statement.as_ref(), SeriesStatement::Source(_))
        || premise.rule != statement.primitive()
    {
        return Err(invalid());
    }
    Ok(statement)
}

/// Reconstruct ordered original syntax and the complete finite object inference.
#[expect(
    clippy::too_many_lines,
    reason = "Closed ordered premise vectors remain visible beside their exact operator joins"
)]
fn lifted(
    cas: &Cas,
    value: &SourceStatement,
    premises: &[CheckedApplication],
    budget: &mut Budget,
) -> Result<(), Fault> {
    use SeriesStatement as S;
    use SourceClause as L;
    let basis = cas.series_object(&value.output)?;
    let left = source(cas, value, &premises[0], budget)?;
    if value.clause == L::Truncate {
        let S::Truncate { input, output } = object(&premises[1])? else {
            return Err(invalid());
        };
        return if left.source == value.source && input == &left.output && output == &value.output {
            Ok(())
        } else {
            Err(invalid())
        };
    }
    if cas.series_object(&left.output)?.order != basis.order {
        return Err(invalid());
    }
    defined(cas, &value.source, basis, &premises[premises.len() - 1..])?;
    let expression = cas.state.expression(cas.checked(&value.source)?)?;
    budget.work(size(expression.operands.len(), AT)?, AT)?;
    match value.clause {
        L::Add | L::Sub | L::Mul => {
            let right = source(cas, value, &premises[1], budget)?;
            let operator = match value.clause {
                L::Add => Operator::Add,
                L::Sub => Operator::Sub,
                _ => Operator::Mul,
            };
            if expression.operator != operator
                || expression.operands.as_slice()
                    != [cas.checked(&left.source)?, cas.checked(&right.source)?]
                || cas.series_object(&right.output)?.order != basis.order
            {
                return Err(invalid());
            }
            let (right_object, computation) = if value.clause == L::Sub {
                let S::Scale {
                    input,
                    scalar,
                    output,
                } = object(&premises[2])?
                else {
                    return Err(invalid());
                };
                if input != &right.output {
                    return Err(invalid());
                }
                equal(
                    number(cas, scalar)?,
                    &Rational::one().negated(budget)?,
                    budget,
                )?;
                (output, &premises[3])
            } else {
                (&right.output, &premises[2])
            };
            match object(computation)? {
                S::Add {
                    left: a,
                    right: b,
                    output,
                    order,
                    ..
                } if value.clause != L::Mul => {
                    if a != &left.output
                        || b != right_object
                        || output != &value.output
                        || *order != basis.order
                    {
                        return Err(invalid());
                    }
                }
                S::Mul {
                    left: a,
                    right: b,
                    output,
                    order,
                    ..
                } if value.clause == L::Mul => {
                    if a != &left.output
                        || b != right_object
                        || output != &value.output
                        || *order != basis.order
                    {
                        return Err(invalid());
                    }
                }
                _ => return Err(invalid()),
            }
        }
        L::ScaleLeft | L::ScaleRight | L::Neg => {
            let S::Scale {
                input,
                scalar,
                output,
            } = object(&premises[1])?
            else {
                return Err(invalid());
            };
            if input != &left.output || output != &value.output {
                return Err(invalid());
            }
            super::check::constant(cas, basis, scalar, budget)?;
            let child = cas.checked(&left.source)?;
            let scalar_id = cas.checked(scalar)?;
            let correct = match value.clause {
                L::Neg => {
                    equal(
                        number(cas, scalar)?,
                        &Rational::one().negated(budget)?,
                        budget,
                    )?;
                    expression.operator == Operator::Neg
                        && expression.operands.as_slice() == [child]
                }
                L::ScaleLeft => {
                    expression.operator == Operator::Mul
                        && expression.operands.as_slice() == [scalar_id, child]
                }
                _ => {
                    expression.operator == Operator::Mul
                        && expression.operands.as_slice() == [child, scalar_id]
                }
            };
            if !correct {
                return Err(invalid());
            }
        }
        _ => return Err(unsupported()),
    }
    Ok(())
}

/// Reserve a complete polynomial vector before allocating any coefficient cells.
fn zeros(count: usize, budget: &mut Budget) -> Result<Vec<Rational>, Fault> {
    let n = size(count, AT)?;
    budget.specialized(BudgetKind::JetEntries, n, true, AT)?;
    let cell = budget
        .limits()
        .component_bits
        .checked_div(4)
        .and_then(|n| n.checked_add(128))
        .ok_or_else(invalid)?;
    let bytes = n.checked_mul(cell).ok_or_else(invalid)?;
    budget.work(bytes, AT)?;
    budget.reserve_temporary(bytes, AT)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Fault::allocation(AT))?;
    values.resize_with(count, Rational::zero);
    Ok(values)
}

/// Full multiplication never uses the requested output order as a degree bound.
fn product(a: &[Rational], b: &[Rational], budget: &mut Budget) -> Result<Vec<Rational>, Fault> {
    let count = a
        .len()
        .checked_add(b.len())
        .and_then(|n| n.checked_sub(1))
        .ok_or_else(invalid)?;
    let mut result = zeros(count, budget)?;
    for (i, left) in a.iter().enumerate() {
        for (j, right) in b.iter().enumerate() {
            let index = i.checked_add(j).ok_or_else(invalid)?;
            result[index] = result[index].add(&left.mul(right, budget)?, budget)?;
        }
    }
    Ok(result)
}

/// Traverse every original operand before cancellation, zero multiplication or exponent zero.
fn polynomial(
    cas: &Cas,
    handle: &Handle,
    basis: &SeriesObject<Handle>,
    depth: u64,
    budget: &mut Budget,
) -> Result<Vec<Rational>, Fault> {
    budget.work(1, AT)?;
    budget.check(BudgetKind::Depth, depth, AT)?;
    let expression = cas.state.expression(cas.checked(handle)?)?;
    crate::rule_check::prepay_shape(expression, budget)?;
    let context = cas.state.context(cas.checked(&basis.context)?)?;
    let header = context.header_id;
    if expression.header != header
        || expression.scope != basis.scope
        || expression.sort != Sort::Value(AtomicSort::Scalar(Field::Real))
    {
        return Err(invalid());
    }
    if handle == &basis.variable {
        let mut result = zeros(2, budget)?;
        result[0] = super::produce::copied(number(cas, &basis.centre)?, budget)?;
        result[1] = Rational::one();
        return Ok(result);
    }
    if let Operator::Rational(value) = &expression.operator {
        let mut result = zeros(1, budget)?;
        result[0] = super::produce::copied(value, budget)?;
        return Ok(result);
    }
    let next = depth.checked_add(1).ok_or_else(invalid)?;
    match (&expression.operator, expression.operands.as_slice()) {
        (Operator::Neg, [child]) => {
            let mut result = polynomial(cas, &cas.handle(*child), basis, next, budget)?;
            for value in &mut result {
                *value = value.negated(budget)?;
            }
            Ok(result)
        }
        (Operator::Add | Operator::Sub | Operator::Mul, [left, right]) => {
            let a = polynomial(cas, &cas.handle(*left), basis, next, budget)?;
            let b = polynomial(cas, &cas.handle(*right), basis, next, budget)?;
            if expression.operator == Operator::Mul {
                return product(&a, &b, budget);
            }
            let mut result = zeros(a.len().max(b.len()), budget)?;
            for (index, value) in result.iter_mut().enumerate() {
                let zero = Rational::zero();
                let left = a.get(index).unwrap_or(&zero);
                let right = b.get(index).unwrap_or(&zero);
                *value = if expression.operator == Operator::Add {
                    left.add(right, budget)?
                } else {
                    left.sub(right, budget)?
                };
            }
            Ok(result)
        }
        (Operator::IntegerPower(exponent), [child]) if *exponent >= 0 => {
            let base = polynomial(cas, &cas.handle(*child), basis, next, budget)?;
            let exponent = usize::try_from(*exponent).map_err(|_| unsupported())?;
            let count = base
                .len()
                .checked_sub(1)
                .and_then(|n| n.checked_mul(exponent))
                .and_then(|n| n.checked_add(1))
                .ok_or_else(invalid)?;
            budget.check(BudgetKind::JetEntries, size(count, AT)?, AT)?;
            let mut result = zeros(1, budget)?;
            result[0] = Rational::one();
            for _ in 0..exponent {
                result = product(&result, &base, budget)?;
            }
            Ok(result)
        }
        _ => Err(unsupported()),
    }
}
