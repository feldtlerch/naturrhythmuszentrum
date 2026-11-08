// SPDX-License-Identifier: AGPL-3.0-only
//! Physical requests are derived from the same immutable schedule that grants numerical permits.

use super::{Channel, Run};
use crate::OperationError;
use nrz_scalar::{Enclosure, Precision};

impl Run {
    /// Conditional output means require explicit means on every original input.
    pub(crate) fn has_means(&self) -> bool {
        self.0.bindings.iter().all(|binding| binding.means)
    }
    /// Include capture arrays, contraction rectangles and parameter copies before any are drawn.
    pub(crate) fn physical_bytes(&self) -> Result<[usize; 2], OperationError> {
        let mut bytes = 0_usize;
        for entry in &self.0.specifications {
            let width = match (entry.precision, entry.channel) {
                (Precision::F32, Channel::Truth) => size_of::<Enclosure<f32>>(),
                (Precision::F64, Channel::Truth) => size_of::<Enclosure<f64>>(),
                (Precision::F32, _) => size_of::<f32>(),
                (Precision::F64, _) => size_of::<f64>(),
            };
            let allocation = if entry.channel.backend() {
                let column = match entry.precision {
                    Precision::F32 => {
                        crate::heap::resources::slab_bytes::<f32>(entry.extent.rows())?
                    }
                    Precision::F64 => {
                        crate::heap::resources::slab_bytes::<f64>(entry.extent.rows())?
                    }
                };
                column.checked_mul(entry.extent.columns())
            } else {
                entry.extent.entries().checked_mul(width)
            }
            .ok_or(OperationError::ResourceLimit)?;
            bytes = bytes
                .checked_add(allocation)
                .ok_or(OperationError::ResourceLimit)?;
        }
        Ok([self.0.metadata, bytes])
    }
}
