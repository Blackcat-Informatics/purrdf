// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The walk over first-party Rust source, and the symbol index it builds.
//!
//! Packages are the directories under `crates/` and `bindings/` that hold a
//! `Cargo.toml`. Each package's library and binary roots (and its `build.rs`) are
//! parsed and their `mod` declarations followed, exactly as rustc would reach them,
//! so the module path of every item is the one callers write. So are its test,
//! bench and example targets — `tests/*.rs`, `tests/*/main.rs`, the same under
//! `benches/` and `examples/`, and every `[[test]]`, `[[bench]]` and `[[example]]`
//! path — each a crate of its own, named `<package>::test_crate::<target>` (or
//! `bench_crate`, `example_crate`): test-support code copied between two test
//! files is as much a second implementation as a helper copied between two
//! libraries. Their units are marked as not [`Unit::shipping`]. What the walk
//! never enters:
//!
//! * an item or module under a test-only `#[cfg]` (`test`, or an `all(…)` holding
//!   it) and every `#[test]` function — a test case is not a helper;
//! * from a library, binary or build script, a file under a `tests/`, `benches/`
//!   or `examples/` directory of its package, and from any root a file under
//!   `generated/` or outside its package, however it is reached.
//!
//! A file reached twice — a `tests/support/mod.rs` every test target declares, or
//! a library file a build script includes through `#[path]` — is one source: its
//! items are indexed under every path that reaches them, and fingerprinted once,
//! under the first.
//!
//! Every function, constant and static that survives becomes a [`Unit`] with its
//! fingerprints. Every named item becomes an index entry, and every `use` an alias,
//! so a ledger path such as `purrdf_iri::Iri::resolve` resolves through the
//! re-export to the item that defines it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::normalize::{self, BodyPrint};

/// Directory names whose contents are never shipping source.
const EXCLUDED_DIRS: [&str; 4] = ["tests", "benches", "examples", "generated"];

/// The target directories whose crates the walk reads as test code, with the
/// path segment their crates are named under.
const TEST_TARGETS: [(&str, &str); 3] = [
    ("tests", "test_crate"),
    ("benches", "bench_crate"),
    ("examples", "example_crate"),
];

/// Where the source comes from: the repository on disk, or an in-memory tree in
/// the self-test and the unit tests.
pub(crate) trait Tree {
    /// The text of the file at `path` (repo-relative, `/`-separated).
    fn read(&self, path: &str) -> Option<String>;
    /// The entries of the directory at `path`: `(name, is_directory)`, sorted.
    fn list(&self, path: &str) -> Vec<(String, bool)>;
}

/// The repository on disk.
#[derive(Debug)]
pub(crate) struct Disk {
    /// The repository root.
    pub(crate) root: PathBuf,
}

impl Tree for Disk {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(path)).ok()
    }

    fn list(&self, path: &str) -> Vec<(String, bool)> {
        let Ok(entries) = std::fs::read_dir(self.root.join(path)) else {
            return Vec::new();
        };
        let mut listed: Vec<(String, bool)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let is_dir = entry.file_type().ok()?.is_dir();
                Some((name, is_dir))
            })
            .collect();
        listed.sort();
        listed
    }
}

/// An in-memory tree: repo-relative path to text.
#[derive(Debug, Default)]
pub(crate) struct Memory {
    /// Every file, by repo-relative path.
    pub(crate) files: BTreeMap<String, String>,
}

impl Tree for Memory {
    fn read(&self, path: &str) -> Option<String> {
        self.files.get(path).cloned()
    }

    fn list(&self, path: &str) -> Vec<(String, bool)> {
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        let mut listed = BTreeSet::new();
        for file in self.files.keys() {
            if let Some(rest) = file.strip_prefix(&prefix) {
                match rest.split_once('/') {
                    Some((directory, _)) => listed.insert((directory.to_owned(), true)),
                    None => listed.insert((rest.to_owned(), false)),
                };
            }
        }
        listed.into_iter().collect()
    }
}

/// What kind of unit a fingerprinted item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnitKind {
    /// A function or method with a body.
    Function,
    /// A `const` or `static` item.
    Constant,
}

/// One fingerprinted item: of shipping source, or of a test, bench or example.
#[derive(Clone, Debug)]
pub(crate) struct Unit {
    /// Its resolved symbol path.
    pub(crate) symbol: String,
    /// Its own name (the last path segment).
    pub(crate) name: String,
    /// The package that defines it.
    pub(crate) package: String,
    /// The repo-relative file.
    pub(crate) file: String,
    /// The 1-based line of its name.
    pub(crate) line: usize,
    /// Function or constant.
    pub(crate) kind: UnitKind,
    /// A function body's fingerprints.
    pub(crate) print: Option<BodyPrint>,
    /// Every integer literal it holds, by value.
    pub(crate) constants: BTreeSet<u128>,
    /// The hex-digit tables it holds.
    pub(crate) tables: BTreeSet<&'static str>,
    /// Every string literal it holds, decoded, with its 1-based line.
    pub(crate) strings: Vec<(String, usize)>,
    /// Whether it is shipping source: reached from a library, binary or build
    /// script root rather than from a test, bench or example target.
    pub(crate) shipping: bool,
}

