// SPDX-License-Identifier: AGPL-3.0-only
//! Original exact deterministic points produce fresh impedance groups after the retained invariant.

mod execution;
use crate::{CompanionAdapter, LcDrivenError as Error, LcSpectrum, ReadingCoverage, ReadingImport};
use nrz_bindings::{
    Budget, LcDrivenImpedance,
    readings::algebra::{Cas, limits::BudgetKind},
};
use nrz_scalar::{LiteralLimits, Scalar};
use std::rc::Rc;

/// Complete original deterministic driven source and fresh impedance output inventory.
#[derive(Clone, Debug)]
pub struct LcImpedancePublication<P: Scalar>(Rc<Plan<P>>);

/// Every original point and output declaration is fixed before numerical execution.
#[derive(Debug)]
struct Plan<P: Scalar> {
    /// Original bound autonomous plan, sharing exactly the same L/C readings.
    spectrum: LcSpectrum<P>,
    /// Explicit deterministic point, written expression, drive and current equation.
    definition: LcDrivenImpedance,
    /// Graph containing the complete fresh impedance and future current declarations.
    graph: Rc<Cas>,
    /// Original whole impedance block and all four component views.
    coverage: ReadingCoverage<usize>,
    /// Own-target total and numerical radius for each actual stored copy.
    companions: Vec<CompanionAdapter<usize>>,
    /// Bounded literal conversion policy, prepaid on the same execution ledger.
    literals: LiteralLimits,
    /// Scoped conservative metadata allowance; shared original owners remain separate.
    bytes: u64,
}

impl<P: Scalar> LcImpedancePublication<P> {
    /// Bind a complete fresh impedance output to the original L/C numerical plan.
    /// # Errors
    /// Refuses another original law, malformed output coverage or exhausted caller resources.
    pub fn new(
        spectrum: LcSpectrum<P>,
        definition: LcDrivenImpedance,
        graph: Rc<Cas>,
        coverage: ReadingCoverage<usize>,
        companions: Vec<CompanionAdapter<usize>>,
        literals: LiteralLimits,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        if !spectrum.law().shares_allocation(definition.law()) || spectrum.extension().is_some() {
            return Err(Error::Identity);
        }
        let output = definition.impedance().equation().impedance();
        crate::binding::live::phasor::publication::check::impedance(
            output,
            &coverage,
            &companions,
            budget,
        )?;
        let bytes = crate::binding::live::phasor::publication::check::storage(
            &coverage,
            &companions,
            0,
            budget,
        )?;
        budget.check(BudgetKind::OutputBytes, bytes, super::AT)?;
        Ok(Self(Rc::new(Plan {
            spectrum,
            definition,
            graph,
            coverage,
            companions,
            literals,
            bytes,
        })))
    }
    /// Original deterministic assignment and complete written expression remain inspectable.
    #[must_use]
    pub fn definition(&self) -> &LcDrivenImpedance {
        &self.0.definition
    }
    /// Exact original bound invariant owner required by this publication consumer.
    #[must_use]
    pub fn spectrum(&self) -> &LcSpectrum<P> {
        &self.0.spectrum
    }
}

/// Complete impedance and all stored-copy radii, including a successful exact zero block.
#[derive(Clone, Debug)]
pub struct PublishedLcImpedance<P: Scalar> {
    /// One atomic output group retains all original L/C source ancestors.
    readings: ReadingImport<P>,
    /// Original exact assignment and expression, shared with the heap's result origin.
    definition: LcDrivenImpedance,
    /// Actual cumulative caller snapshots around this point-varying consumer.
    usage: [nrz_bindings::readings::algebra::limits::BudgetUsage; 2],
}

impl<P: Scalar> PublishedLcImpedance<P> {
    /// Whole phasor and every component/companion become available together.
    #[must_use]
    pub const fn readings(&self) -> &ReadingImport<P> {
        &self.readings
    }
    /// Original model, deterministic source point, frequency and complete expression.
    #[must_use]
    pub const fn definition(&self) -> &LcDrivenImpedance {
        &self.definition
    }
    /// Point-varying work consumes the original caller ledger after the invariant result.
    #[must_use]
    pub const fn usage(&self) -> [nrz_bindings::readings::algebra::limits::BudgetUsage; 2] {
        self.usage
    }
}

/// Preserve original exact association failures in the existing continuation cause chain.
fn exact(error: nrz_bindings::DecisionError) -> Error {
    crate::LiveDecisionError::Admission(Box::new(error)).into()
}
