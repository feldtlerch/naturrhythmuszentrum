// SPDX-License-Identifier: AGPL-3.0-only
//! Factor construction admits its known owners beside the opaque LU storage allowance.

use super::SolveContract;
use crate::OperationError;
use nrz_scalar::Scalar;

impl SolveContract {
    /// Packing ends before the identity solve; its two rectangles coexist with the inverse copy.
    pub(crate) fn physical_factor<P: Scalar>(&self) -> Result<usize, OperationError> {
        let operator = self
            .inputs
            .first()
            .ok_or(OperationError::InputCount)?
            .shape()
            .extent();
        let rectangle = crate::heap::resources::slab_bytes::<P>(operator.rows())?
            .checked_mul(operator.columns())
            .ok_or(OperationError::ResourceLimit)?;
        let vector = operator
            .entries()
            .checked_mul(size_of::<P>())
            .ok_or(OperationError::ResourceLimit)?;
        let inverse_copy = rectangle
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(vector))
            .ok_or(OperationError::ResourceLimit)?;
        // The inverse candidate survives both backend rectangles and then coexists with defects.
        let verification = vector.checked_mul(2).ok_or(OperationError::ResourceLimit)?;
        Ok(rectangle.max(inverse_copy).max(verification))
    }
}
