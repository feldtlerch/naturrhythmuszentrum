// SPDX-License-Identifier: AGPL-3.0-only
//! Finite exact recognition of a concrete integer multiple of half Pi.

#![expect(
    clippy::redundant_pub_crate,
    reason = "closed statement, codec and checker dispatch share these entrypoints"
)]

use crate::budget::Budget;
use crate::error::{Fault, InvalidCode, Location};
use crate::graph::{Cas, Handle, Vertex};
use crate::model::{AtomicSort, Constant, Field, Operator, PredicateKind, Sort};
use crate::number::Rational;
use crate::proof_model::{
    ApplicationDraft, Certificate, CheckedApplication, ScopedConditions, Statement,
};

/// Recognition never supplies a trigonometric value or a numerical approximation.
const AT: Location = Location::new("integer half-Pi witness");

/// Forged recognition is malformed evidence, not a mathematical negative result.
const fn invalid() -> Fault {
    Fault::invalid(InvalidCode::EndpointMismatch, AT)
}

/// Validate the value payload and graph roles independently of any supplied certificate.
pub(crate) fn roles(
    cas: &Cas,
    u: &Handle,
    k: &Rational,
    context: &Handle,
    domain: &Handle,
    budget: &mut Budget,
) -> Result<(), Fault> {
    k.check(budget, AT)?;
    budget.work(1, AT)?;
    if !k.is_integer() {
        return Err(invalid());
    }
    let operand = cas.state.expression(cas.checked(u)?)?;
    let frame = cas.state.context(cas.checked(context)?)?;
    let region = cas.state.region(cas.checked(domain)?)?;
    if operand.sort != Sort::Value(AtomicSort::Scalar(Field::Real))
        || operand.space.is_some()
        || (!region.scope.is_empty() && region.scope != operand.scope)
    {
        return Err(Fault::invalid(InvalidCode::Sort, AT));
    }
    for header in [operand.header, region.header] {
        if !crate::header::embeds(&cas.state, header, frame.header_id, budget)? {
            return Err(Fault::invalid(InvalidCode::Context, AT));
        }
    }
    Ok(())
}

/// Both inputs have already been independently checked by the ordinary backward proof pipeline.
/// Their complete conditions survive recognition, including original divisor requirements.
pub(crate) fn check(
    cas: &Cas,
    draft: &ApplicationDraft,
    premises: &[CheckedApplication],
    budget: &mut Budget,
) -> Result<ScopedConditions, Fault> {
    let Statement::HalfPiWitness {
        u,
        k,
        context,
        domain,
    } = &draft.conclusion
    else {
        return Err(invalid());
    };
    let Certificate::HalfPiWitness {
        u: original,
        k: integer,
        context: home,
        domain: region,
        defined,
        equality,
    } = &draft.certificate
    else {
        return Err(invalid());
    };
    roles(cas, u, k, context, domain, budget)?;
    integer.check(budget, AT)?;
    if (u, k, context, domain) != (original, integer, home, region)
        || *context != draft.context
        || (*defined, *equality) != (0, 1)
        || !draft.substitutions.is_empty()
    {
        return Err(invalid());
    }
    let [guard, equality] = premises else {
        return Err(invalid());
    };
    for premise in premises {
        if premise.judgment.context != *context {
            return Err(invalid());
        }
    }
    definedness(cas, u, domain, &guard.judgment.statement)?;
    let Statement::Equality {
        left,
        right,
        domain: equality_domain,
        ..
    } = &equality.judgment.statement
    else {
        return Err(invalid());
    };
    if left != u || equality_domain != domain {
        return Err(invalid());
    }
    coefficient(cas, u, right, k, budget)?;
    let mut conditions = ScopedConditions::default();
    for premise in premises {
        crate::statement::merge(cas, &mut conditions, &premise.conditions, budget)?;
    }
    Ok(conditions)
}

/// Only the original domain or the exact Defined predicate supplies the first slot.
fn definedness(cas: &Cas, u: &Handle, domain: &Handle, statement: &Statement) -> Result<(), Fault> {
    match statement {
        Statement::DefinedDomain {
            function,
            domain: region,
        } if function == u && region == domain => Ok(()),
        Statement::PredicateFact { region, predicate } if region == domain => {
            let Vertex::Condition(value) = cas.state.node(cas.checked(predicate)?)? else {
                return Err(invalid());
            };
            if value.kind != PredicateKind::Defined || value.arguments != [cas.checked(u)?] {
                return Err(invalid());
            }
            Ok(())
        }
        _ => Err(invalid()),
    }
}

/// Reconstruct half the integer using bounded exact arithmetic, never printed digits.
fn coefficient(
    cas: &Cas,
    u: &Handle,
    right: &Handle,
    k: &Rational,
    budget: &mut Budget,
) -> Result<(), Fault> {
    let operand = cas.state.expression(cas.checked(u)?)?;
    let endpoint = cas.state.expression(cas.checked(right)?)?;
    if endpoint.sort != operand.sort
        || endpoint.scope != operand.scope
        || endpoint.ambient != operand.ambient
    {
        return Err(invalid());
    }
    let two = Rational::from_ratio(2, 1, budget)?;
    let half = k.div(&two, budget)?;
    let (pi, rational) =
        crate::rule_check::binary(&cas.state, cas.checked(right)?, &Operator::Mul)?;
    let pi = cas.state.expression(pi)?;
    let rational = cas.state.expression(rational)?;
    let Operator::Rational(q) = &rational.operator else {
        return Err(invalid());
    };
    q.check(budget, AT)?;
    if pi.operator != Operator::Constant(Constant::Pi) || *q != half || q.mul(&two, budget)? != *k {
        return Err(invalid());
    }
    Ok(())
}

/// Statement subtype 31 follows the lattice nearest-selection subtype 30.
pub(crate) fn encode_statement(
    value: &Statement,
    out: &mut crate::encoding::Encoder<'_>,
    write: &mut impl FnMut(&Handle, &mut crate::encoding::Encoder<'_>) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let Statement::HalfPiWitness {
        u,
        k,
        context,
        domain,
    } = value
    else {
        return Err(invalid());
    };
    out.byte(31)?;
    write(u, out)?;
    out.number(k)?;
    write(context, out)?;
    write(domain, out)
}

/// Certificate mode 19 follows lattice selection mode 18, independently of statement tags.
pub(crate) fn encode_certificate(
    value: &Certificate,
    out: &mut crate::encoding::Encoder<'_>,
    write: &mut impl FnMut(&Handle, &mut crate::encoding::Encoder<'_>) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let Certificate::HalfPiWitness {
        u,
        k,
        context,
        domain,
        defined,
        equality,
    } = value
    else {
        return Err(invalid());
    };
    out.byte(19)?;
    write(u, out)?;
    out.number(k)?;
    write(context, out)?;
    write(domain, out)?;
    out.unsigned(u64::from(*defined))?;
    out.unsigned(u64::from(*equality))
}
