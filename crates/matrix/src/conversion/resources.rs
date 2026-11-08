// SPDX-License-Identifier: AGPL-3.0-only
//! Cold affine coefficients are retained by their checked definition's shared cache.

use super::{ConversionContract, ConversionError};
use crate::OperationError;
use nrz_scalar::Scalar;

/// Count every cache populated by either phase without lowering a numerical coefficient.
pub(crate) fn physical_bytes<P: Scalar>(
    contracts: &[ConversionContract],
    statistics: bool,
) -> Result<[usize; 2], OperationError> {
    let mut retained = 0_usize;
    let mut scratch = contracts
        .len()
        .checked_mul(size_of::<
            std::rc::Rc<crate::coordinate_numeric::LoweredMap<P>>,
        >())
        .ok_or(OperationError::ResourceLimit)?;
    for (slot, contract) in contracts.iter().enumerate() {
        let rational = contract.map.scale().exact().as_rational().is_some()
            && contract.map.shift().exact().as_rational().is_some();
        let shared = contracts[..slot].iter().any(|prior| {
            let same_cache = std::rc::Rc::ptr_eq(&prior.cache, &contract.cache);
            let local_share = !statistics && rational && prior.map == contract.map;
            same_cache || local_share
        });
        if shared {
            continue;
        }
        let cache = contract
            .cache
            .try_borrow()
            .map_err(|_| ConversionError::Contract)?;
        let bytes = cache.physical_growth::<P>()?;
        retained = retained
            .checked_add(bytes[0])
            .ok_or(OperationError::ResourceLimit)?;
        scratch = scratch
            .checked_add(bytes[1])
            .ok_or(OperationError::ResourceLimit)?;
    }
    // Cache owners belong to the checked definition outside the heap. This call admits their
    // creation; heap residence does not report them after execution.
    Ok([retained, scratch])
}
