// SPDX-License-Identifier: AGPL-3.0-only
//! Exact finite objects modulo a positive power of the declared increment.

use crate::budget::{Budget, BudgetKind};
use crate::encoding::Encoder;
use crate::error::{Fault, InvalidCode, Location};
use crate::graph::{Cas, Edge, Handle, LogicalId, Reference, State, Vertex, size};
use crate::model::{AtomicSort, Field, Operator, Scope, Sort};
use crate::solving::{CoefficientDomainData, SolveDraft};

/// A complete immutable formal object; admission supplies syntax, never an arithmetic theorem.
#[derive(Clone, Debug)]
pub struct SeriesObject<R = Reference> {
    /// Checked coefficient interpretation; the executable interpretation is Q.
    pub field: R,
    /// Exact rational centre of the increment.
    pub centre: R,
    /// Declared Real coordinate, retaining declaration identity.
    pub variable: R,
    /// Explicit outer-to-inner binder frames.
    pub scope: Scope,
    /// Original complete assumption frame.
    pub context: R,
    /// Positive coefficient count, defining congruence modulo this power.
    pub order: u32,
    /// Exactly `order` ascending coefficients, including trailing zeros.
    pub coefficients: Vec<R>,
}

/// Object admission and identity accounting location.
const AT: Location = Location::new("formal series object");

impl<R> SeriesObject<R> {
    /// Every retained dependency, in canonical payload order.
    pub(crate) fn references(&self) -> impl Iterator<Item = &R> {
        [&self.field, &self.centre, &self.variable, &self.context]
            .into_iter()
            .chain(self.coefficients.iter())
    }

    /// Complete object encoding shared by keys and the typed portable record.
    pub(crate) fn encode(
        &self,
        out: &mut Encoder<'_>,
        write: &mut impl FnMut(&R, &mut Encoder<'_>) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        write(&self.field, out)?;
        write(&self.centre, out)?;
        write(&self.variable, out)?;
        crate::encoding::scope(&self.scope, out)?;
        write(&self.context, out)?;
        out.byte(0)?;
        crate::wire::index(self.order, out)?;
        out.count(self.coefficients.len())?;
        for coefficient in &self.coefficients {
            write(coefficient, out)?;
        }
        Ok(())
    }

    /// Resolve only after the caller has prepaid the complete payload and reference array.
    pub(crate) fn resolve<T>(
        &self,
        write: &mut impl FnMut(&R) -> Result<T, Fault>,
    ) -> Result<SeriesObject<T>, Fault> {
        let mut coefficients = Vec::new();
        coefficients
            .try_reserve_exact(self.coefficients.len())
            .map_err(|_| Fault::allocation(AT))?;
        for coefficient in &self.coefficients {
            coefficients.push(write(coefficient)?);
        }
        Ok(SeriesObject {
            field: write(&self.field)?,
            centre: write(&self.centre)?,
            variable: write(&self.variable)?,
            scope: self.scope.clone(),
            context: write(&self.context)?,
            order: self.order,
            coefficients,
        })
    }

    /// Bound the coefficient count through the existing jet dimensions before copying.
    pub(crate) fn preflight(&self, budget: &mut Budget) -> Result<(), Fault> {
        let degree = self.order.checked_sub(1).ok_or_else(invalid)?;
        if size(self.coefficients.len(), AT)? != u64::from(self.order) {
            return Err(invalid());
        }
        budget.check(BudgetKind::JetOrder, u64::from(degree), AT)?;
        budget.check(BudgetKind::JetEntries, u64::from(self.order), AT)?;
        crate::validation::scope_limit(&self.scope, budget)
    }
}

impl Cas {
    /// Borrow a complete immutable series object without copying its coefficient vector.
    ///
    /// # Errors
    /// Refuses a foreign handle or a handle of another vertex kind.
    pub fn series_object(&self, handle: &Handle) -> Result<&SeriesObject<Handle>, Fault> {
        match self.state.node(self.checked(handle)?)? {
            Vertex::SeriesObject(value) => Ok(value),
            _ => Err(Fault::invalid(InvalidCode::Reference, AT)),
        }
    }

    /// Admit syntax within the ordinary graph journal and complete key index.
    pub(crate) fn admit_series(
        &self,
        state: &mut State,
        draft: &SeriesObject,
        locals: &[LogicalId],
        pending: u64,
        budget: &mut Budget,
    ) -> Result<LogicalId, Fault> {
        draft.preflight(budget)?;
        let value = draft
            .resolve(&mut |reference| Ok(self.handle(self.resolve(state, reference, locals)?)))?;
        validate(state, &value, budget)?;
        let count = value.references().count();
        budget.work(size(count, AT)?, AT)?;
        budget.check(BudgetKind::Edges, size(count, AT)?, AT)?;
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(count)
            .map_err(|_| Fault::allocation(AT))?;
        for (index, dependency) in value.references().enumerate() {
            edges.push((
                dependency.logical_id(),
                Edge::Input(crate::graph::position_u32(index)?),
            ));
        }
        self.insert(
            state,
            Vertex::SeriesObject(Box::new(value)),
            &edges,
            pending,
            budget,
        )
    }
}

/// Validate full metadata without treating equal coefficient vectors as equal objects.
fn validate(state: &State, value: &SeriesObject<Handle>, budget: &mut Budget) -> Result<(), Fault> {
    budget.specialized(BudgetKind::JetOrder, u64::from(value.order - 1), false, AT)?;
    budget.specialized(BudgetKind::JetEntries, u64::from(value.order), true, AT)?;
    let Vertex::Solve(field) = state.node(value.field.logical_id())? else {
        return Err(Fault::invalid(InvalidCode::Reference, AT));
    };
    let SolveDraft::CoefficientDomain(field) = field.as_ref() else {
        return Err(Fault::invalid(InvalidCode::Reference, AT));
    };
    if !matches!(field, CoefficientDomainData::Rational) {
        return Err(Fault::invalid(InvalidCode::UnsupportedFragment, AT));
    }
    let context = state.context(value.context.logical_id())?;
    let header = context.header_id;
    let region = state.region(context.admissible)?;
    if region.scope != value.scope {
        return Err(Fault::invalid(InvalidCode::Scope, AT));
    }
    let variable = state.expression(value.variable.logical_id())?;
    if variable.header != header || variable.scope != value.scope {
        return Err(Fault::invalid(InvalidCode::Scope, AT));
    }
    if variable.sort != Sort::Value(AtomicSort::Scalar(Field::Real))
        || !matches!(variable.operator, Operator::Free(_))
    {
        return Err(Fault::invalid(InvalidCode::UnsupportedFragment, AT));
    }
    for reference in std::iter::once(&value.centre).chain(value.coefficients.iter()) {
        budget.work(1, AT)?;
        let expression = state.expression(reference.logical_id())?;
        if expression.header != header || expression.scope != value.scope {
            return Err(Fault::invalid(InvalidCode::Scope, AT));
        }
        let Operator::Rational(number) = &expression.operator else {
            return Err(Fault::invalid(InvalidCode::UnsupportedFragment, AT));
        };
        if expression.sort != Sort::Value(AtomicSort::Scalar(Field::Real)) {
            return Err(Fault::invalid(InvalidCode::UnsupportedFragment, AT));
        }
        let (numerator, denominator) = number.component_bits();
        budget.check(BudgetKind::ComponentBits, numerator.max(denominator), AT)?;
    }
    Ok(())
}

/// A malformed extent has no interpretation as a finite congruence.
const fn invalid() -> Fault {
    Fault::invalid(InvalidCode::Sort, AT)
}
