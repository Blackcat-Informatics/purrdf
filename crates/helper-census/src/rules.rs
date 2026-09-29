// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Census rules: syntactic properties checked over every first-party Rust file,
//! tests, benches and examples included.
//!
//! `std-default-hasher` finds every place a `std` hash map or set is left on its
//! default hasher state, whose per-process random keys randomise
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
//!
//! Three rules hold base16 to its one home, `purrdf_hash::hex`:
//!
//! * `hex-format-loop`: a formatting macro (`format!`, `write!`, `writeln!`,
//!   `format_args!` and the `print` family) whose format string renders an
//!   argument as a two-digit hex pair (`{:02x}`, `{byte:02X}`, `{0:02x}`),
//!   inside a loop body or a closure — the per-byte rendering loop the home's
//!   encoders replace. A single pair formatted outside any loop, and a pair in
//!   an assertion or panic message, are not renderings of a byte string.
//! * `hex-pair-radix`: `u8::from_str_radix(_, 16)`, the per-pair decode the
//!   home's readers replace. A wider radix-16 parse (`u32::from_str_radix`
//!   over a code point's digits) is a grammar terminal's, not a byte pair's.
//! * `hex-table`: a hex-digit table, in either case, spelt as a string or byte
//!   string of the sixteen digits in order or as a bracketed array of them.
//!   Unlike the other rules it is exempt inside the job's home package, which
//!   holds the vector kernels' lookup tables.
//!
//! Three rules hold the lexical terminals to their home, `purrdf_lex`. Each is
//! checked over shipping code only: an item under `#[test]` or a `#[cfg]` that
//! needs `cfg(test)`, and every file under a `tests`, `benches` or `examples`
//! directory, may spell the pattern, because a test that pins a trap has to
//! name it.
//!
//! * `grammar-ws`: `is_ascii_whitespace`, called as a method or named as a
//!   path. It admits U+000C FORM FEED, which no grammar's `WS` names, and
//!   refuses U+000B; read `WS` through `purrdf_lex::terminals`.
//! * `json-pointer-escape`: the string literal `"~0"` or `"~1"` on its own,
//!   the RFC 6901 escape spelled outside `purrdf_lex::json_pointer`. Exempt
//!   inside the job's home package.
//! * `hex-digit-radix`: `.to_digit(16)`, or `from_str_radix(_, 16)` into any
//!   integer type but `u8` (which is `hex-pair-radix`): a hex digit or code
//!   point read past `purrdf_hash::hex::nibble`, and `from_str_radix` accepts a
//!   leading `+` that no grammar's `HEX` does.

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

/// A two-digit hex format spec inside a loop or closure.
pub(crate) const HEX_FORMAT_LOOP: &str = "rule:hex-format-loop";
/// `u8::from_str_radix(_, 16)`.
pub(crate) const HEX_PAIR_RADIX: &str = "rule:hex-pair-radix";
/// A hex-digit table outside the home package.
pub(crate) const HEX_TABLE: &str = "rule:hex-table";
/// `is_ascii_whitespace` in shipping code.
pub(crate) const GRAMMAR_WS: &str = "rule:grammar-ws";
/// The RFC 6901 escape `"~0"`/`"~1"` spelled as a literal in shipping code.
pub(crate) const JSON_POINTER_ESCAPE: &str = "rule:json-pointer-escape";
/// `.to_digit(16)` or a wide `from_str_radix(_, 16)` in shipping code.
pub(crate) const HEX_DIGIT_RADIX: &str = "rule:hex-digit-radix";

/// Every rule the census computes itself.
pub(crate) const RULES: [&str; 9] = [
    STD_DEFAULT_HASHER,
    VOCABULARY_LITERAL,
    HOME_LITERAL,
    HEX_FORMAT_LOOP,
    HEX_PAIR_RADIX,
    HEX_TABLE,
    GRAMMAR_WS,
    JSON_POINTER_ESCAPE,
    HEX_DIGIT_RADIX,
];

/// The formatting macros whose format string `hex-format-loop` reads.
const FORMAT_MACROS: [&str; 8] = [
    "format",
    "format_args",
    "write",
    "writeln",
    "print",
    "println",
    "eprint",
    "eprintln",
];

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
    /// Whether a hit inside the job's home package is the home's own (the
    /// rule forbids the pattern only outside it).
    pub(crate) home_exempt: bool,
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
struct StdHashMapRule<'a> {
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

