// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PRODUCT-MODEL CENSUS — the closed structure a SHACL prepared product must cover.
//!
//! A prepared-product codec writes a parsed [`Shapes`] out and reads it back. The
//! failure that codec shape invites is the QUIET one: a variant nobody taught the
//! writer about is skipped, the bytes still verify, the reader still loads, and a
//! shape that should have constrained the data simply is not there. Nothing is
//! broken enough to notice. So "never silently omit a shape" is made structural
//! here rather than promised in a comment, in two rules:
//!
//!   RULE 1 — TOTALITY. [`census`] reads the declarative model out of
//!            `crates/shapes/src/**/*.rs` with `syn` and reports every type, every
//!            variant and every field a codec has to handle. A model type that
//!            gains a variant gains a census row, and gains a stage id under
//!            RULE 2. The codec's own totality is enforced where it belongs — by
//!            the compiler, through wildcard-free matches in the encoder and
//!            exhaustive struct construction in the decoder, so a new variant or
//!            field does not build until the codec handles it.
//!
//!            The census list is not an enumeration someone maintains by hand. It
//!            is CHECKED against the transitive closure of the model reachable from
//!            [`Shapes`] through public fields
//!            ([`census_closure_is_complete`]) — an enumeration silently omits the
//!            types nobody happened to think of, and here that set is load-bearing:
//!            `Shapes::target_types` carries the `sh:SPARQLTargetType` declarations
//!            and `Rule` / `RuleSchedule` carry the SHACL-AF rules.
//!
//!   RULE 2 — STAGE ID. [`stage_id`] is a content-derived capability digest over
//!            the whole census plus the tables the model's MEANING depends on. It
//!            is not hand-incremented, because a hand-incremented version is
//!            exactly how an authenticated cache serves stale-but-verified wrong
//!            answers: the bytes verify, the digest matches, and the meaning moved
//!            underneath both. Here the digest IS the meaning, so it cannot.
//!
//! # The closure's frontier
//!
//! The walk crosses public fields and stops at types this crate does not define.
//! That frontier is derived, not declared: `Arc`, `BTreeMap`, `String`,
//! `purrdf_sparql_eval::UserFunctionRegistry` and
//! `purrdf_core::xsd_regex::CompiledPattern` are outside
//! `crates/shapes/src`, so the scan never sees a definition for them and they are
//! leaves. Every type the crate DOES define and that a public field can reach —
//! including the RDF term primitives in `term.rs` — is in the census, because a
//! codec has to write those too.
//!
//! `pub(crate)` fields are deliberately not crossed. `Shapes::shapes_dataset` and
//! `Shapes::parse_provenance` are reconstructed rather than carried, and a caller
//! cannot name them at all.
//!
//! # Not vacuous
//!
//! The `detector_self_tests` module below proves the scan is not asleep: a
//! synthetic source carrying an undeclared reachable type MUST be flagged, a
//! synthetic added variant MUST appear in the census rows and MUST move the stage
//! id, and `#[cfg(test)]` items MUST NOT be seen at all.
//!
//! The mirror checks are here too, because a census that fired on prose would be
//! re-pinned reflexively and stop being read: a reworded doc comment must NOT move
//! the stage id, and a duplicate type name nothing reaches must NOT be refused —
//! `crates/shapes/src` really does declare several, in emitters no shape model
//! touches.
//!
//! # What this file deliberately does NOT assert
//!
//! It does not re-check that the codec has a branch per census row. That was tried
//! and removed: the compiler already enforces it — the decoder constructs each model
//! type exhaustively, so a new field does not build until the codec reads it, and the
//! encoder's matches carry no wildcard arm. A second, hand-maintained declaration of
//! the same fact is the duplication this census exists to eliminate, not an extra
//! layer of it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::visit::{self, Visit};

// ── The declared census ─────────────────────────────────────────────────────────

/// The root the model closure is computed from: the whole parsed shapes graph.
const CENSUS_ROOT: &str = "Shapes";

/// THE CENSUS: every type of the declarative SHACL model a prepared-product codec
/// must cover.
///
/// Checked for equality against the closure in [`census_closure_is_complete`], so
/// this list cannot go stale in either direction — a reachable type missing from it
/// fails, and a row here that the model no longer reaches fails too.
const CENSUS_TYPES: [&str; 27] = [
    "ArgKey",
    "BoxRoleVocab",
    "ComponentValidator",
    "Constraint",
    "CustomFnKind",
    "CustomFunction",
    "FnCall",
    "Literal",
    "NamedNode",
    "NodeExpr",
    "NodeKindValue",
    "OrderKey",
    "Path",
    "PropertyShape",
    "Rule",
    "RuleBody",
    "RuleSchedule",
    "Severity",
    "Shape",
    "ShapeArg",
    "Shapes",
    "SparqlCallForm",
    "SparqlTargetType",
    "Target",
    "TargetTypeParam",
    "Term",
    "Triple",
];

/// Census rows that no public field of the model can reach, admitted on purpose.
///
/// `SparqlCallForm` is the lowering table for SHACL 1.2 Node Expressions §5
/// `sparql:<NAME>` calls. Its OUTPUT is baked into `FnCall::Sparql::expr` at
/// shapes-load, so no field of a parsed `Shapes` holds one — but the table decides
/// what that baked text SAYS. Changing `sparql:add` from `Infix("+")` to anything
/// else changes the meaning of every prepared product that carries a lowered call,
/// while leaving every reachable field byte-identical. That is precisely the stale
/// meaning RULE 2 exists to catch, so the form set is censused and digested.
///
/// A row is only admitted here with a reason of that kind. Every other row must
/// earn its place by being reachable.
const UNREACHABLE_BY_DESIGN: [&str; 1] = ["SparqlCallForm"];

/// The profile this census describes, mixed into [`stage_id`].
///
/// Two builds that agree on every table but describe different profiles must not
/// share a stage id.
const PROFILE_ID: &str = "purrdf-shacl-core-v1";

/// The stage id this build SHIPS, as lowercase hex.
///
/// Derived from `purrdf_shapes::product::STAGE_ID` rather than copied beside it. A
/// second copy of the digest could drift from the constant products are actually
/// written under, and a stage id that no longer describes the model is the
/// stale-but-verified failure the derivation exists to prevent. One digest, read from
/// the place it ships from. A change to it is a change to what a prepared product
/// MEANS, and every product minted under the old id describes a different model.
fn shipped_stage_id() -> String {
    purrdf_shapes::product::STAGE_ID
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

// ── Census data model ───────────────────────────────────────────────────────────

/// Whether a census row was read off a `struct` or an `enum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeKind {
    /// A `struct` — one implicit variant holding every field.
    Struct,
    /// An `enum` — one variant per arm, each with its own fields.
    Enum,
}

impl TypeKind {
    /// The keyword this kind is spelled with, as it appears in a digest line.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Enum => "enum",
        }
    }
}

/// One field of a struct or of an enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The declared field name, or the positional index for a tuple field.
    pub name: String,
    /// A structural rendering of the field's type (see [`type_signature`]).
    pub signature: String,
    /// Whether a caller outside this crate can read the field. Enum variant
    /// fields are always public; struct fields need `pub` (not `pub(crate)`).
    pub public: bool,
    /// Every named type the field's type mentions, in source order.
    pub mentions: Vec<String>,
}

/// One variant of an enum, or the single implicit variant of a struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The variant name, or the empty string for a struct's implicit variant.
    pub name: String,
    /// The variant's fields, in declaration order.
    pub fields: Vec<Field>,
}

/// One census row: a model type with everything a codec has to cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelType {
    /// The type's name.
    pub name: String,
    /// The repository-relative file the type is declared in.
    pub file: String,
    /// Whether the type is a struct or an enum.
    pub kind: TypeKind,
    /// The variants, in declaration order. Variant ORDER is significant: a codec
    /// that numbers its arms by position changes meaning when they are reordered.
    pub variants: Vec<Variant>,
}

/// A set of Rust sources to census, as `(repository-relative path, text)` pairs.
pub type Sources = Vec<(String, String)>;

/// Everything the scan read out of `crates/shapes/src`.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    /// Every struct and enum found, keyed by name. A name declared more than once
    /// maps to more than one row and is refused if anything reaches it.
    pub types: BTreeMap<String, Vec<ModelType>>,
    /// Every `type X = …` alias found, keyed by name, mapping to the named types
    /// its target mentions. Aliases are transparent to the closure.
    pub aliases: BTreeMap<String, Vec<String>>,
}

