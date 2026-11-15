// SPDX-License-Identifier: AGPL-3.0-only
//! CAS value enclosures and fresh certificate replay, independent of the Python route.

use super::*;
use nrz_cas::domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr};
use nrz_cas::enclosure::{
    CoordinateBound, EnclosureCertificate, EnclosureGoal, EnclosureRequest, Interval,
    RefinementPolicy,
};
use nrz_cas::syntax::{
    AtomicSort, Declaration, Draft, Environment, ExpressionDraft, Field, Operator, Sort,
};
use nrz_cas::{Cas, Handle, Reference};
/// Fallible independent CAS control result.
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Construct the original clamp node and its checked coordinate context.
fn original(lo: &Rational, hi: &Rational, wall: &Rational) -> TestResult<(Cas, EnclosureRequest)> {
    let mut b = ledger()?;
    let mut cas = Cas::new(b.limits().clone())?;
    let sort = Sort::Value(AtomicSort::Scalar(Field::Real));
    let mut drafts = vec![
        Draft::Header(HeaderDraft::Core(Environment {
            declarations: vec![Declaration {
                key: "x".into(),
                display_name: "x".into(),
                sort: sort.clone(),
            }],
            ..Environment::default()
        })),
        Draft::Region(RegionDraft {
            header: Reference::Local(0),
            scope: Vec::new(),
            ambient: AmbientDomain::Assignments,
            expression: RegionExpr::Ambient,
        }),
        Draft::Context(ContextDraft::Root {
            header: Reference::Local(0),
            admissible: Reference::Local(1),
            assumptions: Vec::new(),
        }),
    ];
    for (operator, operands) in [
        (Operator::Free("x".into()), vec![]),
        (Operator::Rational(wall.clone()), vec![]),
        (
            Operator::ClampFloor,
            vec![Reference::Local(3), Reference::Local(4)],
        ),
    ] {
        drafts.push(Draft::Expression(ExpressionDraft {
            space: None,
            operator,
            sort: sort.clone(),
            header: Reference::Local(0),
            scope: Vec::new(),
            binding: None,
            operands,
        }));
    }
    let handles: [Handle; 6] = cas
        .admit(&drafts, &mut b)?
        .try_into()
        .map_err(|_| "handles")?;
    let request = EnclosureRequest {
        target: handles[5].clone(),
        context: handles[2].clone(),
        coordinates: vec![CoordinateBound {
            coordinate: handles[3].clone(),
            interval: Interval::new(lo.clone(), hi.clone(), &mut b)?,
        }],
        goal: EnclosureGoal::Width,
    };
    Ok((cas, request))
}

/// Enclose the original node and independently replay its certificate.
/// # Panics
/// Panics only if fresh replay changes the produced value interval.
fn enclosure(lo: &Rational, hi: &Rational, wall: &Rational) -> TestResult<[Rational; 2]> {
    let policy = RefinementPolicy {
        version: 1,
        precision: 1100,
        grid_ceiling: 1200,
        rounds: 1,
        reduction_ceiling: 0,
        term_ceiling: 1,
        checking_work: 0,
    };
    let (mut cas, request) = original(lo, hi, wall)?;
    let checked = cas
        .enclose(&request, policy, &mut ledger()?, &mut |_| false)?
        .checked
        .ok_or("CAS clamp evidence")?;
    let certificate = EnclosureCertificate::from_bytes(
        &checked.certificate().to_bytes(&mut ledger()?)?,
        &mut ledger()?,
    )?;
    let (mut fresh, expected) = original(lo, hi, wall)?;
    let replay = fresh
        .check_enclosure(&expected, policy, &certificate, &mut ledger()?, &mut |_| {
            false
        })?
        .checked
        .ok_or("replayed clamp")?;
    assert_eq!(checked.interval(), replay.interval(), "fresh replay");
    Ok([
        checked.interval().lower().clone(),
        checked.interval().upper().clone(),
    ])
}

/// Original fixed-wall CAS queries cover both Cartesian extremes and nominal values.
/// # Panics
/// Panics only if independently replayed CAS images differ from retained golden values.
/// ´claim:docgen:clamp-golden-cas´
/// ´test:unit:clamp-golden-cas´
#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "Names follow the contract's represented roles."
)]
fn clamp_golden_cas() -> TestResult {
    let payload = read(include_bytes!("../../../../data/clamp/floor.json").as_slice())?;
    let mut count = 0;
    for row in &payload.entries {
        let Some(golden) = &row.golden else { continue };
        let mut b = ledger()?;
        let xb = bits(&row.input.x)?;
        let rb = bits(&row.input.r)?;
        let x = dyadic(xb, &mut b)?;
        let r = dyadic(rb, &mut b)?;
        let c = dyadic(bits(&row.wall.c)?, &mut b)?;
        let t = dyadic(bits(&row.wall.t)?, &mut b)?;
        let lo = if x == r {
            0
        } else {
            rounded_neighbor(nearest(&x.sub(&r, &mut b)?, &mut b)?.ok_or("lower")?, false)
                .ok_or("lower neighbor")?
        };
        let hi = if x.is_zero() {
            rb & !SIGN
        } else if r.is_zero() {
            xb
        } else {
            rounded_neighbor(nearest(&x.add(&r, &mut b)?, &mut b)?.ok_or("upper")?, true)
                .ok_or("upper neighbor")?
        };
        let low_wall = c.sub(&t, &mut b)?;
        let high_wall = c.add(&t, &mut b)?;
        let lower = enclosure(&dyadic(lo, &mut b)?, &dyadic(hi, &mut b)?, &low_wall)?;
        let upper = enclosure(&dyadic(lo, &mut b)?, &dyadic(hi, &mut b)?, &high_wall)?;
        assert_eq!(
            [
                lower[0].exact_string(&mut b)?,
                upper[1].exact_string(&mut b)?
            ]
            .as_slice(),
            golden.image.as_slice(),
            "{} image",
            row.id
        );
        let nominal_lo = enclosure(&x, &x, &low_wall)?;
        let nominal_hi = enclosure(&x, &x, &high_wall)?;
        assert_eq!(
            [
                nominal_lo[0].exact_string(&mut b)?,
                nominal_hi[1].exact_string(&mut b)?
            ]
            .as_slice(),
            golden.nominal.as_slice(),
            "{} nominal",
            row.id
        );
        count += 1;
    }
    assert!(count > 20, "CAS rows executed");
    Ok(())
}
