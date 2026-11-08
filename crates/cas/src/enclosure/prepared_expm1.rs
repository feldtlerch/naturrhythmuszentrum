// SPDX-License-Identifier: AGPL-3.0-only
//! Closed binary64 schedule authority; this module executes no native kernel.

mod check;
mod math;
mod produce;
mod service;
mod wire;

use super::EnclosureRequest;
use crate::budget::{Budget, BudgetKind};
use crate::{Fault, InvalidCode, Location, Rational};

/// Accounting boundary of the prepared exponential family.
const AT: Location = Location::new("prepared expm1");

/// Closed analytic and IEEE schedule law, separate from rational exponential enclosures.
pub const SCHEMA: crate::emptiness::LemmaSchema = crate::emptiness::LemmaSchema {
    name: "enclosure.expm1-horner",
    label: "thm:cas:enclosure-expm1-horner",
    version: 1,
    standing: crate::proof::Standing::Lemma(crate::proof::LemmaProvenance::CasChecked),
    statement: "Under the admitted binary64 environment, the checked separate multiply/add alternating Horner program and outward endpoint export enclose exp(x)-1 and satisfy the relative endpoint target. Zero is exact after validation.",
    premises: "Original exp(argument)-1 scope and ordered domain; alternating Taylor remainder and positive magnitude bound; independently checked coefficients, every rounded stage, gradual underflow, nearest input import, identity restoration and explicit binary64 endpoint export.",
};

/// The complete original claim, independent of a proposed schedule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    /// Original unsimplified exponential-minus-one expression, context and coordinate box.
    pub original: EnclosureRequest,
    /// Relative per-endpoint target exponent.
    pub bits: u32,
    /// Scalar arithmetic context revision; only revision one is admitted.
    pub environment: u32,
}

/// Finite preparation search and proof capacities, in addition to the shared budget.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// Maximum polynomial term count, at most 128 for this implementation.
    pub degree: u32,
    /// Maximum coefficient significand precision, between 2 and 53.
    pub coefficient_bits: u32,
    /// Maximum complete candidate attempts.
    pub attempts: u32,
    /// Maximum target exponent encoding.
    pub target_bits: u32,
    /// Work kept available for checking and publication, never a second ledger.
    pub checking_work: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            degree: 64,
            coefficient_bits: 53,
            attempts: 64,
            target_bits: 4096,
            checking_work: 0,
        }
    }
}

/// Exact, normalized positive dyadic coefficient, representable in binary64.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Coefficient {
    /// Integer significand in the declared coefficient precision.
    pub significand: u64,
    /// Power of two multiplying the significand.
    pub exponent: i32,
}

impl Coefficient {
    /// Reconstruct the exact coefficient without floating arithmetic.
    /// # Errors
    /// Refuses an unrepresentable coefficient or exhausted arithmetic resources.
    pub fn exact(&self, budget: &mut Budget) -> Result<Rational, Fault> {
        if self.significand == 0
            || self.significand > (1_u64 << 53)
            || !(-1074..=0).contains(&self.exponent)
        {
            return Err(invalid(InvalidCode::Expm1Coefficient));
        }
        Rational::from_ratio(
            i64::try_from(self.significand).map_err(|_| invalid(InvalidCode::Expm1Coefficient))?,
            1,
            budget,
        )?
        .mul(&math::two_power(i64::from(self.exponent), budget)?, budget)
    }
}

/// One explicitly rounded operation; fusion is not an admitted interpretation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageKind {
    /// Multiply the current accumulator by the negative imported magnitude.
    Multiply,
    /// Add the next positive imported coefficient.
    Add,
    /// Final multiplication by the negative imported magnitude.
    FinalMultiply,
}

/// Exact finite bounds at one rounded stage over imported magnitudes in `[0,2]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stage {
    /// Required operation in execution order.
    pub kind: StageKind,
    /// Bound for the corresponding ideal polynomial magnitude.
    pub magnitude: Rational,
    /// Bound for its native-versus-ideal absolute error.
    pub error: Rational,
}

/// Portable untrusted evidence; decoding alone grants no authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proof {
    /// Closed schema version.
    pub version: u32,
    /// Complete canonical original claim and arithmetic model.
    pub scope: Vec<u8>,
    /// Term count N, at least one.
    pub degree: u32,
    /// Coefficient significand precision selected for this candidate.
    pub coefficient_bits: u32,
    /// Coefficients in increasing factorial order, beginning with one.
    pub coefficients: Vec<Coefficient>,
    /// Separate operations in actual Horner execution order.
    pub stages: Vec<Stage>,
    /// Uniform absolute analytic tail at magnitude two.
    pub absolute_tail: Rational,
    /// Uniform relative analytic tail, using the a/3 lower magnitude bound.
    pub relative_tail: Rational,
    /// Input-import error divided by the original nonzero magnitude.
    pub import_ratio: Rational,
    /// Nominal output error divided by the original nonzero magnitude.
    pub central_ratio: Rational,
    /// Radius multiplier is exactly two to minus this exponent.
    pub radius_bits: u32,
    /// Final relative per-endpoint bound, including explicit endpoint export.
    pub endpoint_ratio: Rational,
    /// Identity output restoration has exactly zero error.
    pub restoration_error: Rational,
}

