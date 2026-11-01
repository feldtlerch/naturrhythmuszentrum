// SPDX-License-Identifier: AGPL-3.0-only
//! A client retains its complete original and symbolic evidence through the frozen owner.

use nrz_bindings::{Descriptor, SensitivityRecipe};
use nrz_cas::{
    limits::{Budget, Limits, LimitsOverride},
    source::WalkEvent,
};

use crate::testing::declarations::{TestResult, context, input};
use crate::testing::sensitivity::{coefficient, equal_fraction};

/// Existing checks, native lowering and read-only proof inspection share one immutable owner.
/// ´test:integration:native-sensitivity-frozen-owner-lifecycle´
#[test]
fn native_sensitivity_frozen_owner_lifecycle() -> TestResult {
    let mut builder = context(&["x"])?;
    input(&mut builder, "x", &Descriptor::ratio())?;
    let original = builder.parse("client square", "x^2")?;
    let earlier = builder.check(&original)?;
    let sensitivity = builder.check_sensitivity(
        SensitivityRecipe::Square,
        std::slice::from_ref(&original),
        &["x".into()],
        &LimitsOverride::default(),
    )?;
    sensitivity.declarations().validate_checked(&earlier)?;
    let cas = sensitivity.declarations().cas()?;
    let walk = cas.walk_original(&original.root, &mut Budget::new(Limits::default())?)?;
    assert!(
        walk.events
            .iter()
            .all(|event| matches!(event, WalkEvent::Visit(_)))
    );
    assert_eq!(
        sensitivity
            .jacobian()
            .originals()
            .first()
            .ok_or("missing original")?
            .original()
            .root,
        original.root
    );
    let entry = sensitivity
        .jacobian()
        .entries()
        .first()
        .ok_or("missing derivative")?;
    equal_fraction(
        coefficient(&cas, entry.expression(), &[("x", -4)])?,
        (-8, 1),
    )?;
    assert!(entry.identity().is_discharged());
    assert_eq!(entry.pairing().target(), &Descriptor::ratio());
    assert_eq!(sensitivity.snapshot_id(), earlier.snapshot_id());
    Ok(())
}

/// Physical input measures remain distinct from the stored-coordinate product law.
/// ´test:integration:quantity-product-sensitivity-coordinate-oracle´
/// ´claim:bindings:quantity-product-sensitivity-coordinate-oracle´ A checked product retains composed noncoherent measures and replayed coordinate derivatives two and three, with its original frozen evidence.
#[test]
fn quantity_product_sensitivity_coordinate_oracle() -> TestResult {
    let mut builder = context(&["x", "y"])?;
    let mut budget = Budget::new(Limits::default())?;
    let first = Descriptor::new(
        nrz_bindings::Kind::Ratio,
        nrz_bindings::Measure::new(
            "first",
            nrz_bindings::Scale::Exact(nrz_cas::Rational::from_ratio(2, 1, &mut budget)?),
            nrz_cas::Rational::zero(),
        )?,
        nrz_bindings::Role::Difference,
    )?;
    let second = Descriptor::new(
        nrz_bindings::Kind::Ratio,
        nrz_bindings::Measure::new(
            "second",
            nrz_bindings::Scale::Exact(nrz_cas::Rational::from_ratio(3, 1, &mut budget)?),
            nrz_cas::Rational::zero(),
        )?,
        nrz_bindings::Role::Difference,
    )?;
    input(&mut builder, "x", &first)?;
    input(&mut builder, "y", &second)?;
    let original = builder.parse("quantity product", "x*y")?;
    let earlier = builder.check(&original)?;
    let proof = builder.check_quantity_product_sensitivity(
        &original,
        &["x".into(), "y".into()],
        &LimitsOverride::default(),
    )?;
    proof.declarations().validate_checked(&earlier)?;
    assert_eq!(proof.inputs(), [first, second]);
    assert_eq!(proof.outputs()[0].measure().name(), "(first*second)");
    assert_eq!(proof.recipe(), SensitivityRecipe::QuantityProduct);
    assert_eq!(
        proof.obligations(),
        [] as [nrz_bindings::SensitivityObligation; 0]
    );
    let cas = proof.declarations().cas()?;
    for (entry, expected) in proof.jacobian().entries().iter().zip([2, 3]) {
        equal_fraction(
            coefficient(&cas, entry.expression(), &[("x", 3), ("y", 2)])?,
            (expected, 1),
        )?;
        assert!(entry.identity().is_discharged());
    }
    Ok(())
}