/// One named item in the index.
#[derive(Clone, Debug)]
pub(crate) struct Definition {
    /// The repo-relative file.
    pub(crate) file: String,
    /// The 1-based line of its name.
    pub(crate) line: usize,
    /// The package that defines it.
    pub(crate) package: String,
    /// The module it lives in (a method's module is its type's).
    pub(crate) module: String,
    /// Whether it carries documentation.
    pub(crate) documented: bool,
}

/// A `use` as written, resolved lazily once every module is known.
#[derive(Clone, Debug)]
struct Import {
    /// The module that holds the `use`.
    module: String,
    /// The imported path as written.
    target: Vec<String>,
}

/// An impl-block item whose symbol depends on where its self type is defined.
#[derive(Clone, Debug)]
struct Pending {
    module: String,
    self_type: Vec<String>,
    name: String,
    definition: Definition,
    unit: Option<usize>,
}

/// The walk's result.
#[derive(Debug, Default)]
pub(crate) struct Workspace {
    /// Every fingerprinted unit, in walk order.
    pub(crate) units: Vec<Unit>,
    /// Every named item by symbol path.
    pub(crate) definitions: BTreeMap<String, Definition>,
    /// `module::name` introduced by a `use`, to what it names.
    imports: BTreeMap<String, Import>,
    /// Glob imports per module.
    globs: BTreeMap<String, Vec<Import>>,
    /// Every crate root ident the walk entered.
    pub(crate) crates: BTreeSet<String>,
    /// Files the walk could not parse (a hard failure for the caller).
    pub(crate) errors: Vec<String>,
    /// Every census rule hit over every first-party `.rs` file, tests,
    /// benches and examples included.
    pub(crate) rule_hits: Vec<crate::rules::RuleHit>,
    pending: Vec<Pending>,
    /// `(file, byte offset)` of every item already fingerprinted, so a file
    /// reached twice yields its units once.
    fingerprinted: BTreeSet<(String, usize)>,
}

impl Workspace {
    /// Walk every package under `crates/` and `bindings/` of `tree`.
    pub(crate) fn walk(tree: &dyn Tree) -> Self {
        let mut workspace = Self::default();
        for parent in ["crates", "bindings"] {
            for (name, is_dir) in tree.list(parent) {
                let package_dir = format!("{parent}/{name}");
                if is_dir && let Some(manifest) = tree.read(&format!("{package_dir}/Cargo.toml")) {
                    workspace.walk_package(tree, &package_dir, &manifest);
                }
            }
        }
        workspace.settle();
        workspace
    }

    fn walk_package(&mut self, tree: &dyn Tree, package_dir: &str, manifest: &str) {
        let manifest = Manifest::read(manifest);
        let Some(package) = manifest.package.clone() else {
            // A virtual manifest has no source of its own.
            return;
        };
        let mut parsed = BTreeMap::new();
        self.scan_rules(tree, &package, package_dir, package_dir, &mut parsed);
        let shipping = manifest.roots(tree, package_dir, &package);
        let test_code = manifest.test_roots(tree, package_dir, &package);
        for (roots, shipping) in [(shipping, true), (test_code, false)] {
            for (ident, root) in roots {
                self.crates.insert(ident.clone());
                let context = Context {
                    tree,
                    package: &package,
                    package_dir,
                    shipping,
                    parsed: &parsed,
                };
                let directory = parent_dir(&root);
                self.walk_file(&context, &root, &ident, &directory);
            }
        }
    }

    /// Keep `unit` unless its item was already fingerprinted through another path
    /// to the same file; its index in [`Self::units`] when kept.
    fn keep(&mut self, unit: Unit, offset: usize) -> Option<usize> {
        if self.fingerprinted.insert((unit.file.clone(), offset)) {
            self.units.push(unit);
            Some(self.units.len() - 1)
        } else {
            None
        }
    }

    /// Run the census rules over every `.rs` file under `directory`, whatever
    /// compiles it: a test's randomly seeded map is as nondeterministic as a
    /// library's. The byte-layout rules read shipping code only: a file under
    /// one of the package's test, bench, example or generated directories is a
    /// test's independent oracle, not a second implementation. Every file is
    /// parsed once: its text and syntax tree are kept in `parsed`, by path, for
    /// the walk.
    fn scan_rules(
        &mut self,
        tree: &dyn Tree,
        package: &str,
        package_dir: &str,
        directory: &str,
        parsed: &mut BTreeMap<String, (String, syn::File)>,
    ) {
        for (name, is_dir) in tree.list(directory) {
            let path = format!("{directory}/{name}");
            if is_dir {
                if name != "target" && !name.starts_with('.') {
                    self.scan_rules(tree, package, package_dir, &path, parsed);
                }
                continue;
            }
            if Path::new(&name)
                .extension()
                .is_none_or(|extension| extension != "rs")
            {
                continue;
            }
            let Some(source) = tree.read(&path) else {
                continue;
            };
            match syn::parse_file(&source) {
                Ok(file) => {
                    self.rule_hits
                        .extend(crate::rules::std_default_hasher(package, &path, &file));
                    self.rule_hits
                        .extend(crate::rules::hex_rules(package, &path, &file));
                    self.rule_hits
                        .extend(crate::rules::lex_rules(package, &path, &file));
                    self.rule_hits
                        .extend(crate::structure::structure_rules(package, &path, &file));
                    if !is_excluded(package_dir, &path) {
                        self.rule_hits
                            .extend(crate::layout::layout_rules(package, &path, &file));
                    }
                    parsed.insert(path, (source, file));
                }
                Err(error) => self.errors.push(format!("{path}: {error}")),
            }
        }
    }

