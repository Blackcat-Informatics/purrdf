// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS-AUTHORSHIP CENSUS — the closed list of places that can mint a GTS file.
//!
//! Every authoring decision (which in-band dictionaries a
//! pack pins, which one primes which frame, the declared `zstd` level) is
//! carried in [`WriterOptions`] and chosen at ONE moment: when a
//! header-minting `Writer` constructor runs. A new call site added later that
//! forgets to state a plan does not fail — it silently emits a pack at the
//! writer's ~level-1 default with no dictionary, which is exactly the kind of
//! quiet capability degradation the plan exists to prevent.
//!
//! So this gate is a STRUCTURAL source scan over `crates/*/src` and
//! `bindings/*/src`, in two rules:
//!
//!   RULE 1 — `Writer`'s public constructor set is exactly
//!            `{new, deterministic, with_layout, with_options}` (header-MINTING)
//!            plus `{appending}` (segment-CONTINUING, mints no header and so
//!            makes no authoring choices — it adopts the ones already on the
//!            wire). A fifth minting constructor must be added here on purpose.
//!
//!   RULE 2 — the set of `(file, enclosing fn)` production call sites of those
//!            four minting constructors is exactly [`AUTHORSHIP_SITES`]. This is
//!            the census proper: the complete list of functions in this
//!            repository that can bring a GTS segment header into existence.
//!
//! The `tests` module below proves the detector is not vacuous: a synthetic
//! source carrying a constructor reference MUST be flagged. Comments and calls
//! inside `#[cfg(test)]` modules MUST NOT be flagged. A `#[cfg(test)]` function
//! at file scope is still counted, preferring a loud row over a silent omission.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use syn::parse::Parser;
use syn::visit::{self, Visit};

/// The `Writer` constructors that MINT a new segment header, and therefore
/// decide the pack's `WriterOptions` (dictionaries, declared level, layout).
const MINTING_CONSTRUCTORS: [&str; 4] = ["new", "deterministic", "with_layout", "with_options"];

/// The `Writer` constructors that do NOT mint a header. `appending` continues an
/// existing segment and adopts the catalog/dictionaries already on the wire, so
/// it makes no authoring choice and is deliberately outside the census.
const CONTINUING_CONSTRUCTORS: [&str; 1] = ["appending"];

