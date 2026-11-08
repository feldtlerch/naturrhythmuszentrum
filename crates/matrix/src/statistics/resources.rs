// SPDX-License-Identifier: AGPL-3.0-only
//! The finite statistical schedule joins common physical admission before numerical capture.

use super::Plan;
use crate::{CheckedOperation, OperationError};
use nrz_scalar::Scalar;

impl<P: Scalar> Plan<P> {
    /// All possible publication channels, including conditional means, precede allocation.
    pub(crate) fn parameter_lengths<'a>(
        &self,
        operation: &'a CheckedOperation,
    ) -> Result<impl Iterator<Item = usize> + Clone + 'a, OperationError> {
        let inputs = operation.inputs().iter().try_fold(0_usize, |sum, input| {
            sum.checked_add(input.shape().extent().entries())
                .ok_or(OperationError::ResourceLimit)
        })?;
        let (outputs, columns, means) = if let Some(lineage) = &self.lineage {
            let run = self
                .inventory
                .as_ref()
                .ok_or(super::StatisticsError::Contract)?;
            (
                operation.outputs().len(),
                lineage.columns.len(),
                run.has_means(),
            )
        } else {
            (0, 0, false)
        };
        for output in operation.outputs().iter().take(outputs) {
            let rows = output.shape().extent().entries();
            for columns in [rows, columns, inputs] {
                rows.checked_mul(columns)
                    .ok_or(OperationError::ResourceLimit)?;
            }
        }
        Ok(operation
            .outputs()
            .iter()
            .take(outputs)
            .flat_map(move |output| {
                let rows = output.shape().extent().entries();
                [rows, columns, inputs]
                    .into_iter()
                    .flat_map(move |columns| std::iter::repeat_n(rows * columns, 2))
                    .chain(std::iter::repeat_n(rows, 2 * usize::from(means)))
            }))
    }
    /// Metadata survives publication; every finite numerical request and backend allowance
    /// contributes to a conservative simultaneous peak ``dec:nrz:matrix-loans-common-preflight``.
    pub(crate) fn physical_bytes<Q: Scalar>(
        &self,
        operation: &CheckedOperation,
    ) -> Result<[usize; 2], OperationError> {
        let Some(run) = &self.inventory else {
            return Ok([0, 0]);
        };
        let [metadata, numerical] = run.physical_bytes()?;
        let inputs = operation
            .inputs()
            .len()
            .checked_mul(size_of::<super::capture::Input<P>>() + size_of::<super::Matrix<P>>())
            .ok_or(OperationError::ResourceLimit)?;
        let outputs = operation
            .outputs()
            .len()
            .checked_mul(size_of::<super::MomentPayload<Q>>())
            .ok_or(OperationError::ResourceLimit)?;
        let regions = self
            .parameter_lengths(operation)?
            .count()
            .checked_mul(size_of::<super::allocation::RegionData<Q>>())
            .ok_or(OperationError::ResourceLimit)?;
        let scratch = self
            .work
            .checked_mul(size_of::<Q>())
            .and_then(|backend| backend.checked_add(numerical))
            .and_then(|bytes| bytes.checked_add(inputs))
            .and_then(|bytes| bytes.checked_add(outputs))
            .and_then(|bytes| bytes.checked_add(regions))
            .ok_or(OperationError::ResourceLimit)?;
        Ok([metadata, scratch])
    }
}

/// Reserve only the admitted numerical owner headers, with no geometric spare capacity.
pub(super) fn owner_buffer<T>(count: usize) -> Result<Vec<T>, OperationError> {
    let result = crate::binding::buffer(count)?;
    if result.capacity() != count {
        return Err(OperationError::ResourceLimit);
    }
    #[cfg(test)]
    crate::loan::fault::entered_statistical_headers(result.capacity() * size_of::<T>());
    Ok(result)
}

/// Independent layout terms for fixed-fixture controls; no resource plan is evaluated.
#[cfg(test)]
pub(crate) const fn fixture_header_sizes<P: Scalar>() -> [usize; 4] {
    [
        size_of::<super::capture::Input<P>>(),
        size_of::<super::Matrix<P>>(),
        size_of::<super::MomentPayload<P>>(),
        size_of::<super::allocation::RegionData<P>>(),
    ]
}
