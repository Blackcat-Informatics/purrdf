// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Census rules: syntactic properties checked over every first-party Rust file,
//! tests, benches and examples included.
//!
//! `std-default-hasher` finds every place a `std` hash map or set is left on its
//! default hasher, `RandomState`, whose per-process random keys randomise
//! iteration order. The clippy bans on `HashMap::new`/`with_capacity` cannot reach
//! the rest: `HashMap::default()`, `HashMap::from(…)`, `FromIterator` through
//! `.collect()`, and a field or binding typed `HashMap<K, V>` that `Default`
//! fills — a `default` ban would also refuse `FastMap::default()`. The rule is
//! therefore stated on the types and paths themselves:
//!
//! * a type path naming `std`'s `HashMap` with fewer than three generic type
//!   arguments, or `HashSet` with fewer than two — the hasher left to default;
//! * a constructor path `HashMap::{default, from, from_iter, new,
//!   with_capacity}` (and the `HashSet` ones) whose `HashMap` segment does not
//!   name a hasher by turbofish, unless it is the whole initialiser of a `let`
//!   whose annotation names the hasher (`let m: HashMap<K, V, FixedState> =
//!   HashMap::default();`). Anywhere else the call's hasher comes from inference
//!   at best; the fix is to name the alias that carries the hasher
//!   (`FastMap::default()`, `FastSet::from_iter(…)`).
//!
//! A path names `std`'s map when it is spelt through `std`/`core`/`alloc`
//! (`std::collections::HashMap`, `collections::HashMap` after `use
//! std::collections`, `hash_map::HashMap`), or bare after a `use` that brings the
//! `std` type in under that name. A bare name the file brings in from anywhere
//! else (`hashbrown::HashMap`) or declares itself is not `std`'s. A bare name the
//! file neither imports nor declares is taken as `std`'s: a glob import from a
//! parent module is the only way it can be in scope, and nothing in this
//! workspace defines another type by that name. Type aliases that name the
//! hasher (`FastMap`, `FixedMap`, …) are other names and never match.
//!
//! Macro arguments are parsed as expressions where they are expressions
//! (`assert_eq!(m, HashMap::from(…))`), so a constructor inside one is seen.
//!
//! `vocabulary-literal` finds a vocabulary term written out as a string where
//! the vocabulary's constant should be named. Its namespaces are not listed
//! here: they are read from the job's home and entry-point modules, every
//! `const` named `NS` or ending in `_NS` declared under them. A string literal in
//! shipping source outside those modules that equals one of those namespaces or
//! starts with one is a hit — it is a term, or a namespace, typed a second
//! time. A literal holding a line break and a Turtle or SPARQL keyword is an
//! embedded document (a test query, a shapes graph) and is not a hit: the
//! namespaces inside it are that document's own text.
//!
//! `home-literal` finds a token the job's home spells written out again: a
//! string literal in shipping source outside the home item that equals one of
//! the string literals the home item's own bodies hold. A home whose job is to
//! spell and read a closed set of tokens (the `ltr`/`rtl` of a base direction)
//! is the one place those tokens are typed; anywhere else a literal equal to
//! one of them is a second spelling or a second parser.

use std::collections::BTreeSet;

use syn::visit::Visit;

/// The rule id a ledger job's `forbidden.fingerprints` names to take the hits.
pub(crate) const STD_DEFAULT_HASHER: &str = "rule:std-default-hasher";

/// Rule ids a ledger job may name whose hits another gate computes: the census
/// accepts the id and reports no hit of its own, and `check-shared-helpers.py`
/// adds that gate's hits to the job before judging it.
///
/// Both are `scripts/check-hash-domains.py`'s rules: `raw-hash-domain`, a
/// domain-shaped literal handed to a hasher, or declared as a domain constant,
/// without going through `purrdf_hash::Domain`; and `shared-hash-domain`, one
/// registered domain opening hashers in two functions of its file. Their reading
/// of Rust — string escapes, `#[cfg(test)]` items, test-only modules — lives in
/// that script alone.
pub(crate) const DELEGATED_RULES: [&str; 2] = ["rule:raw-hash-domain", "rule:shared-hash-domain"];