/// THE CENSUS: every production `(file, enclosing fn)` that mints a GTS header.
///
/// Adding a row here is a deliberate act — the reviewer's question for any new
/// row is "does this function state its dictionaries and its zstd level, or does
/// it silently take the writer defaults?".
///
/// The three general-purpose PUBLIC authoring facades a consumer is expected to
/// reach for are `purrdf_rdf::gts_write::{to_writer, to_gts}` (RDF dataset →
/// GTS; `to_gts` delegates to `to_writer`, so only the latter is a direct site)
/// and `purrdf_gts::compact::compact_streamable` (§10.1 repack). The remaining
/// rows are narrower producers — a bundle emitter, an archive packer, an example
/// store, and the frozen-vector fixtures — each of which is still a way to mint
/// a header and so still belongs in the census.
const AUTHORSHIP_SITES: [(&str, &str); 10] = [
    // §10.1 streamable compaction: authors a pack under a `DictPlan`.
    ("crates/gts/src/compact.rs", "compact_streamable"),
    // The append-only agent-memory example store (mints the header once, then
    // continues that segment through `Writer::appending`).
    ("crates/gts/src/examples/agent_memory.rs", "writer"),
    // The `files` profile archive packers.
    ("crates/gts/src/files.rs", "pack_to_writer"),
    ("crates/gts/src/files.rs", "build_entries_v2_prefix"),
    // The single-shot snapshot-bundle authoring helper.
    ("crates/gts/src/writer.rs", "snapshot_from_graph"),
    // The `dist` snapshot-bundle emitter, driven by a `MediumPlan`.
    ("crates/rdf/src/gts_compose.rs", "emit_gts"),
    // The frozen dict-vector fixtures (`vectors/30`–`33`).
    ("crates/rdf/src/gts_dict_vectors.rs", "fixed_source"),
    (
        "crates/rdf/src/gts_dict_vectors.rs",
        "size_comparison_source",
    ),
    ("crates/rdf/src/gts_dict_vectors.rs", "multi_dict_pack"),
    // The public RDF-dataset → GTS surface (`to_gts` delegates to this).
    ("crates/rdf/src/gts_write.rs", "to_writer"),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// Every `.rs` file under `crates/*/src` and `bindings/*/src`.
fn production_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for group in ["crates", "bindings"] {
        let group_dir = root.join(group);
        let entries = std::fs::read_dir(&group_dir)
            .unwrap_or_else(|err| panic!("read {}: {err}", group_dir.display()));
        for entry in entries {
            let crate_dir = entry.expect("directory entry").path();
            let src = crate_dir.join("src");
            if src.is_dir() {
                collect_rs(&src, &mut out);
            }
        }
    }
    out.sort();
    assert!(
        out.len() > 50,
        "the scan found only {} sources — the walker is broken, not the repo",
        out.len()
    );
    out
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

// Resolve only explicit Rust paths, without expanding macros or evaluating cfg.
// Comments and literals cannot become paths. Imports and type aliases are
// hoisted in each lexical scope; unrelated local types shadow imported names.
type Names = BTreeMap<String, Vec<String>>;

fn path_names(path: &syn::Path) -> Vec<String> {
    path.leading_colon
        .iter()
        .map(|_| "::".to_string())
        .chain(path.segments.iter().map(|part| part.ident.to_string()))
        .collect()
}

fn expression_path(expr: &syn::ExprPath) -> Vec<String> {
    let mut path = path_names(&expr.path);
    if let Some(qself) = &expr.qself {
        // The separator after `<Type>` is not an external-crate root.
        if path.first().is_some_and(|part| part == "::") {
            path.remove(0);
        }
        // A trait-associated function is not an inherent Writer constructor.
        if qself.position != 0 {
            return Vec::new();
        }
        if let syn::Type::Path(ty) = qself.ty.as_ref() {
            let mut qualified = path_names(&ty.path);
            qualified.append(&mut path);
            path = qualified;
        }
    }
    path
}

fn is_writer(path: &[String]) -> bool {
    path == ["purrdf_gts", "writer", "Writer"] || path == ["purrdf", "gts", "writer", "Writer"]
}

fn test_module(module: &syn::ItemMod) -> bool {
    module.attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
    })
}

fn module_path(relative: &str) -> Vec<String> {
    let parts: Vec<_> = relative.split('/').collect();
    let package = parts[1].replace('-', "_");
    let name = if package == "purrdf" {
        package
    } else {
        format!("purrdf_{package}")
    };
    let mut path = vec![name];
    for part in &parts[3..] {
        let name = part.strip_suffix(".rs").unwrap_or(part);
        if !matches!(name, "lib" | "main" | "mod") {
            path.push(name.to_string());
        }
    }
    path
}

fn imports(tree: &syn::UseTree, prefix: &[String], out: &mut Vec<(String, Vec<String>)>) {
    match tree {
        syn::UseTree::Path(path) => {
            let mut prefix = prefix.to_vec();
            prefix.push(path.ident.to_string());
            imports(&path.tree, &prefix, out);
        }
        syn::UseTree::Name(name) => {
            let mut path = prefix.to_vec();
            let name = name.ident.to_string();
            if name != "self" {
                path.push(name);
            }
            let binding = path.last().expect("use path is nonempty").clone();
            out.push((binding, path));
        }
        syn::UseTree::Rename(rename) => {
            let mut path = prefix.to_vec();
            if rename.ident != "self" {
                path.push(rename.ident.to_string());
            }
            out.push((rename.rename.to_string(), path));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                imports(item, prefix, out);
            }
        }
        syn::UseTree::Glob(_) => out.push(("*".into(), prefix.to_vec())),
    }
}