impl Scan {
    /// The single definition of `name`, or `None` when this crate declares none.
    ///
    /// # Panics
    ///
    /// Panics when the crate declares `name` more than once: the census is keyed by
    /// bare name, so an ambiguous name has no answer and guessing one would be the
    /// silent omission this gate exists to prevent.
    #[must_use]
    pub fn definition(&self, name: &str) -> Option<&ModelType> {
        let found = self.types.get(name)?;
        assert!(
            found.len() == 1,
            "the model census reaches `{name}`, but `crates/shapes/src` declares it {} times ({}); \
             rename one, or the census cannot say which type a codec must cover",
            found.len(),
            found
                .iter()
                .map(|row| row.file.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        found.first()
    }

    /// The census rows for `names`, sorted by name.
    ///
    /// # Panics
    ///
    /// Panics when a requested name is not declared in `crates/shapes/src`.
    #[must_use]
    pub fn rows(&self, names: &[&str]) -> Vec<ModelType> {
        names
            .iter()
            .map(|name| {
                self.definition(name)
                    .unwrap_or_else(|| {
                        panic!("census row `{name}` names no struct or enum in `crates/shapes/src`")
                    })
                    .clone()
            })
            .collect()
    }

    /// The transitive closure of types reachable from `root` through PUBLIC fields.
    ///
    /// Enum variant fields are always public. Struct fields count only when
    /// declared `pub`. The walk is transparent through type aliases and stops at
    /// every type this crate does not declare.
    ///
    /// # Panics
    ///
    /// Panics when `root` is not declared, or when a reachable name is ambiguous.
    #[must_use]
    pub fn closure(&self, root: &str) -> BTreeSet<String> {
        assert!(
            self.definition(root).is_some(),
            "the census root `{root}` is not declared in `crates/shapes/src`"
        );
        let mut seen = BTreeSet::new();
        let mut queue = vec![root.to_owned()];
        while let Some(name) = queue.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(row) = self.definition(&name) else {
                continue;
            };
            for variant in &row.variants {
                for field in &variant.fields {
                    if !field.public && row.kind == TypeKind::Struct {
                        continue;
                    }
                    for mention in self.expand(&field.mentions) {
                        if self.definition(&mention).is_some() && !seen.contains(&mention) {
                            queue.push(mention);
                        }
                    }
                }
            }
        }
        seen
    }

    /// `mentions` with every type alias replaced by the names its target mentions.
    ///
    /// Bounded by the number of aliases so a (non-compiling) cyclic alias chain
    /// cannot hang the gate.
    fn expand(&self, mentions: &[String]) -> Vec<String> {
        let mut out: Vec<String> = mentions.to_vec();
        for _ in 0..=self.aliases.len() {
            let mut next = Vec::with_capacity(out.len());
            let mut changed = false;
            for name in out {
                if let Some(target) = self.aliases.get(&name) {
                    next.extend(target.iter().cloned());
                    changed = true;
                } else {
                    next.push(name);
                }
            }
            out = next;
            if !changed {
                break;
            }
        }
        out
    }
}

// ── Type rendering ──────────────────────────────────────────────────────────────

/// A structural rendering of `ty`, plus every named type it mentions.
///
/// The rendering is a total, parenthesised walk rather than the source text,
/// because `syn` does not retain the text. It distinguishes what a codec cares
/// about: `Option<Box<Shape>>` and `Box<Option<Shape>>` render differently,
/// `Vec<Shape>` and `Box<Shape>` render differently, and `u64` narrowing to `u32`
/// renders differently. Doc comments and formatting do not appear in it at all.
///
/// # Panics
///
/// Panics on a macro-typed or `verbatim` field. Those are opaque to any structural
/// scan, so the census refuses them loudly instead of censusing a type it cannot
/// read — a silently-skipped field is the exact defect this file exists to stop.
#[must_use]
pub fn type_signature(ty: &syn::Type) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut mentions = Vec::new();
    render_type(ty, &mut out, &mut mentions);
    (out, mentions)
}

fn render_type(ty: &syn::Type, out: &mut String, mentions: &mut Vec<String>) {
    match ty {
        syn::Type::Array(array) => {
            out.push_str("array(");
            render_type(&array.elem, out, mentions);
            out.push(';');
            render_expr(&array.len, out);
            out.push(')');
        }
        syn::Type::BareFn(bare) => {
            out.push_str("fn(");
            for input in &bare.inputs {
                render_type(&input.ty, out, mentions);
                out.push(',');
            }
            if let syn::ReturnType::Type(_, ret) = &bare.output {
                out.push_str("->");
                render_type(ret, out, mentions);
            }
            out.push(')');
        }
        // Invisible grouping and explicit parentheses carry no meaning of their own.
        syn::Type::Group(group) => render_type(&group.elem, out, mentions),
        syn::Type::Paren(paren) => render_type(&paren.elem, out, mentions),
        syn::Type::ImplTrait(imp) => {
            out.push_str("impl(");
            render_bounds(&imp.bounds, out, mentions);
            out.push(')');
        }
        syn::Type::TraitObject(obj) => {
            out.push_str("dyn(");
            render_bounds(&obj.bounds, out, mentions);
            out.push(')');
        }
        syn::Type::Infer(_) => out.push_str("infer"),
        syn::Type::Never(_) => out.push_str("never"),
        syn::Type::Path(path) => {
            out.push_str("path(");
            if let Some(qself) = &path.qself {
                out.push('<');
                render_type(&qself.ty, out, mentions);
                out.push('>');
            }
            render_path(&path.path, out, mentions);
            out.push(')');
        }
        syn::Type::Ptr(ptr) => {
            out.push_str(if ptr.mutability.is_some() {
                "ptrmut("
            } else {
                "ptr("
            });
            render_type(&ptr.elem, out, mentions);
            out.push(')');
        }
        syn::Type::Reference(reference) => {
            out.push_str(if reference.mutability.is_some() {
                "refmut("
            } else {
                "ref("
            });
            if let Some(lifetime) = &reference.lifetime {
                let _ = write!(out, "'{},", lifetime.ident);
            }
            render_type(&reference.elem, out, mentions);
            out.push(')');
        }
        syn::Type::Slice(slice) => {
            out.push_str("slice(");
            render_type(&slice.elem, out, mentions);
            out.push(')');
        }
        syn::Type::Tuple(tuple) => {
            out.push_str("tuple(");
            for elem in &tuple.elems {
                render_type(elem, out, mentions);
                out.push(',');
            }
            out.push(')');
        }
        other => panic!(
            "the product-model census cannot read the field type `{}` structurally; express it as \
             a named type so the census can state what a codec must cover",
            opaque_description(other)
        ),
    }
}

/// Name an opaque field type in a refusal, so the message says what it refused.
///
/// A macro type and a `verbatim` type are both raw [`proc_macro2::TokenStream`]s
/// that `syn` declined to interpret; quoting the tokens is the only way the
/// diagnostic can point at the actual field.
fn opaque_description(ty: &syn::Type) -> String {
    let tokens = |stream: &TokenStream| stream.to_string();
    match ty {
        syn::Type::Macro(mac) => format!(
            "{}!({})",
            mac.mac
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            tokens(&mac.mac.tokens)
        ),
        syn::Type::Verbatim(stream) => tokens(stream),
        _ => "<unknown syn::Type form>".to_owned(),
    }
}

fn render_bounds(
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
    out: &mut String,
    mentions: &mut Vec<String>,
) {
    for bound in bounds {
        match bound {
            syn::TypeParamBound::Trait(tr) => render_path(&tr.path, out, mentions),
            syn::TypeParamBound::Lifetime(lifetime) => {
                let _ = write!(out, "'{}", lifetime.ident);
            }
            _ => out.push_str("<bound>"),
        }
        out.push('+');
    }
}

fn render_path(path: &syn::Path, out: &mut String, mentions: &mut Vec<String>) {
    if path.leading_colon.is_some() {
        out.push_str("::");
    }
    let last = path.segments.len().saturating_sub(1);
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            out.push_str("::");
        }
        out.push_str(&segment.ident.to_string());
        // Only the final segment names a type; the leading ones name modules and
        // crates, and recording them would make a module share a type's identity.
        if index == last {
            mentions.push(segment.ident.to_string());
        }
        match &segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(args) => {
                out.push('<');
                for arg in &args.args {
                    match arg {
                        syn::GenericArgument::Type(ty) => render_type(ty, out, mentions),
                        syn::GenericArgument::Lifetime(lifetime) => {
                            let _ = write!(out, "'{}", lifetime.ident);
                        }
                        syn::GenericArgument::Const(expr) => render_expr(expr, out),
                        syn::GenericArgument::AssocType(assoc) => {
                            let _ = write!(out, "{}=", assoc.ident);
                            render_type(&assoc.ty, out, mentions);
                        }
                        _ => out.push_str("<arg>"),
                    }
                    out.push(',');
                }
                out.push('>');
            }
            syn::PathArguments::Parenthesized(args) => {
                out.push('(');
                for input in &args.inputs {
                    render_type(input, out, mentions);
                    out.push(',');
                }
                if let syn::ReturnType::Type(_, ret) = &args.output {
                    out.push_str("->");
                    render_type(ret, out, mentions);
                }
                out.push(')');
            }
        }
    }
}

