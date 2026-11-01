// SPDX-License-Identifier: AGPL-3.0-only
//! Private publication evidence must agree with the exact closed quantity recipe.

use super::{ScalarQuantityOp, ScalarQuantityRecipe};
use crate::binding::NativeRecipe;
use crate::kernel_testing::{TestResult, axis, owner, shape};
use crate::kernels::contract::Recipe;
use crate::{
    Binding, CheckedOperation, ExecutionLimits, Extent, Heap, HeapLimits, OperationDraft,
    OperationError, RecipeEvidence, ShapeLimits,
};
use nrz_bindings::Descriptor;
use std::rc::Rc;

/// Both directions of an evidence substitution refuse before any visible publication.
/// ´test:unit:quantity-kernel-evidence-pairing´
/// ´claim:matrix:quantity-kernel-evidence-pairing´ Quantity and ordinary kernel evidence cannot substitute for each other; the matching recipes publish in fresh stores.
#[test]
fn quantity_kernel_evidence_pairing() -> TestResult {
    let quantity = operation(true)?;
    let ordinary = operation(false)?;
    assert!(quantity.carries_statistics());
    assert!(ordinary.carries_statistics());
    let evidence = run(&quantity, None)?.ok_or("quantity evidence")?;
    let RecipeEvidence::QuantityKernel(claim) = &evidence else {
        return Err("quantity claim".into());
    };
    run(
        &quantity,
        Some(RecipeEvidence::Kernel(claim.kernel.clone())),
    )?;
    run(&ordinary, Some(evidence))?;
    run(&ordinary, None)?;
    Ok(())
}

/// Construct a scalar contract through existing shape checks, then select the private quantity law.
fn operation(quantity: bool) -> TestResult<CheckedOperation> {
    let declarations = owner()?;
    let coordinate = axis(1)?;
    let contract = shape(&declarations, &coordinate, &coordinate)?;
    let mut operation = OperationDraft::add(contract.clone(), contract, ShapeLimits::default())?
        .check(ExecutionLimits::default())?;
    if quantity {
        let NativeRecipe::Kernel(contract) = &mut operation.recipe else {
            return Err("kernel recipe".into());
        };
        let recipe = ScalarQuantityRecipe {
            operation: ScalarQuantityOp::Product,
            allowance: 1_681,
        };
        let contract = Rc::make_mut(contract);
        contract.recipe = Recipe::Quantity(recipe);
        contract.work = recipe.work();
    }
    Ok(operation)
}

/// A refused candidate leaves the fresh heap's descriptors and provenance unchanged.
fn run(
    operation: &CheckedOperation,
    substitute: Option<RecipeEvidence>,
) -> TestResult<Option<RecipeEvidence>> {
    let mut heap = Heap::new(HeapLimits::default())?;
    let source = heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[2.0_f64])?;
    let before = heap.provenance_usage();
    let bound = Binding::<f64, f64>::new(
        operation.clone(),
        vec![source.value(), source.value()],
        ExecutionLimits::default(),
    )
    .validate(&mut heap)?;
    let mut candidate = bound.evaluate(bound.normalized_inputs()?)?;
    let evidence = candidate.evidence.first().ok_or("kernel evidence")?.clone();
    if let Some(substitute) = substitute {
        candidate.evidence[0] = substitute;
        let Err(fault) = bound.publish(candidate) else {
            panic!("mismatched quantity recipe evidence must refuse");
        };
        assert!(matches!(fault.cause(), OperationError::CandidateContract));
        assert_eq!(heap.provenance_usage(), before);
        assert_eq!(heap.descriptors.iter().count(), 1);
        Ok(None)
    } else {
        bound.publish(candidate)?;
        assert_eq!(heap.provenance_usage().records(), before.records() + 2);
        Ok(Some(evidence))
    }
}
