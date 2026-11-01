// SPDX-License-Identifier: AGPL-3.0-only
//! Existing original partials compose with both source maps before full correlated contraction.

use super::{AT, Error, Heap, Input, LcSourceStatistics, LcSpectralEvaluation};
use crate::{HeapError, JointLimits};
use nrz_bindings::{Budget, readings::algebra::limits::BudgetKind};
use nrz_scalar::{ArithmeticContext, Enclosure, Finite, LiteralLimits, NumericLiteral, Scalar};

/// One immutable owner for the full result and all original-source sensitivity columns.
#[derive(Debug)]
pub(super) struct Values<P: Scalar> {
    /// Complete signed operator/root parameter rectangle.
    pub(super) covariance: [[Enclosure<P>; 8]; 8],
    /// One eight-output sensitivity column per original source.
    pub(super) sensitivities: Vec<[Enclosure<P>; 8]>,
}

/// Existing positive-box spectral evaluation and implicit root differentiation are reused unchanged.
pub(super) fn evaluate<P: Scalar + 'static>(
    heap: &Heap,
    input: &Input<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<(LcSpectralEvaluation<P>, Values<P>), Error> {
    storage(input, budget)?;
    let spectrum = heap.evaluate_lc_spectrum(&input.spectrum, context, budget)?;
    let constants = super::super::numeric::constants(&input.spectrum, context, budget)?;
    let partials = super::super::statistics::numeric::partials(
        heap,
        &input.derivative,
        &constants,
        context,
        budget,
    )?;
    let original = input.derivative.values(heap, context, budget)?;
    if original.len() != 8 {
        return Err(Error::Identity);
    }
    let sensitivities = sensitivity(input, original, partials, context, budget)?;
    let covariance = contract(heap, input, &sensitivities, context, budget)?;
    Ok((
        spectrum,
        Values {
            covariance,
            sensitivities,
        },
    ))
}

/// Exact maps have two complete rows; no coefficient is inferred from evaluated covariance.
fn sensitivity<P: Scalar>(
    input: &Input<P>,
    original: &[Enclosure<P>],
    roots: [Enclosure<P>; 2],
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Vec<[Enclosure<P>; 8]>, Error> {
    let mut result = crate::binding::buffer(input.columns.len())?;
    for column in 0..input.columns.len() {
        let mut maps = [Enclosure::singleton(Finite::ZERO); 2];
        for (side, map) in maps.iter_mut().enumerate() {
            let text = input.map.maps()[side].coefficients()[column].exact_string(budget)?;
            let work = u64::try_from(text.len())
                .map_err(|_| HeapError::ExtentOverflow)?
                .checked_mul(12)
                .and_then(|work| work.checked_add(8))
                .ok_or(HeapError::ExtentOverflow)?;
            budget.work(work, AT)?;
            let policy = input.spectrum.0.literals;
            let limits = LiteralLimits::new(
                policy.max_digits(),
                policy.max_exponent(),
                policy.max_work().min(work),
            );
            *map = NumericLiteral::new(&text, limits)?
                .decode::<P>(context)?
                .enclosure();
        }
        budget.work(15, AT)?;
        let mut values = [Enclosure::singleton(Finite::ZERO); 8];
        for row in 0..4 {
            values[row] = original[row]
                .mul(maps[0], context)?
                .add(original[row + 4].mul(maps[1], context)?, context)?;
        }
        values[5] = roots[0]
            .mul(maps[0], context)?
            .add(roots[1].mul(maps[1], context)?, context)?;
        values[7] = values[5].negated();
        result.push(values);
    }
    Ok(result)
}

/// Each complete original covariance entry is enclosed once, then reused in all output blocks.
fn contract<P: Scalar + 'static>(
    heap: &Heap,
    input: &Input<P>,
    sensitivity: &[[Enclosure<P>; 8]],
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<[[Enclosure<P>; 8]; 8], Error> {
    let view = heap.statistical(input.companion)?;
    let count = input.columns.len();
    let entries = count.checked_mul(count).ok_or(HeapError::ExtentOverflow)?;
    let work = view
        .model()
        .representation()
        .terms()
        .checked_mul(3)
        .and_then(|work| work.checked_add(19))
        .and_then(|work| work.checked_mul(entries))
        .ok_or(HeapError::ExtentOverflow)?;
    budget.work(
        u64::try_from(work).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    let limits = JointLimits::new(count, entries, work);
    view.model()
        .require_blocks(&input.columns, &input.columns, limits)?;
    let mut source = crate::binding::buffer(entries)?;
    for &left in &input.columns {
        for &right in &input.columns {
            source.push(view.model().covariance(left, right, context, limits)?);
        }
    }
    let mut result = [[Enclosure::singleton(Finite::ZERO); 8]; 8];
    for (position, row) in [1, 2, 5].into_iter().enumerate() {
        for &column in &[1, 2, 5][position..] {
            let mut value = Enclosure::singleton(Finite::ZERO);
            for (left, first) in sensitivity.iter().enumerate() {
                for (right, second) in sensitivity.iter().enumerate() {
                    value = value.add(
                        first[row]
                            .mul(source[left * count + right], context)?
                            .mul(second[column], context)?,
                        context,
                    )?;
                }
            }
            result[row][column] = value;
            result[column][row] = value;
        }
    }
    for index in 0..7 {
        result[7][index] = result[5][index].negated();
        result[index][7] = result[index][5].negated();
    }
    result[7][7] = result[5][5];
    Ok(result)
}

/// Complete result, coefficient literals, source covariance and original derivative scratch coexist.
fn storage<P: Scalar>(input: &Input<P>, budget: &Budget) -> Result<(), Error> {
    let count = input.columns.len();
    let literal = input
        .map
        .maps()
        .iter()
        .flat_map(nrz_bindings::readings::algebra::readings::AffineMap::coefficients)
        .try_fold(0_u64, |largest, value| {
            let (numerator, denominator) = value.component_bits();
            let bytes = numerator
                .checked_add(denominator)
                .and_then(|bits| bits.checked_add(8))
                .and_then(|bits| bits.checked_mul(4))
                .ok_or(HeapError::ExtentOverflow)?;
            Ok::<_, HeapError>(largest.max(bytes))
        })?;
    let literal = usize::try_from(literal).map_err(|_| HeapError::ExtentOverflow)?;
    let entries = count.checked_mul(count).ok_or(HeapError::ExtentOverflow)?;
    let sensitivities = count
        .checked_mul(size_of::<[Enclosure<P>; 8]>())
        .ok_or(HeapError::ExtentOverflow)?;
    let output =
        size_of::<LcSourceStatistics<P>>() + size_of::<Values<P>>() + 2 * size_of::<usize>();
    let output = output
        .checked_add(sensitivities)
        .ok_or(HeapError::ExtentOverflow)?;
    let scratch = entries
        .checked_add(40)
        .and_then(|count| count.checked_mul(size_of::<Enclosure<P>>()))
        .ok_or(HeapError::ExtentOverflow)?;
    // Component-bit bounds cover exact decimal export and the temporary literal owner before allocation.
    let staged = output
        .checked_add(scratch)
        .and_then(|bytes| bytes.checked_add(literal))
        .ok_or(HeapError::ExtentOverflow)?;
    budget.check(
        BudgetKind::MatrixEntries,
        u64::try_from(entries).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    budget.check(
        BudgetKind::StagedPayloadBytes,
        u64::try_from(staged).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    budget.check(
        BudgetKind::OutputBytes,
        u64::try_from(output).map_err(|_| HeapError::ExtentOverflow)?,
        AT,
    )?;
    Ok(())
}
