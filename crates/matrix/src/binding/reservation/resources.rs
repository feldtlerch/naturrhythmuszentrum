// SPDX-License-Identifier: AGPL-3.0-only
//! Ordinary reservations and their complete statistical replacements share one physical permit.

use super::{Reservations, specifications};
use crate::heap::resources::{ResourcePlan, add, slab_bytes};
use crate::{CheckedOperation, Heap, InputSnapshot, OperationError};
use nrz_scalar::Scalar;

impl<Q: Scalar + 'static> Reservations<Q> {
    /// Admit final channels and both simultaneous unpublished slab sets before allocating either.
    pub(crate) fn resource_plan<P: Scalar + 'static>(
        heap: &Heap,
        inputs: &[InputSnapshot<P>],
        operation: &CheckedOperation,
        parameters: impl Iterator<Item = usize> + Clone,
    ) -> Result<ResourcePlan, OperationError> {
        let specifications = specifications(inputs, operation)?;
        let lengths = specifications.iter().flat_map(|specification| {
            let count = 1
                + usize::from(specification.bounded)
                + usize::from(specification.evaluation)
                + 3 * usize::from(specification.contributions);
            std::iter::repeat_n(specification.extent.entries(), count)
        });
        let ordinary = lengths.clone().count();
        let parameter_count = parameters.clone().count();
        let count = add(ordinary, parameter_count)?;
        let mut plan =
            heap.storage_plan::<Q>(lengths.clone().chain(parameters), std::iter::empty())?;
        let (descriptors, replacement_peak) = heap
            .descriptors
            .slots
            .growth_bytes_sequence(ordinary, count)?;
        plan.retained = add(plan.retained, descriptors)?;
        plan.scratch = add(plan.scratch, replacement_peak)?;
        plan.scratch = add(
            plan.scratch,
            Self::payload_entries(inputs, operation)?
                .checked_mul(size_of::<Q>())
                .ok_or(OperationError::ResourceLimit)?,
        )?;
        // Zero-fill and candidate scalar backings have disjoint lifetimes and equal channel
        // lengths. Their outer owner backings are also disjoint, but have different layouts.
        let zero_headers = ordinary
            .checked_mul(size_of::<Vec<Q>>())
            .ok_or(OperationError::ResourceLimit)?;
        let candidate_headers = operation
            .outputs()
            .len()
            .checked_mul(size_of::<crate::publication::Payload<Q>>())
            .ok_or(OperationError::ResourceLimit)?;
        plan.scratch = add(plan.scratch, zero_headers.max(candidate_headers))?;
        plan.scratch = add(
            plan.scratch,
            count
                .checked_mul(size_of::<crate::heap::identity::Key>())
                .ok_or(OperationError::ResourceLimit)?,
        )?;
        let record_count = if parameter_count == 0 {
            count
        } else {
            add(ordinary, count)?
        };
        plan.scratch = add(
            plan.scratch,
            record_count
                .checked_mul(size_of::<crate::heap::records::Record>())
                .ok_or(OperationError::ResourceLimit)?,
        )?;
        if parameter_count != 0 {
            let direct = heap.storage.growth_bytes::<Q>(count, &std::iter::empty())?;
            let sequence = heap.storage.growth_bytes_sequence::<Q>(ordinary, count)?;
            plan.retained = add(
                plan.retained,
                sequence
                    .0
                    .checked_sub(direct.0)
                    .ok_or(OperationError::ResourceLimit)?,
            )?;
            plan.scratch = add(
                plan.scratch,
                sequence
                    .1
                    .checked_sub(
                        direct
                            .1
                            .checked_mul(2)
                            .ok_or(OperationError::ResourceLimit)?,
                    )
                    .ok_or(OperationError::ResourceLimit)?,
            )?;
            // The original reserved slabs and their owner vector remain alive until replacement
            // assignment ``rule:nrz:matrix-loans-legacy-preflight``.
            for length in lengths.clone() {
                plan.scratch = add(plan.scratch, slab_bytes::<Q>(length)?)?;
            }
            plan.scratch = add(
                plan.scratch,
                ordinary
                    .checked_mul(size_of::<faer::Mat<Q>>())
                    .ok_or(OperationError::ResourceLimit)?,
            )?;
            let keys = ordinary
                .checked_mul(2 * size_of::<crate::heap::identity::Key>())
                .ok_or(OperationError::ResourceLimit)?;
            plan.scratch = add(plan.scratch, keys)?;
        }
        Ok(plan)
    }
}