impl StdHashMapRule<'_> {
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
            home_exempt: false,
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
                        format!("`{std_name}` with {given} type argument(s) leaves its hasher as std's random default"),
                    );
                }
            } else if !is_last {
                let method = path.segments[index + 1].ident.to_string();
                if CONSTRUCTORS.contains(&method.as_str()) && given < required {
                    self.hit(
                        segment.ident.span(),
                        format!(
                            "`{std_name}::{method}` builds std's random default hasher state {}; name the hasher's alias instead",
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

impl<'ast> Visit<'ast> for StdHashMapRule<'_> {
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
    let mut visitor = StdHashMapRule {
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

/// Whether a format string renders an argument as a two-digit hex pair: a
/// `{…:02x}` or `{…:02X}` placeholder, `{{` escapes skipped.
fn has_pair_spec(format: &str) -> bool {
    let mut rest = format;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        if let Some(escaped) = rest.strip_prefix('{') {
            rest = escaped;
            continue;
        }
        let Some(close) = rest.find('}') else {
            return false;
        };
        let placeholder = &rest[..close];
        if placeholder
            .split_once(':')
            .is_some_and(|(_, spec)| spec == "02x" || spec == "02X")
        {
            return true;
        }
        rest = &rest[close + 1..];
    }
    false
}

/// The visitor for the three base16 rules.
struct HexRules<'a> {
    package: &'a str,
    file: &'a str,
    scope: Vec<String>,
    /// How many loop bodies and closures enclose the current node.
    repeated: usize,
    hits: Vec<RuleHit>,
}

impl HexRules<'_> {
    fn hit(&mut self, rule: &'static str, line: usize, detail: String) {
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
            detail,
            home_exempt: rule == HEX_TABLE,
        });
    }

    fn scoped(&mut self, name: String, walk: impl FnOnce(&mut Self)) {
        self.scope.push(name);
        walk(self);
        self.scope.pop();
    }

    fn repeated(&mut self, walk: impl FnOnce(&mut Self)) {
        self.repeated += 1;
        walk(self);
        self.repeated -= 1;
    }

    fn table(&mut self, digits: &str, line: usize) {
        if let Some(table) = crate::normalize::classify_digits(digits) {
            self.hit(
                HEX_TABLE,
                line,
                format!("a `{table}` digit table; render and read base16 through the home"),
            );
        }
    }
}

impl<'ast> Visit<'ast> for HexRules<'_> {
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.visit_expr(&node.expr);
        self.repeated(|this| this.visit_block(&node.body));
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.visit_expr(&node.cond);
        self.repeated(|this| this.visit_block(&node.body));
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.repeated(|this| this.visit_block(&node.body));
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.repeated(|this| syn::visit::visit_expr_closure(this, node));
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if self.repeated > 0 && FORMAT_MACROS.contains(&name.as_str()) {
            for token in node.tokens.clone() {
                if let proc_macro2::TokenTree::Literal(literal) = token
                    && let Ok(syn::Lit::Str(text)) =
                        syn::parse2::<syn::Lit>(proc_macro2::TokenTree::Literal(literal).into())
                    && has_pair_spec(&text.value())
                {
                    self.hit(
                        HEX_FORMAT_LOOP,
                        text.span().start().line,
                        format!(
                            "`{name}!` renders a two-digit hex pair inside a loop; render the bytes \
                             through purrdf_hash::hex"
                        ),
                    );
                }
            }
        }
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

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = node.func.as_ref() {
            let segments: Vec<String> = function
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let names_u8 = segments.len() >= 2 && segments[segments.len() - 2] == "u8"
                || function
                    .qself
                    .as_ref()
                    .is_some_and(|qself| matches!(qself.ty.as_ref(), syn::Type::Path(path) if path.path.is_ident("u8")));
            if names_u8
                && segments.last().is_some_and(|last| last == "from_str_radix")
                && node.args.len() == 2
                && let Some(syn::Expr::Lit(radix)) = node.args.iter().nth(1)
                && let syn::Lit::Int(radix) = &radix.lit
                && radix.base10_digits() == "16"
            {
                self.hit(
                    HEX_PAIR_RADIX,
                    radix.span().start().line,
                    "`u8::from_str_radix(_, 16)` decodes a hex pair; read base16 through purrdf_hash::hex".to_owned(),
                );
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        self.table(&node.value(), node.span().start().line);
    }

    fn visit_lit_byte_str(&mut self, node: &'ast syn::LitByteStr) {
        if let Ok(text) = String::from_utf8(node.value()) {
            self.table(&text, node.span().start().line);
        }
    }

    fn visit_expr_array(&mut self, node: &'ast syn::ExprArray) {
        let digits: Option<String> = node
            .elems
            .iter()
            .map(|element| match element {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Char(c),
                    ..
                }) => Some(c.value()),
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Byte(b),
                    ..
                }) => Some(char::from(b.value())),
                _ => None,
            })
            .collect();
        if let Some(digits) = digits {
            self.table(&digits, node.bracket_token.span.open().start().line);
        }
        syn::visit::visit_expr_array(self, node);
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

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_mod(this, node);
        });
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_const(this, node);
        });
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_static(this, node);
        });
    }
}

