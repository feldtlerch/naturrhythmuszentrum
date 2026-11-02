// SPDX-License-Identifier: AGPL-3.0-only
//! One checked exact LC schema can be rebound without exporting its constants again.

use super::{AT, Heap, HeapError, LcSpectrum, SpectralError as Error, input, plan};
use crate::ReadingBinding;
use nrz_bindings::{
    Budget,
    readings::{
        LcSpectralLaw,
        algebra::{Cas, limits::BudgetKind},
    },
};
use nrz_scalar::{LiteralLimits, Scalar};
use std::rc::Rc;

/// An original checked spectral law, immutable literal policy and shared exact constants.
///
/// This schema has no numerical owner. Each binding authenticates its own original
/// L/C selections and every companion; equal numbers never identify physical sources.
#[derive(Clone, Debug)]
pub struct LcSpectralSchema(Rc<Data>);

/// The schema owns only its fixed record and three constant buffers.
#[derive(Debug)]
struct Data {
    /// Original complete-root theorem and exact physical references.
    law: LcSpectralLaw,
    /// Immutable exact exports in the existing upper/lower/time order.
    constants: [String; 3],
    /// Explicit scalar import policy shared by every instance.
    literals: LiteralLimits,
    /// Fixed record, reference counts and observed string capacities.
    bytes: u64,
}

impl LcSpectralSchema {
    /// Authenticate the original law and export its exact constants once.
    /// # Errors
    /// Refuses foreign exact authority, unsupported exports and exhausted resources.
    pub fn new(
        graph: &Cas,
        law: LcSpectralLaw,
        literals: LiteralLimits,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        graph.validate_reuse_key(law.operator().reuse_key(), budget)?;
        let constants = plan::export(&law, budget)?;
        let mut bytes = size_of::<Data>() + 2 * size_of::<usize>();
        for value in &constants {
            bytes = bytes
                .checked_add(value.capacity())
                .ok_or(HeapError::ExtentOverflow)?;
        }
        let bytes = u64::try_from(bytes).map_err(|_| HeapError::ExtentOverflow)?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(Self(Rc::new(Data {
            law,
            constants,
            literals,
            bytes,
        })))
    }

    /// Bind compatible original selections in this heap without rechecking the immutable theorem.
    /// # Errors
    /// Refuses substituted meanings/maps, stale or foreign inputs and exhausted resources.
    pub fn bind<P: Scalar + 'static>(
        &self,
        heap: &Heap,
        inputs: [ReadingBinding<P>; 2],
        budget: &mut Budget,
    ) -> Result<LcSpectrum<P>, Error> {
        let before = budget.usage();
        let bytes = u64::try_from(size_of::<plan::Plan<P>>() + 2 * size_of::<usize>())
            .map_err(|_| HeapError::ExtentOverflow)?;
        budget.with_retained_payload(self.0.bytes, AT, |budget| {
            input::check(&self.0.law, &inputs, budget)?;
            for source in &inputs {
                heap.validate_reading(source)?;
            }
            budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
            budget.check(BudgetKind::OutputBytes, bytes, AT)?;
            Ok::<_, Error>(())
        })?;
        Ok(LcSpectrum(Rc::new(plan::Plan {
            law: self.0.law.clone(),
            inputs,
            parameters: plan::Parameters::Shared(self.clone()),
            literals: self.0.literals,
            bytes,
            usage: [before, budget.usage()],
        })))
    }

    /// Original immutable complete-root authority.
    #[must_use]
    pub fn law(&self) -> &LcSpectralLaw {
        &self.0.law
    }

    /// New fixed record and constant capacities, excluding shared exact evidence and bindings.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }

    /// Share the exact exports rather than copying or numerically reinterpreting them.
    pub(super) fn constants(&self) -> &[String; 3] {
        &self.0.constants
    }

    /// Immutable import policy is part of the common numerical key.
    pub(super) fn literals(&self) -> LiteralLimits {
        self.0.literals
    }

    /// A retained sealed schema reference abbreviates its complete immutable content.
    pub(super) fn authenticates<P: Scalar>(&self, spectrum: &LcSpectrum<P>) -> bool {
        matches!(&spectrum.0.parameters, plan::Parameters::Shared(schema) if Rc::ptr_eq(&self.0, &schema.0))
    }
}