/// Render a const-position expression (an array length or a const generic).
///
/// Only the forms a data model uses are rendered; anything else is refused rather
/// than collapsed to a value the census cannot distinguish.
fn render_expr(expr: &syn::Expr, out: &mut String) {
    match expr {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(int) => out.push_str(int.base10_digits()),
            syn::Lit::Bool(b) => {
                let _ = write!(out, "{}", b.value);
            }
            syn::Lit::Str(s) => {
                let _ = write!(out, "{:?}", s.value());
            }
            _ => panic!(
                "the product-model census cannot read the literal in a const position of a field \
                 type; spell it as an integer, a bool or a string"
            ),
        },
        syn::Expr::Path(path) => {
            let mut sink = Vec::new();
            render_path(&path.path, out, &mut sink);
        }
        syn::Expr::Group(group) => render_expr(&group.expr, out),
        syn::Expr::Paren(paren) => render_expr(&paren.expr, out),
        _ => panic!(
            "the product-model census cannot read the const expression in a field type \
             structurally; spell it as a literal or a named const"
        ),
    }
}

// ── The source scan ─────────────────────────────────────────────────────────────

/// Whether `attrs` carries `#[cfg(test)]`.
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
    })
}

fn fields_of(fields: &syn::Fields, always_public: bool) -> Vec<Field> {
    let collect = |index: usize, field: &syn::Field| {
        let (signature, mentions) = type_signature(&field.ty);
        Field {
            name: field
                .ident
                .as_ref()
                .map_or_else(|| index.to_string(), ToString::to_string),
            signature,
            public: always_public || matches!(field.vis, syn::Visibility::Public(_)),
            mentions,
        }
    };
    match fields {
        syn::Fields::Named(named) => named
            .named
            .iter()
            .enumerate()
            .map(|(index, field)| collect(index, field))
            .collect(),
        syn::Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, field)| collect(index, field))
            .collect(),
        syn::Fields::Unit => Vec::new(),
    }
}

struct Collector<'a> {
    file: &'a str,
    scan: Scan,
}

impl Collector<'_> {
    fn push(&mut self, row: ModelType) {
        self.scan
            .types
            .entry(row.name.clone())
            .or_default()
            .push(row);
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        // Test-only items describe no product, so they are not censused and cannot
        // shadow a production type by name.
        if is_cfg_test(&module.attrs) {
            return;
        }
        visit::visit_item_mod(self, module);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if is_cfg_test(&item.attrs) {
            return;
        }
        self.push(ModelType {
            name: item.ident.to_string(),
            file: self.file.to_owned(),
            kind: TypeKind::Struct,
            variants: vec![Variant {
                name: String::new(),
                fields: fields_of(&item.fields, false),
            }],
        });
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        if is_cfg_test(&item.attrs) {
            return;
        }
        self.push(ModelType {
            name: item.ident.to_string(),
            file: self.file.to_owned(),
            kind: TypeKind::Enum,
            variants: item
                .variants
                .iter()
                .map(|variant| Variant {
                    name: variant.ident.to_string(),
                    // An enum variant's fields are as public as the enum itself.
                    fields: fields_of(&variant.fields, true),
                })
                .collect(),
        });
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        if is_cfg_test(&item.attrs) {
            return;
        }
        let (_, mentions) = type_signature(&item.ty);
        self.scan.aliases.insert(item.ident.to_string(), mentions);
    }
}

/// Scan `(relative path, source text)` pairs for the declarative model.
///
/// # Panics
///
/// Panics when a source does not parse as Rust.
#[must_use]
pub fn scan_sources(sources: &[(String, String)]) -> Scan {
    let mut scan = Scan::default();
    for (file, text) in sources {
        let parsed = syn::parse_file(text).unwrap_or_else(|error| {
            panic!("{file}: the product-model census cannot parse Rust source: {error}")
        });
        let mut collector = Collector {
            file,
            scan: Scan::default(),
        };
        collector.visit_file(&parsed);
        for (name, rows) in collector.scan.types {
            scan.types.entry(name).or_default().extend(rows);
        }
        scan.aliases.extend(collector.scan.aliases);
    }
    scan
}

// ── Repository sources ──────────────────────────────────────────────────────────

/// The workspace root.
///
/// # Panics
///
/// Panics when the workspace root cannot be resolved.
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every `crates/shapes/src/**/*.rs` source, as `(repository-relative path, text)`.
///
/// # Panics
///
/// Panics when the tree cannot be walked, or when the walk finds implausibly few
/// sources — a broken walker must not pass as a small crate.
#[must_use]
pub fn shapes_sources() -> Sources {
    let root = repo_root();
    let src = root.join("crates/shapes/src");
    let mut paths = Vec::new();
    collect_rs(&src, &mut paths);
    paths.sort();
    assert!(
        paths.len() > 20,
        "the scan found only {} sources under crates/shapes/src — the walker is broken, not the \
         crate",
        paths.len()
    );
    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .expect("source below the workspace root")
                .to_string_lossy()
                .replace('\\', "/");
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            (relative, text)
        })
        .collect()
}

// ── The public census surface ───────────────────────────────────────────────────

/// THE CENSUS: every census type's variants and fields, read out of the live
/// sources and sorted by type name.
///
/// This is the surface a codec-coverage gate consumes: for every row, every
/// variant, and every field, the codec must have a branch. That gate lands with
/// the codec — there is nothing to compare the census against until a codec
/// exists, and a coverage assertion with no codec behind it would pass for the
/// wrong reason, which is the failure mode this whole file is built against. What
/// is complete here is the census itself and the two rules that keep it honest.
///
/// # Panics
///
/// Panics when a census row names no type, or names an ambiguous one.
#[must_use]
pub fn census() -> Vec<ModelType> {
    scan_sources(&shapes_sources()).rows(&CENSUS_TYPES)
}

// ── Stage id ────────────────────────────────────────────────────────────────────

/// The `builtin_function_keyword` table: every name the SPARQL parser's built-in
/// table accepts, paired with the canonical keyword the resolver answers with.
///
/// SHACL 1.2 Node Expressions §5 resolves `sparql:<NAME>` through exactly this
/// seam at shapes-load, and the rendered text is what a prepared product carries.
/// A name entering or leaving the table, or resolving to a different keyword,
/// changes what a prepared product can mean.
///
/// # Panics
///
/// Panics when the parser's table cannot be located, or is implausibly small.
#[must_use]
pub fn builtin_function_table() -> Vec<(String, String)> {
    let path = repo_root().join("crates/sparql-algebra/src/parser.rs");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let parsed = syn::parse_file(&text).expect("the SPARQL parser source parses as Rust");

    struct Names(Vec<String>);
    impl<'ast> Visit<'ast> for Names {
        fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
            for arm in &node.arms {
                let mut patterns = vec![&arm.pat];
                while let Some(pattern) = patterns.pop() {
                    match pattern {
                        syn::Pat::Lit(lit) => {
                            if let syn::Lit::Str(name) = &lit.lit {
                                self.0.push(name.value());
                            }
                        }
                        syn::Pat::Or(or) => patterns.extend(or.cases.iter()),
                        _ => {}
                    }
                }
            }
            visit::visit_expr_match(self, node);
        }
    }

    let table = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "builtin_function" => Some(function),
            _ => None,
        })
        .expect("the SPARQL parser's `builtin_function` name table exists");
    let mut names = Names(Vec::new());
    names.visit_block(&table.block);
    let mut names = names.0;
    names.sort();
    names.dedup();
    assert!(
        names.len() > 50,
        "the scrape found only {} built-in names, which cannot be the whole table — the table's \
         shape changed and this census is no longer reading it",
        names.len()
    );
    names
        .into_iter()
        .map(|name| {
            let keyword = purrdf_sparql_algebra::builtin_function_keyword(&name)
                .map_or_else(|| "<none>".to_owned(), ToOwned::to_owned);
            (name, keyword)
        })
        .collect()
}

/// Every `pub const NAME: &str = "…"` declared inside `mod` `name` in `source`.
fn string_consts_in_module(source: &syn::File, module: &str) -> BTreeMap<String, String> {
    struct Consts {
        want: String,
        depth: usize,
        found: BTreeMap<String, String>,
    }
    impl<'ast> Visit<'ast> for Consts {
        fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
            if is_cfg_test(&node.attrs) {
                return;
            }
            let inside = node.ident == self.want.as_str();
            self.depth += usize::from(inside);
            visit::visit_item_mod(self, node);
            self.depth -= usize::from(inside);
        }
        fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
            if self.depth == 0 || is_cfg_test(&node.attrs) {
                return;
            }
            if let syn::Expr::Lit(lit) = node.expr.as_ref()
                && let syn::Lit::Str(value) = &lit.lit
            {
                self.found.insert(node.ident.to_string(), value.value());
            }
        }
    }
    let mut consts = Consts {
        want: module.to_owned(),
        depth: 0,
        found: BTreeMap::new(),
    };
    consts.visit_file(source);
    consts.found
}