/// A checked, graph-retained theorem; current authority is authenticated by its owner.
#[derive(Clone, Debug)]
pub struct Prepared {
    /// Complete original claim in the current store.
    request: Request,
    /// Independently checked witness.
    proof: Proof,
    /// Program statement identity, excluding error-bound witness choices.
    statement: Vec<u8>,
}

impl Prepared {
    /// Borrow the original claim.
    #[must_use]
    pub const fn request(&self) -> &Request {
        &self.request
    }
    /// Borrow portable evidence; use receiving-store replay before semantic reuse.
    #[must_use]
    pub const fn proof(&self) -> &Proof {
        &self.proof
    }
    /// Borrow the complete content-keyed program statement.
    #[must_use]
    pub fn statement_key(&self) -> &[u8] {
        &self.statement
    }
}

/// Construction attempts remain visible even if arithmetic or publication refuses.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    /// Public preparation invocations.
    pub preparations: u64,
    /// Canonical retained-lookup attempts.
    pub lookups: u64,
    /// Candidate proof constructions.
    pub proposals: u64,
    /// Coefficient constructions, including discarded candidates.
    pub coefficients: u64,
    /// Independent checker invocations, including rejected proofs.
    pub checks: u64,
    /// Admitted caller proof records visited by the prepaid inventory walk.
    pub proof_records: u64,
    /// Admitted caller coordinate records visited by the prepaid inventory walk.
    pub coordinates: u64,
    /// Graph publication attempts, including refused capacity checks.
    pub publications: u64,
}

/// A bounded search failure is not resource exhaustion or mathematical impossibility.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// Graph-owned authority met the endpoint target.
    Ready(Box<Prepared>),
    /// No candidate in the declared finite search certified the target.
    TargetNotCertifiable,
}

/// Count before construction without wrapping or adding a Work tariff.
fn count(value: &mut u64, operation: &'static str) -> Result<(), Fault> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| Fault::capacity(BudgetKind::Work, AT))?;
    tracing::event!(name: "nrz.construct", target: "nrz::work", tracing::Level::TRACE,
        family = "prepared-expm1", revision = 1_u64, operation, count = 1_u64);
    Ok(())
}

/// Family-specific typed refusal with the original engine Fault representation.
const fn invalid(code: InvalidCode) -> Fault {
    Fault::invalid(code, AT)
}

/// Conservative owned bytes, checked before copying a proposed or retained witness.
pub(crate) fn footprint(value: &Prepared) -> u64 {
    proof_bytes(&value.proof)
        .saturating_add(super::request_bytes(&value.request.original))
        .saturating_add(value.statement.len() as u64)
        .saturating_add(256)
}

/// Logical allocation bound includes all exact components, not only wire bytes.
fn proof_bytes(proof: &Proof) -> u64 {
    let fixed = (proof.scope.len() as u64)
        .saturating_add((proof.coefficients.len() as u64).saturating_mul(32))
        .saturating_add(512);
    let bounds = [
        &proof.absolute_tail,
        &proof.relative_tail,
        &proof.import_ratio,
        &proof.central_ratio,
        &proof.endpoint_ratio,
        &proof.restoration_error,
    ];
    proof.stages.iter().fold(
        bounds
            .into_iter()
            .fold(fixed, |s, n| s.saturating_add(super::number_bytes(n))),
        |s, stage| {
            s.saturating_add(super::number_bytes(&stage.magnitude))
                .saturating_add(super::number_bytes(&stage.error))
        },
    )
}

/// Bound all retained rational inventories and fixed records before their allocation.
fn workspace_bytes(degree: u32, budget: &Budget) -> u64 {
    let number = budget
        .limits()
        .component_bits
        .div_ceil(8)
        .saturating_mul(8)
        .saturating_add(128);
    u64::from(degree)
        .saturating_mul(6)
        .saturating_add(32)
        .saturating_mul(number)
        .saturating_add(u64::from(degree).saturating_mul(64))
        .saturating_add(1024)
}

/// A retained theorem's validity never waives a caller's rational component limit.
fn check_components(proof: &Proof, budget: &Budget) -> Result<(), Fault> {
    for value in [
        &proof.absolute_tail,
        &proof.relative_tail,
        &proof.import_ratio,
        &proof.central_ratio,
        &proof.endpoint_ratio,
        &proof.restoration_error,
    ] {
        value.check(budget, AT)?;
    }
    for stage in &proof.stages {
        stage.magnitude.check(budget, AT)?;
        stage.error.check(budget, AT)?;
    }
    Ok(())
}
