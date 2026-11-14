// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit outward endpoint readings preserve incoming error and exact source associations.

use super::{
    ExecutionAccount, HostChannel, LoanError, MaskedTileReader, NumericalErrorScope, charge,
};
use nrz_scalar::{ArithmeticContext, ArithmeticFailure, Enclosure, Finite, Scalar};

/// A selected exact-dyadic radius reading converted to outward native endpoints.
///
/// Conversion slack is separate from the incoming numerical error. Neither is a
/// new physical assumption or a whole-expression error bound.
#[derive(Clone, Copy, Debug)]
pub struct EndpointReading<P: Scalar> {
    /// Outward lower and upper endpoints of the represented center/radius interval.
    bounds: Enclosure<P>,
    /// Lower and upper conversion discrepancies, in that order.
    conversion_error: [Finite<P>; 2],
    /// Original numerical-only radius, unchanged by endpoint conversion.
    input_error: P,
    /// Original error publication scope, including historical readings.
    scope: NumericalErrorScope,
}
impl<P: Scalar> EndpointReading<P> {
    /// Outward finite endpoints; their exact targets are the original dyadics c-r and c+r.
    #[must_use]
    pub const fn bounds(&self) -> Enclosure<P> {
        self.bounds
    }
    /// Bounds on the two endpoint conversion discrepancies, without incoming error.
    #[must_use]
    pub const fn conversion_error(&self) -> [Finite<P>; 2] {
        self.conversion_error
    }
    /// Original numerical-only error, not replaced by endpoint slack.
    #[must_use]
    pub const fn input_error(&self) -> P {
        self.input_error
    }
    /// Scope of the preserved original numerical-only error.
    #[must_use]
    pub const fn numerical_error_scope(&self) -> NumericalErrorScope {
        self.scope
    }
}

/// Typed distinction between authority/coverage refusal and a finite-range conversion failure.
#[derive(Debug, thiserror::Error)]
pub enum EndpointReadError {
    /// Selection, authority or resources refused before numerical observation.
    #[error("endpoint selection: {0}")]
    Loan(#[from] LoanError),
    /// Conservative outward arithmetic failed, including finite exact range-limit results.
    /// Subtracting zero from a nonzero center still requires finite outward neighbours;
    /// an exact endpoint at either maximum finite magnitude can therefore be refused.
    #[error("endpoint arithmetic: {0}")]
    Arithmetic(#[from] ArithmeticFailure),
}

/// Borrowed endpoint representation with all three native roles bound once per tile.
pub struct EndpointTileReader<'a, P: Scalar> {
    /// Original selected bundle and map; no new retaining edge or allocation.
    selected: MaskedTileReader<'a, P>,
    /// Complete original companion owners resolved to the native format once.
    channels: [&'a faer::Mat<P>; 3],
    /// Validated round-to-nearest execution environment.
    context: ArithmeticContext,
}
impl<P: Scalar> std::fmt::Debug for EndpointTileReader<'_, P> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Native planes may contain holes: diagnostics expose only authenticated metadata.
        formatter
            .debug_struct("EndpointTileReader")
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}
impl<'a, P: Scalar + 'static> MaskedTileReader<'a, P> {
    /// Bind an explicit endpoint view of complete ordinary center/total/error readings.
    /// # Errors
    /// Refuses unsupported representations, another account or exhausted Work before reads.
    pub fn into_endpoint_view(
        self,
        account: &mut ExecutionAccount,
    ) -> Result<EndpointTileReader<'a, P>, EndpointReadError> {
        account.authenticate(self.source.account)?;
        account
            .ledger
            .with_work_phase(super::FAMILY, "endpoint view binding", |budget| {
                charge(budget, 4)?;
                let bundle = self.source.bundle;
                if bundle.endpoints.is_some()
                    || bundle.retained_law.is_some()
                    || bundle.channels.iter().any(Option::is_none)
                {
                    return Err(LoanError::UnsupportedEndpointRepresentation.into());
                }
                let [center, total, error] = bundle.channels.each_ref();
                let channels = [center, total, error].map(|channel| {
                    channel
                        .as_ref()
                        .ok_or(LoanError::UnsupportedEndpointRepresentation)?
                        .matrix
                        .typed::<P>()
                });
                let [center, total, error] = channels;
                let channels = [center?, total?, error?];
                if channels.iter().any(|matrix| matrix.ncols() != 1)
                    || channels
                        .iter()
                        .any(|matrix| matrix.nrows() != channels[0].nrows())
                {
                    return Err(LoanError::LayoutMismatch.into());
                }
                Ok(EndpointTileReader {
                    channels,
                    selected: self,
                    context: ArithmeticContext::new()?,
                })
            })
    }
}
impl<P: Scalar + 'static> EndpointTileReader<'_, P> {
    /// Selected logical capacity, including unfilled cells which remain unreadable.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.selected.capacity()
    }

    /// Convert one selected original component without allocating or constructing descriptors.
    /// # Errors
    /// Refuses holes, bad components, authority and Work before loads; finite-range failure
    /// follows the three authenticated loads and leaves the immutable source unchanged.
    pub fn read_component(
        &self,
        cell: usize,
        component: usize,
        account: &mut ExecutionAccount,
    ) -> Result<EndpointReading<P>, EndpointReadError> {
        account.authenticate(self.selected.source.account)?;
        account
            .ledger
            .with_work_phase(super::FAMILY, "endpoint view conversion", |budget| {
                let projected = matches!(
                    self.selected.projection,
                    super::masked_reader::Projection::Product(..)
                );
                charge(budget, 34 + usize::from(projected))?;
                if !self
                    .selected
                    .selection
                    .contains(cell)
                    .map_err(LoanError::from)?
                {
                    return Err(LoanError::CoverageMismatch.into());
                }
                let original = match self.selected.selection.gather() {
                    Some(map) => *map.indices().get(cell).ok_or(LoanError::LayoutMismatch)?,
                    None => cell,
                };
                let original = self.selected.projection.original(original)?;
                for role in HostChannel::ALL {
                    self.selected
                        .source
                        .selected_owner(role, original, component)?;
                }
                let address = self
                    .selected
                    .source
                    .bundle
                    .layout
                    .components
                    .index(original, component)
                    .map_err(LoanError::from)?;
                if address >= self.channels[0].nrows() {
                    return Err(LoanError::LayoutMismatch.into());
                }
                let values = self.channels.map(|matrix| {
                    #[cfg(test)]
                    super::fault::loaded();
                    matrix[(address, 0)]
                });
                convert(
                    values,
                    self.selected.source.numerical_error_scope(),
                    &self.context,
                )
            })
    }
}

/// Exact dyadic endpoint targets use the scalar enclosure law, with separate displacement bounds.
fn convert<P: Scalar>(
    [center, radius, input_error]: [P; 3],
    scope: NumericalErrorScope,
    context: &ArithmeticContext,
) -> Result<EndpointReading<P>, EndpointReadError> {
    let center = Enclosure::singleton(Finite::new(center)?);
    let radius = Enclosure::singleton(Finite::new(radius)?);
    let lower = center.sub(radius, context)?;
    let upper = center.add(radius, context)?;
    let bounds = Enclosure::new(lower.lower().get(), upper.upper().get())?;
    let conversion_error = [
        lower.displacement_from(bounds.lower(), context)?,
        upper.displacement_from(bounds.upper(), context)?,
    ];
    Ok(EndpointReading {
        bounds,
        conversion_error,
        input_error,
        scope,
    })
}