    fn walk_file(&mut self, context: &Context<'_>, file: &str, module: &str, directory: &str) {
        let excluded = if context.shipping {
            is_excluded(context.package_dir, file)
        } else {
            is_outside_or_generated(context.package_dir, file)
        };
        if excluded {
            return;
        }
        let read;
        let (source, parsed) = if let Some((source, parsed)) = context.parsed.get(file) {
            (source, parsed)
        } else {
            // A file the package scan did not reach (not a `.rs` name, or under a
            // hidden directory) is read here; one it could not parse is already
            // an error.
            let Some(source) = context.tree.read(file) else {
                self.errors
                    .push(format!("{file}: declared as a module but not found"));
                return;
            };
            match syn::parse_file(&source) {
                Ok(parsed) => {
                    read = (source, parsed);
                    (&read.0, &read.1)
                }
                Err(error) => {
                    let error = format!("{file}: {error}");
                    if !self.errors.contains(&error) {
                        self.errors.push(error);
                    }
                    return;
                }
            }
        };
        let place = Place {
            file,
            source,
            module,
            directory,
        };
        self.walk_items(context, &place, &parsed.items);
    }

    fn walk_items(&mut self, context: &Context<'_>, place: &Place<'_>, items: &[syn::Item]) {
        for item in items {
            if is_test_only(item_attrs(item)) {
                continue;
            }
            match item {
                syn::Item::Fn(function) => {
                    let unit = self.function_unit(
                        context,
                        place,
                        &function.sig,
                        &function.block,
                        format!("{}::{}", place.module, function.sig.ident),
                    );
                    self.define(
                        context,
                        place,
                        &function.sig.ident,
                        &function.attrs,
                        place.module,
                    );
                    self.keep(
                        unit,
                        function.block.brace_token.span.join().byte_range().start,
                    );
                }
                syn::Item::Const(constant) => {
                    self.constant_unit(
                        context,
                        place,
                        &constant.ident,
                        &constant.expr,
                        &constant.attrs,
                    );
                }
                syn::Item::Static(item_static) => {
                    self.constant_unit(
                        context,
                        place,
                        &item_static.ident,
                        &item_static.expr,
                        &item_static.attrs,
                    );
                }
                syn::Item::Struct(item) => {
                    self.define(context, place, &item.ident, &item.attrs, place.module);
                }
                syn::Item::Enum(item) => {
                    self.define(context, place, &item.ident, &item.attrs, place.module);
                }
                syn::Item::Union(item) => {
                    self.define(context, place, &item.ident, &item.attrs, place.module);
                }
                syn::Item::Type(item) => {
                    self.define(context, place, &item.ident, &item.attrs, place.module);
                }
                syn::Item::TraitAlias(item) => {
                    self.define(context, place, &item.ident, &item.attrs, place.module);
                }
                syn::Item::Trait(item) => self.walk_trait(context, place, item),
                syn::Item::Impl(item) => self.walk_impl(context, place, item),
                syn::Item::Mod(item) => self.walk_mod(context, place, item),
                syn::Item::Use(item) => self.record_use(place.module, &item.tree, &mut Vec::new()),
                syn::Item::Macro(item) => {
                    if let Some(ident) = &item.ident {
                        let exported = item
                            .attrs
                            .iter()
                            .any(|attr| attr.path().is_ident("macro_export"));
                        let module = if exported {
                            place
                                .module
                                .split("::")
                                .next()
                                .unwrap_or(place.module)
                                .to_owned()
                        } else {
                            place.module.to_owned()
                        };
                        self.define(context, place, ident, &item.attrs, &module);
                    }
                }
                _ => {}
            }
        }
    }

    fn walk_trait(&mut self, context: &Context<'_>, place: &Place<'_>, item: &syn::ItemTrait) {
        self.define(context, place, &item.ident, &item.attrs, place.module);
        let owner = format!("{}::{}", place.module, item.ident);
        for trait_item in &item.items {
            match trait_item {
                syn::TraitItem::Fn(method) if !is_test_only(&method.attrs) => {
                    self.define(context, place, &method.sig.ident, &method.attrs, &owner);
                    if let Some(block) = &method.default {
                        let unit = self.function_unit(
                            context,
                            place,
                            &method.sig,
                            block,
                            format!("{owner}::{}", method.sig.ident),
                        );
                        self.keep(unit, block.brace_token.span.join().byte_range().start);
                    }
                }
                syn::TraitItem::Const(constant) if !is_test_only(&constant.attrs) => {
                    self.define(context, place, &constant.ident, &constant.attrs, &owner);
                }
                _ => {}
            }
        }
    }