/// The rule id a ledger job's `forbidden.fingerprints` names to forbid a
/// vocabulary term or namespace written out as a string literal.
pub(crate) const VOCABULARY_LITERAL: &str = "rule:vocabulary-literal";

/// The rule id a ledger job's `forbidden.fingerprints` names to forbid a
/// literal its home item spells, written out anywhere else.
pub(crate) const HOME_LITERAL: &str = "rule:home-literal";

/// The keywords that mark a multi-line literal as an embedded Turtle, TriG or
/// SPARQL document; compared case-insensitively, as whole words.
const DOCUMENT_KEYWORDS: [&str; 15] = [
    "@prefix",
    "@base",
    "prefix",
    "base",
    "select",
    "construct",
    "ask",
    "describe",
    "insert",
    "delete",
    "where",
    "load",
    "clear",
    "create",
    "drop",
];

/// Whether a string literal is an embedded document: it spans lines and holds
/// a Turtle or SPARQL keyword as a whole word.
pub(crate) fn is_embedded_document(text: &str) -> bool {
    text.contains('\n')
        && text
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '@' || c == '_'))
            .any(|word| {
                DOCUMENT_KEYWORDS
                    .iter()
                    .any(|keyword| word.eq_ignore_ascii_case(keyword))
            })
}

/// Whether `name` names a namespace constant.
pub(crate) fn is_namespace_name(name: &str) -> bool {
    name == "NS" || name.ends_with("_NS")
}

/// The namespace `literal` equals or starts with, if any: the longest, so the
/// report names the most specific vocabulary.
pub(crate) fn vocabulary_namespace<'a>(literal: &str, namespaces: &'a [String]) -> Option<&'a str> {
    if is_embedded_document(literal) {
        return None;
    }
    namespaces
        .iter()
        .filter(|namespace| literal.starts_with(namespace.as_str()))
        .max_by_key(|namespace| namespace.len())
        .map(String::as_str)
}

/// The constructors whose hasher is the default unless the type names one.
const CONSTRUCTORS: [&str; 5] = ["default", "from", "from_iter", "new", "with_capacity"];

/// One place a rule fired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleHit {
    /// The rule id.
    pub(crate) rule: &'static str,
    /// The package whose directory holds the file.
    pub(crate) package: String,
    /// The repo-relative file.
    pub(crate) file: String,
    /// 1-based line.
    pub(crate) line: usize,
    /// The enclosing items, outermost first, joined by `::` after the file.
    pub(crate) symbol: String,
    /// What was found.
    pub(crate) detail: String,
}

/// How many type arguments name the hasher explicitly.
fn required_arguments(name: &str) -> Option<usize> {
    match name {
        "HashMap" => Some(3),
        "HashSet" => Some(2),
        _ => None,
    }
}

/// What the file says about the names `HashMap` and `HashSet`.
#[derive(Debug, Default)]
struct Names {
    /// Local names bound to `std`'s `HashMap`/`HashSet`: `(local, std name)`.
    std_types: BTreeSet<(String, String)>,
    /// Local names of `HashMap`/`HashSet` brought in from elsewhere, or declared.
    foreign: BTreeSet<String>,
    /// Local names bound to a `std` module that holds them (`collections`,
    /// `hash_map`, `hash_set`).
    std_modules: BTreeSet<String>,
}

impl Names {
    fn collect(file: &syn::File) -> Self {
        let mut names = Self::default();
        names.visit_file(file);
        names
    }

