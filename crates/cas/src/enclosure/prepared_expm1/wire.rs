// SPDX-License-Identifier: AGPL-3.0-only
//! Closed bounded portable framing; statement identity is distinct from witness identity.

use super::{AT, Coefficient, Proof, Stage, StageKind, invalid};
use crate::budget::{Budget, BudgetKind};
use crate::encoding::Encoder;
use crate::wire_read::Reader;
use crate::{Fault, InvalidCode};

impl Proof {
    /// Export untrusted evidence in the sole admitted version.
    /// # Errors
    /// Refuses unknown framing or exhausted byte, arithmetic and allocation ceilings.
    pub fn to_bytes(&self, budget: &mut Budget) -> Result<Vec<u8>, Fault> {
        if self.version != 1 {
            return Err(invalid(InvalidCode::UnsupportedVersion));
        }
        budget.check(BudgetKind::ProofNodes, self.stages.len() as u64, AT)?;
        let mut out = Encoder::witness(budget, AT);
        out.raw(b"NRZXM1P1")?;
        out.unsigned(u64::from(self.version))?;
        out.count(self.scope.len())?;
        out.raw(&self.scope)?;
        out.unsigned(u64::from(self.degree))?;
        out.unsigned(u64::from(self.coefficient_bits))?;
        out.count(self.coefficients.len())?;
        for coefficient in &self.coefficients {
            encode_coefficient(coefficient, &mut out)?;
        }
        out.count(self.stages.len())?;
        for stage in &self.stages {
            out.byte(match stage.kind {
                StageKind::Multiply => 0,
                StageKind::Add => 1,
                StageKind::FinalMultiply => 2,
            })?;
            out.number(&stage.magnitude)?;
            out.number(&stage.error)?;
        }
        out.unsigned(u64::from(self.radius_bits))?;
        for value in [
            &self.absolute_tail,
            &self.relative_tail,
            &self.import_ratio,
            &self.central_ratio,
            &self.endpoint_ratio,
            &self.restoration_error,
        ] {
            out.number(value)?;
        }
        Ok(out.finish())
    }

    /// Decode bounded, canonical proposals without granting proof authority.
    /// # Errors
    /// Refuses malformed or noncanonical bytes, unknown tags and exhausted resources.
    pub fn from_bytes(bytes: &[u8], budget: &mut Budget) -> Result<Self, Fault> {
        let mut reader = Reader::new(bytes, budget)?;
        if reader.take(8)? != b"NRZXM1P1" {
            return Err(invalid(InvalidCode::Encoding));
        }
        let version = reader.small()?;
        if version != 1 {
            return Err(invalid(InvalidCode::UnsupportedVersion));
        }
        let count = reader.count(BudgetKind::WitnessBytes, 1, 1)?;
        let scope = reader.take(count)?.to_vec();
        let degree = reader.small()?;
        let coefficient_bits = reader.small()?;
        let count = reader.count(BudgetKind::ProofNodes, 16, 32)?;
        let mut coefficients = Vec::new();
        coefficients
            .try_reserve_exact(count)
            .map_err(|_| Fault::allocation(AT))?;
        for _ in 0..count {
            let significand = reader.unsigned()?;
            let offset = reader.small()?;
            if offset > 1074 {
                return Err(invalid(InvalidCode::Encoding));
            }
            coefficients.push(Coefficient {
                significand,
                exponent: i32::try_from(offset).map_err(|_| invalid(InvalidCode::Encoding))? - 1074,
            });
        }
        let count = reader.count(BudgetKind::ProofNodes, 3, 512)?;
        let mut stages = Vec::new();
        stages
            .try_reserve_exact(count)
            .map_err(|_| Fault::allocation(AT))?;
        for _ in 0..count {
            let kind = match reader.byte()? {
                0 => StageKind::Multiply,
                1 => StageKind::Add,
                2 => StageKind::FinalMultiply,
                _ => return Err(invalid(InvalidCode::Encoding)),
            };
            stages.push(Stage {
                kind,
                magnitude: reader.rational()?,
                error: reader.rational()?,
            });
        }
        let radius_bits = reader.small()?;
        let result = Self {
            version,
            scope,
            degree,
            coefficient_bits,
            coefficients,
            stages,
            radius_bits,
            absolute_tail: reader.rational()?,
            relative_tail: reader.rational()?,
            import_ratio: reader.rational()?,
            central_ratio: reader.rational()?,
            endpoint_ratio: reader.rational()?,
            restoration_error: reader.rational()?,
        };
        reader.finished()?;
        Ok(result)
    }
}

/// Frame a dyadic without machine-dependent integer or floating encodings.
fn encode_coefficient(coefficient: &Coefficient, out: &mut Encoder<'_>) -> Result<(), Fault> {
    if !(-1074..=0).contains(&coefficient.exponent) {
        return Err(invalid(InvalidCode::Encoding));
    }
    out.unsigned(coefficient.significand)?;
    out.unsigned(
        u64::try_from(coefficient.exponent + 1074).map_err(|_| invalid(InvalidCode::Encoding))?,
    )
}

/// Mathematical program identity includes the complete finite coefficient specialization.
pub(super) fn statement(proof: &Proof, budget: &mut Budget) -> Result<Vec<u8>, Fault> {
    let mut out = Encoder::witness(budget, AT);
    out.byte(crate::graph::VertexKind::AnonymousLemma.tag())?;
    out.byte(5)?;
    out.unsigned(1)?;
    out.count(proof.scope.len())?;
    out.raw(&proof.scope)?;
    out.unsigned(u64::from(proof.degree))?;
    out.unsigned(u64::from(proof.coefficient_bits))?;
    for coefficient in &proof.coefficients {
        encode_coefficient(coefficient, &mut out)?;
    }
    out.unsigned(u64::from(proof.radius_bits))?;
    Ok(out.finish())
}

/// Prefix lookup does not require constructing coefficients or their proof first.
pub(super) fn prefix(scope: &[u8], budget: &mut Budget) -> Result<Vec<u8>, Fault> {
    let mut out = Encoder::witness(budget, AT);
    out.byte(crate::graph::VertexKind::AnonymousLemma.tag())?;
    out.byte(5)?;
    out.unsigned(1)?;
    out.count(scope.len())?;
    out.raw(scope)?;
    Ok(out.finish())
}

/// Full retained identity additionally distinguishes every proposed bound in the witness.
pub(super) fn retention(proof: &Proof, budget: &mut Budget) -> Result<Vec<u8>, Fault> {
    let statement = statement(proof, budget)?;
    budget.with_retained_payload(statement.len() as u64, AT, |budget| {
        let witness = proof.to_bytes(budget)?;
        budget.with_retained_payload(witness.len() as u64, AT, |budget| {
            let mut out = Encoder::witness(budget, AT);
            out.raw(&statement)?;
            out.count(witness.len())?;
            out.raw(&witness)?;
            Ok(out.finish())
        })
    })
}
