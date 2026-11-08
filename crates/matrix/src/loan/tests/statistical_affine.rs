// SPDX-License-Identifier: AGPL-3.0-only
//! Carried statistics populate each distinct affine cache after deterministic local sharing.

use super::{Case, TestResult};
use crate::{ExecutionAccount, ExecutionLimits, HeapError, OperationDraft, OperationError};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Distinct cold, shared cold, one warm and two warm caches exercise actual owner identities.
fn fixture<P: Scalar + 'static>(state: usize) -> TestResult<Case<P>> {
    let mut case = Case::<P>::new(true)?;
    let declarations = crate::conversion_testing::owner()?;
    let shape = crate::conversion_testing::shape(&declarations, &Descriptor::ratio(), 4)?;
    let map = declarations.conversion_map(&Descriptor::ratio(), &Descriptor::ratio())?;
    let first = crate::conversion_testing::contract(shape.clone(), shape.clone(), map.clone())?;
    let second = if state == 1 {
        first.clone()
    } else {
        crate::conversion_testing::contract(shape.clone(), shape, map)?
    };
    if state >= 2 {
        drop(first.cache.borrow_mut().get::<P>(
            &first.map,
            first.literals,
            &ArithmeticContext::new()?,
        )?);
    }
    if state == 3 {
        drop(second.cache.borrow_mut().get::<P>(
            &second.map,
            second.literals,
            &ArithmeticContext::new()?,
        )?);
    }
    case.operation =
        OperationDraft::convert(vec![first, second]).check(ExecutionLimits::default())?;
    Ok(case)
}

/// Eighteen destination slabs, 66 metadata requests and each cold retained coefficient owner.
fn retained<P: Scalar>(state: usize) -> usize {
    let word = size_of::<usize>();
    let coefficient = 4 * size_of::<P>() + size_of::<u64>();
    let shared = (word + 2 * coefficient + size_of::<u32>()).div_ceil(word) * word + 2 * word;
    let cache_entry = 2 * word;
    let owners = match state {
        0 => 2,
        1 | 2 => 1,
        _ => 0,
    };
    let slabs = 6 * 64
        + 8 * (16 * size_of::<P>()).div_ceil(64) * 64
        + 4 * (32 * size_of::<P>()).div_ceil(64) * 64;
    crate::statistics::inventory::affine_fixture_metadata_bytes()
        + slabs
        + size_of::<crate::ExecutionId>()
        + 7 * word
        + owners * (shared + cache_entry)
}

/// Refusal precedes numerical preparation; success fills every required cache and publishes both outputs.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for state in 0..4 {
        let needed = retained::<P>(state);
        let mut cases = vec![(needed - 1, false), (needed, true)];
        if state == 0 {
            // The reviewed ceiling counted only one cold cache despite two statistical derivatives.
            cases.push((retained::<P>(1), false));
        }
        for (limit, succeeds) in cases {
            let mut case = fixture::<P>(state)?;
            let mut account = ExecutionAccount::new(Budget::new(Limits {
                output_bytes: u64::try_from(limit)?,
                ..Limits::default()
            })?)?;
            let injection =
                crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
            match case
                .binding()
                .validate_with_account(&mut case.heap, &mut account)
            {
                Ok(bound) => {
                    assert!(succeeds);
                    assert_eq!(bound.resource_permit.retained, needed);
                    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                    let outcome = bound.publish(candidate)?;
                    assert_eq!(outcome.output().len(), 2);
                    let crate::binding::NativeRecipe::Affine(contracts) = &case.operation.recipe
                    else {
                        return Err("affine recipe required".into());
                    };
                    for contract in contracts.iter() {
                        assert!(contract.cache.borrow().contains::<P>());
                    }
                }
                Err(error) => {
                    assert!(!succeeds, "{error:?}");
                    assert!(
                        matches!(error,
                            OperationError::Heap(HeapError::ResourceBytes { staged: false, used, limit: ceiling })
                            if used == u64::try_from(needed)? && ceiling == u64::try_from(limit)?
                        ),
                        "{error:?}; state {state}"
                    );
                    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                }
            }
            drop(injection);
        }
    }
    Ok(())
}

/// Deterministic requests with carried statistics retain both distinct caches and borrow warm owners.
/// ´test:unit:loan-statistical-affine-cache-retained-neighbours´
#[test]
fn loan_statistical_affine_cache_retained_neighbours() -> TestResult {
    assert_eq!(retained::<f32>(0), 9_440);
    assert_eq!(retained::<f64>(0), 10_528);
    boundaries::<f32>()?;
    boundaries::<f64>()
}