    fn record_use(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.record_use(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                self.bind(prefix, &name.ident.to_string(), &name.ident.to_string());
            }
            syn::UseTree::Rename(rename) => {
                self.bind(
                    prefix,
                    &rename.ident.to_string(),
                    &rename.rename.to_string(),
                );
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.record_use(item, prefix);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }

    fn bind(&mut self, prefix: &[String], imported: &str, local: &str) {
        let from_std = prefix
            .first()
            .is_some_and(|root| matches!(root.as_str(), "std" | "core" | "alloc"));
        if imported == "self" {
            if let Some(module) = prefix.last()
                && from_std
                && matches!(module.as_str(), "collections" | "hash_map" | "hash_set")
            {
                let local = if local == "self" {
                    module.as_str()
                } else {
                    local
                };
                self.std_modules.insert(local.to_owned());
            }
            return;
        }
        if required_arguments(imported).is_some() {
            if from_std {
                self.std_types
                    .insert((local.to_owned(), imported.to_owned()));
            } else {
                self.foreign.insert(local.to_owned());
            }
        } else if from_std && matches!(imported, "collections" | "hash_map" | "hash_set") {
            self.std_modules.insert(local.to_owned());
        } else if required_arguments(local).is_some() {
            self.foreign.insert(local.to_owned());
        }
    }

    /// The `std` type (`HashMap`/`HashSet`) `path`'s segment at `index` names, if
    /// it names one.
    fn std_type_at(&self, path: &syn::Path, index: usize) -> Option<&'static str> {
        let ident = path.segments[index].ident.to_string();
        let prefix: Vec<String> = path
            .segments
            .iter()
            .take(index)
            .map(|segment| segment.ident.to_string())
            .collect();
        let named = if prefix.is_empty() {
            if let Some((_, std_name)) = self.std_types.iter().find(|(local, _)| *local == ident) {
                std_name.clone()
            } else if self.foreign.contains(&ident) {
                return None;
            } else {
                ident
            }
        } else {
            let std_spelt = matches!(prefix[0].as_str(), "std" | "core" | "alloc")
                || (prefix.len() == 1 && self.std_modules.contains(&prefix[0]));
            if !std_spelt {
                return None;
            }
            ident
        };
        match named.as_str() {
            "HashMap" => Some("HashMap"),
            "HashSet" => Some("HashSet"),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for Names {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.record_use(&item.tree, &mut Vec::new());
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        if required_arguments(&item.ident.to_string()).is_some() {
            self.foreign.insert(item.ident.to_string());
        }
        syn::visit::visit_item_type(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if required_arguments(&item.ident.to_string()).is_some() {
            self.foreign.insert(item.ident.to_string());
        }
        syn::visit::visit_item_struct(self, item);
    }
}

/// The visitor that reports hits.
struct DefaultHasher<'a> {
    names: &'a Names,
    package: &'a str,
    file: &'a str,
    scope: Vec<String>,
    hits: Vec<RuleHit>,
    /// The constructor path a hasher-naming `let` annotation vouches for.
    vouched: Option<*const syn::ExprPath>,
}

fn type_arguments(segment: &syn::PathSegment) -> usize {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .filter(|argument| matches!(argument, syn::GenericArgument::Type(_)))
            .count(),
        _ => 0,
    }
}

impl DefaultHasher<'_> {
    fn hit(&mut self, span: proc_macro2::Span, detail: String) {
        let mut symbol = self.file.to_owned();
        for scope in &self.scope {
            symbol.push_str("::");
            symbol.push_str(scope);
        }
        self.hits.push(RuleHit {
            rule: STD_DEFAULT_HASHER,
            package: self.package.to_owned(),
            file: self.file.to_owned(),
            line: span.start().line,
            symbol,
            detail,
        });
    }