struct Census<'a> {
    file: &'a str,
    module: Vec<String>,
    names: Names,
    modules: BTreeMap<Vec<String>, Names>,
    enclosing: Option<String>,
    sites: Vec<String>,
    constructors: Vec<String>,
    writer_impl: bool,
    constructor_body: bool,
}

impl<'a> Census<'a> {
    fn new(file: &'a str) -> Self {
        Self {
            file,
            module: module_path(file),
            names: Names::new(),
            modules: BTreeMap::new(),
            enclosing: None,
            sites: Vec::new(),
            constructors: Vec::new(),
            writer_impl: false,
            constructor_body: false,
        }
    }

    fn resolve(&self, path: &[String]) -> Vec<String> {
        let Some(first) = path.first() else {
            return Vec::new();
        };
        let mut resolved = match first.as_str() {
            "::" => Vec::new(),
            "crate" => vec![self.module[0].clone()],
            "self" => self.module.clone(),
            "super" => {
                let mut parent = self.module.clone();
                parent.pop();
                parent
            }
            _ => self
                .names
                .get(first)
                .cloned()
                .unwrap_or_else(|| vec![first.clone()]),
        };
        for part in &path[1..] {
            if part == "super" {
                resolved.pop();
                continue;
            }
            if let Some(target) = self
                .modules
                .get(&resolved)
                .and_then(|names| names.get(part))
            {
                resolved = target.clone();
            } else {
                resolved.push(part.clone());
            }
        }
        resolved
    }

    fn declare(&mut self, items: &[&syn::Item], local: bool) {
        let mut aliases = Vec::new();
        for item in items {
            let ident = match item {
                syn::Item::Struct(item) => Some(&item.ident),
                syn::Item::Enum(item) => Some(&item.ident),
                syn::Item::Union(item) => Some(&item.ident),
                syn::Item::Trait(item) => Some(&item.ident),
                syn::Item::Mod(item) if !test_module(item) => Some(&item.ident),
                syn::Item::Type(item) => {
                    if let syn::Type::Path(ty) = item.ty.as_ref() {
                        aliases.push((item.ident.to_string(), path_names(&ty.path)));
                    }
                    Some(&item.ident)
                }
                syn::Item::Use(item) => {
                    let root = item
                        .leading_colon
                        .iter()
                        .map(|_| "::".to_string())
                        .collect::<Vec<_>>();
                    imports(&item.tree, &root, &mut aliases);
                    None
                }
                _ => None,
            };
            if let Some(ident) = ident {
                let mut path = self.module.clone();
                if local {
                    path.push("<local>".into());
                }
                path.push(ident.to_string());
                self.names.insert(ident.to_string(), path);
            }
        }
        // Repeated resolution admits alias chains regardless of declaration order.
        for _ in 0..=aliases.len() {
            let before = self.names.clone();
            for (name, path) in &aliases {
                let resolved = self.resolve(path);
                if name == "*" {
                    if let Some(names) = self.modules.get(&resolved) {
                        for (name, path) in names {
                            self.names
                                .entry(name.clone())
                                .or_insert_with(|| path.clone());
                        }
                    }
                    let mut writer = resolved;
                    writer.push("Writer".into());
                    if is_writer(&writer) {
                        self.names.entry("Writer".into()).or_insert(writer);
                    }
                } else {
                    self.names.insert(name.clone(), resolved);
                }
            }
            if self.names == before {
                break;
            }
        }
    }

    fn shadow_generics(&mut self, generics: &syn::Generics) {
        for param in generics.type_params() {
            self.names
                .insert(param.ident.to_string(), vec!["<generic>".into()]);
        }
    }

    fn record(&mut self, path: &[String]) {
        if let Some((method, ty)) = path.split_last()
            && MINTING_CONSTRUCTORS.contains(&method.as_str())
            && is_writer(&self.resolve(ty))
            && !self.constructor_body
        {
            self.sites.push(
                self.enclosing
                    .clone()
                    .unwrap_or_else(|| "<no enclosing fn>".into()),
            );
        }
    }

