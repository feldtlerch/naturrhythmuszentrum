// SPDX-License-Identifier: AGPL-3.0-only
//! Fixed LC factors retain the original checked pi region without a second pi evaluation.

use super::LcProductError;
use crate::{KernelError, LoanError, PiConstant};
use nrz_bindings::{Budget, readings::algebra::limits::BudgetKind};
use nrz_scalar::{ArithmeticContext, Enclosure, Finite, Scalar};

/// Local numerical factor storage; this is not an exact cache or a managed heap prelude.
#[derive(Debug)]
pub struct LcFrequencyPrelude<P: Scalar> {
    /// Original checked numerical pi and reciprocal allocation.
    pi: PiConstant<P>,
    /// Turn then inverse-turn center, total and complete constant error.
    factors: [[P; 3]; 2],
}
impl<P: Scalar + 'static> LcFrequencyPrelude<P> {
    /// Derive both factors once from the original checked pi and reciprocal.
    /// # Errors
    /// Refuses Work/storage limits, incompatible arithmetic or a nonfinite enclosure.
    pub fn new(
        pi: &PiConstant<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, LcProductError> {
        budget.with_work_phase(crate::loan::FAMILY, "LC frequency factors", |budget| {
            crate::loan::charge(budget, 5)?;
            for kind in [BudgetKind::StagedPayloadBytes, BudgetKind::OutputBytes] {
                budget
                    .check(
                        kind,
                        u64::try_from(size_of::<Self>()).map_err(|_| LoanError::ByteOverflow)?,
                        crate::loan::AT,
                    )
                    .map_err(LoanError::from)?;
            }
            if pi.arithmetic_revision() != context.revision() {
                return Err(LoanError::UnsupportedContract.into());
            }
            let turn = factor(pi.pi().enclosure(), P::from_u8(2), context)?;
            let inverse = factor(pi.reciprocal(), P::ONE / P::from_u8(2), context)?;
            Ok(Self {
                pi: pi.clone(),
                factors: [turn, inverse],
            })
        })
    }
    /// Original checked pi owner; clones share its allocation and theorem evidence.
    #[must_use]
    pub const fn pi(&self) -> &PiConstant<P> {
        &self.pi
    }
    /// Enclosed turn and inverse-turn factors, with complete constant discrepancy.
    #[must_use]
    pub const fn factors(&self) -> [[P; 3]; 2] {
        self.factors
    }
    /// Fixed local storage; the shared pi allocation remains separately owned and counted.
    #[must_use]
    pub const fn stored_bytes() -> usize {
        size_of::<Self>()
    }
}
/// Scale one original enclosure and enclose displacement from its lower endpoint candidate.
fn factor<P: Scalar>(
    input: Enclosure<P>,
    scale: P,
    context: &ArithmeticContext,
) -> Result<[P; 3], LcProductError> {
    let result = input
        .mul(
            Enclosure::singleton(Finite::new(scale).map_err(KernelError::from)?),
            context,
        )
        .map_err(KernelError::from)?;
    let center = result.lower();
    let radius = result
        .displacement_from(center, context)
        .map_err(KernelError::from)?
        .get();
    Ok([center.get(), radius, radius])
}
