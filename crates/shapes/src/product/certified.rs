// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The admission TYPE GATE: the only two ways a restored shapes graph becomes a
//! [`PreparedShapes`] a caller may validate with.
//!
//! # Why a type and not a check
//!
//! Every other file in this codec answers "are these bytes well formed?". This one
//! answers a different question: "has the whole admission sequence actually run?".
//! The dangerous failure at an admission boundary is not a check that returns the
//! wrong answer — it is a check that is SKIPPED, on a path someone added later that
//! returns early with a value that looks finished. That defect is invisible: the
//! bytes verified, the model decoded, the report comes back well formed, and the
//! one thing that never happened was the binding.
//!
//! So the finished value is unforgeable rather than merely guarded.
//! [`CertifiedParts`] wraps the [`PreparedShapes`] and its field is private to this
//! module; the two constructors below are the only expressions in the crate that
//! produce one, and each performs its seam's complete final check before it does.
//! A future early return in `super` cannot hand back a half-restored preparation,
//! because there is no expression it could write to build one. The refusal is a
//! COMPILE ERROR, not a runtime counter that someone has to remember to read.
//!
//! The rejected alternative was a `validated: bool` (or a "checks run" tally)
//! carried alongside the parts and asserted at the end. It type-checks, it is less
//! code, and it fails in the exact way this gate exists to prevent: the flag is set
//! by the code that is supposed to do the work, so a path that forgets the work
//! also forgets the flag, and the assertion passes. A boolean records an intention;
//! a private constructor records a fact.
//!
//! # The two seams
//!
//! | Constructor | What it re-derives | What it proves, beyond [`install`]'s own check |
//! |-------------|--------------------|-----------------------------------------------|
//! | [`from_admitted`] | nothing expensive | the product's declared [`Identity`] is the identity of the environment about to execute it |
//! | [`from_rebuilt`] | the whole shapes graph, from the carried dataset | nothing further — the derivation from the authenticated dataset IS the binding |
//!
//! They are two entry points at ONE boundary, not a mode flag on one entry point —
//! see the [module docs](super) for why the writer stays single-path.
//!
//! # Installing is not fingerprinting
//!
//! Both seams **install** the host's bindings into the restored
//! [`Shapes`](crate::shapes::Shapes) rather than merely checking them.
//! `Shapes::aggregates` and `Shapes::functions` are PUBLIC fields every validation
//! entry point reads directly to build the scopes its query bodies evaluate under
//! (see `crate::engine`'s `enter_aggregate_scope`/`enter_function_scope` calls). A
//! restore that fingerprinted them and left the restored value's own empty
//! registries in place would pass every check here and then fail at evaluation with
//! "no custom aggregate is registered" — a capability loss no fingerprint can catch,
//! because the fingerprints agreed.
//!
//! Nothing here touches the filesystem, a clock, a thread, or a source of
//! randomness; it stays `wasm32-unknown-unknown` compatible.
//!
//! [`from_admitted`]: CertifiedParts::from_admitted
//! [`from_rebuilt`]: CertifiedParts::from_rebuilt

use std::sync::Arc;

use purrdf_core::artifact::Identity;
use purrdf_sparql_eval::user_fn::FnPopulation;
use purrdf_sparql_eval::{UserFunctionRegistry, user_fn};

use crate::engine::PreparedShapes;
use crate::shapes::{Shapes, link};

use super::error::{ProductDimension, ShapesProductError};
use super::{HostBindings, ast, identity};

/// A restored preparation that has passed its seam's complete final check.
///
/// Constructible only by the two functions below — see the [module docs](self) for
/// why that is the point.
pub(super) struct CertifiedParts {
    /// The finished preparation. Private, and written at exactly two sites.
    prepared: PreparedShapes,
}

impl CertifiedParts {
    /// The ADMIT seam's final check: install the host's bindings, re-derive the
    /// class catalog, and prove the product's declared [`Identity`] is the identity
    /// of the environment about to execute it.
    ///
    /// The class-catalog recompute is not a separate step: it is row 10 of the
    /// identity, so deriving the catalog from the restored shapes and handing it to
    /// the check IS the comparison against the digest the product pinned.
    ///
    /// # Errors
    ///
    /// The [`ProductDimension`] of the first identity component that disagrees, or
    /// the installation refusals [`install`] reports.
    pub(super) fn from_admitted(
        declared: &Identity,
        mut shapes: Shapes,
        host: &HostBindings<'_>,
    ) -> Result<Self, ShapesProductError> {
        install(&mut shapes, host)?;

        let shapes = Arc::new(shapes);
        let prepared = PreparedShapes::new(Arc::clone(&shapes));
        let classes = prepared.class_catalog();
        identity::check_restored_identity(declared, &shapes, host.property_functions(), &classes)?;
        Ok(Self { prepared })
    }