/// The constraint-component parameter table: which components exist, and which of
/// their parameters SHACL makes single-valued.
///
/// Two halves, both read out of the live sources:
///
/// * every `sh:…ConstraintComponent` IRI `crates/shapes/src/model.rs` declares —
///   the component identities a validation result is reported under; and
/// * the native parameter-cardinality tables in
///   `crates/shapes/src/shapes/parser/cardinality.rs`, resolved to the IRIs they
///   name — which decide, with no vocabulary import, whether a second value of a
///   parameter is a load error or a second constraint.
///
/// * the spec symbol table (`purrdf_shapes::spec`), one line per fact in its own
///   canonical rendering — which spec terms bind natively, with which signature,
///   under which alias, and which declared components the engine refuses as
///   unimplemented.
///
/// Any of the three moving changes what a shapes graph parses INTO, so a prepared
/// product minted before the move describes a model that no longer exists.
///
/// # Panics
///
/// Panics when a table cannot be located or is implausibly small, or when a table
/// entry names a constant `model.rs` does not declare.
#[must_use]
pub fn constraint_component_parameter_table() -> Vec<(String, String)> {
    let root = repo_root();
    let model_path = root.join("crates/shapes/src/model.rs");
    let model_text = std::fs::read_to_string(&model_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", model_path.display()));
    let model = syn::parse_file(&model_text).expect("model.rs parses as Rust");
    let sh = string_consts_in_module(&model, "sh");
    assert!(
        sh.len() > 100,
        "the scrape found only {} sh: constants, which cannot be the whole vocabulary",
        sh.len()
    );

    let mut table: Vec<(String, String)> = sh
        .iter()
        .filter(|(name, _)| name.ends_with("_CONSTRAINT_COMPONENT"))
        .map(|(name, iri)| (format!("component {name}"), iri.clone()))
        .collect();
    assert!(
        table.len() >= 30,
        "the scrape found only {} sh:…ConstraintComponent constants, which cannot be SHACL Core \
         plus the AF components",
        table.len()
    );

    let cardinality_path = root.join("crates/shapes/src/shapes/parser/cardinality.rs");
    let cardinality_text = std::fs::read_to_string(&cardinality_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", cardinality_path.display()));
    let cardinality = syn::parse_file(&cardinality_text).expect("cardinality.rs parses as Rust");
    for want in ["SINGLETON_PREDICATES", "METADATA_SINGLETONS"] {
        let item = cardinality
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Const(konst) if konst.ident == want => Some(konst),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "the native parameter-cardinality table `{want}` is gone from \
                     crates/shapes/src/shapes/parser/cardinality.rs; the census is no longer \
                     reading what decides parameter cardinality"
                )
            });
        let syn::Expr::Reference(reference) = item.expr.as_ref() else {
            panic!("`{want}` is no longer a `&[…]` slice literal");
        };
        let syn::Expr::Array(array) = reference.expr.as_ref() else {
            panic!("`{want}` is no longer a `&[…]` slice literal");
        };
        assert!(
            !array.elems.is_empty(),
            "the parameter-cardinality table `{want}` is empty"
        );
        for (index, elem) in array.elems.iter().enumerate() {
            let syn::Expr::Path(path) = elem else {
                panic!("`{want}[{index}]` is not a `sh::CONST` path");
            };
            let name = path
                .path
                .segments
                .last()
                .expect("a path has a final segment")
                .ident
                .to_string();
            let iri = sh.get(&name).unwrap_or_else(|| {
                panic!("`{want}[{index}]` names `sh::{name}`, which model.rs does not declare")
            });
            table.push((format!("{want}[{index}] sh::{name}"), iri.clone()));
        }
    }
    let spec = purrdf_shapes::spec::implemented().canonical_text();
    assert!(
        spec.lines().count() > 100,
        "the spec symbol table rendered only {} lines, which cannot be the whole table",
        spec.lines().count()
    );
    for (index, line) in spec.lines().enumerate() {
        table.push((format!("spec[{index}]"), line.to_owned()));
    }
    table
}

/// The source file the class-analysis derivation lives in.
const CLASS_ANALYSIS_SOURCE: &str = "crates/shapes/src/plan.rs";

/// THE CLASS-ANALYSIS DERIVATION: every item that decides which classes a prepared
/// product's reusable analysis contains and which binding-row position each gets.
///
/// Each row is `(impl scope, item keyword, item name)`; an empty scope means a free
/// item at file scope.
///
/// # Why an ALGORITHM is censused here when nothing else is
///
/// Every other input to the stage id is a declaration: a type, a variant, a field,
/// a table entry. This one is a walk. It is here because a prepared product now
/// CARRIES the walk's result — a restore reads the class catalog instead of
/// recomputing it — and a carried analysis is only safe while the build that reads
/// it is the build that wrote it.
///
/// Nothing else could establish that. The walk's meaning lives entirely in function
/// bodies: `lower_shape` could stop descending into reifier shapes, or
/// `lower_expression` could stop collecting `shnex:instancesOf`, and not one model
/// type, variant, field or table entry would move. The census would be identical,
/// the stage id would be identical, and every product written by the older build
/// would still `admit` — serving that build's analysis to this build's validator,
/// verified and wrong. That hazard predates the catalog travelling; what carrying
/// the body changes is that it can no longer be left open.
///
/// The walk that decides the catalog is the SAME walk that lowers the shapes, so
/// the rows below are the lowering's own entry points: one visit per node produces
/// both answers, and a change to how a node is visited is a change to both.
///
/// # What is deliberately NOT in this table
///
/// `ClassCatalog::from_entries` (the decoder) and `class_catalog_digest` (the
/// binding) are both algorithms whose meaning matters, and neither is here, because
/// neither can go stale silently: the writer and the reader of a single product run
/// one build's copy of each, so a change to either makes an older product refuse
/// loudly on the class-catalog dimension rather than restore wrongly.
/// `LoweredShapes::bind` is excluded for the same reason — it consumes a catalog
/// rather than deciding one.
///
/// `lower_path` is likewise excluded: a path names no class, so it cannot change
/// which classes a product carries. It is part of the lowering, not of the class
/// analysis, and the stage id speaks for the analysis.
const CLASS_ANALYSIS_SITES: [(&str, &str, &str); 7] = [
    ("", "struct", "ShapeWalk"),
    ("ClassCatalog", "fn", "from_walk"),
    ("", "fn", "lower_shapes"),
    ("", "fn", "lower_shape"),
    ("", "fn", "lower_property"),
    ("", "fn", "lower_constraint"),
    ("", "fn", "lower_expression"),
];

/// The class-analysis derivation of the live repository sources.
///
/// # Panics
///
/// Panics when the source cannot be read — see [`class_analysis_table_from`] for
/// every other way a scrape refuses.
#[must_use]
pub fn class_analysis_table() -> Vec<(String, String)> {
    let path = repo_root().join(CLASS_ANALYSIS_SOURCE);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    class_analysis_table_from(&text)
}

/// Every [`CLASS_ANALYSIS_SITES`] row of `source`, rendered as normalized tokens.
///
/// The rendering is a token stream rather than source text, and that is what makes
/// it safe to digest. Rust's lexer drops `//` comments outright and folds `///`
/// into `#[doc = "…"]` attributes, which this scrape then discards, so reformatting
/// the walk or rewriting its commentary CANNOT move the stage id — a gate that
/// fired on prose would be re-pinned reflexively and stop being read. Attributes
/// that are not documentation are KEPT, because `#[derive]` on the scan
/// accumulator and `#[cfg]` on any of these items really do change what the walk
/// does.
///
/// # Panics
///
/// Panics when `source` does not tokenize, when a census row names an item that is
/// not there or is there more than once, or when a rendering comes back
/// implausibly small — every one of those is the scrape reading something other
/// than the derivation it claims to read, and a silent skip here would leave the
/// hazard this table exists to close wide open while the gate stayed green.
#[must_use]
pub fn class_analysis_table_from(source: &str) -> Vec<(String, String)> {
    let file: Vec<TokenTree> = TokenStream::from_str(source)
        .unwrap_or_else(|error| panic!("{CLASS_ANALYSIS_SOURCE} tokenizes: {error}"))
        .into_iter()
        .collect();
    let class_catalog = impl_body(&file, "ClassCatalog");

    CLASS_ANALYSIS_SITES
        .iter()
        .map(|&(scope, keyword, name)| {
            let (key, tokens) = if scope.is_empty() {
                (name.to_owned(), &file)
            } else {
                (format!("{scope}::{name}"), &class_catalog)
            };
            let rendered = item_tokens(tokens, keyword, name, &key);
            assert!(
                rendered.len() > 40,
                "the class-analysis scrape rendered `{key}` as {} characters, which cannot be the \
                 item it names; the scrape is reading something other than the derivation",
                rendered.len()
            );
            (key, rendered)
        })
        .collect()
}

