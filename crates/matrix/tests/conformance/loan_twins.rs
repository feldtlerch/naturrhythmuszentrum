// SPDX-License-Identifier: AGPL-3.0-only
//! Public ownership and lifetime boundaries have compile-success and refusal twins.
//!
//! Moving an owner transfers its sole capability.
//! ```rust
//! use nrz_matrix::OwnedLoan;
//! fn transfer(loan: OwnedLoan<f64>) -> OwnedLoan<f64> { loan }
//! let _transfer = transfer;
//! ```
//!
//! Writable capabilities cannot be cloned from a borrow.
//! ```compile_fail,E0308
//! use nrz_matrix::OwnedLoan;
//! fn duplicate(loan: &OwnedLoan<f64>) -> OwnedLoan<f64> { loan.clone() }
//! ```
//!
//! Tickets carry no constructor for private numerical ownership.
//! ```compile_fail,E0599
//! use nrz_matrix::{OwnedLoan, LoanTicket};
//! fn rebuild(ticket: LoanTicket) -> OwnedLoan<f64> { OwnedLoan::from_ticket(ticket) }
//! ```
//!
//! A borrowed reader cannot outlive its complete owning pin.
//! ```compile_fail,E0515
//! use nrz_matrix::{BundlePin, ExecutionAccount, LoanError, TileReader};
//! fn escape(pin: BundlePin<f64>, account: &mut ExecutionAccount) -> Result<TileReader<'static, f64>, LoanError> {
//!     pin.historical_reader(account)
//! }
//! ```
//!
//! Published tiles cannot create an unmetered use edge through Clone.
//! ```compile_fail,E0599
//! use nrz_matrix::PublishedTile;
//! fn duplicate(tile: PublishedTile<f64>) { let _copy = tile.clone(); }
//! ```
//!
//! A general reading adapter cannot substitute for the sealed independent symbols input.
//! ```compile_fail,E0308
//! use nrz_matrix::{HostTileDeclaration, ReadingBinding, CellAxis, Coverage, ExecutionAccount};
//! fn substitute(reading: ReadingBinding<f64>, axis: CellAxis, account: &mut ExecutionAccount) {
//!     let _result = HostTileDeclaration::<f64>::check(reading, axis, Coverage::RepresentedInputs, account);
//! }
//! ```
