// SPDX-License-Identifier: AGPL-3.0-only
//! The sealed independent declaration adapter admits only identity cell incidences.

use super::{ExecutionAccount, LoanError, charge};
use crate::{CellAxis, Coverage, Extent};
use nrz_bindings::BindingSymbols;
use nrz_scalar::Scalar;
use std::marker::PhantomData;

/// One of the fixed center, total and overlapping numerical-error channels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostChannel {
    /// Finite represented center.
    Center,
    /// Finite nonnegative total radius.
    Total,
    /// Finite nonnegative numerical radius already covered by the total.
    Error,
}
impl HostChannel {
    /// Fixed declaration order.
    pub(super) const ALL: [Self; 3] = [Self::Center, Self::Total, Self::Error];
    /// Inline channel-array position.
    pub(super) const fn index(self) -> usize {
        match self {
            Self::Center => 0,
            Self::Total => 1,
            Self::Error => 2,
        }
    }
}

/// Checked independent host meaning and immutable identity component incidence.
#[derive(Debug)]
pub struct HostTileDeclaration<P: Scalar> {
    /// Exact original binding witnesses and source relations.
    pub(super) symbols: BindingSymbols,
    /// Original ordered cell association.
    pub(super) axis: CellAxis,
    /// Cells by components, in column order.
    pub(super) extent: Extent,
    /// Explicit represented or declared-bound assumption.
    pub(super) coverage: Coverage,
    /// Original ledger identity.
    pub(super) account: u64,
    /// Declaration and absence-scope identity.
    pub(super) identity: u64,
    /// Sealed precision.
    marker: PhantomData<P>,
}
impl<P: Scalar> HostTileDeclaration<P> {
    /// Check an existing independent witness before reserving any numerical owner.
    ///
    /// Only center, center/total and center/total/error identity representations
    /// are admitted; general reading adapters have no conversion into this type.
    /// # Errors
    /// Refuses derived symbols, unsupported companions, mismatched axes and exhausted Work.
    pub fn check(
        symbols: BindingSymbols,
        axis: CellAxis,
        coverage: Coverage,
        account: &mut ExecutionAccount,
    ) -> Result<Self, LoanError> {
        let extent =
            account
                .ledger
                .with_work_phase(super::FAMILY, "host declaration", |budget| {
                    charge(budget, 1)?;
                    symbols
                        .check_import()
                        .map_err(|_| LoanError::UnsupportedContract)?;
                    let [rows, columns] = symbols.value().shape();
                    let extent = Extent::new(
                        usize::try_from(rows).map_err(|_| LoanError::CountOverflow)?,
                        usize::try_from(columns).map_err(|_| LoanError::CountOverflow)?,
                    )?;
                    let unsupported = columns == 0
                        || extent.rows() != axis.capacity()
                        || (symbols.error().is_some() && symbols.bound().is_none())
                        || (symbols.bound().is_none() && coverage != Coverage::RepresentedInputs);
                    if unsupported {
                        return Err(LoanError::UnsupportedContract);
                    }
                    for symbol in [symbols.bound(), symbols.error()].into_iter().flatten() {
                        charge(budget, 1)?;
                        if symbol == symbols.value()
                            || symbol.target() != Some(symbols.value().node())
                            || symbol.shape() != symbols.value().shape()
                        {
                            return Err(LoanError::UnsupportedContract);
                        }
                    }
                    if symbols.bound().is_some() && symbols.bound() == symbols.error() {
                        return Err(LoanError::UnsupportedContract);
                    }
                    Ok(extent)
                })?;
        Ok(Self {
            symbols,
            axis,
            extent,
            coverage,
            account: account.identity,
            identity: crate::heap::identity::identity()?,
            marker: PhantomData,
        })
    }
    /// Declared logical cell capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.axis.capacity()
    }
    /// Components required in each present channel cell.
    #[must_use]
    pub const fn components(&self) -> usize {
        self.extent.columns()
    }
    /// Original exact binding witness, with no numerical retention edge.
    #[must_use]
    pub const fn symbols(&self) -> &BindingSymbols {
        &self.symbols
    }
    /// Caller-declared coverage assumption.
    #[must_use]
    pub const fn coverage(&self) -> Coverage {
        self.coverage
    }
    /// Channel presence is fixed by the admitted binding witness.
    pub(super) const fn present(&self) -> [bool; 3] {
        [
            true,
            self.symbols.bound().is_some(),
            self.symbols.error().is_some(),
        ]
    }
}
