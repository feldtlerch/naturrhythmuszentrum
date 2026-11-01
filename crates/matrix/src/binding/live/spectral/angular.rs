// SPDX-License-Identifier: AGPL-3.0-only
//! First-class angular frequency retains the native original-center derivative and one live bundle.

mod plan;
mod publication;
pub use plan::LcAngularFrequencyPublication;
pub use publication::PublishedLcAngularFrequency;
pub(super) use publication::pack;

use super::{AT, SpectralError as Error};
use nrz_scalar::{ArithmeticContext, Enclosure, Scalar};

/// One local statistical parameter with its numerical error, never a physical support radius.
#[derive(Clone, Copy, Debug)]
pub struct LcAngularFrequencyParameter<P: Scalar> {
    /// Finite candidate for the local derivative or covariance parameter.
    pub center: P,
    /// Outward numerical displacement from the exact local parameter.
    pub error: P,
}

/// Keep the existing outward parameter enclosure and explicitly record its finite candidate.
pub(super) fn parameter<P: Scalar>(
    value: Enclosure<P>,
    context: &ArithmeticContext,
) -> Result<LcAngularFrequencyParameter<P>, Error> {
    let center = value.lower();
    let error = value.displacement_from(center, context)?.get();
    Ok(LcAngularFrequencyParameter {
        center: center.get(),
        error,
    })
}

/// Heap-owned original output evidence survives dropping the public result wrapper.
pub(crate) struct Origin {
    /// Exact sealed physical recipe and ordered parameter declarations.
    definition: nrz_bindings::readings::LcAngularFrequencyDefinition,
    /// Shared immutable numerical proof, including original joint model, point and sensitivities.
    evidence: std::rc::Rc<dyn std::any::Any>,
}

impl std::fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LcAngularFrequencyOrigin")
            .field("definition", &self.definition)
            .field("evidence_type", &self.evidence.as_ref().type_id())
            .finish_non_exhaustive()
    }
}

impl Origin {
    /// Bind historical exact meaning to one actual native numerical proof allocation.
    fn new<P: Scalar + 'static>(
        definition: nrz_bindings::readings::LcAngularFrequencyDefinition,
        evidence: &std::rc::Rc<Evidence<P>>,
    ) -> Self {
        Self {
            definition,
            evidence: std::rc::Rc::clone(evidence) as std::rc::Rc<dyn std::any::Any>,
        }
    }
    /// The original exact output meaning is available independently of mutable numerical values.
    pub(in crate::binding::live) const fn definition(
        &self,
    ) -> &nrz_bindings::readings::LcAngularFrequencyDefinition {
        &self.definition
    }
    /// The published wrapper must retain the same numerical proof as its heap-owned origin.
    fn owns<P: Scalar + 'static>(&self, evidence: &std::rc::Rc<Evidence<P>>) -> bool {
        let erased = std::rc::Rc::clone(evidence) as std::rc::Rc<dyn std::any::Any>;
        std::rc::Rc::ptr_eq(&self.evidence, &erased)
    }
}

/// Typed immutable numerical provenance is shared by the heap group and returned result.
#[derive(Debug)]
pub(super) struct Evidence<P: Scalar> {
    /// Original evaluated positive-root theorem, source model and every input version.
    pub(super) evaluation: crate::LcSpectralEvaluation<P>,
    /// Original represented physical coordinates defining the local model.
    pub(super) point: [P; 2],
    /// Ordered physical source derivative candidates and numerical errors.
    pub(super) jacobian: [LcAngularFrequencyParameter<P>; 2],
}
