// SPDX-License-Identifier: AGPL-3.0-only
//! Paired source maps extend the original LC derivative without creating generic moments.

mod admission;
mod numeric;

use super::{Heap, LcSpectralEvaluation, LcSpectrum, SpectralError as Error};
use crate::{CompanionRef, ModelId, ReadingBinding, ReciprocalDerivativeEvaluation};
use nrz_bindings::{
    Budget,
    readings::{
        LcRootJacobian, ReadingSourceMap,
        algebra::{Location, limits::BudgetUsage},
    },
};
use nrz_scalar::{ArithmeticContext, Enclosure, Scalar};
use std::rc::Rc;

/// Caller location for the separate mapped-source adapter.
const AT: Location = Location::new("LC mapped source composition");

/// The original physical LC plan and its explicitly bound original source ancestry.
///
/// Physical inputs still satisfy the native scalar identity-coordinate contract. Their
/// source ancestry may contain affine maps, copies and overlapping selections. This
/// adapter creates typed covariance parameters, not generic heap root moment companions.
#[derive(Clone, Debug)]
pub struct LcSourceBinding<P: Scalar>(Rc<Input<P>>);

/// Immutable source and selected-target closure, distinct from native two-column statistics.
#[derive(Debug)]
struct Input<P: Scalar> {
    /// Existing positive-family spectral plan or completed deterministic evaluation.
    spectrum: LcSpectrum<P>,
    /// Original operator partials at the exact selected represented operating point.
    derivative: ReciprocalDerivativeEvaluation<P>,
    /// Shared exact original root derivative and physical references.
    exact: LcRootJacobian,
    /// Both exact target maps over the retained original source order.
    map: ReadingSourceMap,
    /// Original source bindings with their complete selected version closures.
    sources: Vec<ReadingBinding<P>>,
    /// Native joint declaration supplying all required original covariance blocks.
    companion: CompanionRef<P>,
    /// Native model columns in the exact source declaration order.
    columns: Vec<usize>,
    /// Original model identity, independent of covariance-number equality.
    model: ModelId,
    /// Complete retained adapter allowance for nested numerical work.
    bytes: u64,
    /// Cumulative caller usage before and after this owner phase.
    usage: [BudgetUsage; 2],
}

impl<P: Scalar + 'static> LcSourceBinding<P> {
    /// Bind both exact LC source maps to one complete current native source declaration.
    /// # Errors
    /// Refuses substituted source/target owners, incomplete models, changed references,
    /// omitted derivative directions, unsupported physical input maps and caller exhaustion.
    pub fn new(
        heap: &Heap,
        spectrum: LcSpectrum<P>,
        derivative: ReciprocalDerivativeEvaluation<P>,
        map: ReadingSourceMap,
        sources: Vec<ReadingBinding<P>>,
        companion: CompanionRef<P>,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        let before = budget.usage();
        spectrum.validate(heap)?;
        admission::targets(&spectrum, &derivative, &map, budget)?;
        derivative.values(heap, &ArithmeticContext::new()?, budget)?;
        let exact = LcRootJacobian::new(
            spectrum.law().clone(),
            derivative.contract().exact().clone(),
            budget,
        )?;
        let columns = admission::sources(heap, &spectrum, &map, &sources, companion, budget)?;
        let model = heap.statistical(companion)?.model().id();
        let bytes = admission::storage(
            &spectrum,
            &derivative,
            &exact,
            &map,
            sources.capacity(),
            budget,
        )?;
        Ok(Self(Rc::new(Input {
            spectrum,
            derivative,
            exact,
            map,
            sources,
            companion,
            columns,
            model,
            bytes,
            usage: [before, budget.usage()],
        })))
    }
    /// Both maps and the original ordered source symbols, with no covariance-number inference.
    #[must_use]
    pub fn source_map(&self) -> &ReadingSourceMap {
        &self.0.map
    }
    /// The original native operator/root derivative owner, shared without another derivation.
    #[must_use]
    pub fn root_jacobian(&self) -> &LcRootJacobian {
        &self.0.exact
    }
    /// Native model columns in the exact bindings source order.
    #[must_use]
    pub fn source_columns(&self) -> &[usize] {
        &self.0.columns
    }
    /// Original native statistical model identity.
    #[must_use]
    pub fn model(&self) -> ModelId {
        self.0.model
    }
    /// Separate mapped-source owner allowance; existing native owners keep their pins.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
    /// Cumulative admission snapshots, retaining earlier consumed Work.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }
    /// Every original source and selected physical target remains part of the live closure.
    fn validate(&self, heap: &Heap) -> Result<(), Error> {
        self.0.spectrum.validate(heap)?;
        for source in &self.0.sources {
            heap.validate_reading(source)?;
        }
        if heap.statistical(self.0.companion)?.model().id() != self.0.model {
            return Err(Error::Identity);
        }
        Ok(())
    }
}