    fn walk_impl(&mut self, context: &Context<'_>, place: &Place<'_>, item: &syn::ItemImpl) {
        let self_type = match item.self_ty.as_ref() {
            syn::Type::Path(path) => path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>(),
            _ => return,
        };
        for impl_item in &item.items {
            match impl_item {
                syn::ImplItem::Fn(method) if !is_test_only(&method.attrs) => {
                    let unit = self.function_unit(
                        context,
                        place,
                        &method.sig,
                        &method.block,
                        String::new(),
                    );
                    let unit = self.keep(
                        unit,
                        method.block.brace_token.span.join().byte_range().start,
                    );
                    self.pending.push(Pending {
                        module: place.module.to_owned(),
                        self_type: self_type.clone(),
                        name: method.sig.ident.to_string(),
                        definition: self.definition(
                            context,
                            place,
                            &method.sig.ident,
                            &method.attrs,
                            "",
                        ),
                        unit,
                    });
                }
                syn::ImplItem::Const(constant) if !is_test_only(&constant.attrs) => {
                    let mut unit = self.constant(context, place, &constant.ident, &constant.expr);
                    unit.symbol = String::new();
                    let unit = self.keep(unit, constant.ident.span().byte_range().start);
                    self.pending.push(Pending {
                        module: place.module.to_owned(),
                        self_type: self_type.clone(),
                        name: constant.ident.to_string(),
                        definition: self.definition(
                            context,
                            place,
                            &constant.ident,
                            &constant.attrs,
                            "",
                        ),
                        unit,
                    });
                }
                _ => {}
            }
        }
    }

    fn walk_mod(&mut self, context: &Context<'_>, place: &Place<'_>, item: &syn::ItemMod) {
        self.define(context, place, &item.ident, &item.attrs, place.module);
        let module = format!("{}::{}", place.module, item.ident);
        let child_directory = format!("{}/{}", place.directory, item.ident);
        if let Some((_, items)) = &item.content {
            let inner = Place {
                file: place.file,
                source: place.source,
                module: &module,
                directory: &child_directory,
            };
            self.walk_items(context, &inner, items);
            return;
        }
        let (file, directory) = if let Some(explicit) = path_attribute(&item.attrs) {
            let file = join_normalised(&parent_dir(place.file), &explicit);
            let directory = parent_dir(&file);
            (file, directory)
        } else {
            let flat = format!("{child_directory}.rs");
            if context.tree.read(&flat).is_some() {
                (flat, child_directory)
            } else {
                (format!("{child_directory}/mod.rs"), child_directory)
            }
        };
        self.walk_file(context, &file, &module, &directory);
    }

