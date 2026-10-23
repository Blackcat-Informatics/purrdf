// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original regular-grid and inverse source-rectangle refusals.

/// The first rejected regular-grid construction requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GridConfigurationReason {
    /// Fewer than two columns were supplied.
    Columns,
    /// Fewer than two rows were supplied.
    Rows,
    /// The original spacing on this zero-based axis is not positive.
    Spacing {
        /// Original first failed axis.
        axis: usize,
    },
}
impl purrdf_hash::error::StructuredError for GridConfigurationReason {
    purrdf_hash::error_visit! {
        Columns => "grid-configuration-columns" {},
        Rows => "grid-configuration-rows" {},
        Spacing { axis } => "grid-configuration-spacing" { "axis" => Unsigned(*axis as u128) },
    }
}

/// Which original source-rectangle bound extends beyond the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GridDomainSide {
    /// The original lower source bound precedes the grid origin.
    Lower,
    /// The original upper source bound exceeds the computed grid extent.
    Upper,
}
impl purrdf_hash::error::StructuredError for GridDomainSide {
    purrdf_hash::error_visit! {
        Lower => "grid-domain-lower" {},
        Upper => "grid-domain-upper" {},
    }
}

/// Original grid declarations and exact source-domain inputs at refusal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModelInputError {
    /// A grid cannot have the original dimensions or spacing.
    GridConfiguration {
        /// First failed requirement.
        reason: GridConfigurationReason,
        /// Original regular-grid origin.
        origin: [crate::Rat; 2],
        /// Original regular-grid spacing.
        spacing: [crate::Rat; 2],
        /// Original column count.
        columns: u32,
        /// Original row count.
        rows: u32,
        /// Original supplied node count.
        node_count: usize,
    },
    /// A point is outside the extent or closed union of complete patches.
    Location {
        /// Original immutable grid, sharing its native source owners.
        grid: Box<crate::operation::BilinearGrid>,
        /// Original first coordinate.
        x: crate::context::Retained<crate::Rat>,
        /// Original second coordinate.
        y: crate::context::Retained<crate::Rat>,
        /// Original native locator refusal and computed normalized coordinate.
        cause: crate::operation::GridLocationRefusal,
    },
    /// One of the four original patch corners is a missing node.
    MissingNode {
        /// Original immutable grid.
        grid: Box<crate::operation::BilinearGrid>,
        /// Original cell/node coordinates and row-major index.
        cause: crate::operation::GridMissingNode,
    },
    /// The original inverse source rectangle has a nonpositive extent.
    SourceRectangle {
        /// Complete original lower and upper source bounds.
        domain: crate::operation::MetricSourceDomain,
        /// First failed zero-based coordinate axis.
        axis: usize,
    },
    /// The original inverse source rectangle extends beyond the grid.
    InverseRectangle {
        /// Original immutable grid.
        grid: Box<crate::operation::BilinearGrid>,
        /// Original source rectangle.
        domain: crate::operation::MetricSourceDomain,
        /// Original computed upper grid extent.
        upper: [crate::context::Retained<crate::Rat>; 2],
        /// First failed zero-based coordinate axis.
        axis: usize,
        /// First failed bound on that axis.
        side: GridDomainSide,
    },
}
impl purrdf_hash::error::StructuredError for ModelInputError {
    purrdf_hash::error_visit! {
        GridConfiguration { reason, origin, spacing, columns, rows, node_count } => "model-grid-configuration" {
            "reason" => Record(reason),
            "origin" => List(origin.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "spacing" => List(spacing.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "columns" => Unsigned(u128::from(*columns)), "rows" => Unsigned(u128::from(*rows)), "node_count" => Unsigned(*node_count as u128)
        },
        Location { grid, x, y, cause } => "model-grid-location" {
            "grid" => Record(&**grid), "x" => Record(&**x), "y" => Record(&**y), "cause" => Cause(cause)
        },
        MissingNode { grid, cause } => "model-grid-missing-node" { "grid" => Record(&**grid), "cause" => Cause(cause) },
        SourceRectangle { domain, axis } => "model-source-rectangle" { "domain" => Record(domain), "axis" => Unsigned(*axis as u128) },
        InverseRectangle { grid, domain, upper, axis, side } => "model-grid-inverse-rectangle" {
            "grid" => Record(&**grid), "domain" => Record(domain),
            "upper" => List(upper.iter().map(|value| purrdf_hash::error::ErrorValue::Record(&**value))),
            "axis" => Unsigned(*axis as u128), "side" => Record(side)
        },
    }
}
impl ModelInputError {
    fn with_grid(
        grid: &crate::operation::BilinearGrid,
        progress: &mut crate::context::WorkProgress<'_>,
        make: impl FnOnce(Box<crate::operation::BilinearGrid>) -> Self,
    ) -> super::GeoError {
        crate::context::Retained::produce(
            progress,
            |memory| {
                // A completed grid's private scalars and table are intrinsically
                // retained. Clone their existing headers; admit only the new box.
                let grid = memory
                    .boxed(grid.clone())
                    .map_err(super::GeoError::Storage)?;
                Ok(make(grid))
            },
            |_, _| Ok(()),
        )
        .map_or_else(|error| error, super::GeoError::ModelInput)
    }

    pub(crate) fn grid_configuration(
        origin: [crate::Rat; 2],
        spacing: [crate::Rat; 2],
        dimensions: (u32, u32),
        node_count: usize,
        reason: GridConfigurationReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let [origin, spacing] =
            match crate::context::retained::produce_persistent_numeric([origin, spacing], progress)
            {
                Ok(value) => value.into_value(),
                Err(error) => return error,
            };
        let (columns, rows) = dimensions;
        super::GeoError::retained_failure_with_progress(
            Self::GridConfiguration {
                reason,
                origin,
                spacing,
                columns,
                rows,
                node_count,
            },
            super::GeoError::ModelInput,
            progress,
        )
    }

    pub(crate) fn location(
        grid: &crate::operation::BilinearGrid,
        point: [&crate::Rat; 2],
        cause: &crate::operation::GridLocationRefusal,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(cause, None, None, progress, |cause, _, _, progress| {
            let x = match super::GeoError::diagnostic_number(point[0], progress) {
                Ok(value) => value,
                Err(error) => return error,
            };
            let y = match super::GeoError::diagnostic_number(point[1], progress) {
                Ok(value) => value,
                Err(error) => return error,
            };
            Self::with_grid(grid, progress, |grid| Self::Location { grid, x, y, cause })
        })
    }

    pub(crate) fn missing_node(
        grid: &crate::operation::BilinearGrid,
        cause: crate::operation::GridMissingNode,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        Self::with_grid(grid, progress, |grid| Self::MissingNode { grid, cause })
    }

    pub(crate) fn source_rectangle(
        domain: &crate::operation::MetricSourceDomain,
        axis: usize,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(
            domain,
            None,
            None,
            progress,
            |domain, _, _, progress| {
                super::GeoError::retained_failure_with_progress(
                    Self::SourceRectangle { domain, axis },
                    super::GeoError::ModelInput,
                    progress,
                )
            },
        )
    }

    pub(crate) fn inverse_rectangle(
        grid: &crate::operation::BilinearGrid,
        domain: &crate::operation::MetricSourceDomain,
        upper: &[crate::Rat; 2],
        axis: usize,
        side: GridDomainSide,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(
            domain,
            None,
            None,
            progress,
            |domain, _, _, progress| {
                let upper = match (|| {
                    Ok([
                        super::GeoError::diagnostic_number(&upper[0], progress)?,
                        super::GeoError::diagnostic_number(&upper[1], progress)?,
                    ])
                })() {
                    Ok(value) => value,
                    Err(error) => return error,
                };
                Self::with_grid(grid, progress, |grid| Self::InverseRectangle {
                    grid,
                    domain,
                    upper,
                    axis,
                    side,
                })
            },
        )
    }

    /// Whether this is a rejected source location rather than configuration.
    #[must_use]
    pub const fn is_domain_error(&self) -> bool {
        matches!(self, Self::Location { .. } | Self::MissingNode { .. })
    }
}
impl core::fmt::Display for ModelInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GridConfiguration { .. } => {
                f.write_str("bilinear grid needs positive spacing and at least two nodes per axis")
            }
            Self::Location { cause, .. } => core::fmt::Display::fmt(cause, f),
            Self::MissingNode { cause, .. } => core::fmt::Display::fmt(cause, f),
            Self::SourceRectangle { .. } => {
                f.write_str("inverse source rectangle requires positive metre extents")
            }
            Self::InverseRectangle { .. } => {
                f.write_str("grid inverse source rectangle extends beyond declared grid")
            }
        }
    }
}
purrdf_lex::error_source!(ModelInputError => {
    Self::Location { grid: _, x: _, y: _, cause } => cause,
    Self::MissingNode { grid: _, cause } => cause,
});
