// SPDX-License-Identifier: AGPL-3.0-only
//! Opaque mask and cell-layout occupations expose only checked immutable borrows.

use crate::{HeapError, Layout, heap::identity::Key};

/// A native ordered cell-axis association, independent of component addresses or physical meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellAxis {
    /// Owning heap, checked before interpreting an identity.
    pub(crate) scope: u64,
    /// Nonwrapping original axis identity.
    pub(crate) identity: u64,
    /// Logical cells, never the number of floating components.
    pub(crate) capacity: usize,
}

impl CellAxis {
    /// Number of logical cells in this ordered axis.
    #[must_use]
    pub const fn capacity(self) -> usize {
        self.capacity
    }
}

/// One immutable cell/component address map in the heap's ordinary generation discipline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellLayoutVersion {
    /// Original numerical owner.
    pub(crate) scope: u64,
    /// Layout table occupation; replacing a component stride advances its generation.
    pub(crate) key: Key,
}

/// Validated identity correspondence; each operation rechecks its layout occupation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellCorrespondence {
    /// The only layout whose bit positions may be combined through this correspondence.
    pub(crate) layout: CellLayoutVersion,
}

/// An immutable packed allocation occupation qualified by its cell-layout version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaskVersion {
    /// Exact heap owner, checked before indexing.
    pub(crate) scope: u64,
    /// Allocation occupation; retirement and reuse never revive a stale version.
    pub(crate) key: Key,
    /// Capacity, component strides and ordered axis belong to this layout version.
    pub(crate) layout: CellLayoutVersion,
}

impl MaskVersion {
    /// Cell-layout version captured at publication; access still requires heap resolution.
    #[must_use]
    pub const fn layout(self) -> CellLayoutVersion {
        self.layout
    }
}

/// Bounded native metadata mapping each compacted cell to one original selected cell.
///
/// This records mask movement only. It neither initializes float components nor pins
/// a numerical region; a value-moving consumer must validate and retain its own inputs.
#[derive(Debug)]
pub struct CellGather {
    /// Immediate input branch occupation whose selected cells moved.
    pub(crate) parent: MaskVersion,
    /// Original immutable branch occupation and layout association.
    pub(crate) source: MaskVersion,
    /// Original ordered axis, retained independently of future slot reuse.
    pub(crate) axis: CellAxis,
    /// Strictly increasing original cell indices, checked before publication.
    pub(crate) indices: Vec<usize>,
}

impl CellGather {
    /// Immediate source branch, distinct from the original source after nested compaction.
    #[must_use]
    pub const fn parent(&self) -> MaskVersion {
        self.parent
    }

    /// Original branch version; equal bits do not substitute a different occupation.
    #[must_use]
    pub const fn source(&self) -> MaskVersion {
        self.source
    }

    /// Original ordered cell/axis association.
    #[must_use]
    pub const fn original_axis(&self) -> CellAxis {
        self.axis
    }

    /// New-to-original cell indices in compacted order, without per-cell descriptors.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }
}

/// A checked borrow of packed words and the exact cell/component correspondence.
#[derive(Debug)]
pub struct MaskView<'a> {
    /// Actual immutable allocation.
    pub(crate) words: &'a [u64],
    /// Validated logical cell axis.
    pub(crate) axis: CellAxis,
    /// Row is the logical cell; column is the floating component.
    pub(crate) components: &'a Layout,
    /// Native gather edge when this allocation is a compacted branch.
    pub(crate) gather: Option<&'a CellGather>,
}

impl MaskView<'_> {
    /// Zero-padded words, bit `j % 64` of word `j / 64` for cell `j`.
    #[must_use]
    pub const fn words(&self) -> &[u64] {
        self.words
    }

    /// Exact ordered cell-axis association and capacity.
    #[must_use]
    pub const fn axis(&self) -> CellAxis {
        self.axis
    }

    /// Checked cell-row/component-column address map captured at publication.
    #[must_use]
    pub const fn components(&self) -> &Layout {
        self.components
    }

    /// Complete injective layout map, shared by every mask selecting that compacted layout.
    #[must_use]
    pub const fn gather(&self) -> Option<&CellGather> {
        self.gather
    }

    /// Read one in-capacity cell without lending mutable storage.
    /// # Errors
    /// Refuses a cell outside the logical capacity, including allocation padding.
    pub fn contains(&self, cell: usize) -> Result<bool, MaskError> {
        if cell >= self.axis.capacity {
            return Err(MaskError::CellOutOfBounds);
        }
        let word = self
            .words
            .get(cell / 64)
            .ok_or(MaskError::CellOutOfBounds)?;
        Ok(word & (1_u64 << (cell % 64)) != 0)
    }
}

/// Typed native mask refusal; no partial result is published.
#[derive(Clone, Debug, thiserror::Error)]
pub enum MaskError {
    /// Existing owner, generation, layout or allocator validation refused.
    #[error("mask heap: {0}")]
    Heap(#[from] HeapError),
    /// The caller's existing Work or byte allowance refused.
    #[error("mask accounting: {0}")]
    Budget(#[from] nrz_bindings::readings::algebra::Fault),
    /// Bit positions do not share the validated layout occupation.
    #[error("mask cell layouts do not correspond")]
    LayoutMismatch,
    /// General pullback and pushforward are outside the identity correspondence surface.
    #[error("non-identity mask correspondence is unsupported")]
    NonIdentityCorrespondence,
    /// A logical bit names no cell in the declared capacity.
    #[error("mask cell is outside the declared capacity")]
    CellOutOfBounds,
    /// Unused compacted capacity has no original cell association to select.
    #[error("mask cell has no original association in the compacted layout")]
    UnmappedCell,
    /// Native population, prefix or conversion cannot be represented.
    #[error("native mask count overflow")]
    CountOverflow,
    /// Payload or owner metadata arithmetic cannot be represented.
    #[error("mask byte reservation overflow")]
    ByteOverflow,
    /// A decision has overlapping outcomes.
    #[error("decision masks overlap")]
    PartitionOverlap,
    /// A decision omits participating demand or includes an unfilled cell.
    #[error("decision masks do not exactly cover participating demand")]
    PartitionCoverage,
    /// Pointwise demand requires at least one declared input.
    #[error("pointwise mask demand has no required inputs")]
    MissingInputs,
    /// Gather metadata exceeds its explicit native entry bound.
    #[error("mask gather requires {required} entries but the limit is {limit}")]
    MapCapacity {
        /// Required native entries.
        required: usize,
        /// Admitted native entries.
        limit: usize,
    },
    /// Compacted capacity cannot hold every selected original cell.
    #[error("compacted mask capacity is smaller than its population")]
    CompactionCapacity,
    /// Gather publication did not prove injectivity and complete mask movement.
    #[error("compacted mask movement is incomplete or not injective")]
    IncompleteMovement,
}
