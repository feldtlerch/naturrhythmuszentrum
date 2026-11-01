// SPDX-License-Identifier: AGPL-3.0-only
//! Both frequencies and their covariance share one original native evaluation and one live group.

mod plan;
mod publication;
mod values;
pub use plan::LcFrequencyPublication;
pub use publication::PublishedLcFrequency;

use super::{AT, SpectralError as Error, angular};
use crate::{LcAngularFrequencyParameter, PiConstant, SpectralValue};
use nrz_bindings::readings::LcFrequencyDefinition;
use nrz_scalar::Scalar;
use std::rc::Rc;

/// Heap-owned exact recipe and numerical proof survive dropping the public wrapper.
pub(crate) struct Origin {
    /// Complete checked declaration, including the ordered cross-covariance target.
    definition: LcFrequencyDefinition,
    /// Shared concrete proof with original model, point and certified coefficient owner.
    evidence: Rc<dyn SourceEvidence>,
}

impl std::fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LcFrequencyOrigin")
            .field("definition", &self.definition)
            .field("evidence_type", &self.evidence.as_ref().type_id())
            .finish_non_exhaustive()
    }
}

impl Origin {
    /// Bind the original exact definition to the actual retained numerical proof allocation.
    fn new<P: Scalar + 'static>(
        definition: LcFrequencyDefinition,
        evidence: &Rc<Evidence<P>>,
    ) -> Self {
        Self {
            definition,
            evidence: Rc::clone(evidence) as Rc<dyn SourceEvidence>,
        }
    }

    /// Historical exact meaning is available independently of current numerical validity.
    pub(in crate::binding::live) const fn definition(&self) -> &LcFrequencyDefinition {
        &self.definition
    }

    /// Raw reading access must also revalidate the original statistical and numerical evidence.
    pub(in crate::binding::live) fn validate(
        &self,
        heap: &crate::Heap,
    ) -> Result<(), crate::HeapError> {
        self.evidence.validate(heap)
    }

    /// A result wrapper may lend only the proof retained by its heap origin.
    fn owns<P: Scalar + 'static>(&self, evidence: &Rc<Evidence<P>>) -> bool {
        let erased = Rc::clone(evidence) as Rc<dyn SourceEvidence>;
        Rc::ptr_eq(&self.evidence, &erased)
    }
}

/// The connected producer adds only linear transport to the original angular proof.
#[derive(Debug)]
struct Evidence<P: Scalar> {
    /// Original evaluation, represented L/C point and exact-law angular Jacobian parameters.
    angular: angular::Evidence<P>,
    /// One finite candidate and both deterministic channels for each frequency, angular first.
    values: [SpectralValue<P>; 2],
    /// Cyclic partials in physical L/C order, each with its own numerical error.
    jacobian: [LcAngularFrequencyParameter<P>; 2],
    /// Angular variance, cyclic variance and their ordered cross covariance.
    covariance: [LcAngularFrequencyParameter<P>; 3],
    /// Exact-map-bound immutable certified affine coefficients.
    map: Rc<crate::coordinate_numeric::LoweredMap<P>>,
    /// Source-free pi owner retained independently of the caller's conversion cache.
    pi: PiConstant<P>,
}

/// Precision erasure retains validation of the original joint model and every selected source.
trait SourceEvidence: std::any::Any {
    /// A stale statistical target must retire raw output readings even when its base values survive.
    fn validate(&self, heap: &crate::Heap) -> Result<(), crate::HeapError>;
}

impl<P: Scalar + 'static> SourceEvidence for Evidence<P> {
    fn validate(&self, heap: &crate::Heap) -> Result<(), crate::HeapError> {
        let context =
            nrz_scalar::ArithmeticContext::new().map_err(|_| crate::HeapError::StaleEvidence)?;
        self.angular
            .evaluation
            .validate(heap, &context)
            .map_err(|_| crate::HeapError::StaleEvidence)
    }
}