    fn record_use(&mut self, module: &str, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.record_use(module, &path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let ident = name.ident.to_string();
                if ident == "self" {
                    if let Some(last) = prefix.last() {
                        self.import(module, last, prefix.clone());
                    }
                } else {
                    let mut target = prefix.clone();
                    target.push(ident.clone());
                    self.import(module, &ident, target);
                }
            }
            syn::UseTree::Rename(rename) => {
                let mut target = prefix.clone();
                let ident = rename.ident.to_string();
                if ident != "self" {
                    target.push(ident);
                }
                self.import(module, &rename.rename.to_string(), target);
            }
            syn::UseTree::Glob(_) => {
                self.globs
                    .entry(module.to_owned())
                    .or_default()
                    .push(Import {
                        module: module.to_owned(),
                        target: prefix.clone(),
                    });
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.record_use(module, item, prefix);
                }
            }
        }
    }

    fn import(&mut self, module: &str, name: &str, target: Vec<String>) {
        if name == "_" {
            return;
        }
        self.imports
            .entry(format!("{module}::{name}"))
            .or_insert_with(|| Import {
                module: module.to_owned(),
                target,
            });
    }

    fn definition(
        &self,
        context: &Context<'_>,
        place: &Place<'_>,
        ident: &syn::Ident,
        attrs: &[syn::Attribute],
        module: &str,
    ) -> Definition {
        Definition {
            file: place.file.to_owned(),
            line: ident.span().start().line,
            package: context.package.to_owned(),
            module: module.to_owned(),
            documented: attrs.iter().any(|attr| attr.path().is_ident("doc")),
        }
    }

    fn define(
        &mut self,
        context: &Context<'_>,
        place: &Place<'_>,
        ident: &syn::Ident,
        attrs: &[syn::Attribute],
        module: &str,
    ) {
        let definition = self.definition(context, place, ident, attrs, module);
        self.definitions
            .entry(format!("{module}::{ident}"))
            .or_insert(definition);
    }

    fn function_unit(
        &self,
        context: &Context<'_>,
        place: &Place<'_>,
        signature: &syn::Signature,
        block: &syn::Block,
        symbol: String,
    ) -> Unit {
        let body = relex(place.source, block.brace_token.span.join(), true);
        let mut params = ParamNames::default();
        for input in &signature.inputs {
            params.visit_fn_arg(input);
        }
        let mut constants = BTreeSet::new();
        let mut tables = BTreeSet::new();
        normalize::integer_constants(&body, &mut constants);
        normalize::hex_tables(&body, &mut tables);
        let strings = absolute_lines(&body, block.brace_token.span.join().start().line);
        Unit {
            symbol,
            name: signature.ident.to_string(),
            package: context.package.to_owned(),
            file: place.file.to_owned(),
            line: signature.ident.span().start().line,
            kind: UnitKind::Function,
            print: Some(normalize::body_print(&body, &params.0)),
            constants,
            tables,
            strings,
            shipping: context.shipping,
        }
    }

    fn constant(
        &self,
        context: &Context<'_>,
        place: &Place<'_>,
        ident: &syn::Ident,
        expr: &syn::Expr,
    ) -> Unit {
        let tokens = relex(place.source, expr.span(), false);
        let mut constants = BTreeSet::new();
        let mut tables = BTreeSet::new();
        normalize::integer_constants(&tokens, &mut constants);
        normalize::hex_tables(&tokens, &mut tables);
        let strings = absolute_lines(&tokens, expr.span().start().line);
        Unit {
            symbol: format!("{}::{ident}", place.module),
            name: ident.to_string(),
            package: context.package.to_owned(),
            file: place.file.to_owned(),
            line: ident.span().start().line,
            kind: UnitKind::Constant,
            print: None,
            constants,
            tables,
            strings,
            shipping: context.shipping,
        }
    }

    fn constant_unit(
        &mut self,
        context: &Context<'_>,
        place: &Place<'_>,
        ident: &syn::Ident,
        expr: &syn::Expr,
        attrs: &[syn::Attribute],
    ) {
        let unit = self.constant(context, place, ident, expr);
        self.define(context, place, ident, attrs, place.module);
        self.keep(unit, ident.span().byte_range().start);
    }

    /// Place every impl item under its self type's defining module.
    fn settle(&mut self) {
        for pending in std::mem::take(&mut self.pending) {
            let owner = self
                .resolve_written(&pending.module, &pending.self_type)
                .unwrap_or_else(|| {
                    format!(
                        "{}::{}",
                        pending.module,
                        pending.self_type.last().map_or("_", String::as_str)
                    )
                });
            let symbol = format!("{owner}::{}", pending.name);
            let mut definition = pending.definition;
            definition.module.clone_from(&owner);
            self.definitions.entry(symbol.clone()).or_insert(definition);
            if let Some(index) = pending.unit {
                self.units[index].symbol = symbol;
            }
        }
    }

    /// Resolve a path as written inside `module` to a defined symbol.
    fn resolve_written(&self, module: &str, written: &[String]) -> Option<String> {
        let absolute = self.absolutise(module, written)?;
        self.resolve(&absolute)
    }

    /// The absolute form of a path written inside `module`.
    fn absolutise(&self, module: &str, written: &[String]) -> Option<String> {
        let (first, rest) = written.split_first()?;
        let crate_root = module.split("::").next().unwrap_or(module);
        let mut base: Vec<String> = match first.as_str() {
            "crate" => vec![crate_root.to_owned()],
            "self" => module.split("::").map(str::to_owned).collect(),
            "super" => {
                let mut parts: Vec<String> = module.split("::").map(str::to_owned).collect();
                parts.pop();
                parts
            }
            _ => {
                let local = format!("{module}::{first}");
                if self.definitions.contains_key(&local) || self.imports.contains_key(&local) {
                    module
                        .split("::")
                        .map(str::to_owned)
                        .chain([first.clone()])
                        .collect()
                } else {
                    vec![first.clone()]
                }
            }
        };
        for segment in rest {
            if segment == "super" {
                base.pop();
            } else {
                base.push(segment.clone());
            }
        }
        Some(base.join("::"))
    }

    /// Resolve `path` (absolute) to a defined symbol, following `use` aliases.
    ///
    /// A breadth-first walk over rewritten candidates with a visited set, so a
    /// glob cycle (`use super::*` both ways) costs one visit per distinct path.
    pub(crate) fn resolve(&self, path: &str) -> Option<String> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        queue.push_back(path.to_owned());
        while let Some(candidate) = queue.pop_front() {
            if self.definitions.contains_key(&candidate) {
                return Some(candidate);
            }
            if !seen.insert(candidate.clone()) || seen.len() > 4096 {
                continue;
            }
            let segments: Vec<&str> = candidate.split("::").collect();
            for split in (1..=segments.len()).rev() {
                let prefix = segments[..split].join("::");
                let rest = &segments[split..];
                if let Some(import) = self.imports.get(&prefix)
                    && let Some(target) = self.absolutise(&import.module, &import.target)
                    && target != prefix
                {
                    queue.push_back(join_path(&target, rest));
                }
                if split < segments.len()
                    && let Some(globs) = self.globs.get(&prefix)
                {
                    for glob in globs {
                        if let Some(target) = self.absolutise(&glob.module, &glob.target)
                            && target != prefix
                        {
                            queue.push_back(join_path(&target, rest));
                        }
                    }
                }
            }
        }
        None
    }
}