    /// The REBUILD seam's final check: install the host's bindings into a shapes
    /// graph re-derived from the carried dataset.
    ///
    /// No identity check runs here, deliberately. Rebuild exists for the reader that
    /// meets a preparation stage it does not know: every identity component this
    /// build could compare against is a claim made by a build whose model is not
    /// this one's, so checking them would refuse precisely the products rebuild
    /// exists to rescue. What binds a rebuilt preparation is the derivation itself —
    /// the shapes come from the dataset the envelope's per-section SHA-256 and
    /// whole-container digest already authenticate.
    ///
    /// # Errors
    ///
    /// The installation refusals [`install`] reports.
    pub(super) fn from_rebuilt(
        mut shapes: Shapes,
        host: &HostBindings<'_>,
    ) -> Result<Self, ShapesProductError> {
        install(&mut shapes, host)?;
        Ok(Self {
            prepared: PreparedShapes::new(Arc::new(shapes)),
        })
    }

    /// The finished preparation.
    pub(super) fn into_prepared(self) -> PreparedShapes {
        self.prepared
    }
}

/// Install the host's bindings into a restored shapes graph, proving that doing so
/// keeps every function the shapes graph itself declares.
///
/// `shapes` arrives carrying the registries its own derivation produced — the
/// linked table on the admit path, the parser's on the rebuild path. Both are
/// replaced here, at the ONE site, so the two seams cannot come to install
/// different things.
///
/// # Errors
///
/// [`ProductDimension::UnsupportedCapability`] when assembly would lose a declared
/// function (see [`verify_declared_functions`]);
/// [`ProductDimension::FunctionRegistry`] when a registry cannot be fingerprinted;
/// [`ProductDimension::DepthLimit`] or [`ProductDimension::Malformed`] when the
/// declaration walk refuses.
fn install(shapes: &mut Shapes, host: &HostBindings<'_>) -> Result<(), ShapesProductError> {
    let assembled = assemble_functions(shapes, host)?;
    verify_declared_functions(&shapes.functions, &assembled)?;
    shapes.functions = Arc::new(assembled);
    shapes.aggregates = Arc::new(host.aggregates().clone());
    Ok(())
}

/// The user-function registry a restore assembles: the host's injected table, plus
/// the shapes graph's own expression-bodied declarations registered into it.
///
/// The direction is forced and it is worth stating. `UserFunctionRegistry` exposes
/// registration but not enumeration — a registered native closure cannot be read
/// back out — so a host's table can never be merged INTO a parser-built registry.
/// It can only be the table the declarations are registered into, which is why
/// every restore path starts from the host's registry rather than from an empty one.
///
/// # Errors
///
/// [`ProductDimension::DepthLimit`] or [`ProductDimension::Malformed`] when the
/// declaration walk over `shapes` refuses.
pub(super) fn assemble_functions(
    shapes: &Shapes,
    host: &HostBindings<'_>,
) -> Result<UserFunctionRegistry, ShapesProductError> {
    let declarations = ast::custom_functions(shapes)?;
    let mut registry = host.functions().clone();
    link::register_expression_bodied_functions(&declarations, &mut registry);
    Ok(registry)
}

/// Refuse when assembly loses a DECLARED function the shapes graph really has.
///
/// The declarative AST carries the custom node-expression declarations and nothing
/// else about the function registry, so the only declared functions a restore can
/// reinstate are the expression-bodied ones SHACL 1.2 SPARQL Extensions §7.3 asks an
/// engine to register. A `sh:SPARQLFunction` declaration is parse output too, and it
/// has no home in the AST section — a product carrying one would restore a shapes
/// graph whose `ex:f(?x)` call sites resolve to nothing.
///
/// Comparing the two registries' [`FnPopulation::Declared`] fingerprints is a total
/// statement of that condition rather than a probe for the one case known today: any
/// future declared-function kind that assembly cannot reproduce fails here without
/// anyone remembering to add a branch. The injected population is deliberately not
/// compared — it is the host's, identical on both sides by construction.
///
/// # Errors
///
/// [`ProductDimension::UnsupportedCapability`] when the populations differ;
/// [`ProductDimension::FunctionRegistry`] when a registry cannot be fingerprinted.
pub(super) fn verify_declared_functions(
    parsed: &UserFunctionRegistry,
    assembled: &UserFunctionRegistry,
) -> Result<(), ShapesProductError> {
    let want = declared_fingerprint(parsed)?;
    let got = declared_fingerprint(assembled)?;
    if want == got {
        return Ok(());
    }
    Err(ShapesProductError::new(
        ProductDimension::UnsupportedCapability,
        "this shapes graph declares SPARQL functions that a prepared product cannot carry — only \
         the SHACL 1.2 §7.3 expression-bodied declarations (`sh:ListParameterExpressionFunction`) \
         survive a restore, and a `sh:SPARQLFunction` does not; remove the declaration or use the \
         shapes graph directly, because a product that restored without it would resolve every \
         call site of that function to nothing and validate green",
    ))
}

/// One registry's declared-population content fingerprint, or a refusal naming the
/// registry that could not state its own contents.
fn declared_fingerprint(
    registry: &UserFunctionRegistry,
) -> Result<purrdf_core::ContentDigest, ShapesProductError> {
    user_fn::content_fingerprint(registry, FnPopulation::Declared).map_err(|error| {
        ShapesProductError::new(
            ProductDimension::FunctionRegistry,
            format!(
                "a SPARQL function registry could not be fingerprinted, so nothing could check \
                 that the restore kept every function the shapes graph declares; supply a \
                 registry whose declarations can be read (registry reported: {error})"
            ),
        )
    })
}
