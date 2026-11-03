// SPDX-License-Identifier: AGPL-3.0-only
//! Complete finite payloads, with bounded exact rows and distinct result tags.

use super::check::{AT, invalid, unsupported};
use super::{SeriesBinaryRow, SeriesCertificate, SeriesProductRow, SeriesStatement};
use crate::budget::{Budget, BudgetKind};
use crate::encoding::Encoder;
use crate::error::Fault;
use crate::graph::{Handle, size};
use crate::wire_read::Reader;

/// Complete local or portable statement payload, independent of its enclosing record.
pub(crate) fn statement(
    value: &SeriesStatement,
    out: &mut Encoder<'_>,
    write: &mut impl FnMut(&Handle, &mut Encoder<'_>) -> Result<(), Fault>,
) -> Result<(), Fault> {
    use SeriesStatement as S;
    out.byte(match value {
        S::Truncate { .. } => 0,
        S::Add { .. } => 1,
        S::Scale { .. } => 2,
        S::Mul { .. } => 3,
        S::Coefficient { .. } => 4,
    })?;
    out.byte(0)?; // Object form, never a source congruence.
    match value {
        S::Truncate { input, output } => {
            write(input, out)?;
            out.byte(0)?;
            write(output, out)?;
        }
        S::Add {
            left,
            right,
            output,
            order,
            default_order,
        }
        | S::Mul {
            left,
            right,
            output,
            order,
            default_order,
        } => {
            write(left, out)?;
            write(right, out)?;
            out.byte(0)?;
            write(output, out)?;
            crate::wire::index(*order, out)?;
            out.byte(u8::from(*default_order))?;
        }
        S::Scale {
            input,
            scalar,
            output,
        } => {
            write(input, out)?;
            write(scalar, out)?;
            out.byte(0)?;
            write(output, out)?;
        }
        S::Coefficient {
            input,
            index,
            value,
        } => {
            write(input, out)?;
            out.byte(1)?;
            write(value, out)?;
            crate::wire::index(*index, out)?;
        }
    }
    Ok(())
}

/// Read only supported forms; result kind is checked before reading a result reference.
pub(crate) fn read_statement(
    reader: &mut Reader<'_, '_>,
    records: &[crate::wire_decode::Decoded],
) -> Result<SeriesStatement, Fault> {
    use crate::wire_decode::object;
    let mode = reader.byte()?;
    if mode > 4 || reader.byte()? != 0 {
        return Err(unsupported());
    }
    let input = object(reader, records)?;
    let second = if matches!(mode, 1..=3) {
        Some(object(reader, records)?)
    } else {
        None
    };
    let tag = reader.byte()?;
    if tag > 1 {
        return Err(unsupported());
    }
    if tag != u8::from(mode == 4) {
        return Err(invalid());
    }
    let output = object(reader, records)?;
    Ok(match mode {
        0 => SeriesStatement::Truncate { input, output },
        1 | 3 => {
            let right = second.ok_or_else(invalid)?;
            let order = reader.index()?;
            let default_order = reader.boolean()?;
            if mode == 1 {
                SeriesStatement::Add {
                    left: input,
                    right,
                    output,
                    order,
                    default_order,
                }
            } else {
                SeriesStatement::Mul {
                    left: input,
                    right,
                    output,
                    order,
                    default_order,
                }
            }
        }
        2 => SeriesStatement::Scale {
            input,
            scalar: second.ok_or_else(invalid)?,
            output,
        },
        4 => SeriesStatement::Coefficient {
            input,
            index: reader.index()?,
            value: output,
        },
        _ => return Err(unsupported()),
    })
}

/// Every row operand is retained; no omitted zero or partial convolution has authority.
pub(crate) fn certificate(value: &SeriesCertificate, out: &mut Encoder<'_>) -> Result<(), Fault> {
    out.byte(value.mode())?;
    out.byte(value.form())?;
    match value {
        SeriesCertificate::Truncate { coefficients, .. } => {
            crate::split_wire::numbers(coefficients, out)?;
        }
        SeriesCertificate::Add {
            order,
            default_order,
            rows,
            ..
        } => {
            crate::wire::index(*order, out)?;
            out.byte(u8::from(*default_order))?;
            binary_rows(rows, out)?;
        }
        SeriesCertificate::Scale { rows, .. } => binary_rows(rows, out)?,
        SeriesCertificate::Mul {
            order,
            default_order,
            rows,
            ..
        } => {
            crate::wire::index(*order, out)?;
            out.byte(u8::from(*default_order))?;
            out.count(rows.len())?;
            for row in rows {
                crate::wire::index(row.degree, out)?;
                crate::wire::index(row.left_index, out)?;
                crate::wire::index(row.right_index, out)?;
                for value in [&row.left, &row.right, &row.product, &row.sum] {
                    out.number(value)?;
                }
            }
        }
        SeriesCertificate::Coefficient { index, value, .. } => {
            crate::wire::index(*index, out)?;
            out.number(value)?;
        }
    }
    Ok(())
}

/// Ordered coefficientwise rows share the exact same closed field sequence.
fn binary_rows(rows: &[SeriesBinaryRow], out: &mut Encoder<'_>) -> Result<(), Fault> {
    out.count(rows.len())?;
    for row in rows {
        crate::wire::index(row.index, out)?;
        for value in [&row.left, &row.right, &row.result] {
            out.number(value)?;
        }
    }
    Ok(())
}

/// Bound row counts before allocation using the existing cumulative coefficient dimension.
pub(crate) fn read_certificate(reader: &mut Reader<'_, '_>) -> Result<SeriesCertificate, Fault> {
    let mode = reader.byte()?;
    let form = reader.byte()?;
    if mode > 4 || form != 0 {
        return Err(unsupported());
    }
    Ok(match mode {
        0 => SeriesCertificate::Truncate {
            form,
            coefficients: read_coefficients(reader)?,
        },
        1 => {
            let order = reader.index()?;
            let default_order = reader.boolean()?;
            SeriesCertificate::Add {
                form,
                order,
                default_order,
                rows: read_binary(reader)?,
            }
        }
        2 => SeriesCertificate::Scale {
            form,
            rows: read_binary(reader)?,
        },
        3 => {
            let order = reader.index()?;
            let default_order = reader.boolean()?;
            let count = reader.count(BudgetKind::JetEntries, 48, 256)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(count)
                .map_err(|_| Fault::allocation(AT))?;
            for _ in 0..count {
                rows.push(SeriesProductRow {
                    degree: reader.index()?,
                    left_index: reader.index()?,
                    right_index: reader.index()?,
                    left: reader.rational()?,
                    right: reader.rational()?,
                    product: reader.rational()?,
                    sum: reader.rational()?,
                });
            }
            SeriesCertificate::Mul {
                form,
                order,
                default_order,
                rows,
            }
        }
        4 => SeriesCertificate::Coefficient {
            form,
            index: reader.index()?,
            value: reader.rational()?,
        },
        _ => return Err(unsupported()),
    })
}

/// Reserve all row containers before importing any variable-size rational components.
fn read_binary(reader: &mut Reader<'_, '_>) -> Result<Vec<SeriesBinaryRow>, Fault> {
    let count = reader.count(BudgetKind::JetEntries, 31, 192)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| Fault::allocation(AT))?;
    for _ in 0..count {
        rows.push(SeriesBinaryRow {
            index: reader.index()?,
            left: reader.rational()?,
            right: reader.rational()?,
            result: reader.rational()?,
        });
    }
    Ok(rows)
}