    fn check_path(&mut self, path: &syn::Path, in_type: bool) {
        for index in 0..path.segments.len() {
            let Some(std_name) = self.names.std_type_at(path, index) else {
                continue;
            };
            let Some(required) = required_arguments(std_name) else {
                continue;
            };
            let segment = &path.segments[index];
            let given = type_arguments(segment);
            let is_last = index + 1 == path.segments.len();
            if is_last && in_type {
                if given < required {
                    self.hit(
                        segment.ident.span(),
                        format!("`{std_name}` with {given} type argument(s) leaves its hasher as `RandomState`"),
                    );
                }
            } else if !is_last {
                let method = path.segments[index + 1].ident.to_string();
                if CONSTRUCTORS.contains(&method.as_str()) && given < required {
                    self.hit(
                        segment.ident.span(),
                        format!(
                            "`{std_name}::{method}` builds a `RandomState` {}; name the hasher's alias instead",
                            if std_name == "HashMap" { "map" } else { "set" }
                        ),
                    );
                }
            }
        }
    }

    /// Whether an annotation names a map or set type whose hasher is explicit: a
    /// `std` map or set with its hasher argument, or any other name (an alias
    /// that carries one). Only a `HashMap`/`HashSet` annotation vouches.
    fn names_hasher(&self, path: &syn::Path) -> bool {
        let Some(last) = path.segments.len().checked_sub(1) else {
            return false;
        };
        let ident = path.segments[last].ident.to_string();
        match self.names.std_type_at(path, last) {
            Some(std_name) => required_arguments(std_name)
                .is_some_and(|required| type_arguments(&path.segments[last]) >= required),
            None => ident.ends_with("Map") || ident.ends_with("Set"),
        }
    }

    fn scoped(&mut self, name: String, walk: impl FnOnce(&mut Self)) {
        self.scope.push(name);
        walk(self);
        self.scope.pop();
    }
}

impl<'ast> Visit<'ast> for DefaultHasher<'_> {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        self.check_path(&node.path, true);
        syn::visit::visit_type_path(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if self.vouched != Some(std::ptr::from_ref(node)) {
            self.check_path(&node.path, false);
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        let previous = self.vouched.take();
        if let syn::Pat::Type(annotated) = &node.pat
            && let syn::Type::Path(annotation) = annotated.ty.as_ref()
            && self.names_hasher(&annotation.path)
            && let Some(init) = &node.init
            && let syn::Expr::Call(call) = init.expr.as_ref()
            && let syn::Expr::Path(constructor) = call.func.as_ref()
        {
            self.vouched = Some(std::ptr::from_ref(constructor));
        }
        syn::visit::visit_local(self, node);
        self.vouched = previous;
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Ok(arguments) = node.parse_body_with(
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        ) {
            for argument in &arguments {
                self.visit_expr(argument);
            }
        } else if let Ok(block) = node.parse_body_with(syn::Block::parse_within) {
            for statement in &block {
                self.visit_stmt(statement);
            }
        }
        syn::visit::visit_macro(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.scoped(node.sig.ident.to_string(), |this| {
            syn::visit::visit_item_fn(this, node);
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.scoped(node.sig.ident.to_string(), |this| {
            syn::visit::visit_impl_item_fn(this, node);
        });
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.scoped(node.sig.ident.to_string(), |this| {
            syn::visit::visit_trait_item_fn(this, node);
        });
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_mod(this, node);
        });
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_struct(this, node);
        });
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_type(this, node);
        });
    }
}

/// Every `std-default-hasher` hit in one parsed file.
pub(crate) fn std_default_hasher(package: &str, file: &str, parsed: &syn::File) -> Vec<RuleHit> {
    let names = Names::collect(parsed);
    let mut visitor = DefaultHasher {
        names: &names,
        package,
        file,
        scope: Vec::new(),
        hits: Vec::new(),
        vouched: None,
    };
    visitor.visit_file(parsed);
    visitor.hits
}

#[cfg(test)]
mod tests {
    use super::{is_embedded_document, std_default_hasher, vocabulary_namespace};

