// SPDX-License-Identifier: AGPL-3.0-only
//! Frozen exact preparation and separately charged binary64 constant imports.

use nrz_bindings::readings::algebra::{
    Cas, Fault, Location, Rational,
    enclosure::{EnclosureRequest, RefinementPolicy},
    limits::{BudgetKind, BudgetUsage},
};
use nrz_bindings::{Budget, ExpressionEnclosure, ExpressionEnclosureError};
use nrz_scalar::{ArithmeticContext, ArithmeticFailure, Enclosure, LiteralLimits, NumericLiteral};
use std::cmp::Ordering;

/// Exact preparation and numerical import refuse without a partial reading.
#[derive(Debug, thiserror::Error)]
pub enum PreparedConstantError {
    /// Bindings admission, refinement or interruption failed.
    #[error(transparent)]
    Preparation(#[from] ExpressionEnclosureError),
    /// Cumulative exact or storage allowance failed.
    #[error(transparent)]
    Exact(#[from] Fault),
    /// Literal conversion or the declared arithmetic environment refused.
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticFailure),
    /// Absolute precision must be a positive bit count.
    #[error("invalid constant endpoint target")]
    InvalidTarget,
    /// The requested absolute endpoint target or stored error bound is unavailable.
    #[error("constant target not certifiable")]
    TargetNotCertifiable,
}

/// Exact authority only; each numerical import is a separate charged operation.
///
/// Construction calls bindings preparation itself and exclusively encapsulates
/// the actual producing graph; no constructor accepts separately supplied evidence.
/// This owner has no numerical cache, heap region, pins or binding authority.
///
/// Encapsulation excludes graph access, including shared-reference mutation APIs:
/// ```compile_fail,E0616
/// use nrz_matrix::PreparedConstant;
/// fn graph(owner: &PreparedConstant) { let _ = &owner.graph; }
/// ```
/// A supplied bindings result cannot construct a numerical authority:
/// ```compile_fail,E0451
/// use nrz_matrix::PreparedConstant;
/// use nrz_bindings::{ExpressionEnclosure, readings::algebra::Cas};
/// fn substitute(graph: Cas, authority: ExpressionEnclosure) -> PreparedConstant {
///     PreparedConstant { authority, graph }
/// }
/// ```
/// Preparation consumes its producing graph:
/// ```compile_fail,E0382
/// use nrz_matrix::PreparedConstant;
/// use nrz_bindings::{Budget, readings::algebra::{Cas, enclosure::{EnclosureRequest, RefinementPolicy}}};
/// fn mutate(graph: Cas, request: &EnclosureRequest, policy: RefinementPolicy, b: &mut Budget) {
///     let _ = PreparedConstant::new(graph, request, policy, b, &mut |_| false, &mut 0);
///     let _ = graph.counts();
/// }
/// ```
#[derive(Debug)]
pub struct PreparedConstant {
    /// Actual returned preparation, including its complete judgment key.
    authority: ExpressionEnclosure,
    /// Producing graph has no accessor and outlives the returned evidence at destruction.
    graph: Cas,
}

/// Transient imported readings, not a heap owner.
#[derive(Debug, Clone, Copy)]
pub struct ConstantImport {
    /// Center, total and numerical-only error, in that order.
    readings: [f64; 3],
    /// Absolute per-endpoint bit target checked by this import.
    bits: u32,
    /// Admitted scalar arithmetic revision.
    arithmetic_revision: u32,
}

impl ConstantImport {
    /// Center, total and numerical-only error; total equals error.
    #[must_use]
    pub const fn readings(self) -> [f64; 3] {
        self.readings
    }
    /// Checked absolute per-endpoint precision.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.bits
    }
    /// Scalar arithmetic proof revision of this import.
    #[must_use]
    pub const fn arithmetic_revision(self) -> u32 {
        self.arithmetic_revision
    }
}

/// One location covers import receipts and retained storage reservations.
const AT: Location = Location::new("constant binary64 import");