/// Meter the full owned row copy before generic proof admission clones it.
pub(crate) fn measure(value: &SeriesCertificate, budget: &mut Budget) -> Result<(), Fault> {
    let count = match value {
        SeriesCertificate::Truncate { coefficients, .. } => coefficients.len(),
        SeriesCertificate::Add { rows, .. } | SeriesCertificate::Scale { rows, .. } => rows.len(),
        SeriesCertificate::Mul { rows, .. } => rows.len(),
        SeriesCertificate::Coefficient { .. } => 1,
    };
    budget.check(BudgetKind::JetEntries, size(count, AT)?, AT)?;
    let mut out = Encoder::new(budget, AT);
    certificate(value, &mut out)?;
    let bytes = size(out.finish().len(), AT)?
        .checked_mul(3)
        .and_then(|n| n.checked_add(size(count, AT).ok()?.checked_mul(256)?))
        .ok_or_else(invalid)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    budget.work(bytes, AT)
}

/// Prepay the finite coefficient vector with the series entry dimension.
fn read_coefficients(reader: &mut Reader<'_, '_>) -> Result<Vec<crate::number::Rational>, Fault> {
    let count = reader.count(BudgetKind::JetEntries, 9, 96)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Fault::allocation(AT))?;
    for _ in 0..count {
        values.push(reader.rational()?);
    }
    Ok(values)
}
