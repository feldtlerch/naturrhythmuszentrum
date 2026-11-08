// SPDX-License-Identifier: AGPL-3.0-only
//! Fixed cleanup receipts follow the acquired prefix; Rust owners perform the actual destruction.

use super::{LoanError, registry::CleanupReceipt};

/// Allocation-free record of the constructed portion of a prepaid preparation.
#[derive(Debug)]
pub(crate) struct Preparation {
    /// One non-cloneable receipt, moved into the sole core on successful admission.
    receipt: Option<CleanupReceipt>,
    /// Successfully constructed allocations, excluding a refused allocation boundary.
    acquired: usize,
    /// Backing buffers transferred immediately to the heap, including retained vacant capacity.
    retained: usize,
    /// Private temporary vectors destroyed before successful admission returns.
    temporary: usize,
    /// Success transfers only the surviving owner entitlements.
    transferred: bool,
}
impl Preparation {
    /// The caller has charged the complete cleanup inventory before construction starts.
    pub(super) const fn new(receipt: CleanupReceipt) -> Self {
        Self {
            receipt: Some(receipt),
            acquired: 0,
            retained: 0,
            temporary: 0,
            transferred: false,
        }
    }
    /// Record a successful allocation before the next fallible construction.
    pub(crate) fn acquired(&mut self, temporary: bool) -> Result<(), LoanError> {
        let acquired = self
            .acquired
            .checked_add(1)
            .ok_or(LoanError::CountOverflow)?;
        if self
            .receipt
            .as_ref()
            .is_none_or(|receipt| acquired > receipt.allocations)
        {
            return Err(LoanError::WrongLoan);
        }
        self.acquired = acquired;
        self.temporary += usize::from(temporary);
        Ok(())
    }
    /// A newly installed table remains a heap owner on any subsequent failure.
    pub(crate) const fn retain_table(&mut self) {
        self.retained += 1;
        self.temporary -= 1;
    }
    /// Move the remaining entitlement into the core without copying a mutable allowance.
    pub(super) fn finish(mut self) -> Result<CleanupReceipt, LoanError> {
        let mut receipt = self.receipt.take().ok_or(LoanError::WrongLoan)?;
        receipt.allocations = self.acquired - self.retained - self.temporary;
        self.transferred = true;
        Ok(receipt)
    }
}
impl Drop for Preparation {
    fn drop(&mut self) {
        // Every acquired owner remains in a local Rust value or has moved to the heap/core.
        // This guard records precisely which prepaid destructions that lexical unwind performs.
        let destroyed = if self.transferred {
            self.temporary
        } else {
            self.acquired - self.retained
        };
        if let Some(receipt) = &mut self.receipt {
            receipt.allocations -= destroyed + self.retained;
        }
        #[cfg(test)]
        super::fault::closed_prefix(self.acquired, self.retained, destroyed);
    }
}
