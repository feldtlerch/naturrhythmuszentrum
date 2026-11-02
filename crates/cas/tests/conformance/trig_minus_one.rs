// SPDX-License-Identifier: AGPL-3.0-only
//! The named recipe is checked composition with visible endpoint-domain discharge.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::trig_minus_one::{fixture, premises};
use nrz_cas::proof::TrigRecipeOutcome;
use nrz_cas::{FaultKind, InvalidCode};

/// Original literal and composite inputs retain their guard through the complete expansion.
/// ´test:integration:trig-minus-one-expansion´
#[test]
fn trig_minus_one_expansion() -> TestResult {
    for kind in [0, 1] {
        let (mut f, function) = fixture(kind, false)?;
        let [guard, equality] = premises(&mut f)?;
        let TrigRecipeOutcome::Proved(witness) =
            f.cas
                .trig_atan_minus_one(&function, &guard, &equality, &mut default_budget()?)?
        else {
            return Err("minus-one recipe".into());
        };
        assert_eq!(witness.judgment(), &f.expected);
        assert_eq!(witness.hypotheses().len(), 1);
        assert_eq!(witness.judgment().residuals, []);
        let bytes = f.cas.export_witness(&witness, &mut default_budget()?)?;
        for key in [
            b"trig.defined".as_slice(),
            b"trig.atan-odd",
            b"trig.atan-one",
            b"eq.congruence",
            b"eq.trans",
            b"ctx.close",
        ] {
            assert!(bytes.windows(key.len()).any(|word| word == key));
        }
        assert!(
            !nrz_cas::proof::PRIMITIVES
                .iter()
                .any(|rule| rule.specification().key == "trig.atan-minus-one")
        );
        assert!(
            !bytes
                .windows(b"trig.atan-minus-one".len())
                .any(|word| word == b"trig.atan-minus-one")
        );
    }
    Ok(())
}

/// Checked quotient, conditional and sum equalities expand with every original residual retained.
/// ´test:integration:trig-minus-one-transport-boundary´
#[test]
fn trig_minus_one_transport_boundary() -> TestResult {
    for kind in [2, 3, 4] {
        let (mut f, function) = fixture(kind, false)?;
        let [guard, equality] = premises(&mut f)?;
        let original_equality =
            f.cas
                .check_proof(&f.context, &f.steps[1..], &mut default_budget()?)?;
        assert_eq!(original_equality.is_discharged(), kind != 2);
        let TrigRecipeOutcome::Proved(witness) =
            f.cas
                .trig_atan_minus_one(&function, &guard, &equality, &mut default_budget()?)?
        else {
            return Err("checked transport did not expand".into());
        };
        assert_eq!(witness.judgment(), &f.expected);
        assert_eq!(witness.judgment().residuals.is_empty(), kind != 2);
    }
    Ok(())
}

/// A function in either premise slot is malformed evidence before capability mapping.
/// ´test:integration:trig-minus-one-premise-categories´
#[test]
fn trig_minus_one_premise_categories() -> TestResult {
    for wrong_guard in [false, true] {
        let (mut f, function) = fixture(0, false)?;
        let [guard, equality] = premises(&mut f)?;
        let (guard, equality) = if wrong_guard {
            (&function, &equality)
        } else {
            (&guard, &function)
        };
        let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
        let counts = f.cas.counts();
        let fault = f
            .cas
            .trig_atan_minus_one(&function, guard, equality, &mut default_budget()?)
            .err()
            .ok_or("wrong-node premise was not invalid")?;
        assert_eq!(fault.kind, FaultKind::Invalid(InvalidCode::Reference));
        assert_eq!(fault.location.operation, "trig.atan-minus-one recipe");
        assert_eq!(f.cas.counts(), counts);
        assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
    }
    Ok(())
}

/// A valid subset between independent roots remains an unsupported conversion route.
/// ´test:integration:trig-minus-one-independent-root-transport´
#[test]
fn trig_minus_one_independent_root_transport() -> TestResult {
    let (mut f, function) = crate::testing::trig_minus_one::independent_root()?;
    let [guard, equality] = premises(&mut f)?;
    let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
    assert!(matches!(
        f.cas
            .trig_atan_minus_one(&function, &guard, &equality, &mut default_budget()?)?,
        TrigRecipeOutcome::Unresolved
    ));
    assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
    Ok(())
}

/// A valid checked equality to a different rational supplies no minus-one conclusion.
/// ´test:integration:trig-minus-one-other-rational´
#[test]
fn trig_minus_one_other_rational() -> TestResult {
    let mut f = crate::testing::trig_values::atan("2", true, false)?;
    f.steps.pop().ok_or("old consumer")?;
    let nrz_cas::proof::Statement::Equality { left: function, .. } = &f.expected.statement else {
        return Err("expected equality".into());
    };
    let function = function.clone();
    let [guard, equality] = premises(&mut f)?;
    let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
    assert!(matches!(
        f.cas
            .trig_atan_minus_one(&function, &guard, &equality, &mut default_budget()?)?,
        TrigRecipeOutcome::Unresolved
    ));
    assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
    Ok(())
}