/// An ordered coordinate law cannot be reassigned to another syntax or input inventory.
/// ´test:integration:quantity-product-sensitivity-refusal-twins´
/// ´claim:bindings:quantity-product-sensitivity-refusal-twins´ Wrong original syntax and reordered quantity coordinates refuse while the matching original product is admitted.
#[test]
fn quantity_product_sensitivity_refusal_twins() -> TestResult {
    for (expression, keys, accepted) in [
        ("x+y", ["x", "y"], false),
        ("x*y", ["y", "x"], false),
        ("x*y", ["x", "y"], true),
    ] {
        let mut builder = context(&["x", "y"])?;
        input(&mut builder, "x", &Descriptor::ratio())?;
        input(&mut builder, "y", &Descriptor::ratio())?;
        let original = builder.parse("quantity product twin", expression)?;
        let result = builder.check_quantity_product_sensitivity(
            &original,
            &keys.map(String::from),
            &LimitsOverride::default(),
        );
        if accepted {
            result?;
        } else {
            let Err(fault) = result else {
                panic!("unmatched quantity product must refuse");
            };
            if expression == "x+y" {
                assert!(matches!(
                    fault,
                    nrz_bindings::SensitivityError::UnsupportedRecipe
                ));
            } else {
                assert!(matches!(fault, nrz_bindings::SensitivityError::Coordinates));
            }
        }
    }
    Ok(())
}

/// Complete preparation has a larger reservation boundary than its actual consumed Work.
/// ´test:integration:quantity-product-sensitivity-work-boundary´
/// ´claim:bindings:quantity-product-sensitivity-work-boundary´ Product sensitivity accepts its literal cumulative allowance, charges only actual preparation and four service receipts, and preserves named exhaustion and spent Work one unit below.
#[test]
fn quantity_product_sensitivity_work_boundary() -> TestResult {
    for accepted in [false, true] {
        let mut budget = Budget::new(Limits {
            work: if accepted { 858_779 } else { 858_778 },
            ..Limits::default()
        })?;
        let ratio = Descriptor::ratio();
        let result = nrz_bindings::CheckedSensitivity::quantity_product(
            &nrz_bindings::Registry::new(nrz_bindings::ExponentPolicy::Rational),
            [&ratio, &ratio],
            &mut budget,
        );
        if accepted {
            let proof = result?;
            assert_eq!(budget.usage().work, 803_767);
            assert_eq!(
                proof.jacobian().work(),
                nrz_bindings::SensitivityWork {
                    construction: 24_965,
                    generation: 71_340,
                    replay: 212_574,
                    requests: 4,
                }
            );
        } else {
            let Err(nrz_bindings::SensitivityError::Exhausted { fault, outcome }) = result else {
                panic!("one-below quantity proof allowance must retain named exhaustion");
            };
            assert!(matches!(
                fault.kind,
                nrz_cas::FaultKind::Exhausted {
                    budget: nrz_cas::limits::BudgetKind::Work
                        | nrz_cas::limits::BudgetKind::ReplayWork,
                    ..
                }
            ));
            assert!(matches!(
                *outcome,
                nrz_cas::request::Outcome::Exhausted { .. }
            ));
            assert_eq!(budget.usage().work, 708_125);
        }
    }
    Ok(())
}