    fn tokens(&mut self, tokens: TokenStream) {
        // Parse normal macro arguments as expressions, then statement-bearing
        // bodies as blocks, so lexical shadowing works inside either shape.
        let expressions =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        if let Ok(expressions) = expressions.parse2(tokens.clone()) {
            for expr in &expressions {
                self.visit_expr(expr);
            }
            return;
        }
        let block = TokenTree::Group(Group::new(Delimiter::Brace, tokens.clone()));
        if let Ok(block) = syn::parse2::<syn::Block>(TokenStream::from(block)) {
            self.visit_block(&block);
            return;
        }
        // An opaque macro DSL still exposes literal Rust paths as tokens. Scan
        // those conservatively; refuse binding declarations alongside GTS paths
        // rather than guessing their expansion or overlooking an alias.
        let tokens: Vec<_> = tokens.into_iter().collect();
        for (index, token) in tokens.iter().enumerate() {
            if !matches!(token, TokenTree::Ident(ident) if ident == "use") {
                continue;
            }
            let parser = |input: syn::parse::ParseStream<'_>| {
                let import = input.parse::<syn::ItemUse>()?;
                let _: TokenStream = input.parse()?;
                Ok(import)
            };
            if let Ok(import) = parser.parse2(tokens[index..].iter().cloned().collect()) {
                let mut aliases = Vec::new();
                let root = import
                    .leading_colon
                    .iter()
                    .map(|_| "::".to_string())
                    .collect::<Vec<_>>();
                imports(&import.tree, &root, &mut aliases);
                assert!(
                    !aliases.iter().any(|(_, path)| {
                        let path = self.resolve(path);
                        path.starts_with(&["purrdf_gts".into()])
                            || path.starts_with(&["purrdf".into(), "gts".into()])
                    }),
                    "{}: opaque macro body mixes GTS Writer paths with binding declarations; express this authoring code as a Rust function so the census can resolve its scope",
                    self.file
                );
            }
        }
        let mut paths = Vec::new();
        let mut cursor = 0;
        while cursor < tokens.len() {
            let parser = |input: syn::parse::ParseStream<'_>| {
                let path = input.parse::<syn::ExprPath>()?;
                let remainder = input.parse::<TokenStream>()?;
                Ok((path, remainder))
            };
            let suffix = tokens[cursor..].iter().cloned().collect();
            if let Ok((path, remainder)) = parser.parse2(suffix) {
                cursor = tokens.len() - remainder.into_iter().count();
                paths.push(expression_path(&path));
            } else {
                cursor += 1;
            }
        }
        let binding = tokens.iter().any(|token| matches!(token, TokenTree::Ident(ident)
            if matches!(ident.to_string().as_str(), "use" | "type" | "struct" | "enum" | "union" | "mod")));
        let writer = paths
            .iter()
            .any(|path| (1..=path.len()).any(|len| is_writer(&self.resolve(&path[..len]))));
        assert!(
            !(binding && writer),
            "{}: opaque macro body mixes GTS Writer paths with binding declarations; express this authoring code as a Rust function so the census can resolve its scope",
            self.file
        );
        for path in paths {
            self.record(&path);
        }
        for token in tokens {
            if let TokenTree::Group(group) = token {
                self.tokens(group.stream());
            }
        }
    }
}

