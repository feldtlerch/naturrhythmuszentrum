// SPDX-License-Identifier: AGPL-3.0-only
//! Reserved regions and demand remain inaccessible until a future admitted LC origin law.

use super::LcOmegaOutput;
use crate::mask::storage::{LayoutRecord, MaskRecord};
use crate::{
    CancelOutcome, CancellationReceipt, ExecutionAccount, Heap, HostTileDeclaration, LcShapeAxes,
    LoanError, LoanRequest, LoanTicket, MaskVersion, OwnedLoan,
};
use nrz_scalar::Scalar;
use std::rc::Rc;

/// Demand membership is independent of initialized output coverage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcOutputCellState {
    /// Outside this application's explicit demand, not a numerical zero.
    NotRequested,
    /// Requested but no admitted producer has evaluated or published this cell.
    Pending,
    /// The requested producer was explicitly cancelled without a value.
    Cancelled,
}
/// Publication refusal never consumes or changes the rightful pending owner.
#[derive(Debug, thiserror::Error)]
pub enum LcOutputPublicationError {
    /// No complete-inventory LC origin law has been admitted for this pending output.
    #[error("LC output publication requires an admitted complete-inventory origin law")]
    OriginUnavailable,
    /// Foreign authority, terminal state or resource refusal.
    #[error("LC output publication: {0}")]
    Loan(#[from] LoanError),
}
/// Exclusive pending output; there is no host writer, raw-loan conversion or reader.
#[derive(Debug)]
pub struct LcOutputPending<'a, 'i, P: Scalar, const N: usize> {
    /// Original immutable output interpretation and complete input inventory.
    schema: &'a LcOmegaOutput<'i, P, N>,
    /// Exact original tuple roles, never reconstructed from capacity.
    axes: LcShapeAxes,
    /// Dynamic demand version remains protected until this continuation is released.
    demand: MaskVersion,
    /// Own both demand and layout edges without duplicating their native allocations.
    mask: Rc<MaskRecord>,
    /// Layout protection preserves the exact original correspondence of the demand.
    layout: Rc<LayoutRecord>,
    /// Sole private pending native owner; absent only after cancellation.
    owner: Option<OwnedLoan<P>>,
}
impl<'i, P: Scalar + 'static, const N: usize> LcOmegaOutput<'i, P, N> {
    /// Bind the predeclared output to a private region and its qualified pending mask.
    /// # Errors
    /// Refuses foreign accounts, changed axes, incompatible demand and reservation limits.
    pub fn reserve<'a>(
        &'a self,
        heap: &mut Heap,
        axes: &LcShapeAxes,
        demand: MaskVersion,
        account: &mut ExecutionAccount,
    ) -> Result<LcOutputPending<'a, 'i, P, N>, LoanError> {
        self.check_demand(heap, axes, demand, account)?;
        let declaration = HostTileDeclaration::check(
            self.symbols().clone(),
            axes.combined.axis(),
            crate::Coverage::RepresentedInputs,
            account,
        )?;
        // Clone only after the entire correspondence gate; both edge drops were prepaid there.
        let mask = Rc::clone(heap.masks.masks.get(demand.key)?.owner()?);
        let layout = Rc::clone(heap.masks.layouts.get(demand.layout.key)?.owner()?);
        let owner = match heap.request_loan(declaration, account) {
            LoanRequest::Granted(owner) => owner,
            LoanRequest::Refused(error) => return Err(error),
        };
        Ok(LcOutputPending {
            schema: self,
            axes: *axes,
            demand,
            mask,
            layout,
            owner: Some(owner),
        })
    }
    /// Immutable role and mask checks plus two retained edges and their drops precede allocation.
    pub(in crate::loan::lc_shape) fn check_demand(
        &self,
        heap: &Heap,
        axes: &LcShapeAxes,
        demand: MaskVersion,
        account: &mut ExecutionAccount,
    ) -> Result<(), LoanError> {
        let original = self
            .inventory
            .input(0)
            .ok_or(LoanError::UnsupportedContract)?;
        original.axes(axes, account)?;
        account
            .ledger
            .with_work_phase(crate::loan::FAMILY, "LC omega reservation", |budget| {
                crate::loan::charge(budget, 6)?;
                let mask = heap.mask(demand, demand.layout())?;
                if mask.axis() != axes.combined.axis()
                    || mask.gather().is_some()
                    || mask.components().extent()
                        != crate::Extent::new(axes.combined.axis().capacity(), 1)?
                {
                    return Err(LoanError::LayoutMismatch);
                }
                Ok(())
            })
    }
}
impl<P: Scalar + 'static, const N: usize> LcOutputPending<'_, '_, P, N> {
    /// Original requested output; no reconstructed descriptor can substitute for it.
    #[must_use]
    pub const fn schema(&self) -> &LcOmegaOutput<'_, P, N> {
        self.schema
    }
    /// Exact tuple association retained while the output is pending.
    #[must_use]
    pub const fn axes(&self) -> LcShapeAxes {
        self.axes
    }
    /// Demand is not a filled mask and grants no numerical access.
    #[must_use]
    pub const fn demand(&self) -> MaskVersion {
        self.demand
    }
    /// Diagnostic pending-region locator; it cannot reconstruct a writable owner.
    /// # Errors
    /// Refuses an already cancelled continuation.
    pub fn ticket(&self) -> Result<LoanTicket, LoanError> {
        self.owner
            .as_ref()
            .map(OwnedLoan::ticket)
            .ok_or(LoanError::WrongLoan)
    }
    /// Inspect pending demand without loading any numerical channel.
    /// # Errors
    /// Refuses a foreign ledger, out-of-capacity tuple or exhausted Work.
    pub fn cell_state(
        &self,
        cell: usize,
        account: &mut ExecutionAccount,
    ) -> Result<LcOutputCellState, LoanError> {
        account.authenticate(
            self.schema
                .inventory
                .input(0)
                .ok_or(LoanError::WrongLoan)?
                .account,
        )?;
        crate::loan::charge(&mut account.ledger, 1)?;
        if cell >= self.layout.axis.capacity() {
            return Err(LoanError::LayoutMismatch);
        }
        if !crate::loan::payload::bit(&self.mask.words, cell) {
            return Ok(LcOutputCellState::NotRequested);
        }
        Ok(if self.owner.is_some() {
            LcOutputCellState::Pending
        } else {
            LcOutputCellState::Cancelled
        })
    }
    /// Explicitly refuse numerical publication while preserving this exact reservation.
    /// # Errors
    /// Always reports unavailable origin after authenticating the pending owner.
    pub fn publish(
        &self,
        heap: &Heap,
        account: &mut ExecutionAccount,
    ) -> Result<(), LcOutputPublicationError> {
        heap.probe_loan(self.ticket()?, account)?;
        Err(LcOutputPublicationError::OriginUnavailable)
    }
    /// Cancel with the existing prepaid native entitlement; rejection restores the same owner.
    /// # Errors
    /// Refuses foreign or already cancelled authority without discarding the rightful owner.
    pub fn cancel(
        &mut self,
        heap: &mut Heap,
        account: &mut ExecutionAccount,
    ) -> Result<CancellationReceipt, LoanError> {
        let owner = self.owner.take().ok_or(LoanError::WrongLoan)?;
        match heap.cancel_loan(owner, account) {
            CancelOutcome::Cancelled(receipt) => Ok(receipt),
            CancelOutcome::Rejected(error, owner) => {
                self.owner = Some(owner);
                Err(error)
            }
        }
    }
}
