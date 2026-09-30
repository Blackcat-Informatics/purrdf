// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Census rules that hold three structural jobs to their homes.
//!
//! * `reifier-quad-new`: `RdfQuad::new(_, rdf:reifies, _)` (or an `RdfQuad::new`
//!   whose object is a triple term) in shipping code. Such a row is built in the
//!   default graph, so it drops the graph the reifier was declared in. The one
//!   home for the flat reifier and annotation rows is `purrdf_rdf::flat_rdf_quads`,
//!   which keeps `reifier.graph`; a caller that must flatten the graph away
//!   clears `graph_name` on the home's quads.
//! * `default-hasher-new`: `DefaultHasher::new()` / `DefaultHasher::default()` in
//!   any file under a `src` directory, test modules included. `std`'s
//!   `DefaultHasher` is an unspecified algorithm that may change between
//!   toolchains; every hash whose value is observable goes through
//!   `purrdf_hash`.
//! * `rdf-list-walk`: a function that names both `rdf:first` and `rdf:rest` and
//!   loops (or calls itself), and calls none of the home's list entry points
//!   (`walk_rdf_list`, `build_rdf_list`, `convertible_list_cells`, `rdf_list*`). That is a hand-rolled list walk with its own
//!   malformed-list contract; the walk lives in `DatasetView` (`rdf_list_with`),
//!   whose policy argument names the contract. Code inside an `impl` of, or the
//!   definition of, `DatasetView` is the home and is not reported.
//!
//! The first and third read shipping code only, as the lexical rules do: an item
//! under `#[test]` or a `#[cfg]` that needs `cfg(test)`, and every file under a
//! `tests`, `benches` or `examples` directory, may spell the pattern.

use syn::visit::Visit;

use crate::rules::RuleHit;

/// `rule:reifier-quad-new`.
pub(crate) const REIFIER_QUAD_NEW: &str = "rule:reifier-quad-new";
/// `rule:default-hasher-new`.
pub(crate) const DEFAULT_HASHER_NEW: &str = "rule:default-hasher-new";
/// `rule:rdf-list-walk`.
pub(crate) const RDF_LIST_WALK: &str = "rule:rdf-list-walk";

/// Every rule this module computes.
pub(crate) const RULES: [&str; 3] = [REIFIER_QUAD_NEW, DEFAULT_HASHER_NEW, RDF_LIST_WALK];

const REIFIER_DETAIL: &str = "an `RdfQuad::new` reifier row is built in the default graph and drops the reifier's \
     graph; build reifier and annotation rows through purrdf_rdf::flat_rdf_quads (which keeps \
     `reifier.graph`), clearing `graph_name` afterwards if the graph must be flattened";
const HASHER_DETAIL: &str = "`std`'s `DefaultHasher` is an unspecified algorithm; hash through purrdf_hash \
     (`fnv`, `mix`, or a `Domain`-keyed digest)";
const LIST_DETAIL: &str = "a hand-rolled `rdf:first`/`rdf:rest` walk; walk lists through \
     `DatasetView::rdf_list_with`, whose policy names the malformed-list contract";

/// The list entry points of the home (`purrdf_core::collections` and
/// `DatasetView`): a function that calls one reads or builds lists through the
/// home, and naming `rdf:first`/`rdf:rest` beside it is supplying the accessors.
const LIST_HOME_ENTRY_POINTS: [&str; 7] = [
    "walk_rdf_list",
    "RdfListWalk",
    "build_rdf_list",
    "convertible_list_cells",
    "rdf_list",
    "rdf_list_strict",
    "rdf_list_with",
];

/// Per-function evidence for the list-walk rule.
#[derive(Default)]
struct FnState {
    name: String,
    rest: Option<usize>,
    /// `rdf:rest` is named inside a loop, directly or through a local bound to it.
    rest_in_loop: bool,
    /// Loop nesting at the current node.
    depth: usize,
    /// Mentions counted so far, so a `let` can tell what its initialiser named.
    first_mentions: usize,
    rest_mentions: usize,
    /// Locals bound to the `rdf:first` / `rdf:rest` name.
    first_locals: Vec<String>,
    rest_locals: Vec<String>,
    recursive: bool,
    /// Calls one of the home's list entry points.
    uses_home: bool,
}