/// The token trees inside `impl <type_name> { … }`, which must appear exactly once.
///
/// Scoping is not a nicety: `bind` is the name of two different functions in
/// `plan.rs` — `LoweredShapes`'s, which resolves a lowering against a dataset, and
/// `StandaloneLowering`'s, which does the same for one expression — and a scrape
/// that digested whichever it met first would be pinning the wrong algorithm. The
/// row this scoping actually carries is `ClassCatalog::from_walk`, the rule that
/// assigns each class its binding-row position.
///
/// # Panics
///
/// Panics when the impl block is absent or duplicated.
fn impl_body(tokens: &[TokenTree], type_name: &str) -> Vec<TokenTree> {
    let mut found: Option<Vec<TokenTree>> = None;
    for window in tokens.windows(3) {
        let [
            TokenTree::Ident(keyword),
            TokenTree::Ident(name),
            TokenTree::Group(body),
        ] = window
        else {
            continue;
        };
        if keyword != "impl" || name != type_name || body.delimiter() != Delimiter::Brace {
            continue;
        }
        assert!(
            found.replace(body.stream().into_iter().collect()).is_none(),
            "{CLASS_ANALYSIS_SOURCE} declares `impl {type_name}` more than once, so a scrape of it \
             cannot say which block it read"
        );
    }
    found.unwrap_or_else(|| {
        panic!(
            "{CLASS_ANALYSIS_SOURCE} no longer declares `impl {type_name}`; the class-analysis \
             census is reading nothing"
        )
    })
}

/// One `<keyword> <name> … { … }` item of `tokens`, rendered as normalized tokens
/// with its documentation stripped and its other attributes kept.
///
/// The item must occur exactly once at this nesting level. A call to a function
/// does not match, because the match requires the declaring keyword immediately
/// ahead of the name.
///
/// # Panics
///
/// Panics when the item is absent, duplicated, or never closes a braced body.
fn item_tokens(tokens: &[TokenTree], keyword: &str, name: &str, key: &str) -> String {
    let mut found: Option<String> = None;
    for index in 0..tokens.len().saturating_sub(1) {
        let (TokenTree::Ident(found_keyword), TokenTree::Ident(found_name)) =
            (&tokens[index], &tokens[index + 1])
        else {
            continue;
        };
        if found_keyword != keyword || found_name != name {
            continue;
        }
        let body = tokens[index..]
            .iter()
            .position(|token| {
                matches!(token, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace)
            })
            .unwrap_or_else(|| {
                panic!("`{key}` in {CLASS_ANALYSIS_SOURCE} never opens a braced body")
            });
        let rendered: TokenStream = undocumented_attributes(tokens, index)
            .into_iter()
            .chain(tokens[index..=index + body].iter().cloned())
            .collect();
        assert!(
            found.replace(rendered.to_string()).is_none(),
            "`{key}` is declared more than once in {CLASS_ANALYSIS_SOURCE}, so a scrape of it \
             cannot say which declaration it read"
        );
    }
    found.unwrap_or_else(|| {
        panic!(
            "{CLASS_ANALYSIS_SOURCE} no longer declares `{keyword} {name}`; the class-analysis \
             census is reading nothing where the derivation used to be"
        )
    })
}

/// The attribute tokens immediately ahead of `index`, minus the `#[doc = "…"]`
/// pairs a `///` comment lexes into.
fn undocumented_attributes(tokens: &[TokenTree], index: usize) -> Vec<TokenTree> {
    let mut start = index;
    while start >= 2
        && matches!(&tokens[start - 2], TokenTree::Punct(punct) if punct.as_char() == '#')
        && matches!(
            &tokens[start - 1],
            TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket
        )
    {
        start -= 2;
    }
    tokens[start..index]
        .chunks(2)
        .filter(|pair| !is_doc_attribute(&pair[1]))
        .flat_map(|pair| pair.iter().cloned())
        .collect()
}

/// Whether an attribute's bracketed body opens with `doc`.
fn is_doc_attribute(body: &TokenTree) -> bool {
    let TokenTree::Group(group) = body else {
        return false;
    };
    matches!(
        group.stream().into_iter().next(),
        Some(TokenTree::Ident(ident)) if ident == "doc"
    )
}

/// The exact bytes [`stage_id`] digests, one line per fact.
///
/// Kept separate from the digest so a failing gate can diff the PREIMAGE and say
/// which fact moved, rather than only that some fact did.
#[must_use]
pub fn stage_id_preimage(
    types: &[ModelType],
    builtins: &[(String, String)],
    components: &[(String, String)],
    analysis: &[(String, String)],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "purrdf-shacl-product-model-census/1");
    let _ = writeln!(out, "profile {PROFILE_ID}");
    for row in types {
        let _ = writeln!(
            out,
            "type {} {} @{}",
            row.kind.keyword(),
            row.name,
            row.file
        );
        for variant in &row.variants {
            let _ = writeln!(out, "  variant {}", variant.name);
            for field in &variant.fields {
                let _ = writeln!(
                    out,
                    "    field {} {} {}",
                    field.name,
                    if field.public { "pub" } else { "private" },
                    field.signature
                );
            }
        }
    }
    for (name, keyword) in builtins {
        let _ = writeln!(out, "builtin {name} -> {keyword}");
    }
    for (key, value) in components {
        let _ = writeln!(out, "parameter {key} = {value}");
    }
    for (key, value) in analysis {
        let _ = writeln!(out, "analysis {key} = {value}");
    }
    out
}

/// The stage id: a BLAKE3-256 digest of [`stage_id_preimage`], hex-encoded.
///
/// Content-derived on purpose. Nobody has to remember to bump it, so nobody can
/// forget to, and a prepared product minted under one id cannot be verified under
/// a build whose model, built-in table, component table, class-analysis derivation
/// or profile has moved.
#[must_use]
pub fn stage_id(
    types: &[ModelType],
    builtins: &[(String, String)],
    components: &[(String, String)],
    analysis: &[(String, String)],
) -> String {
    let preimage = stage_id_preimage(types, builtins, components, analysis);
    purrdf_gts::wire::hex(&purrdf_gts::wire::blake3_256(preimage.as_bytes()))
}

/// The stage id of the live repository sources.
#[must_use]
pub fn live_stage_id() -> String {
    stage_id(
        &census(),
        &builtin_function_table(),
        &constraint_component_parameter_table(),
        &class_analysis_table(),
    )
}

// ── RULE 1: totality ────────────────────────────────────────────────────────────

/// The declared census is exactly the model closure, in both directions.
///
/// A type the model reaches and the census omits is the enumeration going stale —
/// exactly the silent omission a codec would inherit. A census row nothing reaches
/// is dead weight, and dead weight is how an escape-hatch list grows until it stops
/// meaning anything.
#[test]
fn census_closure_is_complete() {
    let scan = scan_sources(&shapes_sources());
    let closure = scan.closure(CENSUS_ROOT);
    let declared: BTreeSet<String> = CENSUS_TYPES.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(
        declared.len(),
        CENSUS_TYPES.len(),
        "duplicate row in CENSUS_TYPES"
    );
    let exempt: BTreeSet<String> = UNREACHABLE_BY_DESIGN
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert!(
        exempt.is_subset(&declared),
        "UNREACHABLE_BY_DESIGN names a type that is not in the census at all"
    );
    assert!(
        exempt.is_disjoint(&closure),
        "UNREACHABLE_BY_DESIGN names a type the model DOES reach; delete the exemption rather \
         than carrying an untrue reason: {:?}",
        exempt.intersection(&closure).collect::<Vec<_>>()
    );

    let missing: Vec<&String> = closure.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "these types are reachable from `{CENSUS_ROOT}` through public fields but are not in the \
         census, so a prepared-product codec could omit them silently: {missing:?}"
    );
    let orphaned: Vec<&String> = declared
        .difference(&closure)
        .filter(|name| !exempt.contains(*name))
        .collect();
    assert!(
        orphaned.is_empty(),
        "these census rows are not reachable from `{CENSUS_ROOT}` and carry no recorded reason; \
         drop them, or give each one an UNREACHABLE_BY_DESIGN entry saying why it still matters: \
         {orphaned:?}"
    );

    // The closure is what makes the list non-vacuous, so it must be a real model,
    // not a root that happened to reach nothing.
    assert!(
        closure.len() > 20,
        "the closure found only {} types from `{CENSUS_ROOT}` — the walk is broken, not the model",
        closure.len()
    );
}

/// The census surface a codec-coverage gate consumes is populated and complete.
#[test]
fn census_rows_carry_every_variant_and_field() {
    let rows = census();
    assert_eq!(rows.len(), CENSUS_TYPES.len());

    let by_name: BTreeMap<&str, &ModelType> =
        rows.iter().map(|row| (row.name.as_str(), row)).collect();

    // Spot-check the rows the guarantees above name explicitly, so a scan that
    // returned structurally-valid empties cannot pass.
    let constraint = by_name["Constraint"];
    assert_eq!(constraint.kind, TypeKind::Enum);
    assert!(
        constraint.variants.len() > 25,
        "Constraint has {} variants — the scan is not reading the real enum",
        constraint.variants.len()
    );
    assert!(
        constraint
            .variants
            .iter()
            .any(|variant| variant.name == "Pattern"
                && variant.fields.iter().any(|field| field.name == "flags")),
        "Constraint::Pattern must carry its `flags` field"
    );

    let shapes = by_name["Shapes"];
    assert_eq!(shapes.kind, TypeKind::Struct);
    let fields = &shapes.variants[0].fields;
    assert!(
        fields
            .iter()
            .any(|field| field.name == "target_types" && field.public),
        "Shapes::target_types carries the sh:SPARQLTargetType declarations and must be censused"
    );
    assert!(
        fields
            .iter()
            .any(|field| field.name == "parse_provenance" && !field.public),
        "Shapes::parse_provenance is pub(crate) and must be censused as non-public"
    );

    assert_eq!(by_name["RuleSchedule"].variants.len(), 2);
    assert_eq!(by_name["RuleBody"].variants.len(), 2);
    assert!(
        by_name["Rule"].variants[0]
            .fields
            .iter()
            .any(|field| field.name == "schedule")
    );

    for row in &rows {
        assert!(
            !row.variants.is_empty(),
            "census row {} has no variants at all",
            row.name
        );
        assert!(
            row.file.starts_with("crates/shapes/src/"),
            "census row {} was read from {}, outside the shapes crate",
            row.name,
            row.file
        );
    }
}

