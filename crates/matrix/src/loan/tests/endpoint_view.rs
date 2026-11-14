// SPDX-License-Identifier: AGPL-3.0-only
//! Independent dyadic endpoint and companion oracles cover native mapped readings.

mod authority;
mod boundaries;
mod mapped;
mod observation;
mod products;
mod ranges;
mod refusals;
use super::{TestResult, empty, fixture, granted};
use crate::loan::application::tests::binary::oracle::Rational;
use crate::{EndpointReading, HandbackOutcome, HostChannel, LoanError, NumericalErrorScope};
use nrz_bindings::Limits;
use nrz_scalar::Scalar;
use std::cmp::Ordering;

/// The oracle decodes native bits and performs unbounded integer fraction arithmetic.
fn oracle<P: Scalar>(reading: &EndpointReading<P>, input: [P; 3]) -> TestResult {
    let [center, radius, error] = input;
    let center = Rational::binary(center.to_f64())?;
    let radius = Rational::binary(radius.to_f64())?;
    let exact = [center.sub(&radius), center.add(&radius)];
    let endpoints = [reading.bounds().lower(), reading.bounds().upper()];
    let lower = Rational::binary(endpoints[0].get().to_f64())?;
    let upper = Rational::binary(endpoints[1].get().to_f64())?;
    assert_ne!(lower.compare(&exact[0]), Ordering::Greater);
    assert_ne!(upper.compare(&exact[1]), Ordering::Less);
    for (index, bound) in reading.conversion_error().into_iter().enumerate() {
        let native = Rational::binary(endpoints[index].get().to_f64())?;
        let discrepancy = if index == 0 {
            exact[index].sub(&native)
        } else {
            native.sub(&exact[index])
        };
        assert_ne!(
            discrepancy.compare(&Rational::binary(bound.get().to_f64())?),
            Ordering::Greater
        );
        // A conversion bound must stay within eight ulps of its finite endpoint.
        let value = endpoints[index].get();
        let ceiling = if value == -P::max_finite() {
            Rational::binary(value.next_up().to_f64())?
                .sub(&native)
                .image(
                    &Rational::fraction(8, 1)?,
                    crate::BinaryElementwise::Product,
                )?
        } else if value == P::max_finite() {
            native
                .sub(&Rational::binary(value.next_down().to_f64())?)
                .image(
                    &Rational::fraction(8, 1)?,
                    crate::BinaryElementwise::Product,
                )?
        } else {
            let up = Rational::binary(value.next_up().to_f64())?;
            let down = Rational::binary(value.next_down().to_f64())?;
            up.sub(&down).image(
                &Rational::fraction(4, 1)?,
                crate::BinaryElementwise::Product,
            )?
        };
        assert_ne!(
            Rational::binary(bound.get().to_f64())?.compare(&ceiling),
            Ordering::Greater
        );
    }
    assert_eq!(reading.input_error(), error);
    assert_eq!(
        reading.numerical_error_scope(),
        NumericalErrorScope::HostInput
    );
    Ok(())
}

/// Publish exact represented triples; errors stay distinct even when they are nonzero.
fn publish<P: Scalar + 'static>(
    values: &[[P; 3]],
) -> TestResult<(
    crate::Heap,
    crate::ExecutionAccount,
    crate::PublishedTile<P>,
    crate::MaskVersion,
)> {
    let (mut heap, mut account, declaration) =
        fixture::<P>(values.len(), 1, [true, true], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    for (cell, values) in values.iter().enumerate() {
        for (role, value) in HostChannel::ALL.into_iter().zip(values) {
            loan.write_component(role, cell, 0, *value, &mut account)?;
        }
    }
    match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, receipt) => {
            let mask = receipt
                .filled(HostChannel::Center)
                .ok_or("center mask")?
                .native();
            Ok((heap, account, tile, mask))
        }
        other => Err(format!("{other:?}").into()),
    }
}

/// Exact integers, cancellation, subnormals and extreme magnitudes keep rational coverage.
/// ´test:unit:loan-endpoint-view-rational-images´
#[test]
fn loan_endpoint_view_rational_images() -> TestResult {
    images::<f32>()?;
    images::<f64>()
}
fn images<P: Scalar + 'static>() -> TestResult {
    let tiny = P::ZERO.next_up();
    let large = P::parse_nearest("1e30").ok_or("large")?;
    let inputs = [
        [P::from_u8(5), P::from_u8(2), P::ONE],
        [P::from_u8(5), P::from_u8(3), P::from_u8(2)],
        [-P::ONE, P::ONE, P::ONE],
        [tiny, tiny, tiny],
        [large, P::ONE, tiny],
        [P::ZERO, P::ZERO, P::ZERO],
        [-P::ZERO, P::ZERO, P::ZERO],
        [P::ZERO, -P::ZERO, P::ZERO],
        [-P::ZERO, -P::ZERO, P::ZERO],
        [P::parse_nearest("0.1").ok_or("decimal")?, tiny, tiny],
    ];
    let (mut heap, mut account, tile, mask) = publish(&inputs)?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    for (cell, input) in inputs.into_iter().enumerate() {
        oracle(&reader.read_component(cell, 0, &mut account)?, input)?;
    }
    drop(tile);
    empty(&mut heap, &mut account)
}