struct Structure<'a> {
    package: &'a str,
    file: &'a str,
    scope: Vec<String>,
    in_test: usize,
    /// Inside `DatasetView`'s definition or an impl of it.
    in_home: usize,
    /// The file is under a `src` directory.
    in_src: bool,
    functions: Vec<FnState>,
    hits: Vec<RuleHit>,
}

fn last_two(path: &syn::Path) -> (Option<String>, Option<String>) {
    let mut segments = path.segments.iter().rev();
    let last = segments.next().map(|segment| segment.ident.to_string());
    let before = segments.next().map(|segment| segment.ident.to_string());
    (before, last)
}

/// Whether `text` spells the `rdf:reifies` predicate.
fn is_reifies_literal(text: &str) -> bool {
    text == "rdf:reifies" || text.ends_with("22-rdf-syntax-ns#reifies")
}

/// Whether `expression` names the reifies predicate: an `RDF_REIFIES`/`REIFIES`
/// constant or the literal IRI.
fn is_reifies(expression: &syn::Expr) -> bool {
    match expression {
        syn::Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
            matches!(
                segment.ident.to_string().as_str(),
                "RDF_REIFIES" | "REIFIES"
            )
        }),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(text),
            ..
        }) => is_reifies_literal(&text.value()),
        syn::Expr::Reference(reference) => is_reifies(&reference.expr),
        syn::Expr::MethodCall(call) => is_reifies(&call.receiver),
        _ => false,
    }
}

/// Whether `expression` is a triple term: `RdfTerm::triple(..)` or
/// `RdfTerm::Triple(..)`.
fn is_triple_term(expression: &syn::Expr) -> bool {
    matches!(expression, syn::Expr::Call(call)
        if matches!(call.func.as_ref(), syn::Expr::Path(path)
            if path.path.segments.last().is_some_and(|segment| segment.ident == "triple" || segment.ident == "Triple")))
}

/// `Some(true)` for an `rdf:first` name, `Some(false)` for `rdf:rest`: the
/// `RDF_FIRST`/`rdf_first` spellings, or `FIRST`/`REST` reached through an `rdf`
/// module path (`vocab::rdf::REST`).
fn list_ident(before: Option<&str>, name: &str) -> Option<bool> {
    match name {
        "RDF_FIRST" | "rdf_first" | "RDF_FIRST_IRI" => Some(true),
        "RDF_REST" | "rdf_rest" | "RDF_REST_IRI" => Some(false),
        "FIRST" if before.is_some_and(|module| module.eq_ignore_ascii_case("rdf")) => Some(true),
        "REST" if before.is_some_and(|module| module.eq_ignore_ascii_case("rdf")) => Some(false),
        _ => None,
    }
}

fn list_literal(text: &str) -> Option<bool> {
    if text == "rdf:first" || text.ends_with("22-rdf-syntax-ns#first") {
        Some(true)
    } else if text == "rdf:rest" || text.ends_with("22-rdf-syntax-ns#rest") {
        Some(false)
    } else {
        None
    }
}

impl Structure<'_> {
    fn hit(&mut self, rule: &'static str, line: usize, detail: &str) {
        let mut symbol = self.file.to_owned();
        for scope in &self.scope {
            symbol.push_str("::");
            symbol.push_str(scope);
        }
        self.hits.push(RuleHit {
            rule,
            package: self.package.to_owned(),
            file: self.file.to_owned(),
            line,
            symbol,
            detail: detail.to_owned(),
            home_exempt: false,
        });
    }

    fn shipping(&self) -> bool {
        self.in_test == 0
    }

    fn note_list(&mut self, which: bool, line: usize) {
        if let Some(state) = self.functions.last_mut() {
            if which {
                state.first_mentions += 1;
            } else {
                state.rest.get_or_insert(line);
                state.rest_mentions += 1;
                state.rest_in_loop |= state.depth > 0;
            }
        }
    }

    fn function(&mut self, name: String, attrs: &[syn::Attribute], walk: impl FnOnce(&mut Self)) {
        let test = crate::source::is_test_only(attrs);
        self.in_test += usize::from(test);
        self.scope.push(name.clone());
        self.functions.push(FnState {
            name,
            ..FnState::default()
        });
        walk(self);
        let state = self.functions.pop().unwrap_or_default();
        if self.shipping()
            && self.in_home == 0
            && !state.uses_home
            && state.first_mentions > 0
            && (state.rest_in_loop || state.recursive)
            && let Some(line) = state.rest
        {
            self.hit(RDF_LIST_WALK, line, LIST_DETAIL);
        }
        self.scope.pop();
        self.in_test -= usize::from(test);
    }

    fn looping(&mut self, walk: impl FnOnce(&mut Self)) {
        if let Some(state) = self.functions.last_mut() {
            state.depth += 1;
        }
        walk(self);
        if let Some(state) = self.functions.last_mut() {
            state.depth -= 1;
        }
    }
}