// ── RULE 2: stage id ────────────────────────────────────────────────────────────

/// The stage id of the live sources is the one this build SHIPS.
///
/// The comparison is against `purrdf_shapes::product::STAGE_ID` — the constant every
/// product is actually written under — rather than against a second copy of the digest
/// kept here. A local copy would let the two drift: the model changes, this test fails,
/// the local copy is updated, and the shipped constant silently keeps stamping products
/// with a stage id describing a model this build no longer has. That is precisely the
/// stale-but-verified failure a derived stage id exists to prevent, so there is exactly
/// one digest and this is the assertion that binds it to the sources.
///
/// A stage id that moves also invalidates `tests/fixtures/prepared-shapes-core.product`,
/// because every product this build writes is stamped with it. Re-preparing that artifact
/// has a supported command —
/// `crates/shapes/tests/product_determinism.rs::regenerate_the_prepared_product_fixture`,
/// `#[ignore]`d and run by name — and the failure message below names it. It is a target
/// rather than something each change improvises so that the artifact is never written by
/// a scratch binary nobody can identify afterwards.
#[test]
fn stage_id_matches_shipped_constant() {
    let computed = live_stage_id();
    let shipped = shipped_stage_id();
    assert_eq!(
        computed, shipped,
        "the SHACL prepared-product stage id moved. Something in the declarative model, the \
         SPARQL built-in table, the constraint-component parameter table (spec symbol table \
         included), the class-analysis derivation or the profile id changed, which means every product written under \
         `{shipped}` describes a preparation this build no longer performs. Update STAGE_ID in \
         the product module to `{computed}` ONLY after confirming the codec covers the change, \
         then re-prepare the product fixture the new stage id invalidates with the supported \
         command — `cargo test -p purrdf-shapes --test product_determinism -- --ignored --exact \
         --nocapture regenerate_the_prepared_product_fixture` — and set GOLDEN_LEN in that file \
         to the length it prints. Do NOT hand-roll a throwaway writer for that step."
    );
}

/// The digest is derived from facts, not from a constant: the same census digests
/// the same way twice, and the preimage is a readable diff surface.
#[test]
fn stage_id_is_reproducible_and_its_preimage_is_readable() {
    assert_eq!(live_stage_id(), live_stage_id());

    let preimage = stage_id_preimage(
        &census(),
        &builtin_function_table(),
        &constraint_component_parameter_table(),
        &class_analysis_table(),
    );
    assert!(preimage.starts_with("purrdf-shacl-product-model-census/1\n"));
    assert!(preimage.contains(&format!("profile {PROFILE_ID}\n")));
    assert!(preimage.contains("type enum Constraint @crates/shapes/src/shapes.rs\n"));
    assert!(preimage.contains("builtin STRLEN -> STRLEN\n"));
    assert!(preimage.contains(
        "parameter component MIN_COUNT_CONSTRAINT_COMPONENT = \
         http://www.w3.org/ns/shacl#MinCountConstraintComponent\n"
    ));
    assert!(preimage.contains("SINGLETON_PREDICATES[0] sh::DATATYPE = "));
    assert!(preimage.contains(
        "= function http://www.w3.org/ns/shacl#SPARQLExprExpression \
         http://www.w3.org/ns/shacl#NamedParameterExpressionFunction keyed \
         http://www.w3.org/ns/shacl#sparqlExpr -> select\n"
    ));
    assert!(preimage.contains("analysis ClassCatalog::from_walk = fn from_walk"));
    assert!(preimage.contains("analysis lower_shape = fn lower_shape"));
    // A non-documentation attribute survives, because `#[derive(Default)]` on the
    // walk accumulator really is part of how the walk starts.
    assert!(preimage.contains("analysis ShapeWalk = # [derive (Default)] struct ShapeWalk"));
    // The walk's own commentary does NOT, in either comment form — a census that
    // fired on prose would be re-pinned reflexively and stop being read.
    for prose in [
        "The accumulator the one total shape walk carries",
        "A SPARQL-based node expression",
    ] {
        assert!(
            !preimage.contains(prose),
            "the commentary `{prose}` reached the stage-id preimage, so the class-analysis scrape \
             is digesting prose rather than the derivation"
        );
    }
}

/// The profile id is mixed in: two builds that agree on every table but describe
/// different profiles must not share a stage id.
#[test]
fn stage_id_depends_on_the_profile_id() {
    let types = census();
    let builtins = builtin_function_table();
    let components = constraint_component_parameter_table();
    let analysis = class_analysis_table();
    let real = stage_id_preimage(&types, &builtins, &components, &analysis);
    let other = real.replace(
        &format!("profile {PROFILE_ID}"),
        "profile purrdf-shacl-core-v2",
    );
    assert_ne!(real, other, "the profile line must appear in the preimage");
    assert_ne!(
        purrdf_gts::wire::hex(&purrdf_gts::wire::blake3_256(other.as_bytes())),
        live_stage_id()
    );
}

/// Adding a variant to `Constraint` moves the stage id.
///
/// Driven from a SYNTHETIC source — the real `shapes.rs` is read, patched in
/// memory, and rescanned. Nothing on disk is touched, so the check costs no edit
/// and cannot rot into "the test that passed once".
#[test]
fn stage_id_changes_when_a_variant_is_added() {
    let (real, patched) = patched_sources(
        "crates/shapes/src/shapes.rs",
        "pub enum Constraint {",
        "pub enum Constraint {\n    /// A synthetic census probe.\n    CensusProbe(bool),",
    );
    let builtins = builtin_function_table();
    let components = constraint_component_parameter_table();
    let analysis = class_analysis_table();
    let before = stage_id(
        &scan_sources(&real).rows(&CENSUS_TYPES),
        &builtins,
        &components,
        &analysis,
    );
    let after = stage_id(
        &scan_sources(&patched).rows(&CENSUS_TYPES),
        &builtins,
        &components,
        &analysis,
    );
    assert_eq!(before, shipped_stage_id());
    assert_ne!(
        before, after,
        "a new Constraint variant left the stage id unchanged; the digest is not reading the model"
    );
}

/// Adding a variant to `RuleBody` moves the stage id.
///
/// The rules half is checked separately because it is reached through a different
/// file and a different field chain (`Shapes` → `Shape::rules` → `Rule::body`), and
/// a walk that lost that edge would still pass the `Constraint` case.
#[test]
fn stage_id_changes_when_a_rule_variant_is_added() {
    let (real, patched) = patched_sources(
        "crates/shapes/src/rules.rs",
        "pub enum RuleBody {",
        "pub enum RuleBody {\n    /// A synthetic census probe.\n    CensusProbe(bool),",
    );
    let builtins = builtin_function_table();
    let components = constraint_component_parameter_table();
    let analysis = class_analysis_table();
    let before = stage_id(
        &scan_sources(&real).rows(&CENSUS_TYPES),
        &builtins,
        &components,
        &analysis,
    );
    let after = stage_id(
        &scan_sources(&patched).rows(&CENSUS_TYPES),
        &builtins,
        &components,
        &analysis,
    );
    assert_eq!(before, shipped_stage_id());
    assert_ne!(
        before, after,
        "a new RuleBody variant left the stage id unchanged; the digest is not reading the rules"
    );
}

/// Changing the built-in function table or the component parameter table moves the
/// stage id, with the model held fixed.
#[test]
fn stage_id_changes_when_a_capability_table_changes() {
    let types = census();
    let builtins = builtin_function_table();
    let components = constraint_component_parameter_table();
    let analysis = class_analysis_table();
    let real = stage_id(&types, &builtins, &components, &analysis);
    assert_eq!(real, shipped_stage_id());

    let mut fewer_builtins = builtins.clone();
    fewer_builtins
        .pop()
        .expect("the built-in table is nonempty");
    assert_ne!(
        real,
        stage_id(&types, &fewer_builtins, &components, &analysis)
    );

    let mut renamed = builtins.clone();
    renamed[0].1 = "SOMETHING_ELSE".to_owned();
    assert_ne!(
        real,
        stage_id(&types, &renamed, &components, &analysis),
        "a built-in resolving to a different keyword must move the stage id"
    );

    let mut moved_iri = components.clone();
    moved_iri[0].1 = "http://example.org/moved".to_owned();
    assert_ne!(
        real,
        stage_id(&types, &builtins, &moved_iri, &analysis),
        "a constraint component changing IRI must move the stage id"
    );

    let mut fewer_components = components;
    fewer_components
        .pop()
        .expect("the component table is nonempty");
    assert_ne!(
        real,
        stage_id(&types, &builtins, &fewer_components, &analysis)
    );
}