fn join_path(base: &str, rest: &[&str]) -> String {
    if rest.is_empty() {
        base.to_owned()
    } else {
        format!("{base}::{}", rest.join("::"))
    }
}

/// Which package is being walked, and from which kind of root.
struct Context<'a> {
    tree: &'a dyn Tree,
    package: &'a str,
    package_dir: &'a str,
    /// A library, binary or build-script root, not a test, bench or example.
    shipping: bool,
    /// Every `.rs` file of the package, by path: its text and syntax tree.
    parsed: &'a BTreeMap<String, (String, syn::File)>,
}

/// Where in the source the walk is.
struct Place<'a> {
    file: &'a str,
    source: &'a str,
    module: &'a str,
    /// The directory a child `mod x;` of this module is looked up in.
    directory: &'a str,
}

/// Every name a parameter pattern binds.
#[derive(Default)]
struct ParamNames(BTreeSet<String>);

impl<'ast> Visit<'ast> for ParamNames {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.0.insert(pat.ident.to_string());
        syn::visit::visit_pat_ident(self, pat);
    }

    fn visit_type(&mut self, _: &'ast syn::Type) {}
}

/// Re-lex the source a span covers. With `strip_braces`, the span is a braced
/// block and only its contents are returned.
fn relex(source: &str, span: proc_macro2::Span, strip_braces: bool) -> TokenStream {
    let range = span.byte_range();
    let (start, end) = if strip_braces {
        (range.start + 1, range.end.saturating_sub(1))
    } else {
        (range.start, range.end)
    };
    source
        .get(start..end.max(start))
        .and_then(|slice| TokenStream::from_str(slice).ok())
        .unwrap_or_default()
}

/// The string literals of a re-lexed stream whose first line is `first_line` of
/// its file, each carrying its line in the file.
fn absolute_lines(stream: &TokenStream, first_line: usize) -> Vec<(String, usize)> {
    let mut strings = Vec::new();
    normalize::string_literals(stream, &mut strings);
    for (_, line) in &mut strings {
        *line += first_line.saturating_sub(1);
    }
    strings
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

/// Whether the attributes make an item test-only: `#[test]`, or a `#[cfg]` whose
/// predicate can only hold under `cfg(test)`.
pub(crate) fn is_test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("test") {
            return true;
        }
        if !attr.path().is_ident("cfg") {
            return false;
        }
        attr.parse_args::<syn::Meta>()
            .is_ok_and(|meta| predicate_needs_test(&meta))
    })
}

fn predicate_needs_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let Ok(children) = list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return false;
            };
            if list.path.is_ident("all") {
                children.iter().any(predicate_needs_test)
            } else if list.path.is_ident("any") {
                !children.is_empty() && children.iter().all(predicate_needs_test)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// The value of a `#[path = "…"]` attribute.
pub(crate) fn path_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        let syn::Meta::NameValue(pair) = &attr.meta else {
            return None;
        };
        if !pair.path.is_ident("path") {
            return None;
        }
        match &pair.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) => Some(text.value()),
            _ => None,
        }
    })
}

fn parent_dir(file: &str) -> String {
    file.rsplit_once('/')
        .map_or_else(String::new, |(parent, _)| parent.to_owned())
}

/// `base/relative` with `.` and `..` folded.
fn join_normalised(base: &str, relative: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in Path::new(base).join(relative).components() {
        match component {
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    parts.join("/")
}

/// Whether `file` sits in a directory of its package that never holds shipping
/// source, or outside the package altogether.
fn is_excluded(package_dir: &str, file: &str) -> bool {
    let Some(inside) = file
        .strip_prefix(package_dir)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return true;
    };
    let mut components: Vec<&str> = inside.split('/').collect();
    components.pop();
    components
        .iter()
        .any(|directory| EXCLUDED_DIRS.contains(directory))
}

/// Whether `file` sits outside its package or under one of its `generated/`
/// directories: what a test, bench or example target never reads as its own.
fn is_outside_or_generated(package_dir: &str, file: &str) -> bool {
    let Some(inside) = file
        .strip_prefix(package_dir)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return true;
    };
    let mut components: Vec<&str> = inside.split('/').collect();
    components.pop();
    components.contains(&"generated")
}

/// The parts of a `Cargo.toml` the walk needs.
#[derive(Debug, Default)]
struct Manifest {
    package: Option<String>,
    lib_name: Option<String>,
    lib_path: Option<String>,
    bins: Vec<(Option<String>, Option<String>)>,
    /// `[[test]]`, `[[bench]]` and `[[example]]` rows: the table name, and the
    /// row's `name` and `path`.
    targets: Vec<(String, Option<String>, Option<String>)>,
}