impl<'ast> Visit<'ast> for Structure<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = node.func.as_ref() {
            let (before, last) = last_two(&function.path);
            let line = function
                .path
                .segments
                .last()
                .map_or(0, |segment| segment.ident.span().start().line);
            if self.shipping()
                && before.as_deref() == Some("RdfQuad")
                && last.as_deref() == Some("new")
                && node.args.len() == 3
                && (node.args.iter().nth(1).is_some_and(is_reifies)
                    || node.args.iter().nth(2).is_some_and(is_triple_term))
            {
                self.hit(REIFIER_QUAD_NEW, line, REIFIER_DETAIL);
            }
            if self.in_src
                && before.as_deref() == Some("DefaultHasher")
                && matches!(last.as_deref(), Some("new" | "default"))
            {
                self.hit(DEFAULT_HASHER_NEW, line, HASHER_DETAIL);
            }
            if let (Some(state), Some(name)) = (self.functions.last_mut(), last.as_deref())
                && LIST_HOME_ENTRY_POINTS.contains(&name)
            {
                state.uses_home = true;
            }
            if function.path.segments.len() == 1
                && let (Some(state), Some(name)) = (self.functions.last_mut(), last.as_deref())
                && state.name == name
            {
                state.recursive = true;
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if let Some(state) = self.functions.last_mut()
            && LIST_HOME_ENTRY_POINTS.contains(&node.method.to_string().as_str())
        {
            state.uses_home = true;
        }
        if matches!(node.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self"))
            && let Some(state) = self.functions.last_mut()
            && node.method == state.name
        {
            state.recursive = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if let Some(name) = node.path.get_ident().map(ToString::to_string)
            && let Some(state) = self.functions.last_mut()
        {
            if state.first_locals.contains(&name) {
                state.first_mentions += 1;
            }
            if state.rest_locals.contains(&name) {
                state
                    .rest
                    .get_or_insert_with(|| node.path.segments[0].ident.span().start().line);
                state.rest_in_loop |= state.depth > 0;
            }
        }
        let (before, last) = last_two(&node.path);
        if let Some(segment) = node.path.segments.last()
            && let Some(which) = last
                .as_deref()
                .and_then(|name| list_ident(before.as_deref(), name))
        {
            self.note_list(which, segment.ident.span().start().line);
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        let before = self
            .functions
            .last()
            .map(|state| (state.first_mentions, state.rest_mentions));
        syn::visit::visit_local(self, node);
        if let (Some((first, rest)), syn::Pat::Ident(pattern)) = (before, &node.pat)
            && let Some(state) = self.functions.last_mut()
        {
            let name = pattern.ident.to_string();
            if state.first_mentions > first {
                state.first_locals.push(name.clone());
            }
            if state.rest_mentions > rest {
                state.rest_locals.push(name);
            }
        }
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        if let syn::Member::Named(name) = &node.member
            && let Some(which) = list_ident(None, &name.to_string())
        {
            self.note_list(which, name.span().start().line);
        }
        syn::visit::visit_expr_field(self, node);
    }

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        if let Some(which) = list_literal(&node.value()) {
            self.note_list(which, node.span().start().line);
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        crate::rules::visit_macro_arguments(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.looping(|this| syn::visit::visit_expr_for_loop(this, node));
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.looping(|this| syn::visit::visit_expr_while(this, node));
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.looping(|this| syn::visit::visit_expr_loop(this, node));
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.function(node.sig.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_fn(this, node);
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.function(node.sig.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_impl_item_fn(this, node);
        });
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.function(node.sig.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_trait_item_fn(this, node);
        });
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.function(node.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_mod(this, node);
        });
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let names_home = |path: &syn::Path| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == "DatasetView")
        };
        let home = node
            .trait_
            .as_ref()
            .is_some_and(|(_, path, _)| names_home(path))
            || matches!(node.self_ty.as_ref(), syn::Type::Path(path) if names_home(&path.path));
        self.in_home += usize::from(home);
        let test = crate::source::is_test_only(&node.attrs);
        self.in_test += usize::from(test);
        syn::visit::visit_item_impl(self, node);
        self.in_test -= usize::from(test);
        self.in_home -= usize::from(home);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        let home = node.ident == "DatasetView";
        self.in_home += usize::from(home);
        let test = crate::source::is_test_only(&node.attrs);
        self.in_test += usize::from(test);
        syn::visit::visit_item_trait(self, node);
        self.in_test -= usize::from(test);
        self.in_home -= usize::from(home);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.function(node.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_const(this, node);
        });
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.function(node.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_static(this, node);
        });
    }
}

