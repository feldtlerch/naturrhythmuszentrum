// SPDX-License-Identifier: AGPL-3.0-only
//! Shared structural trigonometric fixtures with explicit declaration and ambient identities.

use super::common::{TestResult, default_budget, real};
use crate::test_api::domain::HeaderDraft;
use crate::test_api::syntax::{
    AtomicSort, Branch, Declaration, Draft, Environment, ExpressionDraft, Field, Operator, Scope,
    Sort,
};
use crate::test_api::{Cas, Fault, Handle, Reference};

/// Closed constructors and their independently declared opaque spellings.
pub(crate) const fn operators() -> [(Operator, &'static str); 3] {
    [
        (Operator::Sine, "sine"),
        (Operator::Cosine, "cosine"),
        (Operator::Arctangent, "arctangent"),
    ]
}

/// Mixed declarations permit checked Real projections without retagging Complex sources.
pub(crate) struct Fixture {
    pub(crate) cas: Cas,
    pub(crate) header: Handle,
    pub(crate) x: Handle,
    pub(crate) z: Handle,
}

impl Fixture {
    pub(crate) fn new() -> TestResult<Self> {
        let mut cas = Cas::new(default_budget()?.limits().clone())?;
        let mut declarations: Vec<_> = [
            ("x", real()),
            ("y", real()),
            ("z", Sort::Value(AtomicSort::Scalar(Field::Complex))),
        ]
        .into_iter()
        .map(|(key, sort)| Declaration {
            key: key.into(),
            display_name: key.into(),
            sort,
        })
        .collect();
        declarations.extend(operators().into_iter().map(|(_, key)| Declaration {
            key: key.into(),
            display_name: key.into(),
            sort: Sort::Function {
                inputs: vec![AtomicSort::Scalar(Field::Real)],
                output: AtomicSort::Scalar(Field::Real),
            },
        }));
        let header = cas
            .admit(
                &[Draft::Header(HeaderDraft::Core(Environment {
                    field: Field::Complex,
                    branch: Branch::PrincipalComplex,
                    declarations,
                }))],
                &mut default_budget()?,
            )?
            .pop()
            .ok_or("missing header")?;
        let mut leaf = |key: &str, sort| -> TestResult<Handle> {
            Ok(cas
                .admit(
                    &[Draft::Expression(ExpressionDraft {
                        header: Reference::Existing(header.clone()),
                        space: None,
                        operator: Operator::Free(key.into()),
                        sort,
                        scope: Vec::new(),
                        binding: None,
                        operands: Vec::new(),
                    })],
                    &mut default_budget()?,
                )?
                .pop()
                .ok_or("missing leaf")?)
        };
        let x = leaf("x", real())?;
        let z = leaf("z", Sort::Value(AtomicSort::Scalar(Field::Complex)))?;
        Ok(Self { cas, header, x, z })
    }

    pub(crate) fn draft(&self, operator: Operator, operands: &[Handle]) -> ExpressionDraft {
        ExpressionDraft {
            header: Reference::Existing(self.header.clone()),
            space: None,
            operator,
            sort: real(),
            scope: Vec::new(),
            binding: None,
            operands: operands.iter().cloned().map(Reference::Existing).collect(),
        }
    }

    pub(crate) fn term(
        &mut self,
        operator: Operator,
        operands: &[Handle],
    ) -> Result<Handle, Fault> {
        let draft = self.draft(operator, operands);
        self.cas
            .admit(&[Draft::Expression(draft)], &mut default_budget()?)?
            .pop()
            .ok_or_else(|| {
                Fault::invalid(
                    crate::test_api::InvalidCode::Reference,
                    crate::test_api::Location::new("trig fixture"),
                )
            })
    }

    pub(crate) fn scoped(
        &mut self,
        operator: Operator,
        scope: Scope,
        operands: &[Handle],
    ) -> TestResult<Handle> {
        let mut draft = self.draft(operator, operands);
        draft.scope = scope;
        Ok(self
            .cas
            .admit(&[Draft::Expression(draft)], &mut default_budget()?)?
            .pop()
            .ok_or("missing scoped term")?)
    }
}