impl Manifest {
    /// Read the `[package]`, `[lib]` and `[[bin]]` names and paths. The keys this
    /// needs are always plain `key = "value"` lines in this workspace.
    fn read(text: &str) -> Self {
        let mut manifest = Self::default();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                line.clone_into(&mut section);
                if section == "[[bin]]" {
                    manifest.bins.push((None, None));
                }
                if matches!(section.as_str(), "[[test]]" | "[[bench]]" | "[[example]]") {
                    manifest.targets.push((section.clone(), None, None));
                }
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            let Some(value) = value
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
            else {
                continue;
            };
            match (section.as_str(), key) {
                ("[package]", "name") => manifest.package = Some(value.to_owned()),
                ("[lib]", "name") => manifest.lib_name = Some(value.to_owned()),
                ("[lib]", "path") => manifest.lib_path = Some(value.to_owned()),
                ("[[bin]]", "name") => {
                    if let Some(bin) = manifest.bins.last_mut() {
                        bin.0 = Some(value.to_owned());
                    }
                }
                ("[[bin]]", "path") => {
                    if let Some(bin) = manifest.bins.last_mut() {
                        bin.1 = Some(value.to_owned());
                    }
                }
                ("[[test]]" | "[[bench]]" | "[[example]]", "name" | "path") => {
                    if let Some(target) = manifest.targets.last_mut() {
                        let slot = if key == "name" {
                            &mut target.1
                        } else {
                            &mut target.2
                        };
                        *slot = Some(value.to_owned());
                    }
                }
                _ => {}
            }
        }
        manifest
    }

    /// `(crate ident, root file)` for the library, every binary and the build
    /// script, in a fixed order.
    fn roots(&self, tree: &dyn Tree, package_dir: &str, package: &str) -> Vec<(String, String)> {
        let ident = |name: &str| name.replace('-', "_");
        let mut roots: Vec<(String, String)> = Vec::new();
        let lib = self
            .lib_path
            .clone()
            .unwrap_or_else(|| "src/lib.rs".to_owned());
        if tree.read(&format!("{package_dir}/{lib}")).is_some() {
            let name = self.lib_name.clone().unwrap_or_else(|| package.to_owned());
            roots.push((ident(&name), format!("{package_dir}/{lib}")));
        }
        let mut bin_files: BTreeSet<String> = BTreeSet::new();
        for (name, path) in &self.bins {
            let name = name.clone().unwrap_or_else(|| package.to_owned());
            let path = path.clone().unwrap_or_else(|| {
                if name == package {
                    "src/main.rs".to_owned()
                } else {
                    format!("src/bin/{name}.rs")
                }
            });
            if bin_files.insert(path.clone()) {
                roots.push((ident(&name), format!("{package_dir}/{path}")));
            }
        }
        if bin_files.insert("src/main.rs".to_owned())
            && tree.read(&format!("{package_dir}/src/main.rs")).is_some()
        {
            roots.push((ident(package), format!("{package_dir}/src/main.rs")));
        }
        for (name, is_dir) in tree.list(&format!("{package_dir}/src/bin")) {
            let (bin, path) = if is_dir {
                (name.clone(), format!("src/bin/{name}/main.rs"))
            } else if let Some(stem) = name.strip_suffix(".rs") {
                (stem.to_owned(), format!("src/bin/{name}"))
            } else {
                continue;
            };
            if bin_files.insert(path.clone())
                && tree.read(&format!("{package_dir}/{path}")).is_some()
            {
                roots.push((ident(&bin), format!("{package_dir}/{path}")));
            }
        }
        if tree.read(&format!("{package_dir}/build.rs")).is_some() {
            roots.push((
                format!("{}::build_script", ident(package)),
                format!("{package_dir}/build.rs"),
            ));
        }
        roots
    }

    /// `(crate ident, root file)` for every test, bench and example target, in a
    /// fixed order: the ones Cargo discovers under `tests/`, `benches/` and
    /// `examples/` (a `*.rs` file, or a directory's `main.rs`), and every
    /// `[[test]]`, `[[bench]]` and `[[example]]` path.
    fn test_roots(
        &self,
        tree: &dyn Tree,
        package_dir: &str,
        package: &str,
    ) -> Vec<(String, String)> {
        let package_ident = package.replace('-', "_");
        let mut roots: BTreeMap<String, String> = BTreeMap::new();
        for (directory, kind) in TEST_TARGETS {
            for (name, is_dir) in tree.list(&format!("{package_dir}/{directory}")) {
                let (target, path) = if is_dir {
                    (name.clone(), format!("{directory}/{name}/main.rs"))
                } else if let Some(stem) = name.strip_suffix(".rs") {
                    (stem.to_owned(), format!("{directory}/{name}"))
                } else {
                    continue;
                };
                if tree.read(&format!("{package_dir}/{path}")).is_some() {
                    roots.entry(path).or_insert_with(|| {
                        format!("{package_ident}::{kind}::{}", target.replace('-', "_"))
                    });
                }
            }
        }
        for (section, name, path) in &self.targets {
            let (directory, kind) = match section.as_str() {
                "[[test]]" => TEST_TARGETS[0],
                "[[bench]]" => TEST_TARGETS[1],
                _ => TEST_TARGETS[2],
            };
            let Some(name) = name else { continue };
            let path = path
                .clone()
                .unwrap_or_else(|| format!("{directory}/{name}.rs"));
            if tree.read(&format!("{package_dir}/{path}")).is_some() {
                roots.entry(path).or_insert_with(|| {
                    format!("{package_ident}::{kind}::{}", name.replace('-', "_"))
                });
            }
        }
        roots
            .into_iter()
            .map(|(path, ident)| (ident, format!("{package_dir}/{path}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Memory, Workspace, is_excluded, join_normalised};

    fn workspace(files: &[(&str, &str)]) -> Workspace {
        let memory = Memory {
            files: files
                .iter()
                .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
                .collect(),
        };
        let walked = Workspace::walk(&memory);
        assert!(walked.errors.is_empty(), "{:?}", walked.errors);
        walked
    }

    #[test]
    fn modules_resolve_through_reexports_and_impls_land_on_their_type() {
        let walked = workspace(&[
            ("crates/a/Cargo.toml", "[package]\nname = \"demo-a\"\n"),
            (
                "crates/a/src/lib.rs",
                "mod base;\nmod ops;\npub use base::Thing;\n#[cfg(test)]\nmod tests;\n",
            ),
            ("crates/a/src/base.rs", "/// A thing.\npub struct Thing;\n"),
            (
                "crates/a/src/ops.rs",
                "use crate::base::Thing;\nimpl Thing {\n    /// Go.\n    pub fn go(&self) -> u8 { 1 }\n}\n#[cfg(test)]\nmod tests { fn hidden() {} }\n",
            ),
        ]);
        assert_eq!(
            walked.resolve("demo_a::Thing::go").as_deref(),
            Some("demo_a::base::Thing::go")
        );
        assert!(walked.definitions["demo_a::base::Thing::go"].documented);
        assert_eq!(
            walked.definitions["demo_a::base::Thing::go"].module,
            "demo_a::base::Thing"
        );
        assert!(walked.resolve("demo_a::ops::tests::hidden").is_none());
        assert!(walked.resolve("demo_a::Thing::gone").is_none());
    }

    #[test]
    fn test_only_items_and_test_directories_are_not_walked() {
        let walked = workspace(&[
            ("crates/a/Cargo.toml", "[package]\nname = \"demo-a\"\n"),
            (
                "crates/a/src/lib.rs",
                "#[test]\nfn t() {}\n#[cfg(all(test, unix))]\nfn u() {}\n#[cfg(any(test, unix))]\nfn shipped() {}\n#[cfg(not(test))]\nfn also_shipped() {}\n#[path = \"../tests/support.rs\"]\nmod support;\n",
            ),
            ("crates/a/tests/support.rs", "pub fn helper() {}\n"),
        ]);
        let shipping: Vec<&str> = walked
            .units
            .iter()
            .filter(|unit| unit.shipping)
            .map(|unit| unit.name.as_str())
            .collect();
        assert_eq!(shipping, ["shipped", "also_shipped"]);
        // The library's `#[path]` into `tests/` is not followed; the file is a test
        // target of its own, walked as test code.
        let tests: Vec<&str> = walked
            .units
            .iter()
            .filter(|unit| !unit.shipping)
            .map(|unit| unit.symbol.as_str())
            .collect();
        assert_eq!(tests, ["demo_a::test_crate::support::helper"]);
    }

    /// Every test, bench and example target is a crate of its own, the declared
    /// ones included; a file several targets declare is fingerprinted once.
    #[test]
    fn test_targets_are_crates_and_a_shared_file_is_one_source() {
        let walked = workspace(&[
            (
                "crates/a/Cargo.toml",
                "[package]\nname = \"demo-a\"\n[[bench]]\nname = \"speed\"\npath = \"benches/speed/run.rs\"\nharness = false\n",
            ),
            ("crates/a/src/lib.rs", "pub fn lib() {}\n"),
            ("crates/a/tests/one.rs", "mod common;\nfn only_one() {}\n"),
            (
                "crates/a/tests/two/main.rs",
                "#[path = \"../common/mod.rs\"]\nmod common;\n",
            ),
            ("crates/a/tests/common/mod.rs", "pub fn shared() {}\n"),
            ("crates/a/benches/speed/run.rs", "fn main() {}\n"),
            ("crates/a/examples/demo.rs", "fn main() {}\n"),
        ]);
        let mut symbols: Vec<(&str, bool)> = walked
            .units
            .iter()
            .map(|unit| (unit.symbol.as_str(), unit.shipping))
            .collect();
        symbols.sort_unstable();
        assert_eq!(
            symbols,
            [
                ("demo_a::bench_crate::speed::main", false),
                ("demo_a::example_crate::demo::main", false),
                ("demo_a::lib", true),
                ("demo_a::test_crate::one::common::shared", false),
                ("demo_a::test_crate::one::only_one", false),
            ]
        );
        // The second declaration of the shared file still resolves.
        assert!(
            walked
                .resolve("demo_a::test_crate::two::common::shared")
                .is_some()
        );
    }

    #[test]
    fn paths_fold_and_escapes_are_excluded() {
        assert_eq!(
            join_normalised("crates/a/src", "../../b/src/x.rs"),
            "crates/b/src/x.rs"
        );
        assert!(is_excluded("crates/a", "crates/b/src/x.rs"));
        assert!(is_excluded("crates/a", "crates/a/generated/x.rs"));
        assert!(!is_excluded("crates/a", "crates/a/src/x.rs"));
    }
}