impl<'ast> Visit<'ast> for Census<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.declare(&file.items.iter().collect::<Vec<_>>(), false);
        self.modules.insert(self.module.clone(), self.names.clone());
        visit::visit_file(self, file);
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if test_module(module) {
            return;
        }
        let Some((_, items)) = &module.content else {
            return;
        };
        let saved = std::mem::take(&mut self.names);
        self.module.push(module.ident.to_string());
        self.declare(&items.iter().collect::<Vec<_>>(), false);
        self.modules.insert(self.module.clone(), self.names.clone());
        for item in items {
            self.visit_item(item);
        }
        self.module.pop();
        self.names = saved;
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let saved = self.names.clone();
        let items = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.declare(&items, true);
        visit::visit_block(self, block);
        self.names = saved;
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        let saved = self.names.clone();
        let enclosing = self.enclosing.replace(function.sig.ident.to_string());
        let constructor_body = std::mem::replace(&mut self.constructor_body, false);
        self.shadow_generics(&function.sig.generics);
        visit::visit_item_fn(self, function);
        self.constructor_body = constructor_body;
        self.enclosing = enclosing;
        self.names = saved;
    }

    fn visit_item_impl(&mut self, implementation: &'ast syn::ItemImpl) {
        let saved = self.names.clone();
        self.shadow_generics(&implementation.generics);
        let ty = match implementation.self_ty.as_ref() {
            syn::Type::Path(ty) => self.resolve(&path_names(&ty.path)),
            _ => Vec::new(),
        };
        let writer_impl = std::mem::replace(
            &mut self.writer_impl,
            is_writer(&ty) && implementation.trait_.is_none(),
        );
        self.names.insert("Self".into(), ty);
        visit::visit_item_impl(self, implementation);
        self.writer_impl = writer_impl;
        self.names = saved;
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        let saved = self.names.clone();
        let name = function.sig.ident.to_string();
        if self.writer_impl && matches!(function.vis, syn::Visibility::Public(_)) {
            struct ReturnsSelf(bool);
            impl<'ast> Visit<'ast> for ReturnsSelf {
                fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
                    self.0 |= path.path.is_ident("Self");
                    visit::visit_type_path(self, path);
                }
            }
            let mut returns = ReturnsSelf(false);
            returns.visit_return_type(&function.sig.output);
            if returns.0 {
                self.constructors.push(name.clone());
            }
        }
        let constructor_body = std::mem::replace(
            &mut self.constructor_body,
            self.writer_impl && MINTING_CONSTRUCTORS.contains(&name.as_str()),
        );
        let enclosing = self.enclosing.replace(name);
        self.shadow_generics(&function.sig.generics);
        visit::visit_impl_item_fn(self, function);
        self.enclosing = enclosing;
        self.constructor_body = constructor_body;
        self.names = saved;
    }

    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        let path = expression_path(expr);
        self.record(&path);
        visit::visit_expr_path(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.tokens(mac.tokens.clone());
    }
}

fn scan<'a>(relative: &'a str, source: &str) -> Census<'a> {
    let parsed = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("{relative}: census cannot parse Rust source: {error}"));
    let mut census = Census::new(relative);
    census.visit_file(&parsed);
    census
}

/// RULE 1 — the `Writer` constructor set is closed and classified.
#[test]
fn the_writer_constructor_set_is_exactly_the_pinned_minting_and_continuing_sets() {
    let relative = "crates/gts/src/writer.rs";
    let source = std::fs::read_to_string(repo_root().join(relative)).expect("writer.rs");
    let mut found = scan(relative, &source).constructors;
    found.sort();
    found.dedup();
    let mut expected: Vec<_> = MINTING_CONSTRUCTORS
        .iter()
        .chain(&CONTINUING_CONSTRUCTORS)
        .map(|name| (*name).to_string())
        .collect();
    expected.sort();
    assert_eq!(
        found, expected,
        "GTS Writer constructors must be explicitly classified as header-minting or segment-continuing"
    );
}

/// RULE 2 — the census of production header-minting call sites is closed.
#[test]
fn every_production_gts_authorship_site_is_in_the_census() {
    let root = repo_root();
    let mut observed = Vec::new();
    for path in production_sources(&root) {
        let relative = path
            .strip_prefix(&root)
            .expect("source below root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for function in scan(&relative, &source).sites {
            observed.push((relative.clone(), function));
        }
    }
    observed.sort();
    observed.dedup();
    let mut expected: Vec<_> = AUTHORSHIP_SITES
        .iter()
        .map(|(file, function)| ((*file).to_string(), (*function).to_string()))
        .collect();
    expected.sort();
    expected.dedup();
    assert_eq!(
        expected.len(),
        AUTHORSHIP_SITES.len(),
        "duplicate census row"
    );
    assert_eq!(
        observed, expected,
        "the GTS-authorship census changed; every new author must state its dictionaries and declared zstd level before its row is admitted"
    );
}

