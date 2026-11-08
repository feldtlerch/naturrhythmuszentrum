// SPDX-License-Identifier: AGPL-3.0-only
//! Every solve admits its finite solve and optional contraction inventory before input capture.

use super::{Run, schedule, sensitivity, statistics};
use crate::{InputSnapshot, OperationError};
use nrz_scalar::{Enclosure, Scalar};

impl Run {
    /// Reuse the original finite request builders without constructing any numerical owner.
    pub(crate) fn physical_bytes<P: Scalar + 'static>(
        contract: &super::super::SolveContract,
        plan: &crate::statistics::Plan<P>,
        inputs: &[InputSnapshot<P>],
    ) -> Result<[usize; 2], OperationError> {
        let sensitivity = plan.derivative.is_some();
        let count = schedule::count(contract)?
            .checked_add(sensitivity::count(contract, sensitivity)?)
            .and_then(|count| count.checked_add(statistics::count(plan)))
            .ok_or(OperationError::ResourceLimit)?;
        let mut requests = crate::binding::buffer(count)?;
        schedule::specifications(contract, &mut requests)?;
        sensitivity::specifications(contract, sensitivity, &mut requests)?;
        statistics::specifications(contract, plan, &mut requests)?;
        let maps = inputs.iter().try_fold(0_usize, |sum, input| {
            sum.checked_add(input.layout().owned_bytes())
                .ok_or(OperationError::ResourceLimit)
        })?;
        let metadata = super::run::metadata(count, inputs.len(), maps)?
            .checked_add(statistics::Binding::predicted(plan)?)
            .ok_or(OperationError::ResourceLimit)?;
        let mut numerical = 0_usize;
        for request in requests {
            let bytes = if request.stage.is_backend() {
                crate::heap::resources::slab_bytes::<P>(request.extent.rows())?
                    .checked_mul(request.extent.columns())
            } else {
                let width = if request.stage.components() == 2 {
                    size_of::<Enclosure<P>>()
                } else {
                    size_of::<P>()
                };
                request.extent.entries().checked_mul(width)
            }
            .ok_or(OperationError::ResourceLimit)?;
            numerical = numerical
                .checked_add(bytes)
                .ok_or(OperationError::ResourceLimit)?;
        }
        // Reservations already charge the three ordinary output candidate vectors. All other
        // finite requests remain conservatively summed, including optional recovery suffixes.
        let candidate = contract
            .output
            .shape()
            .extent()
            .entries()
            .checked_mul(3 * size_of::<P>())
            .ok_or(OperationError::ResourceLimit)?;
        let numerical = numerical
            .checked_sub(candidate)
            .ok_or(OperationError::ResourceLimit)?;
        Ok([metadata, numerical])
    }
}

/// Independent fixture layout terms; counts are fixed by the test, never read from a solve plan.
#[cfg(test)]
pub(crate) const fn fixture_owner_sizes() -> [usize; 3] {
    [
        size_of::<super::plan::RcLayout>()
            + size_of::<super::SolveInventory>()
            + 2 * size_of::<usize>(),
        size_of::<super::plan::Specification>() + size_of::<super::SolveAllocation>(),
        size_of::<(crate::Dependency, nrz_bindings::BindingSymbols)>(),
    ]
}