/// Nested retained selections preserve sparse independent source tags and protected readers.
/// ´test:unit:loan-endpoint-view-gathered-lifecycle´
#[test]
fn loan_endpoint_view_gathered_lifecycle() -> TestResult {
    gathered::<f32>()?;
    gathered::<f64>()
}
fn gathered<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<P>(65, 2, [true, true], Limits::default())?;
    let (tile, mask) = super::owned_selection::publish(&mut heap, &mut account, declaration)?;
    let identity = heap.mask_correspondence(mask.layout(), mask.layout(), &mut account.ledger)?;
    let compact = heap.compact_mask(identity, mask, 4, 3, &mut account.ledger)?;
    let selected = heap.create_mask(compact.layout(), &[0, 2], &mut account.ledger)?;
    let owned = heap.retain_selected_tile(tile.pin(), selected, &mut account)?;
    drop(tile);
    assert!(matches!(
        heap.residence(&mut account, 100_000)?,
        crate::Residence::Pending { .. }
    ));
    assert!(heap.retire_mask(selected, &mut account.ledger).is_err());
    let reader = owned
        .reader(&mut account)?
        .into_endpoint_view(&mut account)?;
    for (cell, tag) in [(0, 1), (2, 64)] {
        for (component, input) in [
            [P::from_u8(tag), P::from_u8(2), P::ONE],
            [P::ONE, P::from_u8(3), P::from_u8(2)],
        ]
        .into_iter()
        .enumerate()
        {
            oracle(
                &reader.read_component(cell, component, &mut account)?,
                input,
            )?;
        }
    }
    super::super::fault::reset_reads();
    for cell in [1, 3] {
        assert!(matches!(
            reader.read_component(cell, 0, &mut account),
            Err(crate::EndpointReadError::Loan(LoanError::CoverageMismatch))
        ));
    }
    assert_eq!(super::super::fault::reads(), 0);
    drop(owned);
    heap.retire_mask(selected, &mut account.ledger)?;
    heap.retire_mask(compact, &mut account.ledger)?;
    heap.retire_cell_layout(compact.layout(), &mut account.ledger)?;
    empty(&mut heap, &mut account)
}

/// Missing companions refuse during binding, before loads or native allocation.
/// ´test:unit:loan-endpoint-view-representation-refusal´
#[test]
fn loan_endpoint_view_representation_refusal() -> TestResult {
    unsupported::<f32>()?;
    unsupported::<f64>()?;
    refusals::representations::<f32>()?;
    refusals::representations::<f64>()
}
fn unsupported<P: Scalar + 'static>() -> TestResult {
    for companions in [[false, false], [true, false]] {
        let (mut heap, mut account, declaration) =
            fixture::<P>(0, 1, companions, Limits::default())?;
        let loan = granted(heap.request_loan(declaration, &mut account))?;
        let (tile, mask) = match heap.handback(loan, &mut account) {
            HandbackOutcome::Published(tile, receipt) => (
                tile,
                receipt.filled(HostChannel::Center).ok_or("mask")?.native(),
            ),
            other => return Err(format!("{other:?}").into()),
        };
        let reader = tile.reader(&heap, &mut account)?;
        let view = reader.masked_view(heap.mask(mask, mask.layout())?, &mut account)?;
        super::super::fault::reset_reads();
        let injection = super::super::fault::Injection::after(0);
        assert!(matches!(
            view.into_endpoint_view(&mut account),
            Err(crate::EndpointReadError::Loan(
                LoanError::UnsupportedEndpointRepresentation
            ))
        ));
        assert_eq!(
            [
                super::super::fault::reads(),
                super::super::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        drop(tile);
        empty(&mut heap, &mut account)?;
    }
    Ok(())
}

/// Complete zero-capacity views read no holes; finite-range failure is explicitly numerical.
/// ´test:unit:loan-endpoint-view-empty-and-range-refusal´
#[test]
fn loan_endpoint_view_empty_and_range_refusal() -> TestResult {
    ranges::<f32>()?;
    ranges::<f64>()?;
    ranges::finite_edges::<f32>()?;
    ranges::finite_edges::<f64>()?;
    ranges::empty_demand::<f32>()?;
    ranges::empty_demand::<f64>()
}
fn ranges<P: Scalar + 'static>() -> TestResult {
    let (mut heap, mut account, tile, mask) = publish::<P>(&[])?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    assert_eq!(reader.capacity(), 0);
    super::super::fault::reset_reads();
    assert!(matches!(
        reader.read_component(0, 0, &mut account),
        Err(crate::EndpointReadError::Loan(LoanError::Mask(
            crate::MaskError::CellOutOfBounds
        )))
    ));
    assert_eq!(super::super::fault::reads(), 0);
    drop(tile);
    empty(&mut heap, &mut account)?;
    let maximum = P::max_finite();
    let (mut heap, mut account, tile, mask) = publish(&[[maximum, maximum, P::ZERO]])?;
    let source = tile.reader(&heap, &mut account)?;
    let reader = source
        .masked_view(heap.mask(mask, mask.layout())?, &mut account)?
        .into_endpoint_view(&mut account)?;
    super::super::fault::reset_reads();
    assert!(matches!(
        reader.read_component(0, 0, &mut account),
        Err(crate::EndpointReadError::Arithmetic(_))
    ));
    assert_eq!(super::super::fault::reads(), 3);
    drop(tile);
    empty(&mut heap, &mut account)
}
