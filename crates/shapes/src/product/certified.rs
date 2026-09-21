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
//! | [`from_admitted`] | nothing — the class analysis travels in the product | the product's declared [`Identity`], the carried analysis included, is the identity of the environment about to execute it |
//! | [`from_rebuilt`] | the whole shapes graph, from the carried dataset, and the class analysis with it | nothing further — the derivation from the authenticated dataset IS the binding |
//!
//! The asymmetry over the class analysis is deliberate and it is what makes
//! [`from_rebuilt`] a remedy. A rebuild exists for the reader that meets a
//! preparation stage it does not know, and the stage id covers the class walk's own
//! source, so a product it must rescue is by definition one whose carried analysis
//! was produced by a different reachability rule. Reading that body would be
//! serving a stale analysis; re-deriving it is the only answer, and it is free
//! here because the rebuild is already re-deriving the entire shape tree.
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
use crate::plan::ClassCatalog;
use crate::provenance::{ProductRestore, ValidatorProvenance};
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
    /// The ADMIT seam's final check: install the host's bindings and prove the
    /// product's declared [`Identity`] is the identity of the environment about to
    /// execute it — including that the class analysis the product CARRIED is the
    /// analysis that identity pins.
    ///
    /// `classes` arrives decoded from the product's own AST section rather than
    /// derived here, and that is the point of the section carrying it: the class
    /// walk is the "repeated shared analysis" a prepared product exists to
    /// eliminate. It is checked, not trusted — it is row 10 of the identity, so
    /// handing the carried catalog to [`identity::check_restored_identity`] IS the
    /// comparison against the digest the product pinned, and a body that is not the
    /// one written refuses on [`ProductDimension::ClassCatalog`].
    ///
    /// The check runs BEFORE a [`PreparedShapes`] is assembled around the catalog,
    /// not after. Nothing partial escapes either way — this constructor's result is
    /// the only way out — but building a preparation over an analysis that has not
    /// been bound yet is the shape of the defect this gate exists to rule out, and
    /// the order costs nothing.
    ///
    /// # Errors
    ///
    /// The [`ProductDimension`] of the first identity component that disagrees,
    /// [`ProductDimension::ClassCatalog`] when the carried analysis is not the one
    /// the identity pins, or the installation refusals [`install`] reports.
    pub(super) fn from_admitted(
        declared: &Identity,
        mut shapes: Shapes,
        host: &HostBindings<'_>,
        classes: ClassCatalog,
    ) -> Result<Self, ShapesProductError> {
        install(&mut shapes, host)?;

        let shapes = Arc::new(shapes);
        let classes = Arc::new(classes);
        identity::check_restored_identity(
            declared,
            &shapes,
            host.property_functions(),
            host.implementation_identity(),
            &classes,
        )?;
        Ok(Self {
            prepared: PreparedShapes::with_carried_analysis(
                shapes,
                ValidatorProvenance::Restored {
                    identity: declared.clone(),
                    restore: ProductRestore::Admitted,
                },
                classes,
            ),
        })
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
    /// `declared` is RECORDED, not checked — it is the identity the product's own
    /// authenticated identity region carries, and it is what lets a rebuilt
    /// preparation still name the artifact it came from. The distinction between a
    /// recorded identity and a verified one is carried in the type rather than left
    /// to prose: this seam records [`ProductRestore::Rebuilt`], whose documentation
    /// states exactly what has and has not been proven about the value beside it.
    ///
    /// The class analysis the product carried is ignored here for the same reason,
    /// and it is the one place in the codec where ignoring a carried value is the
    /// correct move: [`PreparedShapes::with_provenance`] re-derives it from the
    /// shapes this seam just re-derived, so the analysis is a function of the
    /// authenticated dataset exactly like everything else on this path. A rebuild
    /// that read the carried body would take an analysis from the build whose stage
    /// it could not recognise and execute against it — the stale-analysis failure,
    /// arriving through the door that exists to prevent it.
    ///
    /// # Errors
    ///
    /// The installation refusals [`install`] reports.
    pub(super) fn from_rebuilt(
        declared: &Identity,
        mut shapes: Shapes,
        host: &HostBindings<'_>,
    ) -> Result<Self, ShapesProductError> {
        install(&mut shapes, host)?;
        Ok(Self {
            prepared: PreparedShapes::with_provenance(
                Arc::new(shapes),
                ValidatorProvenance::Restored {
                    identity: declared.clone(),
                    restore: ProductRestore::Rebuilt,
                },
            ),
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
/// declaration walk refuses or the carried shapes dataset does not re-derive its own
/// SPARQL function declarations.
fn install(shapes: &mut Shapes, host: &HostBindings<'_>) -> Result<(), ShapesProductError> {
    let assembled = assemble_functions(shapes, host)?;
    verify_declared_functions(&shapes.functions, &assembled)?;
    shapes.functions = Arc::new(assembled);
    shapes.aggregates = Arc::new(host.aggregates().clone());
    Ok(())
}

/// The user-function registry a restore assembles: the host's injected table, plus
/// BOTH kinds of declaration the shapes graph itself states — the SPARQL-bodied
/// `sh:SPARQLFunction`s re-derived from the carried shapes dataset, and the
/// expression-bodied §7.3 declarations registered from the model.
///
/// The direction is forced and it is worth stating. `UserFunctionRegistry` exposes
/// registration but not enumeration — a registered native closure cannot be read
/// back out — so a host's table can never be merged INTO a parser-built registry.
/// It can only be the table the declarations are registered into, which is why
/// every restore path starts from the host's registry rather than from an empty one.
///
/// # Why the two kinds come from two places
///
/// They are recovered from the two different things a product carries, because that
/// is where each one actually lives. The expression-bodied declarations ARE model —
/// their bodies are node expressions the AST section encodes — so they come off the
/// decoded model. A `sh:SPARQLFunction`'s body is query text in the shapes graph,
/// and the shapes graph travels inside the product under the envelope's per-section
/// SHA-256 and whole-container digest, so it is re-parsed from there. Neither kind
/// needs the host's cooperation, which is what
/// [`FnPopulation::Declared`] means by "rebuildable at restore by re-parsing that
/// graph".
///
/// The order matches the parse's: SPARQL-bodied first, then expression-bodied. The
/// parser's own precedence rule — a node typed BOTH belongs to the declaring class
/// that gives it a body — is enforced inside the re-derivation, so the two sets are
/// disjoint and the order is a statement of intent rather than a tiebreak.
///
/// # Errors
///
/// [`ProductDimension::DepthLimit`] or [`ProductDimension::Malformed`] when the
/// declaration walk over `shapes` refuses, or when the carried shapes dataset no
/// longer re-parses as the shapes graph it claims to be.
pub(super) fn assemble_functions(
    shapes: &Shapes,
    host: &HostBindings<'_>,
) -> Result<UserFunctionRegistry, ShapesProductError> {
    let declarations = ast::custom_functions(shapes)?;
    let mut registry = host.functions().clone();
    crate::shapes::register_declared_sparql_functions(
        shapes.dataset(),
        shapes.provenance(),
        &mut registry,
    )
    .map_err(|error| {
        ShapesProductError::new(
            ProductDimension::Malformed,
            format!(
                "this shapes graph's SPARQL function declarations do not re-derive from the \
                 shapes dataset carried alongside them ({error}); re-prepare the product from a \
                 shapes graph this build parses, because the declarations and the dataset must \
                 describe one parse"
            ),
        )
    })?;
    link::register_expression_bodied_functions(&declarations, &mut registry);
    Ok(registry)
}

/// Refuse when assembly loses a DECLARED function the shapes graph really has.
///
/// A declared function is one the shapes graph's own content states — the
/// SPARQL-bodied `sh:SPARQLFunction`s and the expression-bodied declarations
/// SHACL 1.2 SPARQL Extensions §7.3 asks an engine to register — as opposed to the
/// native closures only a host can wire. [`assemble_functions`] reinstates both
/// kinds, so a restore that drops one is a defect rather than a documented limit;
/// this is what makes that defect loud instead of silent, because the symptom
/// otherwise is a restored shapes graph whose `ex:f(?x)` call sites resolve to
/// nothing and validate green.
///
/// Comparing the two registries' [`FnPopulation::Declared`] fingerprints is a TOTAL
/// statement of that condition rather than a probe for the kinds known today: a
/// future declared-function kind that assembly cannot reproduce fails here without
/// anyone remembering to add a branch. The injected population is deliberately not
/// compared — it is the host's, identical on both sides by construction.
///
/// # What the comparison does and does not cover
///
/// The fingerprint binds each declaration's IRI, kind, arity, parameter variables
/// and type constraints; it deliberately does not digest a body, because the only
/// stable byte form for a parsed query is the algebra serializer's rendering and
/// making that load-bearing would turn a wording change in an unrelated module into
/// a silent invalidation of every product ever written. The binding that DOES cover
/// bodies is the shapes graph itself: every declared entry is a function of that
/// graph's content, and the graph is identity component 0 and a digested section of
/// the container.
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
        "this shapes graph declares a SPARQL function that a restore of this build does not \
         reinstate, so a product written from it would resolve that function's call sites to \
         nothing and validate green; use the shapes graph directly until this build carries the \
         declaration, because a product may only promise what its restore reproduces",
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