/// Whether `file` sits under a `src` directory.
fn is_src_file(file: &str) -> bool {
    file.split('/')
        .rev()
        .skip(1)
        .any(|directory| directory == "src")
}

/// Every `reifier-quad-new`, `default-hasher-new` and `rdf-list-walk` hit in one
/// parsed file.
pub(crate) fn structure_rules(package: &str, file: &str, parsed: &syn::File) -> Vec<RuleHit> {
    let test_file = crate::rules::is_test_file(file);
    let mut visitor = Structure {
        package,
        file,
        scope: Vec::new(),
        // A test file is wholly test code for the shipping-only rules; the
        // `DefaultHasher` rule reads `src` files, test modules included.
        in_test: usize::from(test_file),
        in_home: 0,
        in_src: is_src_file(file),
        functions: Vec::new(),
        hits: Vec::new(),
    };
    visitor.visit_file(parsed);
    visitor.hits
}

#[cfg(test)]
mod tests {
    fn found(file: &str, source: &str) -> Vec<(&'static str, usize)> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        super::structure_rules("p", file, &parsed)
            .iter()
            .map(|hit| (hit.rule, hit.line))
            .collect()
    }

    #[test]
    fn a_reifier_row_built_with_new_is_a_hit_and_a_plain_row_is_not() {
        let source = "\
fn f(s: RdfTerm, t: RdfTriple) {
    let _ = RdfQuad::new(s.clone(), RDF_REIFIES, RdfTerm::triple(t));
    let _ = RdfQuad::new(s.clone(), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies\", o);
    let _ = RdfQuad::new(s.clone(), \"https://example.org/p\", RdfTerm::iri(\"https://example.org/o\"));
}
#[cfg(test)]
mod tests { fn g(s: RdfTerm) { let _ = RdfQuad::new(s, RDF_REIFIES, t); } }
";
        assert_eq!(
            found("crates/p/src/lib.rs", source),
            vec![(super::REIFIER_QUAD_NEW, 2), (super::REIFIER_QUAD_NEW, 3)]
        );
        assert_eq!(found("crates/p/tests/it.rs", source), Vec::new());
    }

    #[test]
    fn a_default_hasher_in_src_is_a_hit_even_in_a_test_module() {
        let source = "\
fn f() { let _ = std::collections::hash_map::DefaultHasher::new(); }
#[cfg(test)]
mod tests { fn g() { let _ = DefaultHasher::default(); } }
fn h() { let _ = FixedState::default(); }
";
        assert_eq!(
            found("crates/p/src/lib.rs", source),
            vec![
                (super::DEFAULT_HASHER_NEW, 1),
                (super::DEFAULT_HASHER_NEW, 3)
            ]
        );
        assert_eq!(found("crates/p/tests/it.rs", source), Vec::new());
    }

    #[test]
    fn a_looping_first_and_rest_reader_is_a_hit_and_a_neighbour_is_not() {
        let source = "\
fn walk(g: &G, mut node: u32) {
    while let Some(next) = g.object(node, RDF_REST) {
        let _ = g.object(node, RDF_FIRST);
        node = next;
    }
}
fn once(g: &G, node: u32) { let _ = g.object(node, RDF_FIRST); let _ = g.object(node, RDF_REST); }
fn only_first(g: &G) { for n in 0..3 { let _ = g.object(n, RDF_FIRST); } }
trait DatasetView { fn list(&self) { loop { let _ = (RDF_FIRST, RDF_REST); } } }
struct S;
impl DatasetView for S { fn list(&self) { loop { let _ = (RDF_FIRST, RDF_REST); } } }
";
        assert_eq!(
            found("crates/p/src/lib.rs", source),
            vec![(super::RDF_LIST_WALK, 2)]
        );
    }
}