/// **Changing the spec symbol table moves the stage id**, with the model and the
/// other tables held fixed: a product prepared under one table — one set of
/// native bindings, signatures and aliases — is refused by a build whose table
/// says something else, rather than restored against a meaning it was not
/// prepared under.
#[test]
fn stage_id_changes_when_the_spec_table_changes() {
    let types = census();
    let builtins = builtin_function_table();
    let components = constraint_component_parameter_table();
    let analysis = class_analysis_table();
    let real = stage_id(&types, &builtins, &components, &analysis);
    assert_eq!(real, shipped_stage_id());

    assert!(
        components.iter().any(|(key, _)| key.starts_with("spec[")),
        "the spec table reaches the preimage"
    );

    // An alias re-pointed at another SPARQL form.
    let mut realiased = components.clone();
    let alias = realiased
        .iter_mut()
        .find(|(_, line)| line.starts_with("sparql-alias plus = add"))
        .expect("the sparql:plus alias is a table fact");
    alias.1 = alias.1.replace("Infix(\"+\")", "Infix(\"-\")");
    assert_ne!(real, stage_id(&types, &builtins, &realiased, &analysis));

    // A component flipped from unimplemented to native.
    let mut flipped = components;
    let row = flipped
        .iter_mut()
        .find(|(_, line)| line.contains("SingleLineConstraintComponent unimplemented"))
        .expect("the unimplemented singleLine row is a table fact");
    row.1 = row.1.replace("unimplemented", "native");
    assert_ne!(real, stage_id(&types, &builtins, &flipped, &analysis));
}

/// Every `Constraint` variant — the model a prepared product carries — is claimed by
/// a component row of the spec symbol table, and every variant a row claims exists.
/// `Constraint::Component` is the one exception: it carries CUSTOM components, which
/// by definition are not spec rows.
#[test]
fn constraint_variants_are_the_spec_table_component_rows() {
    use purrdf_shapes::spec::{Carrier, ComponentStatus};
    let rows = census();
    let constraint = rows
        .iter()
        .find(|row| row.name == "Constraint")
        .expect("Constraint is censused");
    let variants: BTreeSet<&str> = constraint
        .variants
        .iter()
        .map(|variant| variant.name.as_str())
        .filter(|name| *name != "Component")
        .collect();
    let mut claimed: BTreeSet<&str> = BTreeSet::new();
    for row in purrdf_shapes::spec::implemented().components() {
        match (row.status(), row.carrier()) {
            (ComponentStatus::Native, Carrier::Constraint(names)) => claimed.extend(names),
            (ComponentStatus::Native, Carrier::ShapeField(_))
            | (ComponentStatus::Unimplemented, Carrier::None) => {}
            (status, carrier) => panic!(
                "<{}> pairs status {status:?} with carrier {carrier:?}",
                row.iri()
            ),
        }
    }
    assert_eq!(
        claimed, variants,
        "Constraint variants == native component rows"
    );
}

/// **Changing the class walk's reachability rule moves the stage id, with every
/// model type held fixed.**
///
/// This is the case no other check here can reach, and the reason
/// [`CLASS_ANALYSIS_SITES`] exists. The patch below stops `lower_property`
/// descending into a property shape's REIFIER shapes — a genuine change to which
/// classes a prepared product's analysis contains — and it touches no type, no
/// variant, no field, no vocabulary constant and no parameter table. Before the
/// class walk was censused, a build carrying this patch produced a byte-identical
/// census and therefore the identical stage id, so a product written by the other
/// build would `admit` and validate against an analysis that disagreed about the
/// shapes graph. Every test in this file passed on both sides.
///
/// The patch is applied to a copy of `plan.rs` held in memory. Nothing on disk is
/// touched, so this cannot rot into "the test that passed once".
#[test]
fn stage_id_changes_when_the_class_reachability_rule_changes() {
    let path = repo_root().join(CLASS_ANALYSIS_SOURCE);
    let real = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));

    let patched = real.replacen(
        ".map(|reifier| lower_shape(reifier, walk))",
        ".map(|reifier| lower_shape(reifier, &mut ShapeWalk::default()))",
        1,
    );
    assert_ne!(
        patched, real,
        "the class walk no longer descends into reifier shapes by that spelling; the probe is \
         patching nothing and would pass vacuously"
    );

    let types = census();
    let builtins = builtin_function_table();
    let components = constraint_component_parameter_table();
    let before = stage_id(
        &types,
        &builtins,
        &components,
        &class_analysis_table_from(&real),
    );
    let after = stage_id(
        &types,
        &builtins,
        &components,
        &class_analysis_table_from(&patched),
    );

    assert_eq!(before, shipped_stage_id());
    assert_ne!(
        before, after,
        "a changed class-reachability rule left the stage id unchanged, so a product carrying one \
         build's class analysis would still be admitted by a build that computes a different one"
    );
}

/// **Reformatting the class walk does NOT move the stage id.**
///
/// The mandatory mirror of the check above, and not a formality: the whole value of
/// a derived stage id is that it moves when the MEANING moves. One that also moved
/// on a reflow or a reworded comment would fire on changes that mean nothing, get
/// re-pinned without being read, and stop being evidence of anything the day it
/// mattered. The scrape digests tokens for exactly this reason, so the proof is
/// available: rewrite the walk's whitespace and its commentary, in both comment
/// forms, and the digest must be unmoved.
#[test]
fn reformatting_the_class_walk_does_not_move_the_stage_id() {
    let path = repo_root().join(CLASS_ANALYSIS_SOURCE);
    let real = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));

    let reworded = real.replacen(
        "/// The accumulator the one total shape walk carries.",
        "/// An entirely different sentence about the accumulator.\n// And a line comment.\n\n",
        1,
    );
    assert_ne!(
        reworded, real,
        "the walk accumulator's doc comment no longer opens with that sentence; the probe is \
         patching nothing and would pass vacuously"
    );

    assert_eq!(
        class_analysis_table_from(&real),
        class_analysis_table_from(&reworded),
        "rewording the class walk's commentary changed its census rendering, so the stage id \
         fires on prose and will be re-pinned without being read"
    );
}

/// Read the live sources, and return them alongside a copy in which exactly one
/// file has had `from` rewritten to `to`.
fn patched_sources(file: &str, from: &str, to: &str) -> (Sources, Sources) {
    let real = shapes_sources();
    let mut patched = real.clone();
    let target = patched
        .iter_mut()
        .find(|(path, _)| path == file)
        .unwrap_or_else(|| panic!("{file} is part of the scanned source set"));
    let replaced = target.1.replacen(from, to, 1);
    assert_ne!(
        replaced, target.1,
        "{file} no longer contains `{from}`; the synthetic probe is patching nothing and would \
         pass vacuously"
    );
    target.1 = replaced;
    (real, patched)
}

// ── Detector self-tests ─────────────────────────────────────────────────────────

mod detector_self_tests {
    use super::*;

    fn synthetic(source: &str) -> Scan {
        scan_sources(&[(
            "crates/shapes/src/synthetic.rs".to_owned(),
            source.to_owned(),
        )])
    }

    /// A reachable type the census does not declare MUST be flagged.
    ///
    /// This is the detector's whole job. If it cannot see an undeclared type in a
    /// source built to contain one, then `census_closure_is_complete` passing on
    /// the real tree says nothing at all.
    #[test]
    fn an_undeclared_reachable_type_is_flagged() {
        let scan = synthetic(
            r"
            pub struct Shapes {
                pub node_shapes: Vec<Shape>,
                pub(crate) hidden: Hidden,
                private: Private,
            }
            pub struct Shape { pub constraints: Vec<Constraint> }
            pub enum Constraint { Node(Box<Shape>), Probe(Undeclared) }
            pub struct Undeclared { pub iri: String }
            pub struct Hidden;
            pub struct Private;
            pub struct Unrelated { pub shape: Shape }
        ",
        );
        let closure = scan.closure("Shapes");
        assert_eq!(
            closure.iter().map(String::as_str).collect::<Vec<_>>(),
            ["Constraint", "Shape", "Shapes", "Undeclared"],
            "the closure must cross public fields and enum variant fields, must NOT cross \
             pub(crate) or private struct fields, and must not walk backwards into types that \
             merely mention a model type"
        );

        let declared: BTreeSet<String> = ["Shapes", "Shape", "Constraint"]
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        let missing: Vec<&String> = closure.difference(&declared).collect();
        assert_eq!(
            missing,
            [&"Undeclared".to_owned()],
            "the undeclared reachable type must be named"
        );
    }