/// Every `hex-format-loop`, `hex-pair-radix` and `hex-table` hit in one parsed
/// file.
pub(crate) fn hex_rules(package: &str, file: &str, parsed: &syn::File) -> Vec<RuleHit> {
    let mut visitor = HexRules {
        package,
        file,
        scope: Vec::new(),
        repeated: 0,
        hits: Vec::new(),
    };
    visitor.visit_file(parsed);
    visitor.hits
}

/// The visitor for the three lexical-terminal rules, over shipping code.
struct LexRules<'a> {
    package: &'a str,
    file: &'a str,
    scope: Vec<String>,
    /// How many test-only items enclose the current node.
    in_test: usize,
    hits: Vec<RuleHit>,
}

impl LexRules<'_> {
    fn hit(&mut self, rule: &'static str, line: usize, detail: &str) {
        if self.in_test > 0 {
            return;
        }
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
            home_exempt: rule == JSON_POINTER_ESCAPE,
        });
    }

    fn item(&mut self, name: String, attrs: &[syn::Attribute], walk: impl FnOnce(&mut Self)) {
        let test = crate::source::is_test_only(attrs);
        self.in_test += usize::from(test);
        self.scope.push(name);
        walk(self);
        self.scope.pop();
        self.in_test -= usize::from(test);
    }
}

/// Whether `expression` is the integer literal `16`.
fn is_sixteen(expression: &syn::Expr) -> bool {
    matches!(expression, syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(radix), .. }) if radix.base10_digits() == "16")
}

const GRAMMAR_WS_DETAIL: &str = "`is_ascii_whitespace` admits U+000C FORM FEED, which no grammar's `WS` names; \
     read `WS` through purrdf_lex::terminals (`is_ws`, `skip_ws`, `trim_ws`)";
const HEX_DIGIT_DETAIL: &str = "a radix-16 digit read outside purrdf_hash::hex (`nibble`, `parse_u32`); \
     `from_str_radix` accepts a leading `+`";