#[test]
fn the_public_authoring_facades_are_present_and_named() {
    assert!(AUTHORSHIP_SITES.contains(&("crates/rdf/src/gts_write.rs", "to_writer")));
    assert!(AUTHORSHIP_SITES.contains(&("crates/gts/src/compact.rs", "compact_streamable")));
    let source = std::fs::read_to_string(repo_root().join("crates/rdf/src/gts_write.rs"))
        .expect("gts_write.rs");
    let parsed = syn::parse_file(&source).expect("valid Rust");
    let function = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function)
                if function.sig.ident == "to_gts"
                    && matches!(function.vis, syn::Visibility::Public(_)) =>
            {
                Some(function)
            }
            _ => None,
        })
        .expect("to_gts remains a public facade");
    struct Delegates(bool);
    impl<'ast> Visit<'ast> for Delegates {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            self.0 |= matches!(call.func.as_ref(), syn::Expr::Path(path) if path.path.is_ident("to_writer"));
            visit::visit_expr_call(self, call);
        }
    }
    let mut delegates = Delegates(false);
    delegates.visit_block(&function.block);
    assert!(delegates.0, "to_gts must author through to_writer");
}

mod detector_self_tests {
    use super::*;

    fn sites(source: &str) -> Vec<String> {
        scan("crates/rdf/src/synthetic.rs", source).sites
    }

