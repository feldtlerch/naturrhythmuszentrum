// SPDX-License-Identifier: AGPL-3.0-only
//! Original statistical bindings cover captured inputs, generic kernels and standalone cross blocks.

mod backend;
mod identity;
mod metadata;
mod plan;
mod publication;
mod record;
mod resources;
mod run;
mod schedule;
mod stage;

pub(crate) use plan::{Permit, Plan};
pub use record::{StatisticsAllocation, StatisticsInventory};
pub(crate) use run::{Request, Run};
pub use stage::{StatisticsAllocationChannel, StatisticsAllocationRole};
pub(crate) use stage::{StatisticsAllocationChannel as Channel, StatisticsAllocationRole as Role};

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use tests::{
    addition_fixture_metadata_bytes, affine_fixture_metadata_bytes, copy_fixture_metadata_bytes,
    cross_fixture_metadata_bytes,
};
