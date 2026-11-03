// SPDX-License-Identifier: AGPL-3.0-only
//! Exact scaling scalars have checked ordinary definedness without an assumed slot.

use crate::testing::cases::Fixture;
use crate::testing::common::{TestResult, default_budget};
use nrz_cas::Reference;
use nrz_cas::domain::{AmbientDomain, Layer, RegionDraft, RegionExpr};
use nrz_cas::proof::{
    ApplicationDraft, CaseCertificate, Certificate, Judgment, Primitive, RegionCertificate,
    Statement,
};
use nrz_cas::syntax::{Draft, PredicateKind};

/// The ordinary dependency is a hand-authored case theorem, not an assumed predicate fact.
/// ´test:integration:series-scalar-checked-definedness´
#[test]
fn series_scalar_checked_definedness() -> TestResult {
    let mut source = Fixture::new()?;
    let scalar = source.number("-1/2")?;
    let predicate = source.predicate(PredicateKind::Defined, &[scalar])?;
    // The case checker retains its literal intersections rather than simplifying them.
    let regions = source.cas.admit(
        &[
            Draft::Region(RegionDraft {
                header: Reference::Existing(source.header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::Meet(vec![Reference::Existing(source.domain.clone())]),
            }),
            Draft::Region(RegionDraft {
                header: Reference::Existing(source.header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::Meet(vec![
                    Reference::Existing(source.domain.clone()),
                    Reference::Local(0),
                ]),
            }),
        ],
        &mut default_budget()?,
    )?;
    let statement = Statement::PredicateFact {
        region: source.domain.clone(),
        predicate,
    };
    let step = ApplicationDraft {
        rule: Primitive::CaseDomain,
        context: source.context.clone(),
        conclusion: statement.clone(),
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: Certificate::Cases(Box::new(CaseCertificate {
            regions: vec![source.domain.clone(), regions[1].clone()],
            arms: Vec::new(),
            frames: Vec::new(),
            cells: vec![RegionCertificate {
                cuts: Vec::new(),
                membership: vec![vec![true], vec![true]],
            }],
        })),
    };
    let witness = source.cas.check_proof(
        &source.context,
        std::slice::from_ref(&step),
        &mut default_budget()?,
    )?;
    assert_eq!(witness.judgment().statement, statement);
    assert!(witness.is_discharged());
    assert_eq!(witness.hypotheses(), []);
    assert_eq!(witness.judgment().residuals, []);
    let bytes = source
        .cas
        .export_witness(&witness, &mut default_budget()?)?;
    for reordered in [false, true] {
        let mut receiver = Fixture::new()?;
        if reordered {
            receiver.number("7/3")?;
        }
        let scalar = receiver.number("-2/4")?;
        let predicate = receiver.predicate(PredicateKind::Defined, &[scalar])?;
        let expected = Judgment {
            context: receiver.context.clone(),
            layer: Layer::Logic,
            space: None,
            statement: Statement::PredicateFact {
                region: receiver.domain.clone(),
                predicate,
            },
            residuals: Vec::new(),
        };
        let replay = receiver
            .cas
            .replay(&bytes, &expected, &mut default_budget()?)?;
        assert_eq!(replay.judgment(), &expected);
        assert!(replay.is_discharged());
        assert_eq!(replay.hypotheses(), []);
    }
    let mut forged = step;
    let Certificate::Cases(certificate) = &mut forged.certificate else {
        return Err("case certificate".into());
    };
    certificate.cells[0].membership[0][0] = false;
    let before = source.cas.canonical_snapshot(&mut default_budget()?)?;
    assert!(
        source
            .cas
            .check_proof(&source.context, &[forged], &mut default_budget()?,)
            .is_err()
    );
    assert_eq!(
        source.cas.canonical_snapshot(&mut default_budget()?)?,
        before
    );
    Ok(())
}
