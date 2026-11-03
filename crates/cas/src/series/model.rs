// SPDX-License-Identifier: AGPL-3.0-only
//! Finite object computations retain their original operands and explicit precision.

use crate::graph::Handle;
use crate::number::Rational;

/// A finite computation in the increment ring, never an equality of source functions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeriesStatement {
    /// Restrict a complete object to the output's positive coefficient count.
    Truncate {
        /// Original object, including all coefficients beyond the requested order.
        input: Handle,
        /// Proposed restricted object.
        output: Handle,
    },
    /// Add two original objects at an explicitly recorded common order.
    Add {
        /// Original first object.
        left: Handle,
        /// Original second object.
        right: Handle,
        /// Proposed common-order sum.
        output: Handle,
        /// Positive coefficient count shared by the restricted inputs and output.
        order: u32,
        /// Require the recorded order to be the minimum available input order.
        default_order: bool,
    },
    /// Multiply every coefficient by an exact rational scalar.
    Scale {
        /// Original object.
        input: Handle,
        /// Exact scalar whose ordinary definedness proof occupies premise slot zero.
        scalar: Handle,
        /// Proposed object at the unchanged order.
        output: Handle,
    },
    /// Convolve two original objects at an explicitly recorded common order.
    Mul {
        /// Original first object.
        left: Handle,
        /// Original second object.
        right: Handle,
        /// Proposed common-order product.
        output: Handle,
        /// Positive coefficient count shared by the restricted inputs and output.
        order: u32,
        /// Require the recorded order to be the minimum available input order.
        default_order: bool,
    },
    /// Extract a coefficient without asserting a source value or derivative.
    Coefficient {
        /// Original object with its complete available order.
        input: Handle,
        /// Zero-based coefficient index, strictly below the object's order.
        index: u32,
        /// Proposed exact rational value in the object's interpretation.
        value: Handle,
    },
}

/// One independently checked coefficientwise binary arithmetic row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeriesBinaryRow {
    /// Output coefficient index.
    pub index: u32,
    /// Exact first operand, checked against the original object or scaling scalar.
    pub left: Rational,
    /// Exact second operand, checked against the original coefficient.
    pub right: Rational,
    /// Proposed exact sum or product.
    pub result: Rational,
}

/// One ordered convolution pair and the exact partial sum after that pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeriesProductRow {
    /// Output degree, reconstructed independently by the checker.
    pub degree: u32,
    /// First input index.
    pub left_index: u32,
    /// Second input index; the two indices must sum to the degree.
    pub right_index: u32,
    /// Exact first input coefficient.
    pub left: Rational,
    /// Exact second input coefficient.
    pub right: Rational,
    /// Proposed exact pair product.
    pub product: Rational,
    /// Proposed partial sum, starting from zero for each output degree.
    pub sum: Rational,
}

/// Untrusted finite evidence; only a registered checker can establish a judgment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeriesCertificate {
    /// Mode zero, comparing every retained coefficient.
    Truncate {
        /// Only object form zero is executable.
        form: u8,
        /// Ordered proposed coefficients of the restricted object.
        coefficients: Vec<Rational>,
    },
    /// Mode one, with one exact sum row per output coefficient.
    Add {
        /// Only object form zero is executable.
        form: u8,
        /// Recorded common coefficient count.
        order: u32,
        /// Whether the common order is the minimum input order.
        default_order: bool,
        /// Complete ascending coefficient rows.
        rows: Vec<SeriesBinaryRow>,
    },
    /// Mode two; premise zero is the ordinary checked scalar-definedness judgment.
    Scale {
        /// Only object form zero is executable.
        form: u8,
        /// Complete ascending scalar-product rows.
        rows: Vec<SeriesBinaryRow>,
    },
    /// Mode three, with every pair in increasing degree and first-index order.
    Mul {
        /// Only object form zero is executable.
        form: u8,
        /// Recorded common coefficient count.
        order: u32,
        /// Whether the common order is the minimum input order.
        default_order: bool,
        /// Complete finite convolution rows, including zero-valued products.
        rows: Vec<SeriesProductRow>,
    },
    /// Mode four, selecting exactly one available coefficient.
    Coefficient {
        /// Only object form zero is executable.
        form: u8,
        /// Original zero-based index.
        index: u32,
        /// Proposed exact coefficient value.
        value: Rational,
    },
}
