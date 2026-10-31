// SPDX-License-Identifier: AGPL-3.0-only
//! Per-execution scalar records belong to the ordinary immutable operation record.

use crate::{KernelEvidence, KernelKind};
use nrz_scalar::{KernelCertificate, KernelUsage};
use std::rc::Rc;

/// Executed scalar-quantity certificates, owned by the heap's operation provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantityKernelEvidence {
    /// Existing deterministic numerical facts without enlarging their inline representation.
    pub(crate) kernel: KernelEvidence,
    /// Complete executed input, arithmetic and radius-packing records.
    pub(super) certificates: Rc<[KernelCertificate]>,
    /// Actual monotone scalar charges, including every exported record.
    pub(super) usage: KernelUsage,
}

impl QuantityKernelEvidence {
    /// Closed law, scalar format, complete coverage and admitted execution allowance.
    #[must_use]
    pub const fn kernel(&self) -> &KernelEvidence {
        &self.kernel
    }

    /// Executed conditional full-box laws in their original order.
    #[must_use]
    pub fn certificates(&self) -> &[KernelCertificate] {
        &self.certificates
    }

    /// Actual scalar construction and export Work on the caller's monotone ledger.
    #[must_use]
    pub const fn usage(&self) -> KernelUsage {
        self.usage
    }

    /// Full boxed payload and shared certificate allocation, charged only to this new recipe.
    pub(crate) fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + 2 * size_of::<usize>()
            + self.certificates.len() * size_of::<KernelCertificate>()
    }

    /// Refuse evidence for a different closed scalar law before ordinary publication.
    pub(crate) fn matches(&self, recipe: super::ScalarQuantityRecipe) -> bool {
        let same_kind = self.kernel.kind() == recipe.kind();
        let complete_records = self.certificates.len() == recipe.records();
        same_kind && complete_records
    }
}

/// The closed operation, independent of physical kinds and native precision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ScalarQuantityOp {
    /// Full Cartesian multiplication.
    Product,
    /// Nonzero reciprocal followed by product.
    Quotient,
    /// Original exact signed integer exponent with strict zero-power definedness.
    IntegerPower(i64),
    /// Strictly positive square root with squared-endpoint certificates.
    SquareRoot,
}

impl ScalarQuantityOp {
    /// Public numerical identity used by the existing kernel evidence.
    pub(super) const fn kind(self) -> KernelKind {
        match self {
            Self::Product => KernelKind::ScalarQuantityProduct,
            Self::Quotient => KernelKind::ScalarQuantityQuotient,
            Self::IntegerPower(_) => KernelKind::ScalarQuantityIntegerPower,
            Self::SquareRoot => KernelKind::ScalarQuantitySquareRoot,
        }
    }
}