impl PreparedConstant {
    /// Prepare the original checked expression before freezing its producing graph.
    /// # Errors
    /// Preserves bindings' unsupported identity, exact resource, interruption and
    /// finite-refinement failures, returning the consumed graph with the cause.
    /// Work receipts include failed attempts; successful CAS rounds are not rolled back.
    #[expect(
        clippy::result_large_err,
        reason = "refusal returns the same consumed graph without a new allocation"
    )]
    pub fn new(
        mut graph: Cas,
        request: &EnclosureRequest,
        policy: RefinementPolicy,
        budget: &mut Budget,
        interrupt: &mut impl FnMut(BudgetUsage) -> bool,
        work_used: &mut u64,
    ) -> Result<Self, (Cas, PreparedConstantError)> {
        let result =
            ExpressionEnclosure::prepare(&mut graph, request, policy, budget, interrupt, work_used)
                .map_err(PreparedConstantError::from)
                .and_then(|authority| {
                    finalize(&graph, &authority, budget)?;
                    Ok(authority)
                });
        match result {
            Ok(authority) => Ok(Self { authority, graph }),
            Err(error) => Err((graph, error)),
        }
    }

    /// Borrow the original preparation together with its frozen producing graph.
    #[must_use]
    pub const fn authority(&self) -> &ExpressionEnclosure {
        &self.authority
    }

    /// Decode exact endpoints outward and check the guarantee on the actual readings.
    ///
    /// Reusing this owner never prepares or checks the expression again. Literal
    /// receipts are capped by remaining caller Work and reconciled on every outcome.
    /// # Errors
    /// Refuses invalid targets, the binary64 spacing floor, conservative conversion
    /// failing the target, literal limits and cumulative resource exhaustion.
    pub fn import(
        &self,
        bits: u32,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
        work_used: &mut u64,
    ) -> Result<ConstantImport, PreparedConstantError> {
        self.import_with_evidence(bits, literals, context, budget, work_used)
            .map(|(value, _)| value)
    }

    /// Share the charged conversion with controls without retaining its temporary pair.
    fn import_with_evidence(
        &self,
        bits: u32,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
        work_used: &mut u64,
    ) -> Result<(ConstantImport, [f64; 2]), PreparedConstantError> {
        let start = budget.usage().work;
        *work_used = 0;
        let result = self.graph.with_retained_graph(budget, |b| {
            b.with_retained_payload(
                self.authority
                    .owned_bytes()
                    .saturating_add(size_of::<Self>() as u64),
                AT,
                |b| {
                    b.work(1, AT)?;
                    if bits == 0 {
                        return Err(PreparedConstantError::InvalidTarget);
                    }
                    if bits >= 53 {
                        return Err(PreparedConstantError::TargetNotCertifiable);
                    }
                    b.check(
                        BudgetKind::OutputBytes,
                        size_of::<ConstantImport>() as u64,
                        AT,
                    )?;
                    let [lower, upper] = self.authority.endpoints();
                    let lower = decode(lower, literals, context, b)?.lower().get();
                    let upper = decode(upper, literals, context, b)?.upper().get();
                    let endpoints = [lower, upper];
                    let value = finish(&self.authority, endpoints, bits, context, b)?;
                    Ok((value, endpoints))
                },
            )
        });
        *work_used = budget.usage().work.saturating_sub(start);
        result
    }
}

/// The complete returned exact owner is admitted with its producing graph still retained.
fn finalize(graph: &Cas, authority: &ExpressionEnclosure, b: &mut Budget) -> Result<(), Fault> {
    let bytes = authority
        .owned_bytes()
        .saturating_add(size_of::<PreparedConstant>() as u64);
    graph.with_retained_graph(b, |b| {
        b.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        b.check(BudgetKind::OutputBytes, bytes, AT)
    })
}

/// One independent literal attempt reconciles its scan and evaluation receipt once.
fn decode(
    text: &str,
    policy: LiteralLimits,
    context: &ArithmeticContext,
    b: &mut Budget,
) -> Result<Enclosure<f64>, PreparedConstantError> {
    let remaining = b.limits().work.saturating_sub(b.usage().work);
    let limits = LiteralLimits::new(
        policy.max_digits(),
        policy.max_exponent(),
        policy.max_work().min(remaining),
    );
    let mut used = 0;
    let result = NumericLiteral::new_with_work(text, limits, &mut used)
        .and_then(|literal| literal.decode_with_work::<f64>(context, &mut used));
    b.work(used, AT)?;
    Ok(result?.enclosure())
}

/// Exact comparisons certify the endpoint budgets; scalar displacement rounds outward once.
fn finish(
    authority: &ExpressionEnclosure,
    endpoints: [f64; 2],
    bits: u32,
    context: &ArithmeticContext,
    b: &mut Budget,
) -> Result<ConstantImport, PreparedConstantError> {
    let [lower, upper] = endpoints;
    let pair = Enclosure::new(lower, upper)?;
    let lo = represented(pair.lower(), b)?;
    let hi = represented(pair.upper(), b)?;
    let exact = authority.judgment().interval();
    let epsilon = Rational::from_ratio(2, 1, b)?.pow(-i64::from(bits), b)?;
    if lo.compare(exact.lower(), b)? == Ordering::Greater
        || hi.compare(exact.upper(), b)? == Ordering::Less
        || exact.upper().sub(&lo, b)?.compare(&epsilon, b)? == Ordering::Greater
        || hi.sub(exact.lower(), b)?.compare(&epsilon, b)? == Ordering::Greater
    {
        return Err(PreparedConstantError::TargetNotCertifiable);
    }
    b.work(1, AT)?;
    let center = pair.lower();
    let error = pair.displacement_from(center, context)?.get();
    error_target(error, bits)?;
    Ok(ConstantImport {
        readings: [center.get(), error, error],
        bits,
        arithmetic_revision: context.revision(),
    })
}

/// Publication checks the actual numerical error, even after endpoint admission.
fn error_target(error: f64, bits: u32) -> Result<(), PreparedConstantError> {
    let ceiling = f64::from_bits(u64::from(1024 - bits) << 52);
    if error > ceiling {
        return Err(PreparedConstantError::TargetNotCertifiable);
    }
    Ok(())
}

/// Decode a finite binary64 value exactly under the caller's rational ledger.
fn represented(
    value: nrz_scalar::Finite<f64>,
    b: &mut Budget,
) -> Result<Rational, PreparedConstantError> {
    let bits = value.get().to_bits();
    let field = (bits >> 52) & 2047;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if field == 0 {
        (fraction, -1074)
    } else {
        (fraction + (1_u64 << 52), field.cast_signed() - 1075)
    };
    let sign = if bits >> 63 == 0 { 1 } else { -1 };
    Ok(Rational::from_ratio(
        sign * i64::try_from(significand)
            .map_err(|_| Fault::capacity(BudgetKind::ComponentBits, AT))?,
        1,
        b,
    )?
    .mul(&Rational::from_ratio(2, 1, b)?.pow(exponent, b)?, b)?)
}

#[cfg(test)]
mod tests;
