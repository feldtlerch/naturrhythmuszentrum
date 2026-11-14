// SPDX-License-Identifier: AGPL-3.0-only
//! Repeated Cartesian reads preserve the original factor and its companions.

use super::{TestResult, oracle};
use crate::{CellFactor, Extent, Layout};
use nrz_scalar::Scalar;

/// Explicit tuple enumeration checks both factor positions and the 34/35 Work boundary.
/// ´test:unit:loan-endpoint-view-cartesian-images´
#[test]
fn loan_endpoint_view_cartesian_images() -> TestResult {
    factors::<f32>()?;
    factors::<f64>()
}
fn factors<P: Scalar + 'static>() -> TestResult {
    for factor in [CellFactor::Outer, CellFactor::Inner] {
        let values = [
            [P::ONE, P::ONE, P::ONE],
            [P::from_u8(5), P::from_u8(2), P::ONE],
        ];
        let (mut heap, mut account, tile, mask) = super::publish(&values)?;
        let axis = heap.mask(mask, mask.layout())?.axis();
        let other = heap.create_cell_axis(3, &mut account.ledger)?;
        let ordered = match factor {
            CellFactor::Outer => [axis, other],
            CellFactor::Inner => [other, axis],
        };
        let product = heap.create_cell_product(ordered[0], ordered[1], &mut account.ledger)?;
        let layout = heap.create_cell_layout(
            product.axis(),
            Layout::contiguous(Extent::new(6, 1)?)?,
            &mut account.ledger,
        )?;
        let selected =
            heap.expand_product_mask(product, factor, mask, layout, &mut account.ledger)?;
        let owned =
            heap.retain_product_tile(tile.pin(), selected, product, factor, &mut account)?;
        drop(tile);
        let reader = owned
            .reader(&mut account)?
            .into_endpoint_view(&mut account)?;
        for (original, values) in values.into_iter().enumerate() {
            for repeat in 0..3 {
                let cell = match factor {
                    CellFactor::Outer => original * 3 + repeat,
                    CellFactor::Inner => repeat * 2 + original,
                };
                let before = account.usage().work;
                oracle(&reader.read_component(cell, 0, &mut account)?, values)?;
                assert_eq!(account.usage().work - before, 35);
            }
        }
        let spend = account.limits().work - account.usage().work - 34;
        account.ledger.work(spend, crate::loan::AT)?;
        crate::loan::fault::reset_reads();
        assert!(matches!(
            reader.read_component(0, 0, &mut account),
            Err(crate::EndpointReadError::Loan(crate::LoanError::Budget(_)))
        ));
        assert_eq!(crate::loan::fault::reads(), 0);
        drop(owned);
        let mut scanner = crate::ExecutionAccount::new(nrz_bindings::Budget::new(
            nrz_bindings::Limits::default(),
        )?)?;
        heap.retire_mask(selected, &mut scanner.ledger)?;
        heap.retire_cell_layout(layout, &mut scanner.ledger)?;
        super::empty(&mut heap, &mut scanner)?;
    }
    Ok(())
}

/// A compacted sparse product maps gathered positions back to original tuples before projection.
/// ´test:unit:loan-endpoint-view-gathered-cartesian-images´
#[test]
fn loan_endpoint_view_gathered_cartesian_images() -> TestResult {
    gathered::<f32>()?;
    gathered::<f64>()
}
fn gathered<P: Scalar + 'static>() -> TestResult {
    for factor in [CellFactor::Outer, CellFactor::Inner] {
        let values = [
            [P::ONE, P::ONE, P::ONE],
            [P::from_u8(5), P::from_u8(2), P::ONE],
        ];
        let (mut heap, mut account, tile, mask) = super::publish(&values)?;
        let axis = heap.mask(mask, mask.layout())?.axis();
        let other = heap.create_cell_axis(3, &mut account.ledger)?;
        let ordered = match factor {
            CellFactor::Outer => [axis, other],
            CellFactor::Inner => [other, axis],
        };
        let product = heap.create_cell_product(ordered[0], ordered[1], &mut account.ledger)?;
        let layout = heap.create_cell_layout(
            product.axis(),
            Layout::contiguous(Extent::new(6, 1)?)?,
            &mut account.ledger,
        )?;
        let selected = heap.create_mask(layout, &[1, 4, 5], &mut account.ledger)?;
        let identity = heap.mask_correspondence(layout, layout, &mut account.ledger)?;
        let compact = heap.compact_mask(identity, selected, 3, 3, &mut account.ledger)?;
        let owned = heap.retain_product_tile(tile.pin(), compact, product, factor, &mut account)?;
        drop(tile);
        let reader = owned
            .reader(&mut account)?
            .into_endpoint_view(&mut account)?;
        assert_eq!(reader.capacity(), 3);
        for (cell, original_tuple) in [1, 4, 5].into_iter().enumerate() {
            let original_factor = match factor {
                CellFactor::Outer => original_tuple / 3,
                CellFactor::Inner => original_tuple % 2,
            };
            oracle(
                &reader.read_component(cell, 0, &mut account)?,
                values[original_factor],
            )?;
        }
        drop(owned);
        heap.retire_mask(compact, &mut account.ledger)?;
        heap.retire_cell_layout(compact.layout(), &mut account.ledger)?;
        heap.retire_mask(selected, &mut account.ledger)?;
        heap.retire_cell_layout(layout, &mut account.ledger)?;
        super::empty(&mut heap, &mut account)?;
    }
    Ok(())
}
