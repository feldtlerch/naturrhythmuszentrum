// SPDX-License-Identifier: AGPL-3.0-only
//! A sealed original LC output joins the existing positive-rate law to an explicit radian relation.

mod check;
use super::{AT, LcSpectralLaw, ReadingError, SpectralLawError as Error};
use crate::readings::{CompanionReading, EntryMeaning, ParameterErrorReading, Representation};
use crate::{BinaryOp, Descriptor, Kind, Measure, Registry, Role};
use nrz_cas::{
    Cas, Rational,
    limits::{Budget, BudgetKind},
};
use std::rc::Rc;

/// Proposed complete symbols; only the checked definition authorizes their LC interpretation.
#[derive(Clone, Debug)]
pub struct LcAngularFrequencySymbols {
    /// Fresh coherent scalar angular-frequency output.
    pub value: Representation,
    /// Own-target total radius, numerical radius and self covariance, in that order.
    pub companions: [CompanionReading; 3],
    /// Numerical error of the covariance parameter, retaining its actual target.
    pub covariance_error: ParameterErrorReading,
    /// Identity reading of the fresh covariance-error source slot initialized by the producer.
    pub covariance_error_value: Representation,
}

/// Original positive LC law, explicit exact radian and complete predeclared output inventory.
///
/// This definition prescribes `omega = (1 rad) * r_plus`. Its derivative policy is the
/// exact physical law at the original represented L/C center, using `LcRootJacobian`.
/// It grants neither nonlinear moments nor numerical positivity of a runtime input box.
#[derive(Clone, Debug)]
pub struct LcAngularFrequencyDefinition(Rc<Definition>);

/// Shared original evidence and newly owned output interpretation.
#[derive(Debug)]
struct Definition {
    /// Original L/C declarations, positivity correspondence and complete rate theorem.
    law: LcSpectralLaw,
    /// Source-free exact angle difference, with its original reference identity.
    radian: EntryMeaning,
    /// Exact derivative-rate operand of the retained ordered kind relation.
    rate: Descriptor,
    /// Ordered value and parameter declarations checked before numerical execution.
    symbols: LcAngularFrequencySymbols,
    /// Conservative allowance for new metadata and exact representation witnesses.
    bytes: u64,
}

impl LcAngularFrequencyDefinition {
    /// Check one fresh output and its complete companion bundle against the original LC law.
    /// # Errors
    /// Refuses changed kinds, roles, scales, radian magnitude, contexts, targets, maps or limits.
    pub fn new(
        cas: &Cas,
        law: LcSpectralLaw,
        radian: EntryMeaning,
        symbols: LcAngularFrequencySymbols,
        registry: &Registry,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        let rate = relation(&radian, registry, budget)?;
        check::symbols(cas, &law, &symbols, budget)?;
        let bytes = check::storage(&symbols, budget)?;
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(Self(Rc::new(Definition {
            law,
            radian,
            rate,
            symbols,
            bytes,
        })))
    }
    /// Original complete rate law, including ordered L/C and positive-domain assumptions.
    #[must_use]
    pub fn law(&self) -> &LcSpectralLaw {
        &self.0.law
    }
    /// Exact source-free one-radian operand; it creates no random calibration source.
    #[must_use]
    pub fn radian(&self) -> &EntryMeaning {
        &self.0.radian
    }
    /// Exact derivative-rate meaning multiplied by the retained radian.
    #[must_use]
    pub fn rate_descriptor(&self) -> &Descriptor {
        &self.0.rate
    }
    /// Predeclared output and all companion identities in their fixed order.
    #[must_use]
    pub fn symbols(&self) -> &LcAngularFrequencySymbols {
        &self.0.symbols
    }
    /// Exact coherent angular-frequency kind, distinct from rate and cyclic frequency.
    #[must_use]
    pub fn descriptor(&self) -> &Descriptor {
        self.0.symbols.value.reading().entries()[0].descriptor()
    }
    /// New owned metadata allowance; the original physical theorem remains shared.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// The semantic multiplication rule and a unit coherent angle are both required.
fn relation(
    radian: &EntryMeaning,
    registry: &Registry,
    budget: &mut Budget,
) -> Result<Descriptor, Error> {
    budget.work(3, AT)?;
    let angle = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Difference)
        .map_err(ReadingError::from)?;
    if radian.descriptor() != &angle || radian.magnitude().as_rational() != Some(&Rational::one()) {
        return Err(ReadingError::Meaning.into());
    }
    let rate = Descriptor::new(
        Kind::rate().map_err(ReadingError::from)?,
        Measure::coherent(),
        Role::Difference,
    )
    .map_err(ReadingError::from)?;
    let output = registry
        .binary(BinaryOp::Multiply, &rate, radian.descriptor())
        .map_err(ReadingError::from)?;
    if output.kind() != &Kind::angular_frequency().map_err(ReadingError::from)? {
        return Err(ReadingError::Meaning.into());
    }
    Ok(rate)
}
