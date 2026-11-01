// SPDX-License-Identifier: AGPL-3.0-only
//! The full-cycle relation uses the certified affine transaction without identifying kinds.

use super::{Quantity, QuantityContext, QuantityError, QuantityRole, charge};
use crate::{ConversionError, Heap, OperationDraft, OperationError, PiConstant};
use nrz_bindings::{Budget, Descriptor, LimitsOverride};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::marker::PhantomData;

impl QuantityContext {
    /// Convert cyclic and angular frequency through the explicit checked full-cycle relation.
    ///
    /// The source-free coefficient retains its certified numerical error. Existing
    /// deterministic channels and both covariance axes use that same coefficient;
    /// inherited local moments remain local and acquire no nonlinear mean. Generic
    /// `Self::convert` continues to refuse this kind crossing.
    /// # Errors
    /// Refuses wrong kinds/roles, stale bindings, unsupported factors, incompatible
    /// pi evidence and exhausted resources without publishing an output prefix.
    pub fn convert_frequency<P: Scalar + 'static, R: QuantityRole>(
        &self,
        heap: &mut Heap,
        input: &Quantity<P, R>,
        target: &Descriptor,
        pi: &PiConstant<P>,
        exact: &LimitsOverride,
        budget: &mut Budget,
    ) -> Result<Quantity<P, R>, QuantityError<P>> {
        charge(budget, 1)?;
        let map = self
            .declarations
            .frequency_map(input.descriptor(), target)
            .map_err(ConversionError::from)
            .map_err(OperationError::from)?;
        let needs_pi = map.scale().exact().as_rational().is_none()
            || map.shift().exact().as_rational().is_none();
        let contract = self.conversion_contract(heap, input, map, budget)?;
        if needs_pi {
            contract
                .certify_pi(pi, exact, &ArithmeticContext::new()?, budget)
                .map_err(OperationError::from)?;
        }
        let operation = OperationDraft::convert(vec![contract]).check(self.execution)?;
        let output = self.execute(heap, operation, vec![input.binding().clone()], budget)?;
        Ok(Quantity {
            binding: output.binding,
            counts: input.counts.clone(),
            role: PhantomData,
        })
    }
}