    /// A synthetic added variant MUST reach the census rows a codec gate reads,
    /// and MUST move the stage id.
    #[test]
    fn a_new_variant_reaches_the_census_rows_and_the_stage_id() {
        let base = r"
            pub struct Shapes { pub node_shapes: Vec<Shape> }
            pub struct Shape { pub constraints: Vec<Constraint> }
            pub enum Constraint { MinCount(u64) }
        ";
        let grown = r"
            pub struct Shapes { pub node_shapes: Vec<Shape> }
            pub struct Shape { pub constraints: Vec<Constraint> }
            pub enum Constraint { MinCount(u64), Uncovered(String) }
        ";
        let names = ["Constraint", "Shape", "Shapes"];
        let before = synthetic(base).rows(&names);
        let after = synthetic(grown).rows(&names);

        let variants: Vec<&str> = after
            .iter()
            .find(|row| row.name == "Constraint")
            .expect("Constraint is censused")
            .variants
            .iter()
            .map(|variant| variant.name.as_str())
            .collect();
        assert_eq!(
            variants,
            ["MinCount", "Uncovered"],
            "the census rows must carry the new variant, or a codec-coverage gate cannot see it"
        );
        assert_ne!(
            stage_id(&before, &[], &[], &[]),
            stage_id(&after, &[], &[], &[])
        );
    }

    /// The digest reads the field TYPES, not just the names: a narrowing that keeps
    /// every name identical still moves the stage id.
    #[test]
    fn a_field_type_change_alone_moves_the_stage_id() {
        let names = ["Constraint"];
        let wide = synthetic("pub enum Constraint { MinCount { n: u64 } }").rows(&names);
        let narrow = synthetic("pub enum Constraint { MinCount { n: u32 } }").rows(&names);
        let boxed = synthetic("pub enum Constraint { MinCount { n: Box<u64> } }").rows(&names);
        assert_ne!(
            stage_id(&wide, &[], &[], &[]),
            stage_id(&narrow, &[], &[], &[])
        );
        assert_ne!(
            stage_id(&wide, &[], &[], &[]),
            stage_id(&boxed, &[], &[], &[])
        );
    }

    /// Variant ORDER is part of the digest: reordering arms changes any codec that
    /// numbers them by position, so it must not be invisible.
    #[test]
    fn variant_order_moves_the_stage_id() {
        let names = ["Constraint"];
        let one = synthetic("pub enum Constraint { A(u64), B(String) }").rows(&names);
        let two = synthetic("pub enum Constraint { B(String), A(u64) }").rows(&names);
        assert_ne!(stage_id(&one, &[], &[], &[]), stage_id(&two, &[], &[], &[]));
    }

    /// Prose does not move the digest. The mirror of the checks above: a census
    /// that fired on every comment edit would be re-pinned reflexively and stop
    /// being read at all.
    #[test]
    fn doc_comments_and_formatting_do_not_move_the_stage_id() {
        let names = ["Constraint"];
        let plain = synthetic("pub enum Constraint { MinCount(u64) }").rows(&names);
        let documented = synthetic(
            r"
            /// A SHACL constraint.
            ///
            /// Reworded entirely.
            #[derive(Debug, Clone)]
            pub enum Constraint {
                /// `sh:minCount`
                MinCount(
                    u64,
                ),
            }
        ",
        )
        .rows(&names);
        assert_eq!(
            stage_id(&plain, &[], &[], &[]),
            stage_id(&documented, &[], &[], &[])
        );
    }

    /// `#[cfg(test)]` items describe no product and must not be censused — nor may
    /// they shadow a production type of the same name.
    #[test]
    fn cfg_test_items_are_invisible_to_the_census() {
        let scan = synthetic(
            r"
            pub struct Shapes { pub node_shapes: Vec<Shape> }
            pub struct Shape { pub constraints: Vec<Constraint> }
            pub enum Constraint { MinCount(u64) }
            #[cfg(test)]
            mod tests {
                pub struct Shape { pub fake: u8 }
                pub enum Constraint { Fake }
                pub struct OnlyInTests { pub x: u8 }
            }
            #[cfg(test)]
            pub struct AlsoOnlyInTests { pub x: u8 }
        ",
        );
        assert!(!scan.types.contains_key("OnlyInTests"));
        assert!(!scan.types.contains_key("AlsoOnlyInTests"));
        // One definition each — a shadowing test double would make these ambiguous
        // and `definition` would refuse them.
        assert_eq!(
            scan.definition("Constraint")
                .expect("Constraint is censused")
                .variants
                .len(),
            1
        );
        assert_eq!(scan.closure("Shapes").len(), 3);
    }

    /// Type aliases are transparent: a model type reached only through an alias is
    /// still reached.
    #[test]
    fn aliases_do_not_hide_a_reachable_type() {
        let scan = synthetic(
            r"
            pub type Constraints = Vec<Constraint>;
            pub struct Shapes { pub node_shapes: Vec<Shape> }
            pub struct Shape { pub constraints: Constraints }
            pub enum Constraint { MinCount(u64) }
        ",
        );
        assert_eq!(
            scan.closure("Shapes").iter().collect::<Vec<_>>(),
            ["Constraint", "Shape", "Shapes"]
        );
    }

    /// A duplicated name the closure reaches has no answer, and is refused rather
    /// than resolved by guess.
    #[test]
    #[should_panic(expected = "declares it 2 times")]
    fn an_ambiguous_reachable_name_is_refused() {
        let scan = scan_sources(&[
            (
                "crates/shapes/src/a.rs".to_owned(),
                "pub struct Shapes { pub target: Dup } pub struct Dup { pub a: u8 }".to_owned(),
            ),
            (
                "crates/shapes/src/b.rs".to_owned(),
                "pub struct Dup { pub b: u8 }".to_owned(),
            ),
        ]);
        let _ = scan.closure("Shapes");
    }

    /// A duplicated name the closure does NOT reach is not refused.
    ///
    /// The neighbouring valid case for the refusal above. `crates/shapes/src`
    /// really does declare several duplicate names in its emitters
    /// (`ElementKind`, `JsonKind`, `RawParam`, `Renderer`), and a gate that
    /// rejected the crate for having them would be wrong, not strict.
    #[test]
    fn an_ambiguous_unreachable_name_is_not_refused() {
        let scan = scan_sources(&[
            (
                "crates/shapes/src/a.rs".to_owned(),
                "pub struct Shapes { pub ok: u8 } pub struct Dup { pub a: u8 }".to_owned(),
            ),
            (
                "crates/shapes/src/b.rs".to_owned(),
                "pub struct Dup { pub b: u8 }".to_owned(),
            ),
        ]);
        assert_eq!(
            scan.closure("Shapes").iter().collect::<Vec<_>>(),
            ["Shapes"]
        );
    }

    /// A field type no structural scan can read is refused, loudly.
    #[test]
    #[should_panic(expected = "cannot read the field type")]
    fn an_opaque_field_type_is_refused() {
        let _ = synthetic("pub struct Shapes { pub opaque: some_macro!() }");
    }

    /// The neighbouring valid case: the shapes a real model DOES use — references,
    /// tuples, arrays, slices, qualified paths and nested generics — are all read,
    /// not refused.
    #[test]
    fn ordinary_field_types_are_read_not_refused() {
        let scan = synthetic(
            r"
            pub struct Shapes {
                pub owned: std::sync::Arc<std::collections::BTreeMap<String, Target>>,
                pub pairs: Vec<(String, Term)>,
                pub fixed: [Term; 3],
                pub borrowed: &'static str,
                pub sliced: Box<[Term]>,
                pub nested: Option<Box<Vec<Target>>>,
            }
            pub struct Target { pub iri: String }
            pub struct Term { pub iri: String }
        ",
        );
        assert_eq!(
            scan.closure("Shapes").iter().collect::<Vec<_>>(),
            ["Shapes", "Target", "Term"]
        );
        let fields = &scan
            .definition("Shapes")
            .expect("Shapes is censused")
            .variants[0]
            .fields;
        assert_eq!(fields.len(), 6);
        assert!(fields.iter().all(|field| field.public));
        assert!(
            fields[2].signature.contains("array("),
            "an array field must render as an array: {}",
            fields[2].signature
        );
        assert!(
            fields[3].signature.contains("ref('static"),
            "a reference field must render its lifetime: {}",
            fields[3].signature
        );
    }

    /// The scan reads TUPLE-struct and unit variants too, with positional names.
    #[test]
    fn tuple_and_unit_variants_are_censused() {
        let scan = synthetic("pub enum Path { Predicate(Term, bool), Any }");
        let row = scan.definition("Path").expect("Path is censused");
        assert_eq!(row.kind, TypeKind::Enum);
        assert_eq!(row.variants[0].name, "Predicate");
        let names: Vec<&str> = row.variants[0]
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        assert_eq!(names, ["0", "1"]);
        assert_eq!(row.variants[1].name, "Any");
        assert!(
            row.variants[1].fields.is_empty(),
            "a unit variant has no fields, but the scan read {:?}",
            row.variants[1].fields
        );
    }

    /// A source that does not parse fails loudly rather than censusing nothing.
    #[test]
    #[should_panic(expected = "census cannot parse Rust source")]
    fn malformed_source_fails_loudly() {
        let _ = synthetic("pub enum Constraint {");
    }
}