    #[test]
    fn a_literal_equal_to_or_under_a_namespace_is_a_hit_and_a_neighbour_is_not() {
        let namespaces = vec![
            "http://example.org/ns#".to_owned(),
            "http://example.org/ns#sub/".to_owned(),
        ];
        assert_eq!(
            vocabulary_namespace("http://example.org/ns#", &namespaces),
            Some("http://example.org/ns#")
        );
        assert_eq!(
            vocabulary_namespace("http://example.org/ns#term", &namespaces),
            Some("http://example.org/ns#")
        );
        assert_eq!(
            vocabulary_namespace("http://example.org/ns#sub/x", &namespaces),
            Some("http://example.org/ns#sub/")
        );
        assert_eq!(
            vocabulary_namespace("http://example.org/nsX", &namespaces),
            None
        );
        assert_eq!(
            vocabulary_namespace("<http://example.org/ns#term>", &namespaces),
            None
        );
        assert_eq!(
            vocabulary_namespace("xhttp://example.org/ns#", &namespaces),
            None
        );
    }

    #[test]
    fn an_embedded_document_needs_a_line_break_and_a_keyword() {
        assert!(is_embedded_document(
            "@prefix ex: <http://example.org/ns#> .\nex:a ex:b ex:c ."
        ));
        assert!(is_embedded_document(
            "http://example.org/ns#\nselect * { ?s ?p ?o }"
        ));
        assert!(!is_embedded_document(
            "PREFIX ex: <http://example.org/ns#> SELECT * {}"
        ));
        assert!(!is_embedded_document(
            "http://example.org/ns#a\nhttp://example.org/ns#b"
        ));
        assert!(!is_embedded_document("http://example.org/ns#\nselected"));
        assert_eq!(
            vocabulary_namespace(
                "http://example.org/ns#\nSELECT ?s WHERE { ?s ?p ?o }",
                &["http://example.org/ns#".to_owned()]
            ),
            None
        );
    }

    fn lines(source: &str) -> Vec<usize> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        std_default_hasher("p", "f.rs", &parsed)
            .iter()
            .map(|hit| hit.line)
            .collect()
    }

    #[test]
    fn every_spelling_of_a_default_hasher_is_found() {
        let source = "\
use std::collections::{HashMap, HashSet};
use std::collections;
struct S { a: HashMap<u8, u8> }
fn f() -> std::collections::HashSet<u8> { unimplemented!() }
fn g() {
    let m: HashMap<u8, u8> = HashMap::default();
    let s = [1].into_iter().collect::<HashSet<u8>>();
    let t = collections::HashMap::<u8, u8>::from([(1, 2)]);
    assert_eq!(s, HashSet::from_iter([1]));
    let u: std::collections::hash_map::HashMap<u8, u8> = todo!();
    let unannotated = HashMap::<u8, u8>::default();
    let nested: HashMap<u8, u8, FixedState> = wrap(HashMap::default());
}
";
        assert_eq!(lines(source), vec![3, 4, 6, 6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn a_named_hasher_and_a_foreign_map_are_not_hits() {
        let source = "\
use std::collections::{HashMap, HashSet};
type FastMap<K, V> = HashMap<K, V, FixedState>;
struct S { a: FastMap<u8, u8>, b: HashSet<u8, FixedState> }
fn g() {
    let m: FastMap<u8, u8> = FastMap::default();
    let n = HashMap::<u8, u8, FixedState>::default();
    let annotated: HashMap<u8, u8, FixedState> = HashMap::default();
    let aliased: FastSet<u8> = HashSet::from_iter([1]);
    let o: HashMap<u8, u8, FixedState> = HashMap::with_hasher(FixedState::new());
    let p = HashSet::with_capacity_and_hasher(4, FixedState::new());
    let q: hashbrown::HashMap<u8, u8> = todo!();
}
";
        assert_eq!(lines(source), Vec::<usize>::new());
    }

    #[test]
    fn a_bare_name_imported_from_elsewhere_is_not_std() {
        assert_eq!(
            lines("use hashbrown::HashMap;\nfn f(m: HashMap<u8, u8>) {}\n"),
            Vec::<usize>::new()
        );
        assert_eq!(
            lines("use std::collections::HashMap as Map;\nfn f(m: Map<u8, u8>) {}\n"),
            vec![2]
        );
    }
}