impl<'ast> Visit<'ast> for LexRules<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method = node.method.to_string();
        if method == "is_ascii_whitespace" {
            self.hit(
                GRAMMAR_WS,
                node.method.span().start().line,
                GRAMMAR_WS_DETAIL,
            );
        }
        if method == "to_digit" && node.args.len() == 1 && node.args.first().is_some_and(is_sixteen)
        {
            self.hit(
                HEX_DIGIT_RADIX,
                node.method.span().start().line,
                HEX_DIGIT_DETAIL,
            );
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "is_ascii_whitespace")
        {
            self.hit(
                GRAMMAR_WS,
                node.path
                    .segments
                    .last()
                    .map_or(0, |segment| segment.ident.span().start().line),
                GRAMMAR_WS_DETAIL,
            );
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = node.func.as_ref() {
            let segments: Vec<String> = function
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let names_u8 = segments.len() >= 2 && segments[segments.len() - 2] == "u8";
            if !names_u8
                && segments.last().is_some_and(|last| last == "from_str_radix")
                && node.args.len() == 2
                && node.args.iter().nth(1).is_some_and(is_sixteen)
            {
                let line = function
                    .path
                    .segments
                    .last()
                    .map_or(0, |segment| segment.ident.span().start().line);
                self.hit(HEX_DIGIT_RADIX, line, HEX_DIGIT_DETAIL);
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        // Spelled as bytes, so this rule does not match itself.
        if matches!(node.value().as_bytes(), [b'~', b'0' | b'1']) {
            self.hit(
                JSON_POINTER_ESCAPE,
                node.span().start().line,
                "the RFC 6901 escape spelled outside purrdf_lex::json_pointer; escape and read \
                 reference tokens through it",
            );
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Ok(arguments) = node.parse_body_with(
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        ) {
            for argument in &arguments {
                self.visit_expr(argument);
            }
        }
        syn::visit::visit_macro(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.item(node.sig.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_fn(this, node);
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.item(node.sig.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_impl_item_fn(this, node);
        });
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.item(node.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_mod(this, node);
        });
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let test = crate::source::is_test_only(&node.attrs);
        self.in_test += usize::from(test);
        syn::visit::visit_item_impl(self, node);
        self.in_test -= usize::from(test);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.item(node.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_const(this, node);
        });
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.item(node.ident.to_string(), &node.attrs, |this| {
            syn::visit::visit_item_static(this, node);
        });
    }
}

/// Whether `file` sits under a `tests`, `benches` or `examples` directory.
fn is_test_file(file: &str) -> bool {
    file.split('/')
        .rev()
        .skip(1)
        .any(|directory| matches!(directory, "tests" | "benches" | "examples"))
}

/// Every `grammar-ws`, `json-pointer-escape` and `hex-digit-radix` hit in one
/// parsed file of shipping code.
pub(crate) fn lex_rules(package: &str, file: &str, parsed: &syn::File) -> Vec<RuleHit> {
    if is_test_file(file) {
        return Vec::new();
    }
    let mut visitor = LexRules {
        package,
        file,
        scope: Vec::new(),
        in_test: 0,
        hits: Vec::new(),
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

    fn hex_lines(source: &str) -> Vec<(&'static str, usize)> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        super::hex_rules("p", "f.rs", &parsed)
            .iter()
            .map(|hit| (hit.rule, hit.line))
            .collect()
    }

    #[test]
    #[allow(
        clippy::literal_string_with_formatting_args,
        reason = "the fixture is Rust source holding format strings, not a format string"
    )]
    fn a_pair_spec_in_a_loop_or_closure_is_found_and_nothing_else_is() {
        let source = "\
fn f(bytes: &[u8], out: &mut String) {
    for byte in bytes { let _ = write!(out, \"{byte:02x}\"); }
    let s: String = bytes.iter().map(|b| format!(\"{b:02X}\")).collect();
    let t = bytes.iter().fold(String::new(), |mut o, b| { let _ = write!(o, \"{:02x}\", b); o });
    let mut i = 0; while i < 2 { out.push_str(&format!(\"%{0:02X}\", i)); i += 1; }
    let one = format!(\"{:02x}\", bytes[0]);
    for b in bytes { assert!(true, \"{b:02x}\"); let _ = format!(\"{b:04X} {{:02x}}\"); }
}
";
        assert_eq!(
            hex_lines(source),
            vec![
                (super::HEX_FORMAT_LOOP, 2),
                (super::HEX_FORMAT_LOOP, 3),
                (super::HEX_FORMAT_LOOP, 4),
                (super::HEX_FORMAT_LOOP, 5),
            ]
        );
    }

    #[test]
    fn a_byte_pair_radix_parse_and_a_digit_table_are_found() {
        let lower = purrdf_hash::hex::encode(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
        let upper = lower.to_ascii_uppercase();
        let source = format!(
            "fn f(p: &str) -> u8 {{ u8::from_str_radix(p, 16).unwrap() }}\n\
             fn g(p: &str) -> u32 {{ u32::from_str_radix(p, 16).unwrap() }}\n\
             const L: &[u8; 16] = b\"{lower}\";\n\
             const U: &str = \"{upper}\";\n\
             const N: &str = \"{lower}g\";\n"
        );
        let found = hex_lines(&source);
        assert_eq!(
            found,
            vec![
                (super::HEX_PAIR_RADIX, 1),
                (super::HEX_TABLE, 3),
                (super::HEX_TABLE, 4),
            ]
        );
    }

    fn lex_lines(file: &str, source: &str) -> Vec<(&'static str, usize)> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        super::lex_rules("p", file, &parsed)
            .iter()
            .map(|hit| (hit.rule, hit.line))
            .collect()
    }

    #[test]
    fn the_lexical_rules_fire_in_shipping_code_and_not_in_tests() {
        let source = "\
fn f(b: u8, c: char, s: &str) -> bool {
    let _ = b.is_ascii_whitespace();
    let _ = s.bytes().all(|b| b.is_ascii_whitespace() || b == 0);
    let _ = s.bytes().any(u8::is_ascii_whitespace);
    let _ = c.to_digit(16);
    let _ = c.to_digit(10);
    let _ = u32::from_str_radix(s, 16);
    let _ = u32::from_str_radix(s, 10);
    let _ = s.replace('~', \"~0\");
    let _ = \"a~0b\";
    true
}
#[cfg(test)]
mod tests {
    fn g(b: u8) -> bool { b.is_ascii_whitespace() && \"~1\".is_empty() }
}
#[test]
fn h() { let _ = char::from(0).to_digit(16); }
";
        assert_eq!(
            lex_lines("crates/p/src/lib.rs", source),
            vec![
                (super::GRAMMAR_WS, 2),
                (super::GRAMMAR_WS, 3),
                (super::GRAMMAR_WS, 4),
                (super::HEX_DIGIT_RADIX, 5),
                (super::HEX_DIGIT_RADIX, 7),
                (super::JSON_POINTER_ESCAPE, 9),
            ]
        );
        assert_eq!(lex_lines("crates/p/tests/it.rs", source), Vec::new());
    }
}