    #[test]
    fn unrelated_local_and_imported_writers_are_not_gts_authors() {
        assert_eq!(
            sites(
                r#"
            struct Writer<'a>(&'a str);
            impl<'a> Writer<'a> { fn new(s: &'a str) -> Self { Self(s) } }
            fn project() { Writer::new("json"); OkfWriter::new(); other::Writer::new(); }
            mod nested { use other::Writer; fn unrelated() { Writer::new(); } }
        "#
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn qualified_renamed_grouped_module_and_type_alias_paths_are_authors() {
        assert_eq!(
            sites(
                r#"
            use purrdf_gts as gts;
            use gts::writer::{self as writer_module, Writer as PackWriter};
            type Alias = PackWriter;
            fn qualified() { purrdf_gts::writer::Writer::new("x"); }
            fn renamed() { PackWriter :: with_options ("x", options); }
            fn module_alias() { writer_module::Writer::deterministic("x"); }
            fn type_alias() { Alias::with_layout("x", layout); }
            fn split_line() { PackWriter
                :: new
                ("x"); }
            fn function_value() { let make = Alias::new; make("x"); }
            fn qualified_self() { <Alias>::new("x"); }
            fn umbrella() { purrdf::gts::writer::Writer::new("x"); }
        "#
            ),
            [
                "qualified",
                "renamed",
                "module_alias",
                "type_alias",
                "split_line",
                "function_value",
                "qualified_self",
                "umbrella"
            ]
        );
    }

    #[test]
    fn absolute_external_paths_bypass_local_module_shadowing() {
        assert_eq!(
            sites(
                r#"
            mod purrdf_gts { pub mod writer { pub struct Writer; } }
            fn unrelated() { purrdf_gts::writer::Writer::new("local"); }
            fn absolute() { ::purrdf_gts::writer::Writer::new("real"); }
            use ::purrdf_gts::writer::Writer as Pack;
            fn imported() { Pack::new("real"); }
        "#
            ),
            ["absolute", "imported"]
        );
    }

    #[test]
    fn nested_scopes_shadow_and_restore_imported_names() {
        assert_eq!(
            sites(
                r#"
            use purrdf_gts::writer::Writer;
            fn before() { Writer::new("x"); }
            fn nested() {
                { Writer::new("local"); struct Writer; }
                { type Writer = Other; Writer::new("local"); }
                Writer::new("real");
                fn helper() { struct Writer; Writer::new("local"); }
            }
            fn generic<Writer>() { Writer::new("generic"); }
            fn after() { Writer::new("x"); }
            mod child {
                use super::Writer as ParentWriter;
                struct Writer;
                fn real() { ParentWriter::new("x"); }
                fn unrelated() { Writer::new("x"); }
            }
        "#
            ),
            ["before", "nested", "after", "real"]
        );
    }

    #[test]
    fn cfg_test_modules_are_excluded_without_hiding_later_code() {
        assert_eq!(
            sites(
                r#"
            use purrdf_gts::writer::Writer;
            #[cfg(test)] #[allow(dead_code)] mod checks {
                use super::*;
                fn excluded() { Writer::new("test"); }
                mod nested { fn excluded() { purrdf_gts::writer::Writer::new("test"); } }
            }
            #[cfg(test)] fn visible_helper() { Writer::new("helper"); }
            fn later() { Writer::new("real"); }
        "#
            ),
            ["visible_helper", "later"]
        );
    }

    #[test]
    fn comments_and_literals_cannot_mint_headers() {
        assert_eq!(
            sites(
                r##"
            use purrdf_gts::writer::Writer;
            // Writer::new("comment");
            /* Writer::new("block"); /* Writer::new("nested"); */ */
            /// Writer::new("doc");
            fn f() {
                let s = "Writer::new(\"string\") //";
                let s = r#"Writer::with_options("raw", options)"#;
                let s = b"Writer::new(\"bytes\")";
                let s = '\'';
                Writer::new("real");
            }
        "##
            ),
            ["f"]
        );
    }

    #[test]
    fn macro_arguments_and_opaque_token_paths_are_scanned() {
        assert_eq!(
            sites(
                r#"
            use purrdf_gts::writer::Writer as Pack;
            fn expressions() { emit!("header", Pack::new("x")); }
            fn block() { build! { struct Pack; Pack::new("local"); } }
            fn opaque() { build! { header => Pack::new("x") } }
            fn generic_path() { build! { header => Pack::new::<Profile>("x") } }
            fn absolute_path() { build! { header => ::purrdf_gts::writer::Writer::new("x") } }
            fn qualified_self() { build! { header => <Pack>::new("x") } }
            fn literal() { build! { header => "Pack::new(x)" } }
            macro_rules! make { () => { purrdf_gts::writer::Writer::new("x") }; }
        "#
            ),
            [
                "expressions",
                "opaque",
                "generic_path",
                "absolute_path",
                "qualified_self",
                "<no enclosing fn>"
            ]
        );
    }

    #[test]
    #[should_panic(expected = "opaque macro body mixes GTS Writer paths with binding declarations")]
    fn opaque_macro_bindings_cannot_silently_hide_authors() {
        sites(
            r#"fn f() { build! { import => use purrdf_gts::writer::Writer as Pack; Pack::new("x") } }"#,
        );
    }

    #[test]
    #[should_panic(expected = "opaque macro body mixes GTS Writer paths with binding declarations")]
    fn opaque_grouped_imports_cannot_hide_renamed_authors() {
        sites(
            r#"fn f() { build! { import => use purrdf_gts::{writer::Writer as Pack}; Pack::new("x") } }"#,
        );
    }

    #[test]
    fn gts_crate_paths_and_glob_imports_resolve() {
        let source = r#"
            use crate::writer::*;
            fn glob() { Writer::new("x"); }
            fn qualified() { crate::writer::Writer::new("x"); }
            mod nested { use super::*; fn inherited() { Writer::new("x"); } }
        "#;
        assert_eq!(
            scan("crates/gts/src/files.rs", source).sites,
            ["glob", "qualified", "inherited"]
        );
    }

    #[test]
    #[should_panic(expected = "census cannot parse Rust source")]
    fn malformed_source_fails_loudly() {
        sites("#[cfg(test)] mod tests {");
    }
}