/// Complete typed operator/root covariance in the explicit original source order.
#[derive(Clone, Debug)]
pub struct LcSourceStatistics<P: Scalar> {
    /// Retained exact maps, original source owners and native model identity.
    binding: LcSourceBinding<P>,
    /// Existing positive-family spectral plan or completed deterministic evaluation.
    spectrum: LcSpectralEvaluation<P>,
    /// Shared complete covariance and source-sensitivity parameters.
    payload: Rc<numeric::Values<P>>,
    /// Arithmetic theorem revision of the enclosed result.
    revision: u32,
    /// Cumulative caller usage before and after this owner phase.
    usage: [BudgetUsage; 2],
}

impl<P: Scalar + 'static> LcSourceStatistics<P> {
    /// All 8-by-8 blocks in `(B00,B10,B01,B11,Re+,Im+,Re-,Im-)` order.
    /// # Errors
    /// Refuses stale original sources, physical targets, model parameters or arithmetic policy.
    pub fn covariance(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<&[[Enclosure<P>; 8]; 8], Error> {
        self.validate(heap, context)?;
        Ok(&self.payload.covariance)
    }
    /// Each original source column contains the eight signed output sensitivities.
    /// # Errors
    /// Refuses the same stale closure as the complete covariance access.
    pub fn sensitivities(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<&[[Enclosure<P>; 8]], Error> {
        self.validate(heap, context)?;
        Ok(&self.payload.sensitivities)
    }
    /// Retained source order, both target maps and native model-column correspondence.
    #[must_use]
    pub const fn binding(&self) -> &LcSourceBinding<P> {
        &self.binding
    }
    /// The complete existing deterministic LC evaluation, preserving full-box guards.
    #[must_use]
    pub const fn spectrum(&self) -> &LcSpectralEvaluation<P> {
        &self.spectrum
    }
    /// This covariance describes the local linear model, with no nonlinear mean or support claim.
    #[must_use]
    pub const fn assurance(&self) -> crate::MomentAssurance {
        crate::MomentAssurance::LinearizedMoments
    }
    /// Cumulative numerical snapshots under the caller ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
    /// Recheck every original source, selected target and numerical policy before access.
    fn validate(&self, heap: &Heap, context: &ArithmeticContext) -> Result<(), Error> {
        if self.revision != context.revision() {
            return Err(Error::Identity);
        }
        self.binding.validate(heap)?;
        self.spectrum.validate(heap, context)
    }
}

impl Heap {
    /// Compose existing native LC partials with both checked original-source maps and contract all blocks.
    /// # Errors
    /// Refuses stale identities, incomplete covariance, unsafe physical boxes and caller resource limits.
    pub fn evaluate_lc_source_statistics<P: Scalar + 'static>(
        &self,
        binding: &LcSourceBinding<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<LcSourceStatistics<P>, Error> {
        let before = budget.usage();
        binding.validate(self)?;
        let (spectrum, payload) = budget.with_retained_payload(binding.0.bytes, AT, |budget| {
            numeric::evaluate(self, &binding.0, context, budget)
        })?;
        Ok(LcSourceStatistics {
            binding: binding.clone(),
            spectrum,
            payload: Rc::new(payload),
            revision: context.revision(),
            usage: [before, budget.usage()],
        })
    }
}
