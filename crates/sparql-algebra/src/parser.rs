// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The public parse entry point and the recursive-descent parser that turns a
//! SPARQL 1.1/1.2 query into the [`Query`] algebra.
//!
//! The parser translates *directly* into the W3C SPARQL algebra (§18.2) rather
//! than building a separate syntax tree: group graph patterns accumulate into
//! `Join`/`LeftJoin`/`Filter`/`Extend`/`Union`/`Minus`/`Graph`, solution
//! modifiers wrap the result as `Group`/`OrderBy`/`Project`/`Distinct`/`Slice`,
//! and aggregates are lifted to synthetic variables in a `Group` node (the
//! standard §18.2.4 mechanism). Anything outside the corpus-driven scope is a
//! hard [`ParseError::Unsupported`].

use std::collections::HashMap;

use crate::algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, GraphTarget,
    GraphUpdateOperation, NegatedPathElement, OrderExpression, PropertyFunctionCall,
    PropertyPathExpression, Query, QueryDataset, SparqlVersion, Update, UsingClause,
};
use crate::ast::{
    BaseDirection, BlankNode, GroundTerm, GroundTriple, Literal, NamedNode, NamedNodePattern,
    QuadPattern, TermPattern, TriplePattern, Variable,
};
use crate::error::{ParseError, Result};
use crate::lexer::{Spanned, Token, tokenize};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// Maximum number of nested group graph patterns accepted by the parser and evaluator.
///
/// This is a structural safety limit, not an execution governor: it rejects an algebra
/// whose recursive evaluation would otherwise be able to exhaust the native stack before
/// a fuel or stop check could run.
pub const MAX_GRAPH_PATTERN_DEPTH: usize = 128;

/// Maximum number of graph-pattern *combinator* nodes — `Join`/`LeftJoin`/
/// `Lateral`/`Union`/`Filter`/`Extend`/`Graph`/`Service`/`Minus` — a single
/// parse (one `SparqlParser::parse_query`/`parse_update` call) will construct.
///
/// [`MAX_GRAPH_PATTERN_DEPTH`] bounds `{ … }` BRACE nesting only. It does
/// nothing for a run of SIBLING operators at one brace depth — `OPTIONAL { }
/// OPTIONAL { } …`, a `LATERAL { } LATERAL { } …` chain, a `UNION`-arm run at
/// one `{ … }` boundary, or a run of non-BGP triples-block elements (e.g.
/// complex property-path triples, which `join` cannot flatten the way it
/// flattens adjacent `Bgp`s) — each of which grows the algebra tree by one
/// level PER SIBLING while `{ … }` nesting stays at 1. A query built from N
/// such siblings therefore produces a tree of height ~N with no brace ever
/// nesting past depth 1, invisible to `MAX_GRAPH_PATTERN_DEPTH`. The SAME
/// shape arises from a long `SELECT (e1 AS ?v1) … (eN AS ?vN)` projection
/// list, a long `GROUP BY` expression-condition list, or a long `HAVING`
/// condition list — each lowers to a chain of `Extend`/`Filter` nodes wrapped
/// around the WHERE pattern, built by a loop with no brace at all.
///
/// This limit closes that gap at its source: every site in this module that
/// can grow the algebra tree by repetition — the group-parsing loop, its
/// nested `UNION`-arm loop, a triples block's non-BGP (`Path`/property-function)
/// elements, and the three projection/grouping/having list loops above —
/// charges one unit against this budget per node it is about to build, and
/// the parse hard-fails with a typed [`ParseError`] the instant the budget is
/// exhausted, rather than building the (N+1)-th node. Because every node that
/// could deepen the tree is charged, capping the TOTAL count also caps the
/// tree's maximum root-to-leaf HEIGHT by the same number — which is what
/// actually matters: that height is the native-stack recursion depth of
/// every downstream consumer that walks the parsed tree by ordinary
/// recursion (`collect_vars`/`visible_variables`, `find_scope_conflict`,
/// the tree's own recursive `Drop`), none of which can be rewritten to an
/// explicit-stack walk without also rewriting `Drop`, which recursion alone
/// cannot avoid. Bounding construction is therefore the one fix that covers
/// every present AND future consumer at once, `Drop` included.
///
/// The value is chosen with a wide safety margin under the depth at which a
/// left-deep tree of this shape has been observed to exhaust a 2&nbsp;MiB
/// stack (the size `cargo test` gives each test thread) while still leaving
/// several orders of magnitude of headroom over any query in this crate's
/// corpus: a single query with `MAX_GRAPH_PATTERN_NODES` non-BGP siblings is
/// already far outside anything a hand- or tool-written SPARQL query
/// resembles.
pub const MAX_GRAPH_PATTERN_NODES: usize = 2048;

/// Parse-time configuration for the SPARQL front-end.
///
/// Both knobs are caller-supplied IRI-namespace sets that default to EMPTY.
///
/// [`Self::extension_fn_namespaces`] is the set of IRI
/// namespaces the parser recognizes as the **extension-function seam**. An IRI
/// in call position (immediately followed by `(`) whose string starts with any
/// configured namespace is stripped to its local name and dispatched into the
/// CLOSED [`crate::algebra::PurrdfFn`] set; an *unknown* local name under a
/// configured namespace is a hard [`ParseError`] (never a silent
/// [`Function::Custom`] fallthrough).
///
/// The default is **EMPTY**: PurRDF is a library, not an ontology, and mints no
/// vocabulary IRIs of its own — with no configured namespace the extension seam
/// is off and every call-position IRI is an ordinary [`Function::Custom`] (no
/// error, no special-casing). A deployment whose queries spell the closed
/// function set under its own ontology namespace — e.g. gmeow's
/// `https://blackcatinformatics.ca/gmeow/` with `gmeow:heldIn(...)` — supplies
/// that namespace here; the local names are fixed.
///
/// [`Self::property_fn_namespaces`] is the same idea one position over: the set
/// of IRI namespaces recognized as the **property-function seam** in PREDICATE
/// position. A triple whose predicate is a plain IRI under a configured
/// namespace becomes a [`GraphPattern::PropertyFunction`] node — a call into a
/// registered relation — instead of a triple pattern matched against the data; a
/// variable predicate and a property-path predicate are never property functions.
/// Its default is EMPTY too: with no configured namespace the seam is off and
/// every such triple stays an ordinary BGP triple pattern, bit for bit as before.
///
/// [`Self::property_fn_iris`] recognizes the same seam by a different, narrower
/// rule: EXACT-IRI match rather than prefix match. It exists because a relation
/// registry's keys are exact IRIs, not namespaces — a host that registers
/// `https://example.org/rel/a` has registered exactly that IRI, and treating it
/// as a *prefix* would silently reclassify the unrelated, ordinary data
/// predicate `https://example.org/rel/ab` as a call to an unregistered relation,
/// which then hard-errors with a diagnostic that points at the wrong cause (a
/// previously-working query breaking because of a same-prefixed sibling it never
/// mentioned). Populating [`Self::property_fn_namespaces`] from a registry would
/// be that mistake; [`Self::property_fn_iris`] is the field that is safe to
/// derive from one. The two sets are independent and their recognition is a
/// union: an IRI is a property function iff it prefix-matches an entry of
/// [`Self::property_fn_namespaces`] OR exactly matches an entry of
/// [`Self::property_fn_iris`]. Its default is EMPTY as well.
///
/// Note the serializer does **not** consult this configuration: a
/// [`Function::Purrdf`] re-emits the ORIGINAL IRI it was parsed from (recorded
/// in [`crate::algebra::PurrdfCall::iri`] — see `serialize.rs`), so re-parsing
/// that output with the same options round-trips to the same algebra and no
/// namespace is ever fabricated on output. A
/// [`GraphPattern::PropertyFunction`] keeps its own IRI the same way.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParserOptions {
    /// The namespaces recognized as the extension-function seam in call position.
    /// Defaults to empty (extension functions off); order is first-match-wins
    /// for prefix stripping.
    pub extension_fn_namespaces: Vec<String>,
    /// The namespaces recognized as the property-function seam in predicate
    /// position, by PREFIX match. Defaults to empty (property functions off);
    /// recognition is order-independent — an IRI is claimed by the seam if it
    /// prefix-matches ANY configured entry, and nothing is stripped from it, so
    /// no entry's position in the list changes what a call's IRI is. Caller-declared:
    /// a host that wants a whole namespace claimed by the seam — including IRIs it
    /// has deliberately left unregistered, so spelling one is a hard error rather
    /// than a silent data triple — declares it here.
    pub property_fn_namespaces: Vec<String>,
    /// The individual IRIs recognized as the property-function seam in
    /// predicate position, by EXACT match. Defaults to empty (property
    /// functions off). This is the registry-derived set: a relation registry's
    /// keys are exact IRIs, and matching them here rather than folding them into
    /// [`Self::property_fn_namespaces`] as prefixes is what stops registering
    /// `https://example.org/rel/a` from hijacking the unrelated data predicate
    /// `https://example.org/rel/ab` into an (unregistered, hard-erroring)
    /// property-function call.
    pub property_fn_iris: Vec<String>,
}

/// A reusable SPARQL query parser.
///
/// Mirrors the prior oxigraph-family `SparqlParser` surface the existing
/// consumers call so the port is mechanical: `SparqlParser::new().parse_query(text)`.
/// Parse-time configuration (the extension-function namespace set) is passed per
/// call via [`SparqlParser::parse_query_with`] / [`SparqlParser::parse_update_with`];
/// the plain `parse_*` entries use [`ParserOptions::default`].
#[derive(Clone, Debug, Default)]
pub struct SparqlParser {
    base_iri: Option<String>,
}

impl SparqlParser {
    /// Construct a parser with no implicit base IRI.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an implicit base IRI used to resolve relative IRI references that
    /// appear before any in-query `BASE` declaration.
    ///
    /// # Examples
    ///
    /// ```
    /// use purrdf_sparql_algebra::SparqlParser;
    ///
    /// let parser = SparqlParser::new().with_base_iri("http://example.org/data/");
    /// let query = parser
    ///     .parse_query("SELECT ?o WHERE { <cats> <touched> ?o }")
    ///     .expect("relative IRIs resolve against the implicit base");
    /// // Without a base, the same relative-IRI query is a parse error.
    /// assert!(SparqlParser::new().parse_query("SELECT ?o WHERE { <cats> <touched> ?o }").is_err());
    /// # let _ = query;
    /// ```
    #[must_use]
    pub fn with_base_iri(mut self, base_iri: impl Into<String>) -> Self {
        self.base_iri = Some(base_iri.into());
        self
    }

    /// Parse a SPARQL 1.1/1.2 query into the algebra, under [`ParserOptions::default`].
    ///
    /// # Examples
    ///
    /// ```
    /// use purrdf_sparql_algebra::{Query, SparqlParser};
    ///
    /// let parser = SparqlParser::new();
    /// let query = parser
    ///     .parse_query("ASK { <http://example.org/s> <http://example.org/p> ?o }")
    ///     .expect("a well-formed query parses");
    /// assert!(matches!(query, Query::Ask { .. }));
    ///
    /// // Malformed input is a typed error, never a partial algebra.
    /// assert!(parser.parse_query("SELECT WHERE").is_err());
    /// ```
    pub fn parse_query(&self, query: &str) -> Result<Query> {
        self.parse_query_with(query, &ParserOptions::default())
    }

    /// Parse a SPARQL 1.1/1.2 query into the algebra with explicit [`ParserOptions`]
    /// (e.g. an extra extension-function namespace alias).
    pub fn parse_query_with(&self, query: &str, options: &ParserOptions) -> Result<Query> {
        let mut p = self.parser_for(query, options)?;
        let q = p.parse_query()?;
        p.expect_eof()?;
        Ok(q)
    }

    /// Parse a SPARQL 1.1 Update request into the [`Update`] algebra, under
    /// [`ParserOptions::default`].
    ///
    /// # Examples
    ///
    /// ```
    /// use purrdf_sparql_algebra::{GraphUpdateOperation, SparqlParser};
    ///
    /// let update = SparqlParser::new()
    ///     .parse_update(
    ///         "INSERT DATA { <http://example.org/s> <http://example.org/p> \"purr\" }",
    ///     )
    ///     .expect("a well-formed update parses");
    /// assert_eq!(update.operations.len(), 1);
    /// assert!(matches!(
    ///     update.operations[0],
    ///     GraphUpdateOperation::InsertData { .. }
    /// ));
    /// ```
    pub fn parse_update(&self, update: &str) -> Result<Update> {
        self.parse_update_with(update, &ParserOptions::default())
    }

    /// Parse a SPARQL 1.1 Update request into the [`Update`] algebra with explicit
    /// [`ParserOptions`].
    pub fn parse_update_with(&self, update: &str, options: &ParserOptions) -> Result<Update> {
        let mut p = self.parser_for(update, options)?;
        let u = p.parse_update()?;
        p.expect_eof()?;
        Ok(u)
    }

    /// Tokenize `text` and assemble the internal recursive-descent parser state.
    fn parser_for<'a, 'o>(
        &self,
        text: &'a str,
        options: &'o ParserOptions,
    ) -> Result<Parser<'a, 'o>> {
        let tokens = tokenize(text)?.into_iter().map(Some).collect();
        Ok(Parser {
            tokens,
            pos: 0,
            end: text.len(),
            prefixes: HashMap::new(),
            base: self.base_iri.clone(),
            version: None,
            agg_counter: 0,
            anon_counter: 0,
            group_counter: 0,
            group_pattern_depth: 0,
            pattern_node_budget: 0,
            exists_scope_stack: Vec::new(),
            projection_scope_pending: false,
            in_aggregate_argument: false,
            projection_seen_targets: Vec::new(),
            pending_exists_scope_checks: Vec::new(),
            #[cfg(debug_assertions)]
            scope_consultations: 0,
            options,
        })
    }
}

struct Parser<'a, 'o> {
    tokens: Vec<Option<Spanned<'a>>>,
    pos: usize,
    end: usize,
    prefixes: HashMap<String, String>,
    base: Option<String>,
    /// The most recently parsed prologue `VERSION` declaration (last-wins across
    /// repeated declarations — see [`SparqlVersion`]).
    version: Option<SparqlVersion>,
    agg_counter: usize,
    anon_counter: usize,
    group_counter: usize,
    group_pattern_depth: usize,
    /// Running count of graph-pattern combinator nodes charged so far against
    /// [`MAX_GRAPH_PATTERN_NODES`] — see [`Parser::charge_pattern_nodes`].
    pattern_node_budget: usize,
    /// The `EXISTS`/`NOT EXISTS` in-scope-set stack (SEP-0007 Part 3) — see
    /// [`Parser::exists_scope`] for what "in scope" means here and
    /// [`Parser::push_exists_scope_boundary`]/[`Parser::push_exists_scope_isolated`]
    /// for how frames are opened.
    exists_scope_stack: Vec<VarScope>,
    /// True while parsing a `SELECT`'s own projection list — its `(expr AS
    /// ?v)` targets and any aggregate arguments lifted out of them — i.e. the
    /// window BEFORE `WHERE` is even read, where [`Parser::exists_scope`] is
    /// necessarily still empty (see [`Parser::parse_exists_body`]'s doc for
    /// why an immediate check there cannot be correct). An `EXISTS`/`NOT
    /// EXISTS` reached in this window is deferred into
    /// `pending_exists_scope_checks` instead of checked on the spot.
    /// Save/restored around every [`Parser::parse_select`] call, since a
    /// sub-`SELECT` reached mid-projection-list (via `EXISTS { SELECT ... }`)
    /// opens and fully resolves its OWN window without disturbing the outer
    /// one paused around it.
    projection_scope_pending: bool,
    /// True while parsing an aggregate function's own `(...)` argument
    /// expression ([`Parser::parse_aggregate`]/[`Parser::parse_agg_call`]) —
    /// selects which basis a deferred `EXISTS` check recorded under
    /// `projection_scope_pending` resolves against (see
    /// [`ExistsScopeBasis`]). Aggregates cannot themselves nest, but the
    /// `EXISTS` body an aggregate's argument contains may embed a
    /// sub-`SELECT` with its own, unrelated aggregates, so this is
    /// save/restored around `parse_select` the same as
    /// `projection_scope_pending`, not merely set-and-cleared around each
    /// aggregate call.
    in_aggregate_argument: bool,
    /// The `(expr AS ?v)` targets already committed by EARLIER entries in the
    /// SAME projection list, at the moment each deferred `EXISTS` check
    /// (`projection_scope_pending`, `ExistsScopeBasis::Projection`) is
    /// recorded. The parser lowers a `SELECT` list to a CHAIN of nested
    /// `Extend`s (see the loop building `p` from `select_exprs` in
    /// [`Parser::parse_select`]): each later list entry's expression is
    /// therefore evaluated with every earlier entry's target already bound —
    /// exactly as much "the row" as `WHERE`'s own variables are, for this
    /// check's purposes. Reset (via [`std::mem::take`]) and restored around
    /// every `parse_select` call.
    projection_seen_targets: Vec<Variable>,
    /// `EXISTS`/`NOT EXISTS` scope checks recorded while
    /// `projection_scope_pending` was set, resolved once the enclosing
    /// `SELECT`'s post-`WHERE` (and, for an aggregating query,
    /// post-grouping) in-scope set is known — see the post-`WHERE` block in
    /// [`Parser::parse_select`]. Reset (via [`std::mem::take`]) and restored
    /// around every `parse_select` call, so a nested sub-`SELECT`'s own
    /// pending checks never bleed into the outer `SELECT`'s resolution point
    /// (or vice versa).
    pending_exists_scope_checks: Vec<PendingExistsScopeCheck>,
    /// Debug-only regression counter for the group-loop scope-set quadratic:
    /// incremented ONLY at the handful of PRODUCTION sites that consult a
    /// freshly-computed (`visible_variables`-derived) whole-pattern scope
    /// snapshot ([`Parser::note_scope_consultation`]) — never at a per-element
    /// site inside the group loop (`BIND`'s own membership test consults the
    /// incremental [`VarScope`] directly, in O(log n), and is not a
    /// "consultation" in this sense). A query's snapshot-consultation count is
    /// therefore fixed by its STRUCTURE (one per `SELECT`, one per `LATERAL`
    /// keyword) and invariant under how many elements a group between them
    /// holds — see `scope_set_stays_linear_over_two_thousand_binds`, which
    /// reads this field through [`Self::debug_scope_consultations`].
    #[cfg(debug_assertions)]
    scope_consultations: u64,
    options: &'o ParserOptions,
}

impl<'a> Parser<'a, '_> {
    // ── token cursor ─────────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens
            .get(self.pos)
            .and_then(Option::as_ref)
            .map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token<'a>> {
        self.tokens
            .get(self.pos + 1)
            .and_then(Option::as_ref)
            .map(|s| &s.token)
    }

    fn span(&self) -> usize {
        self.tokens
            .get(self.pos)
            .and_then(Option::as_ref)
            .map_or_else(|| self.end, |s| s.start)
    }

    fn bump(&mut self) -> Option<Token<'a>> {
        let token = self.tokens.get_mut(self.pos)?.take()?.token;
        self.pos += 1;
        Some(token)
    }

    /// Clone only the tokens spanning the next balanced `{ … }` block starting
    /// at `self.pos` (which must be the opening `{`), for the two grammar
    /// productions that intentionally parse the same source span twice.
    /// Ordinary cursor advances move token payloads; only these rare
    /// reparsing forms clone lexemes, and only the braced block rather than
    /// the whole remaining token stream — bounding a `;`-separated
    /// multi-operation UPDATE's `DELETE WHERE` reparse to O(block) per
    /// operation instead of O(remaining tokens) (which made the whole
    /// request O(n²) in the number of operations).
    ///
    /// Finds the matching `}` with a brace-depth scan (depth starts at 0,
    /// `{` increments, `}` decrements, stop when it returns to 0). If the
    /// scan never balances back to 0 — `self.pos` was not actually at a `{`,
    /// or the input is malformed — falls back to the full remaining suffix
    /// so behavior stays correct (just unoptimized) on that edge case.
    fn fork_block(&self) -> Self {
        let mut depth: i32 = 0;
        let mut block_end = None;
        for (offset, slot) in self.tokens[self.pos..].iter().enumerate() {
            match slot.as_ref().map(|s| &s.token) {
                Some(Token::LBrace) => depth += 1,
                Some(Token::RBrace) => {
                    depth -= 1;
                    if depth == 0 {
                        block_end = Some(self.pos + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end_idx = block_end.map_or(self.tokens.len(), |idx| idx + 1);
        Self {
            tokens: self.tokens[self.pos..end_idx].to_vec(),
            pos: 0,
            end: self.end,
            prefixes: self.prefixes.clone(),
            base: self.base.clone(),
            version: self.version.clone(),
            agg_counter: self.agg_counter,
            anon_counter: self.anon_counter,
            group_counter: self.group_counter,
            group_pattern_depth: self.group_pattern_depth,
            pattern_node_budget: self.pattern_node_budget,
            // A fork reparses only a bounded braced block for a template/quad
            // reading (`CONSTRUCT`'s short-form template, `DELETE WHERE`'s
            // quad-pattern reading) — neither production can contain `EXISTS`,
            // so the fork never consults this stack; starting it empty (rather
            // than cloning `self`'s, which may be mid-EXISTS-body at the fork
            // point) is simplest and correct either way. The same reasoning
            // covers the deferred-EXISTS-scope fields below: neither
            // production can contain a `SELECT` either, so
            // `projection_scope_pending`/`in_aggregate_argument` start false
            // and the two buffers start empty regardless of `self`'s own
            // mid-projection-list state at the fork point.
            exists_scope_stack: Vec::new(),
            projection_scope_pending: false,
            in_aggregate_argument: false,
            projection_seen_targets: Vec::new(),
            pending_exists_scope_checks: Vec::new(),
            #[cfg(debug_assertions)]
            scope_consultations: self.scope_consultations,
            options: self.options,
        }
    }

    fn set_counters(&mut self, counters: (usize, usize, usize)) {
        (self.agg_counter, self.anon_counter, self.group_counter) = counters;
    }

    /// Charge `n` graph-pattern combinator nodes against
    /// [`MAX_GRAPH_PATTERN_NODES`], hard-failing the instant the running total
    /// would exceed it. Every call site that is ABOUT TO build one more
    /// `Join`/`LeftJoin`/`Lateral`/`Union`/`Filter`/`Extend`/`Graph`/`Service`/
    /// `Minus` node calls this FIRST, so the budget is checked before the node
    /// (and any input it borrows unboundedly, like a `LATERAL` right-hand
    /// side) is built — never after.
    fn charge_pattern_nodes(&mut self, n: usize) -> Result<()> {
        self.pattern_node_budget += n;
        if self.pattern_node_budget > MAX_GRAPH_PATTERN_NODES {
            return Err(ParseError::syntax(
                format!(
                    "graph pattern combinator count exceeds the safety limit of \
                     {MAX_GRAPH_PATTERN_NODES}"
                ),
                self.span(),
            ));
        }
        Ok(())
    }

    /// Record one PRODUCTION consultation of a freshly-computed whole-pattern
    /// scope snapshot (see [`Parser::scope_consultations`]'s doc for exactly
    /// what counts). A no-op in release builds — this exists to make the
    /// group-loop scope-set quadratic falsifiable by a scale-invariant COUNT
    /// rather than a clock (no timing assertion is sound across machines/CI
    /// load; a call count fixed by query STRUCTURE is).
    #[cfg(debug_assertions)]
    fn note_scope_consultation(&mut self) {
        self.scope_consultations += 1;
    }

    #[cfg(not(debug_assertions))]
    #[inline(always)]
    fn note_scope_consultation(&mut self) {}

    /// The current value of the debug-only scope-consultation counter — the
    /// NON-COUNTING read `scope_set_stays_linear_over_two_thousand_binds`
    /// uses (reading the counter does not itself consult anything). Test-only
    /// (no production caller needs a query's own consultation count), hence
    /// `cfg(test)` too — otherwise an ordinary `debug_assertions` lib build
    /// carries a method nothing calls.
    #[cfg(all(debug_assertions, test))]
    fn debug_scope_consultations(&self) -> u64 {
        self.scope_consultations
    }

    // ── EXISTS in-scope-set tracking (SEP-0007 Part 3) ───────────────────────
    //
    // The `Parser::exists_scope_stack` field holds one frame per "current row"
    // region — a query/update operation's own `WHERE` clause, or the body of
    // an `EXISTS`/`NOT EXISTS`/`MINUS` group this parser is presently inside —
    // NOT one frame per `{ … }` brace. Every construct SEP-0007/SEP-0006 treat
    // as scope-transparent (a plain nested group, `OPTIONAL`, `UNION`, `GRAPH`,
    // `SERVICE`, either side of `LATERAL`) writes what it introduces directly
    // into the CURRENT top frame as the group-parsing loop parses it — exactly
    // in parallel with that loop's own local [`VarScope`] (used for the
    // pre-existing BIND/LATERAL rules), via [`Parser::note_exists_scope`] /
    // [`Parser::note_exists_scope_var`] — so a variable bound anywhere in that
    // transparent chain, however deeply `{ }`-nested, is visible to a LATER
    // `EXISTS` reached while the same frame is on top. Only two things ever
    // open a NEW frame: a sub-`SELECT` (the one real §18.2.1 scope boundary —
    // [`Parser::push_exists_scope_boundary`], fresh and EMPTY: a sub-select is
    // evaluated independently, not correlated with its outer query) and an
    // `EXISTS`/`NOT EXISTS`/`MINUS` body ([`Parser::push_exists_scope_isolated`],
    // SEEDED with a copy of the frame beneath it: `EXISTS`/`MINUS` bodies see
    // outer bindings as already bound — the same injection theorem
    // [`find_scope_conflict`]'s rustdoc proves for `LATERAL` — but neither
    // construct's OWN internal introductions are ever visible outside it
    // (`EXISTS` never joins its body's bindings out at all; `MINUS`'s right
    // operand is explicitly out of scope per §18.2.1), so the seeded frame is
    // POPPED AND DISCARDED, never merged back into what it was seeded from.
    fn exists_scope(&self) -> &[Variable] {
        self.exists_scope_stack
            .last()
            .map_or(&[], VarScope::as_slice)
    }

    /// Open a fresh, EMPTY in-scope-set frame — nothing precedes it. Used at
    /// every query/update operation's own top-level `WHERE` clause and at a
    /// sub-`SELECT`'s (the one real scope boundary; see the module doc above).
    fn push_exists_scope_boundary(&mut self) {
        self.exists_scope_stack.push(VarScope::new());
    }

    /// Open a fresh in-scope-set frame SEEDED with a copy of the frame beneath
    /// it — reads see everything the enclosing rows already bound, but nothing
    /// this frame goes on to introduce is written back once it is popped. Used
    /// at an `EXISTS`/`NOT EXISTS`/`MINUS` body (see the module doc above).
    fn push_exists_scope_isolated(&mut self) {
        let mut seed = VarScope::new();
        for v in self.exists_scope() {
            seed.note(v);
        }
        self.exists_scope_stack.push(seed);
    }

    /// Close the innermost in-scope-set frame, discarding it — the caller is
    /// responsible for having already extracted anything from it that DOES
    /// escape (a sub-`SELECT`'s projected variables, via an ordinary
    /// [`Parser::note_exists_scope`] call on the resulting `Project` node).
    fn pop_exists_scope_boundary(&mut self) {
        self.exists_scope_stack.pop();
    }

    /// Record `pattern`'s contribution to the current in-scope-set frame,
    /// using the SAME transparency rules as [`collect_vars`] (a `Project`
    /// contributes only its projected variables; a `Minus` contributes
    /// nothing when called on ITS right operand — callers simply never call
    /// this for a `Minus` right operand, matching the group loop's own
    /// pre-existing non-call for its local `VarScope`).
    fn note_exists_scope(&mut self, pattern: &GraphPattern) {
        if let Some(top) = self.exists_scope_stack.last_mut() {
            collect_vars(pattern, top);
        }
    }

    /// Record a single fresh binding (a `BIND` target) into the current
    /// in-scope-set frame.
    fn note_exists_scope_var(&mut self, variable: &Variable) {
        if let Some(top) = self.exists_scope_stack.last_mut() {
            top.note(variable);
        }
    }

    /// Parse a query/update operation's own top-level `WHERE` group graph
    /// pattern (`ASK`, `DESCRIBE`, and every UPDATE form — never `SELECT`'s,
    /// which needs its frame to stay open across the projection list parsed
    /// BEFORE `WHERE` and the solution modifiers parsed AFTER it, so it opens
    /// and closes its own frame directly instead of calling this): opens a
    /// fresh EMPTY frame (nothing precedes a query/update operation), parses
    /// the pattern, mirrors it into that frame, and closes the frame before
    /// returning — self-contained because none of this function's callers
    /// have anything of their own that needs to keep consulting it
    /// afterward.
    fn parse_where_clause(&mut self) -> Result<GraphPattern> {
        self.push_exists_scope_boundary();
        let pattern = self.parse_group_graph_pattern()?;
        self.note_exists_scope(&pattern);
        self.pop_exists_scope_boundary();
        Ok(pattern)
    }

    /// Parse an `EXISTS`/`NOT EXISTS` group graph pattern (both `"EXISTS"`
    /// and `"NOT" "EXISTS"` call this — the SAME production, so both share
    /// the SAME check; there is no separate "NOT EXISTS" wording), enforcing
    /// SEP-0007 Part 3: neither `BIND`/a sub-`SELECT`'s `(expr AS ?v)`/a
    /// `GROUP BY (expr AS ?v)` target NOR a `VALUES` variable inside it may
    /// rebind a variable already in scope on the row this `EXISTS` is
    /// testing.
    ///
    /// # The in-scope set consulted
    ///
    /// [`Parser::exists_scope`], READ BEFORE the body is parsed — the
    /// current top in-scope-set frame, exactly as [`Parser::exists_scope`]
    /// (and the frame taxonomy on `Parser::exists_scope_stack`'s doc)
    /// defines it: the transitively scope-transparent accumulation (through
    /// a plain nested group, `OPTIONAL`, `UNION`, `GRAPH`, `SERVICE`, either
    /// side of `LATERAL`) of every variable introduced, left-to-right, since
    /// the nearest enclosing TRUE scope boundary — a sub-`SELECT`'s own
    /// `WHERE`, or the query/update operation's own top-level one. For an
    /// `EXISTS` reached while parsing a `FILTER`'s constraint or a `BIND`'s
    /// value expression mid-group, that is exactly the elements of the
    /// SAME enclosing group parsed so far (this production runs inside
    /// `parse_group_graph_pattern_inner`'s own loop, whose every
    /// scope-transparent branch mirrors its contribution into this frame —
    /// see `Parser::exists_scope_stack`'s doc); for a solution-modifier
    /// expression (`GROUP BY`/`HAVING`/`ORDER BY`), it is the complete
    /// `WHERE` clause's scope (parsed in full before modifiers run).
    ///
    /// For a `SELECT`-list `(expr AS ?v)` target — or an aggregate argument
    /// lifted out of one — parsed BEFORE `WHERE` is even read,
    /// [`Parser::exists_scope`] is necessarily still empty: the row this
    /// `EXISTS` will actually be tested against (`WHERE`'s scope, or the
    /// grouped scope, or the aggregate-fold scope — see
    /// [`ExistsScopeBasis`]) cannot be known yet. Rather than skip the check
    /// (nothing precedes it syntactically is NOT the same as nothing will
    /// ever be in scope — SEP-0007 Part 3 is a SEMANTIC rule about the row at
    /// EVALUATION time, not a textual-order one), `Parser::projection_scope_pending`
    /// marks this window, and the check is DEFERRED into
    /// `Parser::pending_exists_scope_checks` instead of run here — resolved
    /// once `Parser::parse_select`'s post-`WHERE` block completes the
    /// missing root scope. See [`PendingExistsScopeCheck`] for why a single
    /// `local_scope ∪ root_scope` union, computed once the root is known,
    /// suffices at ANY nesting depth reached during this window.
    ///
    /// A NESTED `EXISTS` is checked at its OWN call to this function, with
    /// the in-scope set THAT nesting level sees — which, because this body
    /// is parsed inside a freshly SEEDED (not merged-back) frame (see
    /// `Parser::push_exists_scope_isolated`), already includes everything
    /// visible to the outer `EXISTS` plus whatever this body's own elements
    /// have introduced so far, without this body's OWN introductions ever
    /// leaking to what the OUTER `EXISTS`'s LATER siblings see. (Immediately
    /// or, under `projection_scope_pending`, at the SAME deferred resolution
    /// point as its enclosing `EXISTS` — every nested occurrence reached
    /// during that window is recorded as its OWN, independent
    /// `PendingExistsScopeCheck`.)
    fn parse_exists_body(&mut self) -> Result<GraphPattern> {
        // Anchor the error at the body's own opening brace rather than
        // wherever the cursor lands after parsing it, mirroring `LATERAL`'s
        // own `at` capture.
        let at = self.span();
        let scope: Vec<Variable> = self.exists_scope().to_vec();
        self.push_exists_scope_isolated();
        let body = self.parse_group_graph_pattern()?;
        self.pop_exists_scope_boundary();
        if self.projection_scope_pending {
            let mut local_scope = scope;
            let basis = if self.in_aggregate_argument {
                ExistsScopeBasis::AggregateArgument
            } else {
                // A later SELECT-list target sees every earlier one already
                // bound (the `Extend` chain `Parser::parse_select` builds
                // from `select_exprs` nests that way) — see
                // `Parser::projection_seen_targets`'s doc.
                local_scope.extend(self.projection_seen_targets.iter().cloned());
                ExistsScopeBasis::Projection
            };
            self.pending_exists_scope_checks
                .push(PendingExistsScopeCheck {
                    local_scope,
                    body: body.clone(),
                    at,
                    basis,
                });
            return Ok(body);
        }
        if let Some((var, intro)) = find_scope_conflict(&scope, &body) {
            return Err(ParseError::syntax(
                format!(
                    "{} ?{} inside {} is already in scope on {}",
                    intro.as_str(),
                    var.as_str(),
                    ScopeConstruct::Exists.keyword(),
                    ScopeConstruct::Exists.already_in_scope_clause(),
                ),
                at,
            ));
        }
        Ok(body)
    }

    fn at(&self, t: &Token<'a>) -> bool {
        self.peek() == Some(t)
    }

    fn eat(&mut self, t: &Token<'a>) -> bool {
        if self.at(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Token<'a>) -> Result<()> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(ParseError::syntax(
                format!("expected {t:?}, found {:?}", self.peek()),
                self.span(),
            ))
        }
    }

    /// Is the current token the keyword `kw` (case-insensitive `Word`)?
    fn peek_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn peek2_kw(&self, kw: &str) -> bool {
        matches!(self.peek2(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.peek_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<()> {
        if self.eat_kw(kw) {
            Ok(())
        } else {
            Err(ParseError::syntax(
                format!("expected keyword {kw}, found {:?}", self.peek()),
                self.span(),
            ))
        }
    }

    fn expect_eof(&self) -> Result<()> {
        if self.pos >= self.tokens.len() {
            Ok(())
        } else {
            Err(ParseError::syntax(
                format!("unexpected trailing token {:?}", self.peek()),
                self.span(),
            ))
        }
    }

    // ── prologue + query form ────────────────────────────────────────────────

    fn parse_query(&mut self) -> Result<Query> {
        self.parse_prologue()?;
        let base_iri = self.base.clone().map(NamedNode::new).transpose()?;
        if self.peek_kw("SELECT") {
            self.parse_select(base_iri)
        } else if self.peek_kw("CONSTRUCT") {
            self.parse_construct(base_iri)
        } else if self.peek_kw("ASK") {
            self.parse_ask(base_iri)
        } else if self.peek_kw("DESCRIBE") {
            self.parse_describe(base_iri)
        } else {
            Err(ParseError::syntax(
                "expected SELECT, CONSTRUCT, ASK or DESCRIBE",
                self.span(),
            ))
        }
    }

    fn parse_prologue(&mut self) -> Result<()> {
        loop {
            if self.eat_kw("BASE") {
                let iri = self.expect_iriref()?;
                self.base = Some(iri);
            } else if self.eat_kw("PREFIX") {
                let (prefix, _) = self.expect_pname_ns()?;
                let iri = self.expect_iriref()?;
                self.prefixes.insert(prefix, iri);
            } else if self.eat_kw("VERSION") {
                // SPARQL 1.2 version declaration: `VERSION <string>` (SPARQL 1.2 Query
                // specification §4.4). Retained as `self.version`, last-wins across
                // repeated declarations (the grammar permits `Version*`); see
                // `SparqlVersion` for what evaluation does with each spelling. Parsing
                // itself is syntax-only — ANY string is accepted here (vendored W3C
                // `w3c-sparql12` `version-04.rq` declares `"1.1"` and is a
                // `PositiveSyntaxTest`); an unrecognized version is refused only at
                // evaluation admission, not at parse time.
                match self.bump() {
                    Some(Token::StringLit(s)) => {
                        self.version = Some(SparqlVersion::parse(&s));
                    }
                    other => {
                        return Err(ParseError::syntax(
                            format!("expected a version string after VERSION, found {other:?}"),
                            self.span(),
                        ));
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    fn expect_iriref(&mut self) -> Result<String> {
        match self.bump() {
            Some(Token::Iri(s)) => self.resolve_iri(&s),
            other => Err(ParseError::syntax(
                format!("expected IRIREF, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// Expect a `prefix:` namespace token (PNAME_NS), i.e. an empty local part.
    fn expect_pname_ns(&mut self) -> Result<(String, String)> {
        match self.bump() {
            Some(Token::PrefixedName(p, l)) if l.is_empty() => Ok((p.to_string(), l.into_owned())),
            // `PREFIX ex:local <...>` is malformed — a prologue prefix must be a
            // bare PNAME_NS (`ex:`). Reject rather than silently dropping `local`.
            Some(Token::PrefixedName(p, l)) => Err(ParseError::syntax(
                format!("PREFIX declaration must be a bare namespace, found {p}:{l}"),
                self.span(),
            )),
            other => Err(ParseError::syntax(
                format!("expected prefix declaration, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// Resolve a lexical IRIREF against the in-scope `BASE` (relative refs only).
    /// Propagates a typed [`ParseError::Iri`] when the base or the resolution is
    /// malformed instead of silently falling back to the raw string.
    fn resolve_iri(&self, s: &str) -> Result<String> {
        match &self.base {
            Some(base) if !is_absolute_iri(s) => {
                let base_iri = purrdf_iri::parse(base).map_err(|e| ParseError::Iri {
                    lexical: base.clone(),
                    reason: e.to_string(),
                })?;
                let resolved =
                    purrdf_iri::Iri::resolve(&base_iri, s).map_err(|e| ParseError::Iri {
                        lexical: s.to_owned(),
                        reason: e.to_string(),
                    })?;
                Ok(resolved.as_str().to_owned())
            }
            _ => Ok(s.to_owned()),
        }
    }

    fn resolve_prefixed(&self, prefix: &str, local: &str) -> Result<NamedNode> {
        match self.prefixes.get(prefix) {
            Some(ns) => NamedNode::new(format!("{ns}{local}")),
            None => Err(ParseError::syntax(
                format!("undeclared prefix {prefix:?}"),
                self.span(),
            )),
        }
    }

    // ── query forms ──────────────────────────────────────────────────────────

    fn parse_select(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("SELECT")?;
        let distinct = self.eat_kw("DISTINCT");
        let reduced = !distinct && self.eat_kw("REDUCED");

        // A fresh, EMPTY EXISTS in-scope-set frame for this SELECT/sub-SELECT —
        // opened before the projection list is even read, so an `EXISTS`
        // inside a `(expr AS ?v)` SELECT-list target (parsed here, BEFORE
        // `WHERE`) sees no ambient scope leaked in from whatever query this
        // one is nested inside (a sub-SELECT is not correlated with its outer
        // query — see the module doc on `Parser::exists_scope_stack`). Stays
        // open through `WHERE` and the solution modifiers (`GROUP BY`/
        // `HAVING`/`ORDER BY` all read the current row `WHERE` produced), and
        // is popped just once, at this function's single success return.
        self.push_exists_scope_boundary();

        // This SELECT's OWN deferred-EXISTS-scope window (SEP-0007 Part 3's
        // projection-list position — see `Parser::projection_scope_pending`'s doc): save
        // whatever the ENCLOSING parse had (this may itself be a sub-SELECT
        // reached mid-projection-list of an outer one, via `EXISTS { SELECT
        // ... }`), open a fresh one for the projection list about to be
        // parsed, and restore the enclosing state at this function's single
        // success return — matching `push_exists_scope_boundary`'s own
        // pop-once-at-success-return discipline just above: on any parse
        // error the whole request aborts (a single `Result` propagates all
        // the way to the public entry point, and this `Parser` is never
        // consulted again), so an unrestored window on an error path is, as
        // with the unpopped `exists_scope_stack` frame in that same case,
        // never observed.
        let saved_projection_scope_pending = self.projection_scope_pending;
        let saved_in_aggregate_argument = self.in_aggregate_argument;
        let saved_projection_seen_targets = std::mem::take(&mut self.projection_seen_targets);
        let saved_pending_exists_scope_checks =
            std::mem::take(&mut self.pending_exists_scope_checks);
        self.projection_scope_pending = true;
        self.in_aggregate_argument = false;

        // Projection: `*` or a list of Var / (Expr AS Var).
        let mut star = false;
        let mut projected: Vec<Variable> = Vec::new();
        let mut select_exprs: Vec<(Variable, Expression)> = Vec::new();
        let mut aggregates: Vec<(Variable, AggregateExpression)> = Vec::new();
        if self.eat(&Token::Star) {
            star = true;
        } else {
            loop {
                if let Some(Token::Variable(_)) = self.peek() {
                    projected.push(self.expect_var()?);
                } else if self.at(&Token::LParen) {
                    self.expect(&Token::LParen)?;
                    let expr = self.parse_expression_lifting_aggs(&mut aggregates)?;
                    self.expect_kw("AS")?;
                    let var = self.expect_var()?;
                    self.expect(&Token::RParen)?;
                    projected.push(var.clone());
                    // Recorded so a LATER projection-list `EXISTS` deferred
                    // under `projection_scope_pending` sees this target as
                    // already bound — see
                    // `Parser::projection_seen_targets`'s doc.
                    self.projection_seen_targets.push(var.clone());
                    // A long `SELECT (e1 AS ?v1) … (eN AS ?vN)` list lowers to
                    // a chain of N `Extend` nodes wrapped around the WHERE
                    // pattern (below, near the query's assembly) — no brace
                    // anywhere, so `MAX_GRAPH_PATTERN_DEPTH` never sees it.
                    // Charged per condition, at the point each is parsed.
                    self.charge_pattern_nodes(1)?;
                    select_exprs.push((var, expr));
                } else {
                    break;
                }
            }
            if projected.is_empty() {
                return Err(ParseError::syntax("empty SELECT projection", self.span()));
            }
        }

        // The projection list is fully parsed — leave the deferred-EXISTS-scope
        // window. Every `EXISTS`/`NOT EXISTS` reached from here on (`WHERE`,
        // `GROUP BY`/`HAVING`/`ORDER BY`) already has a correct
        // `Parser::exists_scope` to check against immediately, exactly as
        // before this window existed.
        self.projection_scope_pending = false;

        // Dataset clause (FROM / FROM NAMED), §13.2.
        let dataset = self.parse_dataset_clauses()?;

        self.eat_kw("WHERE");
        let where_pat = self.parse_group_graph_pattern()?;
        // A belt-and-suspenders bulk mirror on top of the group loop's own
        // incremental one (`Parser::note_exists_scope`/`_var`, called
        // throughout `parse_group_graph_pattern_inner`): the incremental
        // mirror alone already covers the ordinary case, but `where_pat` can
        // also come back from `parse_group_graph_pattern_inner`'s early
        // sub-SELECT return (a `WHERE` clause that IS just `{ SELECT ... }`,
        // no loop iteration at this level at all) — this call guarantees the
        // solution modifiers below see `where_pat`'s full scope regardless of
        // which path built it. Idempotent with the incremental mirror in the
        // ordinary case ([`VarScope::note`] dedupes).
        self.note_exists_scope(&where_pat);

        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;

        // §19.8: each SELECT `(expr AS ?v)` target must be fresh — not already in
        // scope. When the query aggregates (an explicit `GROUP BY` or any
        // aggregate ⇒ implicit single group), only the grouping keys and
        // group-expression targets stay visible to the projection; the raw WHERE
        // pattern variables are projected away by grouping, so re-binding one via
        // `(expr AS ?v)` is legal (e.g. `SELECT (123 AS ?z) … GROUP BY ?s`).
        if !select_exprs.is_empty() {
            let aggregating = !modifiers.group_by.is_empty()
                || !modifiers.group_extends.is_empty()
                || !aggregates.is_empty();
            let mut in_scope: std::collections::HashSet<Variable> = if aggregating {
                modifiers
                    .group_by
                    .iter()
                    .cloned()
                    .chain(modifiers.group_extends.iter().map(|(v, _)| v.clone()))
                    .collect()
            } else {
                // A PRODUCTION consultation of the whole WHERE pattern's
                // scope — once per SELECT with `(expr AS ?v)` targets, never
                // once per element of that WHERE pattern (see
                // `Parser::scope_consultations`'s doc).
                self.note_scope_consultation();
                visible_variables(&where_pat).into_iter().collect()
            };

            // SEP-0007 Part 3's projection-list position: resolve every `EXISTS`/`NOT EXISTS`
            // deferred out of `Parser::parse_exists_body` while this
            // projection list was being parsed — BEFORE the loop below
            // folds this SELECT's own targets into `in_scope`, so
            // `ExistsScopeBasis::Projection` entries resolve against the
            // SAME root `in_scope` currently holds (the grouped keys, or the
            // full `WHERE` scope) with no contamination from sibling
            // targets `PendingExistsScopeCheck::local_scope` did not already
            // capture. `ExistsScopeBasis::AggregateArgument` entries resolve
            // against the raw `WHERE`/grouping-extend scope instead,
            // computed lazily (at most once, only if this SELECT actually
            // deferred an aggregate-argument `EXISTS`) since it differs from
            // `in_scope` only when the query aggregates.
            if !self.pending_exists_scope_checks.is_empty() {
                // Taken (not drained-in-place): the loop below needs its own
                // unborrowed `&mut self` to call `note_scope_consultation`
                // while computing `agg_arg_scope` lazily.
                let pending_checks = std::mem::take(&mut self.pending_exists_scope_checks);
                let mut agg_arg_scope: Option<std::collections::HashSet<Variable>> = None;
                for pending in pending_checks {
                    let root: &std::collections::HashSet<Variable> = match pending.basis {
                        ExistsScopeBasis::Projection => &in_scope,
                        ExistsScopeBasis::AggregateArgument => {
                            if aggregating {
                                if agg_arg_scope.is_none() {
                                    self.note_scope_consultation();
                                    let scope: std::collections::HashSet<Variable> =
                                        visible_variables(&where_pat)
                                            .into_iter()
                                            .chain(
                                                modifiers
                                                    .group_extends
                                                    .iter()
                                                    .map(|(v, _)| v.clone()),
                                            )
                                            .collect();
                                    agg_arg_scope = Some(scope);
                                }
                                agg_arg_scope.as_ref().expect("just populated above")
                            } else {
                                // Not aggregating: the raw `WHERE` scope IS
                                // the projection's own root already (no
                                // grouping-extend targets exist to add).
                                &in_scope
                            }
                        }
                    };
                    let scope: Vec<Variable> = pending
                        .local_scope
                        .iter()
                        .cloned()
                        .chain(root.iter().cloned())
                        .collect();
                    if let Some((var, intro)) = find_scope_conflict(&scope, &pending.body) {
                        return Err(ParseError::syntax(
                            format!(
                                "{} ?{} inside {} is already in scope on {}",
                                intro.as_str(),
                                var.as_str(),
                                ScopeConstruct::Exists.keyword(),
                                ScopeConstruct::Exists.already_in_scope_clause(),
                            ),
                            pending.at,
                        ));
                    }
                }
            }

            for (var, _) in &select_exprs {
                if !in_scope.insert(var.clone()) {
                    return Err(ParseError::syntax(
                        format!(
                            "SELECT expression target ?{} is already in scope",
                            var.as_str()
                        ),
                        self.span(),
                    ));
                }
            }
        }

        // §11.1 grammar note: the `SELECT *` shorthand is illegal in an aggregate
        // query — an explicit `GROUP BY` (keys or expression conditions) or any
        // aggregate makes the projection ill-defined, so it is a hard syntax
        // error (vendored W3C `syntax-query` `syn-bad-01`: `SELECT * … GROUP BY`).
        if star
            && (!modifiers.group_by.is_empty()
                || !modifiers.group_extends.is_empty()
                || !aggregates.is_empty())
        {
            return Err(ParseError::syntax(
                "SELECT * is not allowed in an aggregate query (GROUP BY or aggregation)",
                self.span(),
            ));
        }

        // §18.2.4.1 grouping constraint: when the query aggregates (an explicit
        // `GROUP BY`, or one or more aggregates in the SELECT clause ⇒ an implicit
        // single group), every BARE projected variable — one named directly as a
        // `Var`, not the fresh target of a `(expr AS ?v)` — must be one of the
        // `GROUP BY` keys (explicit or the synthetic var of an expression-valued
        // GROUP BY condition). A bare projected variable that is neither a group
        // key nor confined to an aggregate is a hard query error, not a silently
        // wrong answer (this is the vendored W3C `grouping/group06`/`group07`
        // negative-syntax cases: `SELECT ?s ?v { ... } GROUP BY ?s` projects the
        // ungrouped, non-aggregated `?v`). `SELECT *` is exempted here: its
        // projection is derived structurally from the (already-grouped) algebra
        // node below, so it can only ever expose grouped/aggregate variables.
        if !star {
            let is_aggregating = !modifiers.group_by.is_empty() || !aggregates.is_empty();
            if is_aggregating {
                let as_targets: std::collections::HashSet<&Variable> =
                    select_exprs.iter().map(|(v, _)| v).collect();
                let group_vars: std::collections::HashSet<&Variable> =
                    modifiers.group_by.iter().collect();
                for var in &projected {
                    if !as_targets.contains(var) && !group_vars.contains(var) {
                        return Err(ParseError::syntax(
                            format!(
                                "SELECT projects ?{}, which is neither a GROUP BY key nor \
                                 confined to an aggregate",
                                var.as_str()
                            ),
                            self.span(),
                        ));
                    }
                }
            }
        }

        // Trailing `ValuesClause` (§18.2.4.3): a `VALUES DataBlock` after the
        // solution modifiers — valid on both a top-level query and a `SubSelect`.
        // It is joined with the WHERE group graph pattern *before* grouping and
        // projection, so the inline data is visible to aggregation and `SELECT *`.
        // Through the shared `join()` helper (not a raw `GraphPattern::Join`),
        // matching the identity-absorbing construction every IN-BODY `VALUES`
        // block already goes through (`parse_group_graph_pattern_inner`'s own
        // `VALUES` arm) — an empty WHERE clause (`{}`) plus a trailing
        // `VALUES` must reach the SAME `Values { .. }` node an in-body
        // `{ VALUES … }` does, not a `Join { Bgp { [] }, Values { .. } }`
        // the round-trip serializer has no way to reproduce (it has no
        // surface form for a Join whose left operand is visibly, deliberately
        // the identity table rather than an omitted one).
        let where_pat = if self.peek_kw("VALUES") {
            let values = self.parse_inline_data()?;
            join(where_pat, values)
        } else {
            where_pat
        };

        // Build the algebra (§18.2.4 ordering).
        let mut p = where_pat;
        // Expression-valued GROUP BY conditions bind their synthetic/explicit
        // grouping variable BELOW the Group, so `eval_group` sees a ready column.
        for (var, expr) in modifiers.group_extends {
            p = GraphPattern::Extend {
                inner: Box::new(p),
                variable: var,
                expression: expr,
            };
        }
        let has_group = !modifiers.group_by.is_empty() || !aggregates.is_empty();
        if has_group {
            p = GraphPattern::Group {
                inner: Box::new(p),
                variables: modifiers.group_by.clone(),
                aggregates,
            };
        }
        for expr in modifiers.having {
            p = GraphPattern::Filter {
                expr,
                inner: Box::new(p),
            };
        }
        for (var, expr) in select_exprs {
            p = GraphPattern::Extend {
                inner: Box::new(p),
                variable: var,
                expression: expr,
            };
        }
        if !modifiers.order_by.is_empty() {
            p = GraphPattern::OrderBy {
                inner: Box::new(p),
                expression: modifiers.order_by,
            };
        }
        let variables = if star {
            // A PRODUCTION consultation of the whole (modifier-wrapped) query
            // pattern's scope — once per `SELECT *`, never once per element
            // of the WHERE pattern it wraps.
            self.note_scope_consultation();
            visible_variables(&p)
        } else {
            projected
        };
        p = GraphPattern::Project {
            inner: Box::new(p),
            variables,
        };
        if distinct {
            p = GraphPattern::Distinct { inner: Box::new(p) };
        } else if reduced {
            p = GraphPattern::Reduced { inner: Box::new(p) };
        }
        if modifiers.offset.is_some() || modifiers.limit.is_some() {
            p = GraphPattern::Slice {
                inner: Box::new(p),
                start: modifiers.offset.unwrap_or(0),
                length: modifiers.limit,
            };
        }
        self.pop_exists_scope_boundary();
        // Every deferred check this SELECT recorded is resolved inside the
        // `!select_exprs.is_empty()` §19.8 block above — entries can exist
        // ONLY if that block ran (an `EXISTS` can be parsed here solely from
        // within a `(expr AS ?v)` target's own expression, which always
        // pushes to `select_exprs`), so nothing should ever reach this point
        // still unresolved. A debug-only guard, not a silent drop: were this
        // invariant ever wrong, restoring the saved (unrelated, outer) list
        // below would discard the unresolved checks instead of erroring.
        debug_assert!(
            self.pending_exists_scope_checks.is_empty(),
            "a deferred EXISTS scope check was never resolved"
        );
        // Restore the enclosing parse's own deferred-EXISTS-scope window —
        // see the save at this function's top.
        self.projection_scope_pending = saved_projection_scope_pending;
        self.in_aggregate_argument = saved_in_aggregate_argument;
        self.projection_seen_targets = saved_projection_seen_targets;
        self.pending_exists_scope_checks = saved_pending_exists_scope_checks;
        Ok(Query::Select {
            pattern: p,
            dataset,
            base_iri,
            version: self.version.clone(),
        })
    }

    fn parse_construct(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("CONSTRUCT")?;
        // `CONSTRUCT GRAPH VarOrIri …` — the whole-template shorthand of the
        // quad-producing form (see the `Query::Construct::template` rustdoc).
        // Read BEFORE the short/long-form fork, because it prefixes both: it
        // names where the instantiated statements go by DEFAULT, and says
        // nothing about how the template is written.
        //
        // Unambiguous at exactly one token of lookahead: the long form's next
        // token is `{` and the short form's is `FROM` or `WHERE`, so a `GRAPH`
        // here can be nothing else. A plain `CONSTRUCT { … }` never reaches
        // `eat_kw` with anything but `{`, so its parse is bit-for-bit what it
        // was.
        let default_graph = if self.eat_kw("GRAPH") {
            Some(self.parse_var_or_iri_name()?)
        } else {
            None
        };
        // Short form (§16.2.1): `CONSTRUCT DatasetClause* WHERE { TriplesTemplate }`
        // with no explicit template — the template *is* the WHERE triples block.
        if !self.at(&Token::LBrace) {
            let dataset = self.parse_dataset_clauses()?;
            self.expect_kw("WHERE")?;
            // The short form's template *is* the WHERE triples block (§16.2.1) — but an
            // RDF 1.2 reifier/annotation (`~ id`, `{| … |}`) inside that block desugars
            // to a FRESH synthetic reifier blank at parse time (`parse_triple_annotations`
            // / `parse_triple_node`). A `.clone()` of the already-desugared triples would
            // give the WHERE match and the CONSTRUCT template the SAME reifier blank
            // identity, which conflates two independent things: the WHERE-side reifier is
            // a non-distinguished (matched-but-discarded) existential witness, while the
            // template-side reifier is minted FRESH per solution row regardless of what it
            // matched (the general CONSTRUCT template blank-node rule). Reparsing the SAME
            // token span a second time in a forked cursor, so `fresh_anon()` mints a
            // NEW counter value — gives the WHERE copy its OWN, independent synthetic
            // reifier blanks, decoupled from the template's (W3C `eval-triple-terms`
            // `construct-5`/`expr-1`: a query-supplied `~`/`{| |}` name IS a real token, so
            // re-tokenizing reproduces the SAME label there — only the auto-generated
            // synthetic blanks differ between the two parses). The fork is bounded to the
            // braced block (`fork_block`, not the whole remaining token stream) — `self`
            // still owns the opening `{` at this point, so the sub-parser must consume its
            // own copy before reading the template.
            let (template, counters) = {
                let mut template_parser = self.fork_block();
                template_parser.expect(&Token::LBrace)?;
                let template = template_parser.parse_short_form_template()?;
                let counters = (
                    template_parser.agg_counter,
                    template_parser.anon_counter,
                    template_parser.group_counter,
                );
                (template, counters)
            };
            self.set_counters(counters);
            self.expect(&Token::LBrace)?;
            let where_patterns = self.parse_short_form_template()?;
            self.expect(&Token::RBrace)?;
            // The short form's block is a `TriplesTemplate`, so every statement
            // is unscoped; the `CONSTRUCT GRAPH …` shorthand — the only graph
            // name this form can carry — supplies the graph for all of them.
            let template = scope_triples(template, default_graph.as_ref());
            let where_pat = GraphPattern::Bgp {
                patterns: where_patterns,
            };
            // The short form's WHERE reading is a plain triples block (no
            // `FILTER`/`BIND` production runs over it at all — see
            // `parse_construct_template`), so there is nothing to mirror
            // incrementally; a fresh EMPTY-then-bulk-mirrored frame covers an
            // `EXISTS` in the trailing `ORDER BY` exactly like the long form's
            // incrementally-built one does.
            self.push_exists_scope_boundary();
            self.note_exists_scope(&where_pat);
            let mut aggregates = Vec::new();
            let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
            if !aggregates.is_empty()
                || !modifiers.group_by.is_empty()
                || !modifiers.having.is_empty()
            {
                return Err(ParseError::unsupported("aggregation/HAVING in CONSTRUCT"));
            }
            let mut p = where_pat;
            if !modifiers.order_by.is_empty() {
                p = GraphPattern::OrderBy {
                    inner: Box::new(p),
                    expression: modifiers.order_by,
                };
            }
            if modifiers.offset.is_some() || modifiers.limit.is_some() {
                p = GraphPattern::Slice {
                    inner: Box::new(p),
                    start: modifiers.offset.unwrap_or(0),
                    length: modifiers.limit,
                };
            }
            self.pop_exists_scope_boundary();
            return Ok(Query::Construct {
                template,
                pattern: p,
                dataset,
                base_iri,
                version: self.version.clone(),
            });
        }
        // Long form: CONSTRUCT { ConstructQuads } WHERE { ... }
        self.expect(&Token::LBrace)?;
        let template = self.parse_construct_quads()?;
        self.expect(&Token::RBrace)?;
        // The `CONSTRUCT GRAPH …` shorthand is a DEFAULT, not an override: it
        // supplies the graph for every template slot that did not name one
        // itself, so an inner `GRAPH` block still wins over it. With no
        // shorthand this is the identity.
        let template = scope_template(template, default_graph.as_ref());
        let dataset = self.parse_dataset_clauses()?;
        self.eat_kw("WHERE");
        self.push_exists_scope_boundary();
        let where_pat = self.parse_group_graph_pattern()?;
        self.note_exists_scope(&where_pat);
        let mut aggregates = Vec::new();
        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
        if !aggregates.is_empty() || !modifiers.group_by.is_empty() || !modifiers.having.is_empty()
        {
            return Err(ParseError::unsupported("aggregation/HAVING in CONSTRUCT"));
        }
        let mut p = where_pat;
        if !modifiers.order_by.is_empty() {
            p = GraphPattern::OrderBy {
                inner: Box::new(p),
                expression: modifiers.order_by,
            };
        }
        if modifiers.offset.is_some() || modifiers.limit.is_some() {
            p = GraphPattern::Slice {
                inner: Box::new(p),
                start: modifiers.offset.unwrap_or(0),
                length: modifiers.limit,
            };
        }
        self.pop_exists_scope_boundary();
        Ok(Query::Construct {
            template,
            pattern: p,
            dataset,
            base_iri,
            version: self.version.clone(),
        })
    }

    fn parse_ask(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("ASK")?;
        let dataset = self.parse_dataset_clauses()?;
        self.eat_kw("WHERE");
        let pattern = self.parse_where_clause()?;
        let mut aggregates = Vec::new();
        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
        // ASK ignores solution modifiers semantically; rather than silently
        // dropping a parsed one, hard-fail (no-optionality / no silent discard).
        if !modifiers.is_empty() || !aggregates.is_empty() {
            return Err(ParseError::unsupported("solution modifiers on ASK"));
        }
        Ok(Query::Ask {
            pattern,
            dataset,
            base_iri,
            version: self.version.clone(),
        })
    }

    fn parse_describe(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("DESCRIBE")?;
        let mut targets = Vec::new();
        if self.eat(&Token::Star) {
            // DESCRIBE * — no explicit targets.
        } else {
            loop {
                match self.peek() {
                    Some(Token::Variable(_)) => {
                        targets.push(NamedNodePattern::Variable(self.expect_var()?));
                    }
                    Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                        targets.push(NamedNodePattern::NamedNode(self.expect_iri_node()?));
                    }
                    _ => break,
                }
            }
            if targets.is_empty() {
                return Err(ParseError::syntax("DESCRIBE needs a target", self.span()));
            }
        }
        let dataset = self.parse_dataset_clauses()?;
        let pattern = if self.eat_kw("WHERE") || self.at(&Token::LBrace) {
            self.parse_where_clause()?
        } else {
            GraphPattern::Bgp { patterns: vec![] }
        };
        let mut aggregates = Vec::new();
        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
        if !modifiers.is_empty() || !aggregates.is_empty() {
            return Err(ParseError::unsupported("solution modifiers on DESCRIBE"));
        }
        Ok(Query::Describe {
            pattern,
            targets,
            dataset,
            base_iri,
            version: self.version.clone(),
        })
    }

    /// Zero or more `FROM [NAMED] <iri>` dataset clauses (§13.2). `FROM <iri>` adds to
    /// the active default graph; `FROM NAMED <iri>` adds an addressable named graph.
    fn parse_dataset_clauses(&mut self) -> Result<QueryDataset> {
        let mut default = Vec::new();
        let mut named = Vec::new();
        while self.eat_kw("FROM") {
            if self.eat_kw("NAMED") {
                named.push(self.expect_iri_node()?);
            } else {
                default.push(self.expect_iri_node()?);
            }
        }
        Ok(QueryDataset { default, named })
    }

    fn parse_triples_template(&mut self) -> Result<Vec<TriplePattern>> {
        // A `TriplesTemplate` (§16.2 grammar) — the same triples-block grammar as
        // a group's BGP, so RDF 1.2 reifiers/annotations and triple terms desugar
        // identically. Property paths are *not* valid in a template.
        //
        // `parse_triples_block` stops of its own accord at a `block_boundary()`,
        // which includes both `GRAPH` and `{`; that is what lets
        // [`Self::parse_construct_quads`] resume at a nested graph block without
        // this function needing to know graph blocks exist at all.
        if self.at(&Token::RBrace) {
            return Ok(Vec::new());
        }
        match self.parse_triples_block()? {
            GraphPattern::Bgp { patterns } => Ok(patterns),
            // A template asserts triples; neither a property path nor a property
            // function (a relation call) can be asserted.
            other => Err(ParseError::syntax(
                if block_has_property_function(&other) {
                    "property functions are not allowed in a CONSTRUCT template"
                } else {
                    "property paths are not allowed in a CONSTRUCT template"
                },
                self.span(),
            )),
        }
    }

    /// The `CONSTRUCT` short form's `WHERE { TriplesTemplate }` block
    /// (§16.2.1): triples only, because that ONE block is read twice — once as
    /// the template and once as the `WHERE` algebra, which is a
    /// [`GraphPattern::Bgp`] and has no graph slot to carry a scope into. A
    /// `GRAPH` block here is refused BY NAME rather than left to fail as an
    /// unexpected-term error further along.
    fn parse_short_form_template(&mut self) -> Result<Vec<TriplePattern>> {
        self.reject_graph_block_in_short_form()?;
        let triples = self.parse_triples_template()?;
        // `parse_triples_block` stops at a `GRAPH`/`{` boundary rather than
        // erroring, so the check has to run on the far side of it too.
        self.reject_graph_block_in_short_form()?;
        Ok(triples)
    }

    /// Refuse a graph block at the cursor with the short form's own diagnostic.
    fn reject_graph_block_in_short_form(&self) -> Result<()> {
        if self.peek_kw("GRAPH") || self.at(&Token::LBrace) {
            return Err(ParseError::syntax(
                "a GRAPH block is not allowed in the CONSTRUCT short form; write the long form \
                 `CONSTRUCT { GRAPH … { … } } WHERE { … }` instead",
                self.span(),
            ));
        }
        Ok(())
    }

    /// Parse a `CONSTRUCT` template body as **quads**:
    ///
    /// ```text
    /// ConstructQuads           ::= TriplesTemplate? ( ConstructQuadsNotTriples '.'? TriplesTemplate? )*
    /// ConstructQuadsNotTriples ::= ( 'GRAPH' VarOrIri )? '{' TriplesTemplate? '}'
    /// ```
    ///
    /// The cursor is positioned just inside the template's opening `{`; parsing
    /// stops at the matching `}`, which the caller consumes.
    ///
    /// Statements written outside any block are unscoped (`graph: None`, the
    /// default graph), which is exactly the SPARQL 1.1 template. A `GRAPH`
    /// block scopes the statements it encloses; blocks may repeat, may name
    /// different graphs, and may be interleaved with unscoped statements in one
    /// template. The graph name is a `VarOrIri`, so it may be a variable whose
    /// binding decides the graph per solution row.
    fn parse_construct_quads(&mut self) -> Result<Vec<QuadPattern>> {
        let mut quads = Vec::new();
        loop {
            if self.at(&Token::RBrace) {
                break;
            }
            // The optional `.` separating a graph block from what follows it.
            if self.eat(&Token::Dot) {
                continue;
            }
            if self.peek_kw("GRAPH") || self.at(&Token::LBrace) {
                // `GRAPH` is optional in this production: a bare nested `{ … }`
                // block is the default graph, spelled as a block.
                let graph = if self.eat_kw("GRAPH") {
                    Some(self.parse_var_or_iri_name()?)
                } else {
                    None
                };
                self.expect(&Token::LBrace)?;
                let triples = self.parse_triples_template()?;
                self.expect(&Token::RBrace)?;
                quads.extend(scope_triples(triples, graph.as_ref()));
                continue;
            }
            // An unscoped `TriplesTemplate` run. `parse_triples_block` always
            // consumes at least one token or errors, and the two block-opening
            // tokens it would refuse are dispatched above, so the loop always
            // makes progress — asserted rather than assumed, because a silent
            // failure of that property would be a hang rather than an error.
            let before = self.pos;
            let triples = self.parse_triples_template()?;
            if self.pos == before {
                return Err(ParseError::syntax(
                    "expected a template statement, a GRAPH block, or `}`",
                    self.span(),
                ));
            }
            quads.extend(scope_triples(triples, None));
        }
        Ok(quads)
    }

    // ── SPARQL 1.1 Update (§3 + grammar §19) ─────────────────────────────────

    /// Parse a full Update request: prologue + a `;`-separated sequence of
    /// graph-update operations. A request with only a prologue (no operations)
    /// is valid, and a trailing `;` is allowed.
    fn parse_update(&mut self) -> Result<Update> {
        self.parse_prologue()?;
        let base_iri = self.base.clone().map(NamedNode::new).transpose()?;

        let mut operations = Vec::new();
        // §4.1.1 + grammar note: a blank node label in `INSERT DATA` ground data
        // is scoped to that one operation — reusing it in another `INSERT DATA`
        // of the same request denotes a fresh vs. same blank ambiguity and is a
        // hard syntax error (vendored W3C `syntax-update-1` `syntax-update-54`).
        // This applies ONLY to ground `INSERT DATA` quads: blank nodes in an
        // `INSERT { … } WHERE` template are minted fresh per solution, so the
        // same template label legitimately recurs across operations (vendored
        // W3C `basic-update` `insert-where-same-bnode`). `DELETE DATA` / DELETE
        // templates are blank-free by invariant, and anonymous blanks carry
        // process-unique ids, so only author-written `_:label`s can collide.
        let mut prior_bnode_labels: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        // Reused across iterations to avoid reallocating the set each loop.
        let mut this_op_labels: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        loop {
            if self.pos >= self.tokens.len() {
                break;
            }
            let op = self.parse_update_operation()?;
            this_op_labels.clear();
            if let GraphUpdateOperation::InsertData { data } = &op {
                collect_quad_bnode_labels(data, &mut this_op_labels);
            }
            for label in &this_op_labels {
                if prior_bnode_labels.contains(label) {
                    return Err(ParseError::syntax(
                        format!("blank node label _:{label} is reused across update operations"),
                        self.span(),
                    ));
                }
            }
            prior_bnode_labels.extend(this_op_labels.drain());
            operations.push(op);
            // An operation separator. Without it, the request is done (a stray
            // trailing token is caught by `expect_eof` at the public entry).
            if !self.eat(&Token::Semicolon) {
                break;
            }
            // A trailing `;` may be followed by more prologue (BASE/PREFIX) and
            // another operation, or by end-of-input.
            self.parse_prologue()?;
        }
        Ok(Update {
            operations,
            base_iri,
            version: self.version.clone(),
        })
    }

    fn parse_update_operation(&mut self) -> Result<GraphUpdateOperation> {
        if self.peek_kw("INSERT") {
            self.parse_insert()
        } else if self.peek_kw("DELETE") {
            self.parse_delete()
        } else if self.peek_kw("WITH") {
            self.parse_with_modify()
        } else if self.peek_kw("LOAD") {
            self.parse_load()
        } else if self.peek_kw("CLEAR") {
            self.parse_clear_or_drop(true)
        } else if self.peek_kw("DROP") {
            self.parse_clear_or_drop(false)
        } else if self.peek_kw("CREATE") {
            self.parse_create()
        } else if self.peek_kw("ADD") || self.peek_kw("MOVE") || self.peek_kw("COPY") {
            self.parse_add_move_copy()
        } else {
            Err(ParseError::syntax(
                format!(
                    "expected an update operation keyword, found {:?}",
                    self.peek()
                ),
                self.span(),
            ))
        }
    }

    /// `INSERT DATA { QuadData }` or `INSERT { QuadPattern } [USING ...] WHERE { ... }`.
    fn parse_insert(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("INSERT")?;
        if self.eat_kw("DATA") {
            let data = self.parse_quad_data()?;
            // INSERT DATA: no variables anywhere; blank nodes ARE allowed (§3.1.1).
            self.enforce_data_invariants(&data, false)?;
            return Ok(GraphUpdateOperation::InsertData { data });
        }
        // INSERT { template } [USING ...] WHERE { ... } — an insert-only modify.
        let insert = self.parse_quad_pattern_block(false)?;
        let using = self.parse_using_clauses()?;
        self.expect_kw("WHERE")?;
        let pattern = self.parse_where_clause()?;
        Ok(GraphUpdateOperation::DeleteInsert {
            delete: Vec::new(),
            insert,
            with: None,
            using,
            pattern: Box::new(pattern),
        })
    }

    /// `DELETE DATA { QuadData }`, `DELETE WHERE { QuadPattern }`, or
    /// `DELETE { template } [INSERT { ... }] [USING ...] WHERE { ... }`.
    fn parse_delete(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("DELETE")?;
        if self.eat_kw("DATA") {
            let data = self.parse_quad_data()?;
            // DELETE DATA: no variables AND no blank nodes (§3.1.2).
            self.enforce_data_invariants(&data, true)?;
            return Ok(GraphUpdateOperation::DeleteData { data });
        }
        if self.eat_kw("WHERE") {
            // DELETE WHERE { QuadPattern } — the template IS the where pattern.
            // `fork_block` bounds the clone to this operation's braced block, so a
            // `;`-separated multi-operation UPDATE stays linear instead of the whole
            // request being O(n²) in the number of `DELETE WHERE` operations.
            let (delete, counters) = {
                let mut delete_parser = self.fork_block();
                let delete = delete_parser.parse_quad_pattern_block(true)?;
                let counters = (
                    delete_parser.agg_counter,
                    delete_parser.anon_counter,
                    delete_parser.group_counter,
                );
                (delete, counters)
            };
            self.set_counters(counters);
            // Parse the same braces as a group graph pattern for the WHERE.
            let pattern = self.parse_where_clause()?;
            return Ok(GraphUpdateOperation::DeleteInsert {
                delete,
                insert: Vec::new(),
                with: None,
                using: Vec::new(),
                pattern: Box::new(pattern),
            });
        }
        // DELETE { template } [INSERT { ... }] [USING ...] WHERE { ... }.
        let delete = self.parse_quad_pattern_block(true)?;
        let insert = if self.eat_kw("INSERT") {
            self.parse_quad_pattern_block(false)?
        } else {
            Vec::new()
        };
        let using = self.parse_using_clauses()?;
        self.expect_kw("WHERE")?;
        let pattern = self.parse_where_clause()?;
        Ok(GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with: None,
            using,
            pattern: Box::new(pattern),
        })
    }

    /// `WITH <iri> (DELETE { ... } | INSERT { ... }) [INSERT { ... }] WHERE { ... }`.
    fn parse_with_modify(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("WITH")?;
        let with = Some(self.expect_iri_node()?);
        let mut delete = Vec::new();
        let mut insert = Vec::new();
        if self.eat_kw("DELETE") {
            delete = self.parse_quad_pattern_block(true)?;
            if self.eat_kw("INSERT") {
                insert = self.parse_quad_pattern_block(false)?;
            }
        } else if self.eat_kw("INSERT") {
            insert = self.parse_quad_pattern_block(false)?;
        } else {
            return Err(ParseError::syntax(
                "WITH must be followed by DELETE and/or INSERT",
                self.span(),
            ));
        }
        let using = self.parse_using_clauses()?;
        self.expect_kw("WHERE")?;
        let pattern = self.parse_where_clause()?;
        Ok(GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            pattern: Box::new(pattern),
        })
    }

    /// Zero or more `USING [NAMED] <iri>` clauses (§3.1.3). The `NAMED` modifier is
    /// preserved: `USING <iri>` folds into the active default graph, `USING NAMED
    /// <iri>` becomes an addressable named graph for the `WHERE`.
    fn parse_using_clauses(&mut self) -> Result<Vec<UsingClause>> {
        let mut using = Vec::new();
        while self.eat_kw("USING") {
            if self.eat_kw("NAMED") {
                using.push(UsingClause::Named(self.expect_iri_node()?));
            } else {
                using.push(UsingClause::Default(self.expect_iri_node()?));
            }
        }
        Ok(using)
    }

    /// `LOAD [SILENT] <iri> [INTO GRAPH <iri>]`.
    fn parse_load(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("LOAD")?;
        let silent = self.eat_kw("SILENT");
        let source = self.expect_iri_node()?;
        let destination = if self.eat_kw("INTO") {
            self.expect_kw("GRAPH")?;
            GraphTarget::Named(self.expect_iri_node()?)
        } else {
            GraphTarget::Default
        };
        Ok(GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        })
    }

    /// `CLEAR [SILENT] <GraphRefAll>` / `DROP [SILENT] <GraphRefAll>`.
    fn parse_clear_or_drop(&mut self, is_clear: bool) -> Result<GraphUpdateOperation> {
        self.expect_kw(if is_clear { "CLEAR" } else { "DROP" })?;
        let silent = self.eat_kw("SILENT");
        let target = self.parse_graph_ref_all()?;
        Ok(if is_clear {
            GraphUpdateOperation::Clear { silent, target }
        } else {
            GraphUpdateOperation::Drop { silent, target }
        })
    }

    /// `CREATE [SILENT] GRAPH <iri>`.
    fn parse_create(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("CREATE")?;
        let silent = self.eat_kw("SILENT");
        self.expect_kw("GRAPH")?;
        let graph = self.expect_iri_node()?;
        Ok(GraphUpdateOperation::Create { silent, graph })
    }

    /// `ADD|MOVE|COPY [SILENT] <GraphOrDefault> TO <GraphOrDefault>`.
    fn parse_add_move_copy(&mut self) -> Result<GraphUpdateOperation> {
        let which = if self.eat_kw("ADD") {
            0u8
        } else if self.eat_kw("MOVE") {
            1
        } else {
            self.expect_kw("COPY")?;
            2
        };
        let silent = self.eat_kw("SILENT");
        let source = self.parse_graph_or_default()?;
        self.expect_kw("TO")?;
        let destination = self.parse_graph_or_default()?;
        Ok(match which {
            0 => GraphUpdateOperation::Add {
                silent,
                source,
                destination,
            },
            1 => GraphUpdateOperation::Move {
                silent,
                source,
                destination,
            },
            _ => GraphUpdateOperation::Copy {
                silent,
                source,
                destination,
            },
        })
    }

    /// `GraphRefAll`: `DEFAULT | NAMED | ALL | GRAPH <iri>`.
    fn parse_graph_ref_all(&mut self) -> Result<GraphTarget> {
        if self.eat_kw("DEFAULT") {
            Ok(GraphTarget::Default)
        } else if self.eat_kw("NAMED") {
            Ok(GraphTarget::NamedGraphs)
        } else if self.eat_kw("ALL") {
            Ok(GraphTarget::All)
        } else if self.eat_kw("GRAPH") {
            Ok(GraphTarget::Named(self.expect_iri_node()?))
        } else {
            Err(ParseError::syntax(
                "expected DEFAULT, NAMED, ALL or GRAPH <iri>",
                self.span(),
            ))
        }
    }

    /// `GraphOrDefault`: `DEFAULT | [GRAPH] <iri>` (no NAMED/ALL here).
    fn parse_graph_or_default(&mut self) -> Result<GraphTarget> {
        if self.eat_kw("DEFAULT") {
            Ok(GraphTarget::Default)
        } else {
            self.eat_kw("GRAPH");
            Ok(GraphTarget::Named(self.expect_iri_node()?))
        }
    }

    /// Parse a `{ ... }` quad block into [`QuadPattern`]s. Triple templates plus
    /// optional nested `GRAPH (<iri>|?var) { triples }` groups. When `is_delete`
    /// is set, any blank node in the templates is a hard error (DELETE templates
    /// disallow blanks per §3.1.3).
    fn parse_quad_pattern_block(&mut self, is_delete: bool) -> Result<Vec<QuadPattern>> {
        let mut quads = Vec::new();
        self.expect(&Token::LBrace)?;
        loop {
            if self.at(&Token::RBrace) {
                break;
            } else if self.eat_kw("GRAPH") {
                let graph = self.parse_var_or_iri_name()?;
                self.collect_quad_group(Some(&graph), is_delete, &mut quads)?;
            } else if self.eat(&Token::Dot) {
                // statement separator between triple blocks
            } else {
                let mut triples = Vec::new();
                self.parse_template_triple(&mut triples)?;
                self.eat(&Token::Dot);
                for triple in triples {
                    if is_delete {
                        reject_blank_in_triple_pattern(&triple, self.span())?;
                    }
                    quads.push(QuadPattern {
                        triple,
                        graph: None,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(quads)
    }

    /// Parse one subject + predicate-object list of an update template
    /// (`TriplesTemplate`), emitting the (RDF 1.2-desugared) triples into
    /// `triples`. Mirrors the subject dispatch of [`parse_triples_block`] so
    /// reifiers, annotations, triple terms, collections and blank-node property
    /// lists all desugar identically; property paths are not admissible here.
    fn parse_template_triple(&mut self, triples: &mut Vec<TriplePattern>) -> Result<()> {
        // `LATERAL` is a *group-graph-pattern* operator (SEP-0006): it has no
        // meaning as a triple to assert, exactly like a property path or a
        // property-function call below, which this function already refuses for
        // the same reason. Unlike those two, `LATERAL` would otherwise be
        // misparsed as a SUBJECT term (this function has no group-boundary
        // dispatch at all — quad templates are TriplesTemplate, not a
        // GroupGraphPattern), producing a confusing "expected a term" error
        // instead of naming the real cause; caught here, before subject
        // parsing, in every quad-template context ([`Self::parse_quad_pattern_block`]'s
        // own loop and the nested `GRAPH { … }` loop in
        // [`Self::collect_quad_group`] — this function's only two callers, so one
        // check here covers `INSERT`/`DELETE`/`DELETE WHERE` templates alike).
        if self.peek_kw("LATERAL") {
            return Err(ParseError::syntax(
                "LATERAL is not allowed in an update template",
                self.span(),
            ));
        }
        let mut sink = BlockSink::default();
        let (subject, standalone_ok) = if self.at(&Token::LBracket) {
            (self.parse_blank_node_property_list(&mut sink)?, true)
        } else if self.at(&Token::LParen) {
            (self.parse_collection(&mut sink)?, false)
        } else if self.at(&Token::TripleOpen) {
            let node = self.parse_triple_node(&mut sink)?;
            let standalone = !matches!(node, TermPattern::Triple(_));
            (node, standalone)
        } else {
            (self.parse_term_pattern()?, false)
        };
        let standalone = standalone_ok
            && (self.at(&Token::Dot) || self.at(&Token::RBrace) || self.at(&Token::LBrace));
        if !standalone {
            self.parse_predicate_object_list(&SubjectArgs::Term(subject), &mut sink)?;
        }
        if !sink.paths.is_empty() {
            return Err(ParseError::syntax(
                "property paths are not allowed in an update template",
                self.span(),
            ));
        }
        // A template asserts triples; a property function is a relation call and
        // has nothing to assert, so a configured property-function predicate is a
        // hard error here rather than a silently-asserted data triple.
        if !sink.prop_fns.is_empty() {
            return Err(ParseError::syntax(
                "property functions are not allowed in an update template",
                self.span(),
            ));
        }
        triples.append(&mut sink.triples);
        Ok(())
    }

    /// Parse a nested `GRAPH g { triples }` group, scoping each parsed triple to
    /// `graph` and pushing the resulting quad patterns into `quads`.
    fn collect_quad_group(
        &mut self,
        graph: Option<&NamedNodePattern>,
        is_delete: bool,
        quads: &mut Vec<QuadPattern>,
    ) -> Result<()> {
        self.expect(&Token::LBrace)?;
        let mut triples = Vec::new();
        while !self.at(&Token::RBrace) {
            self.parse_template_triple(&mut triples)?;
            if !self.eat(&Token::Dot) {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        for triple in triples {
            if is_delete {
                reject_blank_in_triple_pattern(&triple, self.span())?;
            }
            quads.push(QuadPattern {
                triple,
                graph: graph.cloned(),
            });
        }
        Ok(())
    }

    /// Parse a `{ QuadData }` block as quad *patterns* (the same surface as
    /// `parse_quad_pattern_block`). The DATA invariants (no variables; and, for
    /// DELETE DATA, no blank nodes) are enforced separately by
    /// [`enforce_data_invariants`](Self::enforce_data_invariants) so INSERT DATA
    /// can keep its (allowed) blank nodes.
    fn parse_quad_data(&mut self) -> Result<Vec<QuadPattern>> {
        self.parse_quad_pattern_block(false)
    }

    /// Enforce the `INSERT DATA` / `DELETE DATA` invariants by walking the parsed
    /// [`QuadPattern`]s: NO variables anywhere (subject/predicate/object/graph). For
    /// DELETE DATA (`reject_blank`), NO blank nodes either (§3.1.2). INSERT DATA
    /// permits blank nodes (§3.1.1: minted fresh per request). Any violation is a
    /// hard [`ParseError::syntax`].
    fn enforce_data_invariants(&self, quads: &[QuadPattern], reject_blank: bool) -> Result<()> {
        for q in quads {
            if let Some(NamedNodePattern::Variable(_)) = &q.graph {
                return Err(ParseError::syntax(
                    "variable graph in INSERT/DELETE DATA is not allowed",
                    self.span(),
                ));
            }
            self.check_data_triple(&q.triple, reject_blank)?;
        }
        Ok(())
    }

    /// Walk one DATA triple pattern, rejecting variables (always) and blank nodes
    /// (when `reject_blank`). Descends into RDF 1.2 quoted triples.
    fn check_data_triple(&self, t: &TriplePattern, reject_blank: bool) -> Result<()> {
        if let NamedNodePattern::Variable(_) = &t.predicate {
            return Err(ParseError::syntax(
                "variable predicate in INSERT/DELETE DATA is not allowed",
                self.span(),
            ));
        }
        self.check_data_term(&t.subject, reject_blank)?;
        self.check_data_term(&t.object, reject_blank)
    }

    /// Walk one DATA term pattern, rejecting variables (always) and blank nodes
    /// (when `reject_blank`). Descends into RDF 1.2 quoted triples.
    fn check_data_term(&self, t: &TermPattern, reject_blank: bool) -> Result<()> {
        match t {
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => Ok(()),
            TermPattern::Triple(tp) => self.check_data_triple(tp, reject_blank),
            TermPattern::Variable(_) => Err(ParseError::syntax(
                "variable in INSERT/DELETE DATA is not allowed",
                self.span(),
            )),
            TermPattern::BlankNode(_) => {
                if reject_blank {
                    Err(ParseError::syntax(
                        "blank node in DELETE DATA is not allowed",
                        self.span(),
                    ))
                } else {
                    // INSERT DATA blanks are allowed (minted fresh per request).
                    Ok(())
                }
            }
        }
    }

    // ── group graph pattern → algebra (§18.2.2) ──────────────────────────────

    fn parse_group_graph_pattern(&mut self) -> Result<GraphPattern> {
        if self.group_pattern_depth >= MAX_GRAPH_PATTERN_DEPTH {
            return Err(ParseError::syntax(
                format!(
                    "group graph pattern nesting exceeds the safety limit of \
                     {MAX_GRAPH_PATTERN_DEPTH}"
                ),
                self.span(),
            ));
        }
        self.group_pattern_depth += 1;
        let result = self.parse_group_graph_pattern_inner();
        self.group_pattern_depth -= 1;
        result
    }

    fn parse_group_graph_pattern_inner(&mut self) -> Result<GraphPattern> {
        self.expect(&Token::LBrace)?;

        // A sub-SELECT group: `{ SELECT ... }`.
        if self.peek_kw("SELECT") {
            let sub = self.parse_select(None)?;
            self.expect(&Token::RBrace)?;
            return match sub {
                Query::Select { pattern, .. } => Ok(pattern),
                _ => unreachable!("parse_select yields Query::Select"),
            };
        }

        let mut g = GraphPattern::Bgp { patterns: vec![] };
        let mut filters: Vec<Expression> = Vec::new();
        // The incremental in-scope set: kept in lock-step with `g`, one
        // `collect_vars` call over exactly the NEWLY-parsed element (never
        // over the whole, growing `g`) per iteration — see [`VarScope`]'s
        // doc. This is what turns the group loop from O(n²) into O(n log n)
        // over a long run of `BIND`/`LATERAL` elements
        // (`scope_set_stays_linear_over_two_thousand_binds`): the OLD code
        // called `visible_variables(&g)` (a fresh whole-`g` walk) on every
        // `BIND`/`LATERAL`, so the Nth element paid for re-walking the N-1
        // before it.
        let mut scope = VarScope::new();

        loop {
            if self.at(&Token::RBrace) {
                break;
            }
            // A structural charge against `MAX_GRAPH_PATTERN_NODES`, once per
            // group ELEMENT — the choke point that closes the sibling-spine
            // gap `MAX_GRAPH_PATTERN_DEPTH` (brace nesting only) leaves open:
            // every branch below builds (or, for a bracketed sub-group, is
            // about to fold in) exactly one more combinator node onto `g`.
            self.charge_pattern_nodes(1)?;
            if self.at(&Token::LBrace) {
                let mut node = self.parse_group_graph_pattern()?;
                while self.eat_kw("UNION") {
                    // Each ADDITIONAL `UNION` arm is a hidden extra node the
                    // outer per-element charge above does not see (they are
                    // all consumed within this one loop iteration) — charged
                    // here, one per arm past the first.
                    self.charge_pattern_nodes(1)?;
                    let right = self.parse_group_graph_pattern()?;
                    node = GraphPattern::Union {
                        left: Box::new(node),
                        right: Box::new(right),
                    };
                }
                // A bracketed sub-group (possibly a `{ SELECT ... }`, whose
                // contribution is its OWN projection — `collect_vars`'s
                // `Project` arm — not its inner WHERE pattern) or a chain of
                // `UNION` arms: `collect_vars` already knows how to fold
                // either shape into exactly the vars this element puts in
                // scope, in one walk over `node` alone.
                collect_vars(&node, &mut scope);
                self.note_exists_scope(&node);
                g = join(g, node);
            } else if self.eat_kw("OPTIONAL") {
                let inner = self.parse_group_graph_pattern()?;
                let (right, expression) = split_trailing_filter(inner);
                collect_vars(&right, &mut scope);
                self.note_exists_scope(&right);
                g = GraphPattern::LeftJoin {
                    left: Box::new(g),
                    right: Box::new(right),
                    expression,
                };
            } else if self.eat_kw("LATERAL") {
                // Position the error at the start of the RHS block rather than
                // wherever the cursor lands after parsing it (the BIND-scope
                // idiom above captures `self.span()` post hoc; here the RHS can
                // be arbitrarily large, so the useful anchor is the keyword
                // itself).
                let at = self.span();
                let right = self.parse_group_graph_pattern()?;
                // A genuine PRODUCTION consultation (once per `LATERAL`
                // keyword, never per element inside `right`): read the
                // incremental set built so far — `g`'s vars, NOT yet
                // `right`'s — as the LHS scope. Verified against a fresh
                // walk (the non-counting entry point) under
                // `debug_assertions` only.
                self.note_scope_consultation();
                debug_assert_eq!(
                    scope.as_slice(),
                    compute_lateral_left_scope(&g).as_slice(),
                    "the incremental LATERAL left-scope drifted from a fresh visible_variables walk"
                );
                let lhs_scope = scope.as_slice();
                if let Some((var, intro)) = find_scope_conflict(lhs_scope, &right) {
                    return Err(ParseError::syntax(
                        format!(
                            "{} ?{} inside {} is already in scope on {}",
                            intro.as_str(),
                            var.as_str(),
                            ScopeConstruct::Lateral.keyword(),
                            ScopeConstruct::Lateral.already_in_scope_clause(),
                        ),
                        at,
                    ));
                }
                collect_vars(&right, &mut scope);
                self.note_exists_scope(&right);
                g = GraphPattern::Lateral {
                    left: Box::new(g),
                    right: Box::new(right),
                };
            } else if self.eat_kw("MINUS") {
                // A `MINUS` right operand contributes NOTHING to the
                // enclosing group's EXISTS in-scope set either (mirrors the
                // `collect_vars`/§18.2.1 exclusion just below) — but an
                // `EXISTS` INSIDE this right operand must still see whatever
                // the row being tested already has bound (the same injection
                // ground `find_scope_conflict`'s rustdoc proves for
                // `LATERAL`), so the frame is SEEDED, not fresh; it is popped
                // and discarded, never merged back, so nothing this operand
                // itself introduces ever escapes it.
                self.push_exists_scope_isolated();
                let right = self.parse_group_graph_pattern()?;
                self.pop_exists_scope_boundary();
                // SPARQL §18.2.1: `MINUS`'s right operand contributes NOTHING
                // to the enclosing group's scope — no `collect_vars` call
                // here, matching `collect_vars`'s own `Minus` arm.
                g = GraphPattern::Minus {
                    left: Box::new(g),
                    right: Box::new(right),
                };
            } else if self.eat_kw("GRAPH") {
                let name = self.parse_var_or_iri_name()?;
                let inner = self.parse_group_graph_pattern()?;
                let graph = GraphPattern::Graph {
                    name,
                    inner: Box::new(inner),
                };
                collect_vars(&graph, &mut scope);
                self.note_exists_scope(&graph);
                g = join(g, graph);
            } else if self.eat_kw("SERVICE") {
                let silent = self.eat_kw("SILENT");
                let name = self.parse_var_or_iri_name()?;
                let inner = self.parse_group_graph_pattern()?;
                let is_var_endpoint = matches!(name, NamedNodePattern::Variable(_));
                let service = GraphPattern::Service {
                    name,
                    inner: Box::new(inner),
                    silent,
                };
                collect_vars(&service, &mut scope);
                self.note_exists_scope(&service);
                // A variable endpoint (`SERVICE ?g`) is correlated with the
                // enclosing pattern — it must bind the endpoint from the
                // surrounding solution before federating — so it becomes a
                // LATERAL join. A fixed-IRI endpoint stays a plain join.
                g = if is_var_endpoint {
                    GraphPattern::Lateral {
                        left: Box::new(g),
                        right: Box::new(service),
                    }
                } else {
                    join(g, service)
                };
            } else if self.eat_kw("FILTER") {
                filters.push(self.parse_constraint()?);
            } else if self.eat_kw("BIND") {
                self.expect(&Token::LParen)?;
                let expression = self.parse_expression()?;
                self.expect_kw("AS")?;
                let variable = self.expect_var()?;
                self.expect(&Token::RParen)?;
                // §19.6: the variable introduced by BIND must not already be
                // in-scope in the group graph pattern up to this point — a
                // re-binding is a hard syntax error, not a silent shadow
                // (vendored W3C `syntax-query` `syntax-BINDscope6/7/8`). The
                // incremental set answers this in O(log n) — NOT a
                // production "consultation" (`note_scope_consultation` is
                // NOT called here: this is the one site the linear-scan test
                // exists to keep counter-invisible, since it fires once per
                // `BIND` and must not scale the count with the group's
                // element count). The equivalence check still runs, through
                // the free-function (non-counting) `visible_variables`.
                debug_assert_eq!(
                    scope.contains(&variable),
                    visible_variables(&g).contains(&variable),
                    "the incremental BIND-scope check drifted from a fresh visible_variables walk"
                );
                if scope.contains(&variable) {
                    return Err(ParseError::syntax(
                        format!(
                            "BIND target ?{} is already in scope in the group graph pattern",
                            variable.as_str()
                        ),
                        self.span(),
                    ));
                }
                scope.note(&variable);
                self.note_exists_scope_var(&variable);
                g = GraphPattern::Extend {
                    inner: Box::new(g),
                    variable,
                    expression,
                };
            } else if self.peek_kw("VALUES") {
                let values = self.parse_inline_data()?;
                collect_vars(&values, &mut scope);
                self.note_exists_scope(&values);
                g = join(g, values);
            } else if self.eat(&Token::Dot) {
                // statement separator between blocks
            } else {
                // A triples block (BGP / path patterns).
                let block = self.parse_triples_block()?;
                collect_vars(&block, &mut scope);
                self.note_exists_scope(&block);
                g = join(g, block);
            }
        }

        self.expect(&Token::RBrace)?;
        for expr in filters {
            g = GraphPattern::Filter {
                expr,
                inner: Box::new(g),
            };
        }
        Ok(g)
    }

    /// Parse a run of triples (subject + predicate-object lists) into a BGP, any
    /// complex property-path `Path` nodes, and any property-function calls,
    /// assembled together by [`BlockSink::into_pattern`].
    fn parse_triples_block(&mut self) -> Result<GraphPattern> {
        let mut sink = BlockSink::default();
        loop {
            // The subject may be a blank-node property list `[ p o ; … ]` or an RDF
            // collection `( … )`, each of which emits its own triples and yields a
            // fresh node (the BNPL blank, or the collection's head).
            let (subject, standalone_capable) = if self.at(&Token::LBracket) {
                (
                    SubjectArgs::Term(self.parse_blank_node_property_list(&mut sink)?),
                    true,
                )
            } else if self.at(&Token::LParen) {
                // A parenthesized subject is an RDF collection — UNLESS the
                // predicate that follows it is a configured property-function IRI,
                // in which case the parentheses are that call's SUBJECT ARGUMENT
                // LIST and must not be desugared into cons cells. The distinction
                // is settled by a pure token scan to the matching `)` (below),
                // BEFORE anything is parsed, so the collection path is untouched
                // whenever the seam does not fire.
                if self.property_fn_after_group().is_some() {
                    (SubjectArgs::Args(self.parse_prop_fn_arg_list()?), false)
                } else {
                    (SubjectArgs::Term(self.parse_collection(&mut sink)?), false)
                }
            } else if self.at(&Token::TripleOpen) {
                // A reifying triple `<< s p o >>` emits its own reifier triples, so
                // it may stand alone (`<< s p o >> .`) with no predicate-object
                // list. A *triple term* `<<( s p o )>>` is a value: it may head a
                // subject's predicate-object list but must not stand alone.
                let node = self.parse_triple_node(&mut sink)?;
                let standalone_ok = !matches!(node, TermPattern::Triple(_));
                (SubjectArgs::Term(node), standalone_ok)
            } else {
                (SubjectArgs::Term(self.parse_term_pattern()?), false)
            };
            // A standalone `[ … ] .` needs no following predicate-object list (its
            // triples are already emitted); any other subject requires one. A
            // collection always heads a predicate-object list (it is never standalone).
            let standalone = standalone_capable
                && (self.at(&Token::Dot) || self.at(&Token::RBrace) || self.block_boundary());
            if !standalone {
                self.parse_predicate_object_list(&subject, &mut sink)?;
            }
            if !self.eat(&Token::Dot) {
                break;
            }
            // After a `.`, stop if the block ends (`}` or a keyword/brace).
            if self.at(&Token::RBrace) || self.block_boundary() {
                break;
            }
        }
        // `BlockSink::into_pattern` folds every `paths` entry and every
        // `prop_fns` call onto the running pattern with its OWN `join`/
        // `Lateral` node, one per entry — a left-deep spine entirely inside
        // ONE triples block (e.g. a long run of dot-separated COMPLEX
        // property-path triples, which `join` cannot flatten the way it
        // flattens adjacent plain `Bgp` triples) that the group loop's own
        // per-element charge never sees, because the whole block is one
        // element to it. Charged here, once per node `into_pattern` is about
        // to build, before it builds any of them.
        self.charge_pattern_nodes(sink.paths.len() + sink.prop_fns.len())?;
        Ok(sink.into_pattern())
    }

    /// Is `iri` a property function under the configured options — a PREFIX match
    /// against [`ParserOptions::property_fn_namespaces`] OR an EXACT match against
    /// [`ParserOptions::property_fn_iris`]? Always `false` under the default (both
    /// empty) configuration.
    fn is_property_fn(&self, iri: &str) -> bool {
        self.options
            .property_fn_namespaces
            .iter()
            .any(|ns| iri.starts_with(ns.as_str()))
            || self
                .options
                .property_fn_iris
                .iter()
                .any(|exact| exact == iri)
    }

    /// Pure lookahead over a parenthesized subject group starting at the cursor
    /// (which must be at `(`): scan to the MATCHING `)` and resolve the predicate
    /// token that follows it, returning its IRI when that IRI names a configured
    /// property function — i.e. when the group is an argument list rather than an
    /// RDF collection. The cursor is not moved and no error is raised: an
    /// unbalanced group, an unresolvable predicate, or a predicate outside every
    /// configured namespace simply yields `None` and the ordinary collection path
    /// runs (and reports any real error itself).
    fn property_fn_after_group(&self) -> Option<String> {
        if (self.options.property_fn_namespaces.is_empty()
            && self.options.property_fn_iris.is_empty())
            || !matches!(self.token_at(self.pos), Some(Token::LParen))
        {
            return None;
        }
        let mut depth = 0usize;
        let mut idx = self.pos;
        let after = loop {
            match self.token_at(idx)? {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        break idx + 1;
                    }
                }
                _ => {}
            }
            idx += 1;
        };
        let iri = match self.token_at(after)? {
            Token::Iri(s) => self.resolve_iri(s).ok()?,
            Token::PrefixedName(p, l) => self
                .resolve_prefixed(p, l.as_ref())
                .ok()?
                .as_str()
                .to_owned(),
            // `a` is rdf:type spelled as a keyword.
            Token::Word(w) if *w == "a" => RDF_TYPE.to_owned(),
            _ => return None,
        };
        // Only a BARE predicate IRI can be a property function: a path operator
        // trailing it (`pf:p+`, `pf:p/q`, …) makes it a property path, which the
        // seam never claims — so the group before it is an ordinary collection.
        if matches!(
            self.token_at(after + 1),
            Some(
                Token::Star
                    | Token::Plus
                    | Token::Question
                    | Token::Slash
                    | Token::Pipe
                    | Token::LBrace
            )
        ) {
            return None;
        }
        self.is_property_fn(&iri).then_some(iri)
    }

    /// The token at absolute index `idx`, or `None` past the end (or at an
    /// already-consumed slot).
    fn token_at(&self, idx: usize) -> Option<&Token<'a>> {
        self.tokens
            .get(idx)
            .and_then(Option::as_ref)
            .map(|s| &s.token)
    }

    /// Parse one side of a property-function call: a parenthesized argument list
    /// `( … )` or a single bare term (a one-element vector).
    fn parse_prop_fn_args(&mut self) -> Result<Vec<TermPattern>> {
        if self.at(&Token::LParen) {
            self.parse_prop_fn_arg_list()
        } else {
            Ok(vec![self.parse_prop_fn_arg_term()?])
        }
    }

    /// Parse a parenthesized property-function argument list `( t1 t2 … )`,
    /// STRUCTURALLY: the elements are the arguments, with no `rdf:first`/`rdf:rest`
    /// cons-cell desugaring. The empty list `()` is a ZERO-length argument vector
    /// (distinct from a one-element vector holding a bare `rdf:nil` IRI).
    fn parse_prop_fn_arg_list(&mut self) -> Result<Vec<TermPattern>> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        while !self.at(&Token::RParen) {
            args.push(self.parse_prop_fn_arg_term()?);
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    /// Parse ONE property-function argument: any plain term (IRI, literal, blank
    /// node, variable, RDF 1.2 quoted triple). A nested collection and a populated
    /// blank-node property list are hard errors — each would need auxiliary
    /// triples that an argument vector cannot carry.
    fn parse_prop_fn_arg_term(&mut self) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::LParen) => Err(ParseError::syntax(
                "nested collection in property-function argument list",
                self.span(),
            )),
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                if !self.eat(&Token::RBracket) {
                    return Err(ParseError::syntax(
                        "a populated blank-node property list is not allowed in a \
                         property-function argument list",
                        self.span(),
                    ));
                }
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            _ => self.parse_term_pattern(),
        }
    }

    /// Parse a blank-node property list `[ predicate object … ]` (RDF 1.1 §4.2,
    /// SPARQL §19.6). Mints a fresh blank node, emits the embedded triples into
    /// the current block's `triples`/`paths`, and returns the blank node as a term
    /// for use in subject or object position.
    ///
    /// An empty `[]` (SPARQL ANON) is legal and simply mints a fresh blank node
    /// without any associated predicate-object pairs.
    fn parse_blank_node_property_list(&mut self, sink: &mut BlockSink) -> Result<TermPattern> {
        self.expect(&Token::LBracket)?;
        let node = TermPattern::BlankNode(self.fresh_anon());
        if !self.at(&Token::RBracket) {
            self.parse_predicate_object_list(&SubjectArgs::Term(node.clone()), sink)?;
        }
        self.expect(&Token::RBracket)?;
        Ok(node)
    }

    /// Parse an RDF collection `( n1 n2 … )` (RDF 1.1 §4.3, SPARQL §19.5
    /// `Collection`). Desugars to the standard `rdf:first`/`rdf:rest` blank-node
    /// chain terminated by `rdf:nil`, emitting those triples into the current
    /// block's `triples` and returning the HEAD node as a term for use in subject
    /// or object position. An empty list `()` is `rdf:nil` itself.
    ///
    /// Each element is a `GraphNode` — a plain term, a nested blank-node property
    /// list `[ … ]`, or a nested collection `( … )` — so the recursion mirrors the
    /// `parse_blank_node_property_list` object idiom.
    fn parse_collection(&mut self, sink: &mut BlockSink) -> Result<TermPattern> {
        self.expect(&Token::LParen)?;
        // The SPARQL grammar requires at least one node inside the parentheses, but
        // RDF's empty collection `()` is `rdf:nil`; accept it for robustness.
        if self.eat(&Token::RParen) {
            return Ok(TermPattern::NamedNode(NamedNode::new_unchecked(RDF_NIL)));
        }
        let first_pred = NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_FIRST));
        let rest_pred = NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_REST));
        let nil = TermPattern::NamedNode(NamedNode::new_unchecked(RDF_NIL));

        let head = TermPattern::BlankNode(self.fresh_anon());
        let mut node = head.clone();
        loop {
            let element = self.parse_graph_node(sink)?;
            sink.triples.push(TriplePattern {
                subject: node.clone(),
                predicate: first_pred.clone(),
                object: element,
            });
            if self.at(&Token::RParen) {
                // Last element: terminate the chain with rdf:nil.
                sink.triples.push(TriplePattern {
                    subject: node,
                    predicate: rest_pred,
                    object: nil,
                });
                break;
            }
            // Another element follows: link to a fresh tail node.
            let next = TermPattern::BlankNode(self.fresh_anon());
            sink.triples.push(TriplePattern {
                subject: node,
                predicate: rest_pred.clone(),
                object: next.clone(),
            });
            node = next;
        }
        self.expect(&Token::RParen)?;
        Ok(head)
    }

    /// Parse one `GraphNode` (collection element / object): a nested blank-node
    /// property list, a nested collection, or a plain term.
    fn parse_graph_node(&mut self, sink: &mut BlockSink) -> Result<TermPattern> {
        if self.at(&Token::LBracket) {
            self.parse_blank_node_property_list(sink)
        } else if self.at(&Token::LParen) {
            self.parse_collection(sink)
        } else if self.at(&Token::TripleOpen) {
            self.parse_triple_node(sink)
        } else {
            self.parse_term_pattern()
        }
    }

    /// Parse an RDF 1.2 triple node in a term position:
    ///
    /// * `<<( s p o )>>` — a **triple term** (a value), yielded directly; or
    /// * `<< s p o [~ reifier] >>` — a **reifying triple**, desugared to a
    ///   reifier `R` with `R rdf:reifies <<( s p o )>>` (R fresh unless given),
    ///   and `R` is the term.
    ///
    /// The inner `s`/`o` may themselves be triple nodes (nesting is supported).
    fn parse_triple_node(&mut self, sink: &mut BlockSink) -> Result<TermPattern> {
        self.expect(&Token::TripleOpen)?;
        let is_triple_term = self.eat(&Token::LParen);
        let inner = self.parse_inner_triple(sink)?;
        if is_triple_term {
            self.expect(&Token::RParen)?;
            self.expect(&Token::TripleClose)?;
            return Ok(TermPattern::Triple(Box::new(inner)));
        }
        // Reifying triple: optional `~ reifier`, else a fresh blank reifier.
        let reifier = if self.eat(&Token::Tilde) {
            self.parse_reifier_id()?
        } else {
            TermPattern::BlankNode(self.fresh_anon())
        };
        self.expect(&Token::TripleClose)?;
        self.emit_reifies(&reifier, &inner, &mut sink.triples);
        Ok(reifier)
    }

    /// Parse the `s p o` inside a `<< … >>` (reifying triple) / `<<( … )>>`
    /// (triple term), in **graph-pattern** position.
    ///
    /// One component parser serves the subject and the object of both spellings,
    /// because in a pattern the two positions carry the SAME production: a nested
    /// triple node is admissible in either. That is not laxity, it is the SPARQL 1.2
    /// grammar — `TripleTermSubject` includes `TripleTerm`, exactly as SPARQL 1.1's
    /// `VarOrTerm` includes a literal in subject position. The W3C SPARQL 1.2 suite
    /// pins it as two **positive** syntax tests
    /// (`syntax-triple-terms-positive/nested-tripleterm-02.rq`, whose second pattern
    /// is `<<( <<(?S :p :o )>> :r :z )>> :q 1`, and
    /// `syntax-triple-terms-positive/compound-tripleterm-subject.rq`), so refusing it
    /// here would be a non-conformance, not a tightening. A pattern is a *matcher*:
    /// one naming a term the RDF 1.2 term model cannot hold simply matches nothing.
    ///
    /// The term model is therefore enforced where a triple term becomes a **value**
    /// rather than a matcher, and those positions ARE separate parsers:
    ///
    /// * ground data (`VALUES`, `BIND` of a constant) —
    ///   [`parse_ground_triple`](Self::parse_ground_triple) refuses a literal or a
    ///   nested triple term in the subject, which is what the suite's
    ///   `tripleterm-subject-01`..`-06` **negative** syntax tests require;
    /// * expression position (`ExprTripleTerm`, §17.4) —
    ///   [`parse_triple_term_expr`](Self::parse_triple_term_expr) refuses the same;
    /// * a `CONSTRUCT` / `UPDATE` template instantiated per solution row, where a
    ///   variable can bind a triple term no syntax mentions, so no parser could
    ///   decide it: `purrdf-sparql-eval`'s `template::positionally_ill_formed` skips
    ///   the instantiation, which is what SPARQL §16.2 mandates for an ill-formed
    ///   instantiation (skip the statement, keep the rest of the template).
    ///
    /// Neither spelling admits an RDF collection or a populated blank-node property
    /// list in any position (both would emit auxiliary triples a single triple cannot
    /// carry).
    fn parse_inner_triple(&mut self, sink: &mut BlockSink) -> Result<TriplePattern> {
        let subject = self.parse_triple_node_component(sink)?;
        let predicate = self.parse_predicate_name()?;
        let object = self.parse_triple_node_component(sink)?;
        Ok(TriplePattern {
            subject,
            predicate,
            object,
        })
    }

    /// Parse one subject/object component of a triple node. A nested `<< … >>` /
    /// `<<( … )>>` is admissible, but an RDF collection `( … )` or a populated
    /// blank-node property list `[ p o … ]` is not (each would emit auxiliary
    /// triples a single triple cannot carry); only the anonymous `[]` (a fresh
    /// blank node) is.
    fn parse_triple_node_component(&mut self, sink: &mut BlockSink) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::TripleOpen) => self.parse_triple_node(sink),
            Some(Token::LParen) => Err(ParseError::syntax(
                "an RDF collection is not allowed inside a triple term or reifying triple",
                self.span(),
            )),
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                if !self.eat(&Token::RBracket) {
                    return Err(ParseError::syntax(
                        "a populated blank-node property list is not allowed inside a \
                         triple term or reifying triple",
                        self.span(),
                    ));
                }
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            _ => self.parse_term_pattern(),
        }
    }

    /// Emit `reifier rdf:reifies <<( t )>>` for a reification.
    fn emit_reifies(
        &self,
        reifier: &TermPattern,
        t: &TriplePattern,
        triples: &mut Vec<TriplePattern>,
    ) {
        triples.push(TriplePattern {
            subject: reifier.clone(),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_REIFIES)),
            object: TermPattern::Triple(Box::new(t.clone())),
        });
    }

    /// A reifier id after `~` (§ `Reifier ::= '~' VarOrReifierId?`): a variable,
    /// IRI, labelled blank node `_:b`, or anonymous `[]` — or, when none is
    /// present, a fresh blank node.
    fn parse_reifier_id(&mut self) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(TermPattern::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(TermPattern::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(l)) = self.bump() else {
                    unreachable!()
                };
                Ok(TermPattern::BlankNode(BlankNode::new(l)))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                self.expect(&Token::RBracket)?;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            _ => Ok(TermPattern::BlankNode(self.fresh_anon())),
        }
    }

    /// Parse RDF 1.2 annotation syntax trailing an asserted triple `(s, pred, o)`:
    /// zero or more reifiers `~ [id]` and annotation blocks `{| predObjList |}`.
    /// Each emits a fresh (or given) reifier `R` with `R rdf:reifies <<( s p o )>>`;
    /// an annotation block additionally applies its predicate-object list to `R`.
    fn parse_triple_annotations(
        &mut self,
        subject: &TermPattern,
        pred: &NamedNodePattern,
        object: &TermPattern,
        sink: &mut BlockSink,
    ) -> Result<()> {
        let base = TriplePattern {
            subject: subject.clone(),
            predicate: pred.clone(),
            object: object.clone(),
        };
        // An annotation block `{| … |}` binds to the reifier of the immediately
        // preceding `~ id` if one is pending (so `~ :r {| … |}` annotates `:r`
        // rather than a fresh node — important for DELETE templates, which forbid
        // blank nodes); otherwise it mints a fresh blank reifier.
        let mut pending: Option<TermPattern> = None;
        loop {
            if self.eat(&Token::Tilde) {
                let reifier = self.parse_reifier_id()?;
                self.emit_reifies(&reifier, &base, &mut sink.triples);
                pending = Some(reifier);
            } else if self.eat(&Token::AnnotationOpen) {
                let reifier = match pending.take() {
                    Some(r) => r,
                    None => {
                        let r = TermPattern::BlankNode(self.fresh_anon());
                        self.emit_reifies(&r, &base, &mut sink.triples);
                        r
                    }
                };
                self.parse_predicate_object_list(&SubjectArgs::Term(reifier), sink)?;
                self.expect(&Token::AnnotationClose)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// True when the next token starts a non-triples element of a group.
    fn block_boundary(&self) -> bool {
        self.at(&Token::LBrace)
            || self.peek_kw("OPTIONAL")
            || self.peek_kw("MINUS")
            || self.peek_kw("GRAPH")
            || self.peek_kw("SERVICE")
            || self.peek_kw("FILTER")
            || self.peek_kw("BIND")
            || self.peek_kw("VALUES")
            || self.peek_kw("LATERAL")
    }

    fn parse_predicate_object_list(
        &mut self,
        subject: &SubjectArgs,
        sink: &mut BlockSink,
    ) -> Result<()> {
        loop {
            // Verb = VarOrIri | path. A bare variable predicate is a simple
            // triple predicate, not a property path — and never a property
            // function, which is only ever a plain IRI.
            let verb = if let Some(Token::Variable(_)) = self.peek() {
                Verb::Simple(NamedNodePattern::Variable(self.expect_var()?))
            } else {
                let path = self.parse_path()?;
                match simple_predicate(&path) {
                    // A length-1 path is a plain predicate IRI, so it is the one
                    // shape that can name a property function; a complex path
                    // (`p+`, `p1/p2`, `!(…)`, …) never is.
                    Some(NamedNodePattern::NamedNode(n)) if self.is_property_fn(n.as_str()) => {
                        Verb::PropertyFn(n.as_str().to_owned())
                    }
                    Some(pred) => Verb::Simple(pred),
                    None => Verb::Path(path),
                }
            };
            // object list
            loop {
                match &verb {
                    Verb::PropertyFn(iri) => {
                        // Both sides are argument VECTORS, captured structurally:
                        // an object collection is the call's argument list, not a
                        // cons-cell chain, so `parse_graph_node` is bypassed here.
                        let object_args = self.parse_prop_fn_args()?;
                        let subject_args = subject.as_args();
                        sink.push_property_function(PropertyFunctionCall {
                            iri: iri.clone(),
                            subject_args,
                            object_args,
                        });
                        if self.at(&Token::Tilde) || self.at(&Token::AnnotationOpen) {
                            return Err(ParseError::syntax(
                                "RDF 1.2 annotation syntax cannot annotate a \
                                 property-function call (no triple is asserted)",
                                self.span(),
                            ));
                        }
                    }
                    Verb::Simple(pred) => {
                        let subject = subject.as_term(self.span())?;
                        // An object may itself be a blank-node property list
                        // `[ … ]` or an RDF collection `( … )` (both emit their
                        // own triples here).
                        let object = self.parse_graph_node(sink)?;
                        sink.triples.push(TriplePattern {
                            subject: subject.clone(),
                            predicate: pred.clone(),
                            object: object.clone(),
                        });
                        // RDF 1.2 annotation syntax (`~ reifier`, `{| … |}`) may
                        // trail the object, reifying the triple just asserted.
                        self.parse_triple_annotations(subject, pred, &object, sink)?;
                    }
                    Verb::Path(path) => {
                        let subject = subject.as_term(self.span())?;
                        let object = self.parse_graph_node(sink)?;
                        sink.paths.push(GraphPattern::Path {
                            subject: subject.clone(),
                            path: path.clone(),
                            object,
                        });
                    }
                }
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            if !self.eat(&Token::Semicolon) {
                break;
            }
            // allow a trailing `;` before `.`/`}`/`]` (the last closes a
            // blank-node property list).
            if self.at(&Token::Dot)
                || self.at(&Token::RBrace)
                || self.at(&Token::RBracket)
                || self.block_boundary()
            {
                break;
            }
        }
        Ok(())
    }

    // ── property paths (§18.1.7 / §9) ────────────────────────────────────────

    fn parse_path(&mut self) -> Result<PropertyPathExpression> {
        self.parse_path_alternative()
    }

    fn parse_path_alternative(&mut self) -> Result<PropertyPathExpression> {
        let mut left = self.parse_path_sequence()?;
        while self.eat(&Token::Pipe) {
            let right = self.parse_path_sequence()?;
            left = PropertyPathExpression::Alternative(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_path_sequence(&mut self) -> Result<PropertyPathExpression> {
        let mut left = self.parse_path_elt_or_inverse()?;
        while self.eat(&Token::Slash) {
            let right = self.parse_path_elt_or_inverse()?;
            left = PropertyPathExpression::Sequence(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_path_elt_or_inverse(&mut self) -> Result<PropertyPathExpression> {
        if self.eat(&Token::Caret) {
            Ok(PropertyPathExpression::Reverse(Box::new(
                self.parse_path_elt()?,
            )))
        } else {
            self.parse_path_elt()
        }
    }

    fn parse_path_elt(&mut self) -> Result<PropertyPathExpression> {
        let primary = self.parse_path_primary()?;
        Ok(match self.peek() {
            Some(Token::Star) => {
                self.pos += 1;
                PropertyPathExpression::ZeroOrMore(Box::new(primary))
            }
            Some(Token::Plus) => {
                self.pos += 1;
                PropertyPathExpression::OneOrMore(Box::new(primary))
            }
            Some(Token::Question) => {
                self.pos += 1;
                PropertyPathExpression::ZeroOrOne(Box::new(primary))
            }
            // `{n}` / `{n,}` / `{n,m}` / `{,m}` — bounded repetition (a PurRDF
            // extension beyond SPARQL 1.1 §9; symmetric parse for the serializer).
            Some(Token::LBrace) => self.parse_path_range(primary)?,
            _ => primary,
        })
    }

    /// Parse a bounded-repetition postfix `{n}` / `{n,}` / `{n,m}` / `{,m}` — a
    /// PurRDF extension beyond SPARQL 1.1 §9.  The opening `{` is the current token.
    /// Hard-fails (no silent degradation) on an empty `{}`, a non-integer bound,
    /// or a lower bound exceeding the upper bound.
    fn parse_path_range(
        &mut self,
        primary: PropertyPathExpression,
    ) -> Result<PropertyPathExpression> {
        self.expect(&Token::LBrace)?;
        let lower = self.eat_integer()?;
        let has_comma = self.eat(&Token::Comma);
        let upper = if has_comma { self.eat_integer()? } else { None };
        self.expect(&Token::RBrace)?;

        let (min, max) = if has_comma {
            // `{,}` — both bounds absent — is a silent-degrade to `*`; hard-fail instead.
            if lower.is_none() && upper.is_none() {
                return Err(ParseError::syntax(
                    "empty path range {,} is not allowed (use * for zero-or-more)",
                    self.span(),
                ));
            }
            // `{n,}` / `{n,m}` / `{,m}` (missing lower ⇒ 0).
            (lower.unwrap_or(0), upper)
        } else {
            // `{n}` ⇒ exactly n; an empty `{}` is invalid.
            match lower {
                Some(n) => (n, Some(n)),
                None => {
                    return Err(ParseError::syntax(
                        "empty path range {} is not allowed",
                        self.span(),
                    ));
                }
            }
        };
        if let Some(m) = max
            && min > m
        {
            return Err(ParseError::syntax(
                format!("path range lower bound {min} exceeds upper bound {m}"),
                self.span(),
            ));
        }
        Ok(PropertyPathExpression::Range {
            inner: Box::new(primary),
            min,
            max,
        })
    }

    /// Consume an `Integer` token and parse it to `u32`, returning `Ok(None)` when
    /// the current token is not an integer (so the caller can distinguish a missing
    /// bound from a present one).  An out-of-`u32`-range integer is a hard error.
    fn eat_integer(&mut self) -> Result<Option<u32>> {
        let Some(Token::Integer(lex)) = self.peek() else {
            return Ok(None);
        };
        let lex = *lex;
        match lex.parse::<u32>() {
            Ok(n) => {
                self.pos += 1;
                Ok(Some(n))
            }
            Err(_) => Err(ParseError::syntax(
                format!("path range bound {lex:?} is not a valid u32"),
                self.span(),
            )),
        }
    }

    fn parse_path_primary(&mut self) -> Result<PropertyPathExpression> {
        if self.peek_kw("a") && matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
            self.pos += 1;
            return Ok(PropertyPathExpression::NamedNode(NamedNode::new_unchecked(
                RDF_TYPE,
            )));
        }
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(PropertyPathExpression::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let inner = self.parse_path()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }
            Some(Token::Bang) => {
                self.pos += 1;
                self.parse_negated_property_set()
            }
            other => Err(ParseError::syntax(
                format!("expected a property path, found {other:?}"),
                self.span(),
            )),
        }
    }

    fn parse_negated_property_set(&mut self) -> Result<PropertyPathExpression> {
        let mut nodes = Vec::new();
        if self.eat(&Token::LParen) {
            loop {
                nodes.push(self.parse_path_one_in_set()?);
                if !self.eat(&Token::Pipe) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
        } else {
            nodes.push(self.parse_path_one_in_set()?);
        }
        Ok(PropertyPathExpression::NegatedPropertySet(nodes))
    }

    fn parse_path_one_in_set(&mut self) -> Result<NegatedPathElement> {
        // `^iri` — an inverse link inside a negated property set (SPARQL 1.1
        // §18.2 `PathOneInPropertySet`) — excludes a *reverse* hop rather than a
        // forward one; see `NegatedPathElement` and the evaluator's decomposition
        // into a forward/reverse `Alternative`.
        let inverse = self.eat(&Token::Caret);
        if matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
            self.pos += 1;
            return Ok(NegatedPathElement {
                predicate: NamedNode::new_unchecked(RDF_TYPE),
                inverse,
            });
        }
        let predicate = self.expect_iri_node()?;
        Ok(NegatedPathElement { predicate, inverse })
    }

    // ── terms ────────────────────────────────────────────────────────────────

    fn parse_term_pattern(&mut self) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(TermPattern::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(TermPattern::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(l)) = self.bump() else {
                    unreachable!()
                };
                Ok(TermPattern::BlankNode(BlankNode::new(l)))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            Some(
                Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_)
                | Token::Minus
                | Token::Plus,
            ) => Ok(TermPattern::Literal(self.parse_literal()?)),
            Some(Token::Word(w)) if *w == "true" || *w == "false" => {
                Ok(TermPattern::Literal(self.parse_literal()?))
            }
            Some(Token::TripleOpen) => {
                let t = self.parse_quoted_triple()?;
                Ok(TermPattern::Triple(Box::new(t)))
            }
            other => Err(ParseError::syntax(
                format!("expected an RDF term, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// `<<( s p o )>>` or `<< s p o >>` (RDF 1.2 quoted triple / triple term).
    fn parse_quoted_triple(&mut self) -> Result<TriplePattern> {
        self.expect(&Token::TripleOpen)?;
        let parens = self.eat(&Token::LParen);
        let subject = self.parse_term_pattern()?;
        let predicate = self.parse_predicate_name()?;
        let object = self.parse_term_pattern()?;
        if parens {
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::TripleClose)?;
        Ok(TriplePattern {
            subject,
            predicate,
            object,
        })
    }

    /// A predicate in a triple position: an IRI, `a`, or a variable.
    fn parse_predicate_name(&mut self) -> Result<NamedNodePattern> {
        if matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
            self.pos += 1;
            return Ok(NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                RDF_TYPE,
            )));
        }
        match self.peek() {
            Some(Token::Variable(_)) => Ok(NamedNodePattern::Variable(self.expect_var()?)),
            _ => Ok(NamedNodePattern::NamedNode(self.expect_iri_node()?)),
        }
    }

    fn parse_var_or_iri_name(&mut self) -> Result<NamedNodePattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(NamedNodePattern::Variable(self.expect_var()?)),
            _ => Ok(NamedNodePattern::NamedNode(self.expect_iri_node()?)),
        }
    }

    /// Parse any SPARQL literal: a numeral (optionally signed), a boolean, or a
    /// string (optionally `@lang`- or `^^`-typed) — the full `RDFLiteral |
    /// BooleanLiteral | NumericLiteral` production, not just its unsigned-numeral
    /// subset.
    ///
    /// A leading `+`/`-` folds into the numeral here: SPARQL's
    /// `NumericLiteralPositive`/`NumericLiteralNegative` productions tokenize as a
    /// single unit, but this lexer emits the sign as its own
    /// [`Token::Plus`]/[`Token::Minus`] — the shape `UnaryExpression` needs to
    /// distinguish `-?x` from a signed numeral. Every call site reached through
    /// this function (a triple pattern's object, `VALUES`' ground terms, an `AGG`
    /// scalarval) has no unary operator standing between the sign and the
    /// numeral, so the sign is folded back into the literal's lexical form
    /// instead of being left for a caller that does not exist at these
    /// positions. [`Self::parse_primary_with_aggs`] is the one call site where a
    /// leading sign IS a unary operator ([`Self::parse_unary`] consumes it
    /// first), so this function never observes one there.
    fn parse_literal(&mut self) -> Result<Literal> {
        let sign = match self.peek() {
            Some(Token::Minus) => {
                self.pos += 1;
                Some("-")
            }
            Some(Token::Plus) => {
                self.pos += 1;
                Some("+")
            }
            _ => None,
        };
        let signed = |s: &str| match sign {
            Some(sign) => format!("{sign}{s}"),
            None => s.to_owned(),
        };
        match self.bump() {
            Some(Token::Integer(s)) => Ok(Literal::new_typed(
                signed(s),
                NamedNode::new_unchecked(XSD_INTEGER),
            )),
            Some(Token::Decimal(s)) => Ok(Literal::new_typed(
                signed(s),
                NamedNode::new_unchecked(XSD_DECIMAL),
            )),
            Some(Token::Double(s)) => Ok(Literal::new_typed(
                signed(s),
                NamedNode::new_unchecked(XSD_DOUBLE),
            )),
            Some(Token::Word(w)) if sign.is_none() && (w == "true" || w == "false") => {
                Ok(Literal::new_typed(w, NamedNode::new_unchecked(XSD_BOOLEAN)))
            }
            Some(Token::StringLit(s) | Token::LongStringLit(s)) if sign.is_none() => {
                if let Some(Token::LangTag(_)) = self.peek() {
                    let Some(Token::LangTag(tag)) = self.bump() else {
                        unreachable!()
                    };
                    let (lang, dir) = split_lang_dir(tag);
                    Ok(Literal::new_lang(s, lang, dir))
                } else if self.eat(&Token::HatHat) {
                    let dt = self.expect_iri_node()?;
                    Ok(Literal::new_typed(s, dt))
                } else {
                    Ok(Literal::new_simple(s))
                }
            }
            other => Err(ParseError::syntax(
                if sign.is_some() {
                    format!("expected a numeral after the sign, found {other:?}")
                } else {
                    format!("expected a literal, found {other:?}")
                },
                self.span(),
            )),
        }
    }

    fn expect_var(&mut self) -> Result<Variable> {
        match self.bump() {
            Some(Token::Variable(n)) => Ok(Variable::new(n)),
            other => Err(ParseError::syntax(
                format!("expected a variable, found {other:?}"),
                self.span(),
            )),
        }
    }

    fn expect_iri_node(&mut self) -> Result<NamedNode> {
        match self.bump() {
            Some(Token::Iri(s)) => NamedNode::new(self.resolve_iri(&s)?),
            Some(Token::PrefixedName(p, l)) => self.resolve_prefixed(p, l.as_ref()),
            other => Err(ParseError::syntax(
                format!("expected an IRI, found {other:?}"),
                self.span(),
            )),
        }
    }

    // ── VALUES / inline data ─────────────────────────────────────────────────

    fn parse_inline_data(&mut self) -> Result<GraphPattern> {
        self.expect_kw("VALUES")?;
        let mut variables = Vec::new();
        let mut bindings = Vec::new();
        if self.eat(&Token::LParen) {
            // VALUES ( ?a ?b ) { ( v v ) ... }
            while let Some(Token::Variable(_)) = self.peek() {
                let v = self.expect_var()?;
                if variables.contains(&v) {
                    return Err(ParseError::syntax(
                        format!("duplicate variable ?{} in VALUES clause", v.as_str()),
                        self.span(),
                    ));
                }
                variables.push(v);
            }
            self.expect(&Token::RParen)?;
            self.expect(&Token::LBrace)?;
            while self.eat(&Token::LParen) {
                let mut row = Vec::new();
                while !self.at(&Token::RParen) {
                    row.push(self.parse_data_cell()?);
                }
                self.expect(&Token::RParen)?;
                if row.len() != variables.len() {
                    return Err(ParseError::syntax(
                        format!(
                            "VALUES row has {} cells for {} variable(s)",
                            row.len(),
                            variables.len()
                        ),
                        self.span(),
                    ));
                }
                bindings.push(row);
            }
        } else {
            // VALUES ?a { v v ... }
            variables.push(self.expect_var()?);
            self.expect(&Token::LBrace)?;
            while !self.at(&Token::RBrace) {
                bindings.push(vec![self.parse_data_cell()?]);
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(GraphPattern::Values {
            variables,
            bindings,
        })
    }

    fn parse_data_cell(&mut self) -> Result<Option<GroundTerm>> {
        if matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case("UNDEF")) {
            self.pos += 1;
            return Ok(None);
        }
        Ok(Some(self.parse_ground_term()?))
    }

    fn parse_ground_term(&mut self) -> Result<GroundTerm> {
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(GroundTerm::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::TripleOpen) => {
                let t = self.parse_ground_triple()?;
                Ok(GroundTerm::Triple(Box::new(t)))
            }
            // Every other legal ground term — a string, a boolean, or a numeral
            // (optionally signed: `-1`, `+0.5`) — is `parse_literal`'s grammar;
            // an illegal token surfaces through its own catch-all error.
            _ => Ok(GroundTerm::Literal(self.parse_literal()?)),
        }
    }

    fn parse_ground_triple(&mut self) -> Result<GroundTriple> {
        self.expect(&Token::TripleOpen)?;
        let parens = self.eat(&Token::LParen);
        let subject = self.parse_ground_term()?;
        // A ground triple term's subject is an `iri | BlankNode` — never a literal
        // or a nested triple term (only the *object* may nest).
        if matches!(subject, GroundTerm::Triple(_) | GroundTerm::Literal(_)) {
            return Err(ParseError::syntax(
                "a literal or nested triple term may not be the subject of a triple term",
                self.span(),
            ));
        }
        // The predicate is an IRI or the `a` keyword (rdf:type).
        let predicate = if matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
            self.pos += 1;
            NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        } else {
            self.expect_iri_node()?
        };
        let object = self.parse_ground_term()?;
        if parens {
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::TripleClose)?;
        Ok(GroundTriple {
            subject,
            predicate,
            object,
        })
    }

    // ── solution modifiers ───────────────────────────────────────────────────

    /// True when the cursor is at a bare (non-parenthesized) `GROUP BY`
    /// GroupCondition — a `BuiltInCall` or `FunctionCall`. The grammar's bare
    /// conditions all begin with a callee token (a builtin keyword, an IRI, or a
    /// prefixed name); the modifier-list terminators (`HAVING`/`ORDER`/`LIMIT`/
    /// `OFFSET`/`VALUES`) and boolean literals are excluded so the `GROUP BY`
    /// loop stops cleanly at the next clause.
    fn at_bare_group_condition(&self) -> bool {
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => true,
            Some(Token::Word(w)) => !matches!(
                w.to_ascii_uppercase().as_str(),
                "HAVING" | "ORDER" | "LIMIT" | "OFFSET" | "VALUES" | "BINDINGS" | "TRUE" | "FALSE"
            ),
            _ => false,
        }
    }

    /// True when the upcoming token can start a bare (non-parenthesized)
    /// `Constraint` — a `BuiltInCall` or `FunctionCall` (SPARQL 1.1/1.2
    /// `Constraint ::= BrackettedExpression | BuiltInCall | FunctionCall`).
    /// Used by `HAVING`'s `Constraint+` list (both to decide whether the
    /// first, mandatory constraint is bare, and whether a SUBSEQUENT one
    /// begins) and by `ORDER BY`'s `OrderCondition ::= ... | (Constraint |
    /// Var)` alternative. The bracketed form (`Token::LParen`) is recognized
    /// separately at each call site — this only covers the bare spelling, so
    /// it deliberately excludes a bare `Var` or literal (neither is a
    /// `Constraint`, only an `OrderCondition`'s OTHER alternative or a
    /// non-constraint primary expression).
    ///
    /// Same shape as [`Self::at_bare_group_condition`] (a callee token —
    /// IRI/prefixed name/keyword — modulo the clause-terminator words that
    /// can legally follow a `Constraint+`/`OrderCondition*` list), kept
    /// separate because the terminator set differs slightly (`HAVING` can
    /// recur inside itself as a callee-shaped word is never in question
    /// here, since `HAVING` itself cannot re-appear mid-list, but excluding
    /// it is harmless and keeps the two helpers independently auditable).
    fn at_bare_constraint(&self) -> bool {
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => true,
            Some(Token::Word(w)) => !matches!(
                w.to_ascii_uppercase().as_str(),
                "HAVING" | "ORDER" | "LIMIT" | "OFFSET" | "VALUES" | "BINDINGS" | "TRUE" | "FALSE"
            ),
            _ => false,
        }
    }

    fn parse_solution_modifiers(
        &mut self,
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Modifiers> {
        let mut m = Modifiers::default();
        if self.eat_kw("GROUP") {
            self.expect_kw("BY")?;
            loop {
                if let Some(Token::Variable(_)) = self.peek() {
                    m.group_by.push(self.expect_var()?);
                } else if self.at(&Token::LParen) {
                    // `( Expr [AS ?v] )` — SPARQL 1.1 §18.2.4 GroupCondition. Lower
                    // to an Extend(?v := Expr) under the Group, then group by ?v.
                    self.expect(&Token::LParen)?;
                    // Non-lifting parse: an aggregate in a GROUP BY key is illegal
                    // and surfaces here as `Unsupported`.
                    let expr = self.parse_expression()?;
                    let var = if self.eat_kw("AS") {
                        self.expect_var()?
                    } else {
                        self.fresh_group_var()
                    };
                    self.expect(&Token::RParen)?;
                    // An expression-valued `GROUP BY` condition list lowers to
                    // a chain of `Extend` nodes placed directly under `Group`
                    // (§18.2.4) — see `MAX_GRAPH_PATTERN_NODES`'s doc for why
                    // this list is charged the same as the group loop.
                    self.charge_pattern_nodes(1)?;
                    m.group_extends.push((var.clone(), expr));
                    m.group_by.push(var);
                } else if self.at_bare_group_condition() {
                    // A bare `BuiltInCall` / `FunctionCall` GroupCondition, e.g.
                    // `GROUP BY STR(?x)` — lower to a synthetic-var Extend.
                    let expr = self.parse_expression()?;
                    let var = self.fresh_group_var();
                    self.charge_pattern_nodes(1)?;
                    m.group_extends.push((var.clone(), expr));
                    m.group_by.push(var);
                } else {
                    break;
                }
            }
        }
        if self.eat_kw("HAVING") {
            loop {
                let expr = self.parse_having_constraint(aggregates)?;
                // A long `HAVING (c1) (c2) … (cN)` condition list lowers to a
                // chain of `Filter` nodes — same class, same budget. Each
                // `cN` is itself a `Constraint` (`BrackettedExpression |
                // BuiltInCall | FunctionCall`, §Constraint) — bracketed
                // (`Token::LParen`) or bare (`Self::at_bare_constraint`).
                self.charge_pattern_nodes(1)?;
                m.having.push(expr);
                if !(self.at(&Token::LParen) || self.at_bare_constraint()) {
                    break;
                }
            }
        }
        if self.eat_kw("ORDER") {
            self.expect_kw("BY")?;
            loop {
                let cond = if self.eat_kw("ASC") {
                    self.expect(&Token::LParen)?;
                    let e = self.parse_expression_lifting_aggs(aggregates)?;
                    self.expect(&Token::RParen)?;
                    OrderExpression::Asc(e)
                } else if self.eat_kw("DESC") {
                    self.expect(&Token::LParen)?;
                    let e = self.parse_expression_lifting_aggs(aggregates)?;
                    self.expect(&Token::RParen)?;
                    OrderExpression::Desc(e)
                } else if self.order_key_ahead() {
                    OrderExpression::Asc(self.parse_primary_with_aggs(aggregates)?)
                } else {
                    break;
                };
                m.order_by.push(cond);
            }
        }
        // LIMIT / OFFSET in either order.
        loop {
            if self.eat_kw("LIMIT") {
                m.limit = Some(self.expect_integer()?);
            } else if self.eat_kw("OFFSET") {
                m.offset = Some(self.expect_integer()?);
            } else {
                break;
            }
        }
        Ok(m)
    }

    /// True when an `OrderCondition`'s bare alternative starts here —
    /// `Constraint | Var` (SPARQL 1.1/1.2 §OrderCondition), where a bare
    /// `Constraint` is `Token::LParen` (`BrackettedExpression`) or
    /// [`Self::at_bare_constraint`] (`BuiltInCall`/`FunctionCall`, e.g.
    /// `BOUND(?x)`, `EXISTS { ... }`, or a `FunctionCall` under an IRI/
    /// prefixed-name callee).
    fn order_key_ahead(&self) -> bool {
        matches!(self.peek(), Some(Token::Variable(_) | Token::LParen)) || self.at_bare_constraint()
    }

    fn expect_integer(&mut self) -> Result<usize> {
        match self.bump() {
            Some(Token::Integer(s)) => s
                .parse::<usize>()
                .map_err(|_| ParseError::syntax(format!("bad integer {s:?}"), self.span())),
            other => Err(ParseError::syntax(
                format!("expected an integer, found {other:?}"),
                self.span(),
            )),
        }
    }

    // ── expressions ──────────────────────────────────────────────────────────

    /// FILTER constraint: a bracketted expression, a built-in call, or a
    /// function call (§ Constraint).
    fn parse_constraint(&mut self) -> Result<Expression> {
        if self.at(&Token::LParen) {
            self.pos += 1;
            let e = self.parse_expression()?;
            self.expect(&Token::RParen)?;
            Ok(e)
        } else {
            self.parse_primary_expression()
        }
    }

    /// One element of `HAVING`'s `Constraint+` list (§Constraint):
    /// `BrackettedExpression | BuiltInCall | FunctionCall`. The bracketed
    /// form is `'(' Expression ')'`; the bare forms (`BOUND(?x)`,
    /// `EXISTS { ... }`, a custom `FunctionCall`, …) are parsed exactly like
    /// any other primary expression, with aggregates lifted into `aggs` —
    /// `HAVING (COUNT(?x) > 1)` needs that lift for the bracketed form, and a
    /// bare aggregate `BuiltInCall` (e.g. `HAVING COUNT(?x)`, unusual but
    /// grammar-legal) needs the same treatment.
    fn parse_having_constraint(
        &mut self,
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        if self.at(&Token::LParen) {
            self.pos += 1;
            let e = self.parse_expression_lifting_aggs(aggregates)?;
            self.expect(&Token::RParen)?;
            Ok(e)
        } else {
            self.parse_primary_with_aggs(aggregates)
        }
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        let mut sink = Vec::new();
        let e = self.parse_or(&mut sink)?;
        if !sink.is_empty() {
            return Err(ParseError::unsupported(
                "aggregate outside GROUP BY / SELECT / HAVING context",
            ));
        }
        Ok(e)
    }

    fn parse_expression_lifting_aggs(
        &mut self,
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.parse_or(aggregates)
    }

    fn parse_or(&mut self, aggs: &mut Vec<(Variable, AggregateExpression)>) -> Result<Expression> {
        let mut left = self.parse_and(aggs)?;
        while self.eat(&Token::Or) {
            let right = self.parse_and(aggs)?;
            left = Expression::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self, aggs: &mut Vec<(Variable, AggregateExpression)>) -> Result<Expression> {
        let mut left = self.parse_relational(aggs)?;
        while self.eat(&Token::And) {
            let right = self.parse_relational(aggs)?;
            left = Expression::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_relational(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let left = self.parse_additive(aggs)?;
        let op = match self.peek() {
            Some(Token::Eq) => Some("="),
            Some(Token::NotEq) => Some("!="),
            Some(Token::Lt) => Some("<"),
            Some(Token::Gt) => Some(">"),
            Some(Token::LtEq) => Some("<="),
            Some(Token::GtEq) => Some(">="),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let right = self.parse_additive(aggs)?;
            let (l, r) = (Box::new(left), Box::new(right));
            return Ok(match op {
                "=" => Expression::Equal(l, r),
                "!=" => Expression::Not(Box::new(Expression::Equal(l, r))),
                "<" => Expression::Less(l, r),
                ">" => Expression::Greater(l, r),
                "<=" => Expression::LessOrEqual(l, r),
                _ => Expression::GreaterOrEqual(l, r),
            });
        }
        if self.peek_kw("IN") {
            self.pos += 1;
            let list = self.parse_expression_list(aggs)?;
            return Ok(Expression::In(Box::new(left), list));
        }
        if self.peek_kw("NOT") && self.peek2_kw("IN") {
            self.pos += 2;
            let list = self.parse_expression_list(aggs)?;
            return Ok(Expression::Not(Box::new(Expression::In(
                Box::new(left),
                list,
            ))));
        }
        Ok(left)
    }

    fn parse_additive(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let mut left = self.parse_multiplicative(aggs)?;
        loop {
            if self.eat(&Token::Plus) {
                let right = self.parse_multiplicative(aggs)?;
                left = Expression::Add(Box::new(left), Box::new(right));
            } else if self.eat(&Token::Minus) {
                let right = self.parse_multiplicative(aggs)?;
                left = Expression::Subtract(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let mut left = self.parse_unary(aggs)?;
        loop {
            if self.eat(&Token::Star) {
                let right = self.parse_unary(aggs)?;
                left = Expression::Multiply(Box::new(left), Box::new(right));
            } else if self.eat(&Token::Slash) {
                let right = self.parse_unary(aggs)?;
                left = Expression::Divide(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        if self.eat(&Token::Bang) {
            Ok(Expression::Not(Box::new(self.parse_unary(aggs)?)))
        } else if self.eat(&Token::Plus) {
            Ok(Expression::UnaryPlus(Box::new(self.parse_unary(aggs)?)))
        } else if self.eat(&Token::Minus) {
            Ok(Expression::UnaryMinus(Box::new(self.parse_unary(aggs)?)))
        } else {
            self.parse_primary_with_aggs(aggs)
        }
    }

    /// Parse an RDF 1.2 triple term `<<( s p o )>>` in *expression* position
    /// (`ExprTripleTerm`, §17.4). It denotes the same value as `TRIPLE(s, p, o)`,
    /// so it lowers to that function call. Only the triple-*term* form (`<<(`) is
    /// valid here — a reifying triple `<< … >>` is not an expression.
    fn parse_triple_term_expr(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.expect(&Token::TripleOpen)?;
        if !self.eat(&Token::LParen) {
            return Err(ParseError::syntax(
                "a reifying triple `<< … >>` is not valid in expression position; \
                 use a triple term `<<( s p o )>>`",
                self.span(),
            ));
        }
        // A triple term's subject is a `Var | iri` here — never a literal or a
        // nested triple term.
        if matches!(
            self.peek(),
            Some(
                Token::TripleOpen
                    | Token::StringLit(_)
                    | Token::LongStringLit(_)
                    | Token::Integer(_)
                    | Token::Decimal(_)
                    | Token::Double(_)
            )
        ) {
            return Err(ParseError::syntax(
                "a literal or nested triple term may not be the subject of a triple term",
                self.span(),
            ));
        }
        let s = self.parse_primary_with_aggs(aggs)?;
        let p = self.parse_primary_with_aggs(aggs)?;
        let o = self.parse_primary_with_aggs(aggs)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Expression::FunctionCall(Function::Triple, vec![s, p, o]))
    }

    fn parse_primary_expression(&mut self) -> Result<Expression> {
        let mut sink = Vec::new();
        let e = self.parse_primary_with_aggs(&mut sink)?;
        if !sink.is_empty() {
            return Err(ParseError::unsupported("aggregate in this position"));
        }
        Ok(e)
    }

    fn parse_primary_with_aggs(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        match self.peek() {
            Some(Token::LParen) => {
                self.pos += 1;
                let e = self.parse_or(aggs)?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }
            Some(Token::Variable(_)) => Ok(Expression::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => self.parse_iri_or_function(aggs),
            Some(
                Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_),
            ) => Ok(Expression::Literal(self.parse_literal()?)),
            Some(Token::TripleOpen) => self.parse_triple_term_expr(aggs),
            Some(Token::Word(w)) => {
                let w = *w;
                if w == "true" || w == "false" {
                    // No sign precedes a bare boolean word here: `parse_unary` already
                    // intercepts a leading `+`/`-` before a bare boolean word is ever
                    // reached, so `parse_literal` observes none and takes its boolean arm.
                    Ok(Expression::Literal(self.parse_literal()?))
                } else {
                    self.parse_builtin_or_aggregate(w, aggs)
                }
            }
            other => Err(ParseError::syntax(
                format!("expected an expression, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// Human-readable spelling of a SEP-0009 signature, for
    /// [`ParseError::CdtArity`].
    fn describe_cdt_arity(arity: crate::algebra::CdtArity) -> String {
        use crate::algebra::CdtArity;
        match arity {
            CdtArity::Fixed(1) => "exactly 1 argument".to_owned(),
            CdtArity::Fixed(n) => format!("exactly {n} arguments"),
            CdtArity::Range { min, max } => format!("{min} to {max} arguments"),
            CdtArity::AtLeast(0) => "any number of arguments".to_owned(),
            CdtArity::AtLeast(min) => format!("at least {min} arguments"),
            CdtArity::Pairs => "an even number of arguments (key/value pairs)".to_owned(),
        }
    }

    fn parse_iri_or_function(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let node = self.expect_iri_node()?;
        if self.at(&Token::LParen) {
            // A SEP-0009 composite-datatype function, by EXACT IRI match against the
            // closed `CdtFn` registry. Checked FIRST and UNCONDITIONALLY: the spec
            // fixes both the namespace and the local names, so there is no
            // `ParserOptions` seam here and a configured extension namespace can
            // never shadow one of these. Recognizing a spec-defined third-party IRI
            // is not minting it — see `CdtCall`'s own docs.
            if let Some(fn_kind) = crate::algebra::CdtFn::from_iri(node.as_str()) {
                let at = self.span();
                let iri = node.as_str().to_owned();
                let args = self.parse_arg_list(aggs)?;
                // SPARQL has no overloading on argument count, so a wrong-arity call
                // can never evaluate to anything and is refused here rather than
                // silently becoming an expression error at runtime.
                if !fn_kind.arity().admits(args.len()) {
                    return Err(ParseError::CdtArity {
                        iri,
                        expected: Self::describe_cdt_arity(fn_kind.arity()),
                        found: args.len(),
                        at,
                    });
                }
                return Ok(Expression::FunctionCall(
                    Function::Cdt(crate::algebra::CdtCall { fn_kind, iri }),
                    args,
                ));
            }
            // An IRI in call position under ANY configured extension-function namespace
            // (default: NONE — the namespace set is caller configuration supplied via
            // ParserOptions, e.g. the gmeow namespace) dispatches to the CLOSED
            // extension-function seam, recognized here at parse time. The local-name
            // MUST resolve; an unknown <ns>foo(...) under a configured namespace is a
            // hard error (fail-fast), never a silent Function::Custom fallthrough. An
            // IRI under NO configured namespace stays Function::Custom. The original
            // IRI is recorded in the AST node so serialization round-trips exactly.
            let ext_local = self
                .options
                .extension_fn_namespaces
                .iter()
                .find_map(|ns| node.as_str().strip_prefix(ns.as_str()));
            let func = if let Some(local) = ext_local {
                match crate::algebra::PurrdfFn::from_local_name(local) {
                    Some(fn_kind) => Function::Purrdf(crate::algebra::PurrdfCall {
                        fn_kind,
                        iri: node.as_str().to_owned(),
                    }),
                    None => {
                        return Err(ParseError::syntax(
                            format!("unknown extension function <{}>", node.as_str()),
                            self.span(),
                        ));
                    }
                }
            } else {
                Function::Custom(node)
            };
            let args = self.parse_arg_list(aggs)?;
            Ok(Expression::FunctionCall(func, args))
        } else {
            Ok(Expression::NamedNode(node))
        }
    }

    fn parse_builtin_or_aggregate(
        &mut self,
        name: &str,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let upper = name.to_ascii_uppercase();
        // Aggregates lift to a synthetic Group variable.
        if let Some(func) = aggregate_function(&upper) {
            return self.parse_aggregate(func, &upper, aggs);
        }
        // `AGG(<iri>, [DISTINCT] arg, arg, …)` — the custom-aggregate surface;
        // also lifts to a synthetic Group variable, exactly like a named built-in
        // aggregate. Checked here (rather than added to `aggregate_function`)
        // because it does not follow the `NAME(...)` dispatch table shape: its
        // first token inside the parens is an IRI, not an expression.
        if upper == "AGG" {
            return self.parse_agg_call(aggs);
        }
        match upper.as_str() {
            "BOUND" => {
                self.pos += 1;
                self.expect(&Token::LParen)?;
                let v = self.expect_var()?;
                self.expect(&Token::RParen)?;
                Ok(Expression::Bound(v))
            }
            "IF" => {
                self.pos += 1;
                let args = self.parse_arg_list(aggs)?;
                expect_arity(&args, 3, "IF", self.span())?;
                let mut it = args.into_iter();
                Ok(Expression::If(
                    Box::new(it.next().unwrap()),
                    Box::new(it.next().unwrap()),
                    Box::new(it.next().unwrap()),
                ))
            }
            "COALESCE" => {
                self.pos += 1;
                Ok(Expression::Coalesce(self.parse_arg_list(aggs)?))
            }
            "EXISTS" => {
                self.pos += 1;
                Ok(Expression::Exists(Box::new(self.parse_exists_body()?)))
            }
            "NOT" => {
                self.pos += 1;
                self.expect_kw("EXISTS")?;
                Ok(Expression::Not(Box::new(Expression::Exists(Box::new(
                    self.parse_exists_body()?,
                )))))
            }
            "SAMETERM" => {
                self.pos += 1;
                let args = self.parse_arg_list(aggs)?;
                expect_arity(&args, 2, "sameTerm", self.span())?;
                let mut it = args.into_iter();
                Ok(Expression::SameTerm(
                    Box::new(it.next().unwrap()),
                    Box::new(it.next().unwrap()),
                ))
            }
            _ => {
                if let Some(func) = builtin_function(&upper) {
                    self.pos += 1;
                    let args = self.parse_arg_list(aggs)?;
                    // `builtin_function`'s generic dispatch path does not arity-check;
                    // ADJUST(value, timezone) is fixed at 2 (SEP-0002's sole documented
                    // signature — see the `Function::Adjust` rustdoc).
                    if func == Function::Adjust {
                        expect_arity(&args, 2, "ADJUST", self.span())?;
                    }
                    Ok(Expression::FunctionCall(func, args))
                } else {
                    Err(ParseError::unsupported(format!(
                        "function or keyword {name}"
                    )))
                }
            }
        }
    }

    fn parse_aggregate(
        &mut self,
        func: AggregateFunction,
        name: &str,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.pos += 1; // function name
        self.expect(&Token::LParen)?;
        // DISTINCT precedes `*` in `COUNT(DISTINCT *)`; consume it first so the
        // star form carries the flag (an earlier shape hid the DISTINCT behind a
        // separate CountStar variant, making `distinct: true` on the star form
        // unreachable).
        let distinct = self.eat_kw("DISTINCT");
        let agg = if self.eat(&Token::Star) {
            // `*` is the spec's empty exprlist, and the grammar admits it in
            // exactly one production: `Count` (SPARQL 1.1 §18.5.1 / SPARQL 1.2
            // §19.8). `SUM(*)`/`AVG(*)`/`MIN(*)`/`MAX(*)`/`SAMPLE(*)`/
            // `GROUP_CONCAT(*)` — and, symmetrically, a zero-arity custom
            // aggregate — are hard syntax errors, never a silent row count.
            if func != AggregateFunction::Count {
                return Err(ParseError::syntax(
                    format!(
                        "`*` is only valid inside COUNT(...); {name} does not accept an empty \
                         exprlist"
                    ),
                    self.span(),
                ));
            }
            AggregateExpression::new(func, Vec::new(), Vec::new(), distinct)
                .expect("COUNT accepts an empty exprlist")
        } else {
            // Marks any `EXISTS` reached while parsing THIS argument as
            // `ExistsScopeBasis::AggregateArgument` if it is later deferred
            // under `Parser::projection_scope_pending` (a SELECT-list
            // aggregate, e.g. `SUM(IF(EXISTS { ... }, 1, 0))`) — irrelevant,
            // and harmless, outside that window (`GROUP BY`/`HAVING`
            // aggregate arguments check immediately either way). Restored
            // right after: aggregates cannot themselves nest, but the
            // `EXISTS` body this argument may contain can embed a
            // sub-`SELECT` with its own, unrelated aggregate arguments.
            let saved_in_aggregate_argument = self.in_aggregate_argument;
            self.in_aggregate_argument = true;
            let inner = self.parse_expression();
            self.in_aggregate_argument = saved_in_aggregate_argument;
            let inner = inner?;
            if matches!(func, AggregateFunction::Fold) {
                // SEP-0009 `[127+]`: `'FOLD' '(' 'DISTINCT'? Expression
                // ( ',' Expression )? ( 'ORDER' 'BY' OrderCondition+ )? ')'`.
                // The first `Expression` is already parsed above (the shared
                // `in_aggregate_argument` window); the optional SECOND one and
                // the optional `ORDER BY` tail are FOLD's alone.
                let mut args = vec![inner];
                if self.eat(&Token::Comma) {
                    let saved = self.in_aggregate_argument;
                    self.in_aggregate_argument = true;
                    let second = self.parse_expression();
                    self.in_aggregate_argument = saved;
                    args.push(second?);
                }
                let order_by = self.parse_fold_order_conditions()?;
                AggregateExpression::new_fold(args, order_by, distinct)
                    .expect("FOLD parses exactly one or two arguments")
            } else {
                let mut scalarvals = Vec::new();
                if matches!(func, AggregateFunction::GroupConcat)
                    && let Some(sep) = self.parse_optional_separator()?
                {
                    scalarvals.push(("separator".to_owned(), Literal::new_simple(sep)));
                }
                AggregateExpression::new(func, vec![inner], scalarvals, distinct)
                    .expect("a one-element args list is always a valid AggregateExpression")
            }
        };
        self.expect(&Token::RParen)?;
        let synth = self.fresh_agg_var();
        aggs.push((synth.clone(), agg));
        Ok(Expression::Variable(synth))
    }

    /// Parse the `AGG(<iri>, [DISTINCT] arg, arg, … [; NAME=value]*)`
    /// custom-aggregate surface (the normative spelling for a custom-aggregate
    /// call — no `ParserOptions` gate, since it introduces no ambiguity with any
    /// other production). `<iri>` may be any IRI, including a prefixed name, resolved
    /// and retained byte-exact via [`Self::expect_iri_node`]. `DISTINCT`, if
    /// present, precedes the first positional argument. At least one positional
    /// argument is required — an empty argument list is a hard syntax error: the
    /// positional surface is one or more arguments (there is no `AGG(<iri>)`
    /// zero-arity form).
    ///
    /// After the positional arguments, zero or more trailing `; NAME=value`
    /// scalarval clauses are admitted — see [`Self::parse_agg_scalarvals`] for
    /// the grammar and its precedent. This is a purely STRUCTURAL parse: any
    /// `NAME` is accepted here, and any literal `value`; whether a given custom
    /// aggregate accepts a given name (and whether its value's type is right) is
    /// validated at prepare time by the evaluator, against the registered
    /// aggregate's own declaration — never by this parser.
    fn parse_agg_call(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.pos += 1; // `AGG`
        self.expect(&Token::LParen)?;
        let iri = self.expect_iri_node()?;
        self.expect(&Token::Comma)?;
        let distinct = self.eat_kw("DISTINCT");
        let mut args = Vec::new();
        // See the matching comment in `Parser::parse_aggregate`: marks any
        // `EXISTS` reached while parsing these positional arguments as
        // `ExistsScopeBasis::AggregateArgument`, restored once the whole
        // argument list is parsed (or the first one fails).
        let saved_in_aggregate_argument = self.in_aggregate_argument;
        self.in_aggregate_argument = true;
        let mut args_err = None;
        loop {
            match self.parse_expression() {
                Ok(e) => args.push(e),
                Err(e) => {
                    args_err = Some(e);
                    break;
                }
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.in_aggregate_argument = saved_in_aggregate_argument;
        if let Some(e) = args_err {
            return Err(e);
        }
        let scalarvals = self.parse_agg_scalarvals()?;
        self.expect(&Token::RParen)?;
        let agg =
            AggregateExpression::new(AggregateFunction::Custom(iri), args, scalarvals, distinct)
                .expect("the `args.push` loop above always runs at least once");
        let synth = self.fresh_agg_var();
        aggs.push((synth.clone(), agg));
        Ok(Expression::Variable(synth))
    }

    /// Parse SEP-0009 `FOLD`'s optional trailing
    /// `( 'ORDER' 'BY' OrderCondition+ )?` clause — the spec's `OrderGroups`
    /// symbol, which sorts each GROUP's solution sequence before the fold runs.
    ///
    /// The `OrderCondition` alternatives are exactly the query-level ones
    /// ([`Self::parse_solution_modifiers`]'s `ORDER BY` loop): `ASC(expr)`,
    /// `DESC(expr)`, or a bare `Constraint | Var`. Two things differ, both
    /// forced by where the clause sits:
    ///
    /// * The conditions are parsed with `in_aggregate_argument` set, exactly
    ///   like `FOLD`'s own arguments, so an `EXISTS` inside one gets the same
    ///   scope basis as an `EXISTS` inside the aggregate's argument.
    /// * They are NOT parsed through the aggregate-lifting expression parser.
    ///   A nested aggregate is not legal SPARQL anywhere, and a sort key inside
    ///   `FOLD` is inside an aggregate already — lifting one here would hoist
    ///   an aggregate out of an aggregate.
    ///
    /// `OrderCondition+` is ONE OR MORE: `FOLD(?v ORDER BY)` names no sort key
    /// and is a hard syntax error, never a silent unordered fold.
    fn parse_fold_order_conditions(&mut self) -> Result<Vec<OrderExpression>> {
        if !self.eat_kw("ORDER") {
            return Ok(Vec::new());
        }
        self.expect_kw("BY")?;
        let saved = self.in_aggregate_argument;
        self.in_aggregate_argument = true;
        let parsed = self.parse_fold_order_condition_list();
        self.in_aggregate_argument = saved;
        let conditions = parsed?;
        if conditions.is_empty() {
            return Err(ParseError::syntax(
                "FOLD's ORDER BY requires at least one sort condition".to_owned(),
                self.span(),
            ));
        }
        Ok(conditions)
    }

    /// The `OrderCondition+` loop [`Self::parse_fold_order_conditions`] runs
    /// inside its `in_aggregate_argument` window.
    fn parse_fold_order_condition_list(&mut self) -> Result<Vec<OrderExpression>> {
        let mut conditions = Vec::new();
        loop {
            let cond = if self.eat_kw("ASC") {
                self.expect(&Token::LParen)?;
                let e = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                OrderExpression::Asc(e)
            } else if self.eat_kw("DESC") {
                self.expect(&Token::LParen)?;
                let e = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                OrderExpression::Desc(e)
            } else if self.order_key_ahead() {
                OrderExpression::Asc(self.parse_primary_expression()?)
            } else {
                break;
            };
            conditions.push(cond);
        }
        Ok(conditions)
    }

    /// Parse the `AGG(<iri>, …)` surface's optional trailing named
    /// scalar-value clauses: zero or more `; NAME=value` pairs, generalizing
    /// `GROUP_CONCAT`'s own `; SEPARATOR="…"` — SPARQL's existing precedent for
    /// a named scalar aggregate parameter (see
    /// [`AggregateExpression::scalarvals`]'s docs) — to an arbitrary custom
    /// aggregate's own named parameters (`AGG(<{NS}PERCENTILE>, ?v; P=0.95)`,
    /// `AGG(<{NS}TOPK>, ?v; K=3)`). `NAME` is any [`Token::Word`], matched
    /// case-insensitively and stored UPPER-CASED, so `; separator="…"` and
    /// `; SEPARATOR="…"` would normalize to the same key (this grammar itself
    /// is reached only for [`AggregateFunction::Custom`] — a built-in aggregate
    /// keeps its own dedicated `; SEPARATOR="…."` production, unaffected).
    /// `value` is any SPARQL literal via [`Self::parse_literal`] — a numeric
    /// scalarval (`P=0.95`, `K=3`) parses to its natural numeric datatype
    /// rather than being forced through a string the way `SEPARATOR`'s value
    /// is. Duplicate names and names a specific aggregate does not recognize
    /// are accepted here (this is a structural parse only) and refused later,
    /// at prepare time, by the evaluator.
    fn parse_agg_scalarvals(&mut self) -> Result<Vec<(String, Literal)>> {
        let mut scalarvals = Vec::new();
        while self.eat(&Token::Semicolon) {
            let name = match self.bump() {
                Some(Token::Word(w)) => w.to_ascii_uppercase(),
                other => {
                    return Err(ParseError::syntax(
                        format!("expected a scalarval name, found {other:?}"),
                        self.span(),
                    ));
                }
            };
            self.expect(&Token::Eq)?;
            let value = self.parse_literal()?;
            scalarvals.push((name, value));
        }
        Ok(scalarvals)
    }

    fn parse_optional_separator(&mut self) -> Result<Option<String>> {
        if self.eat(&Token::Semicolon) {
            self.expect_kw("SEPARATOR")?;
            self.expect(&Token::Eq)?;
            match self.bump() {
                Some(Token::StringLit(s) | Token::LongStringLit(s)) => Ok(Some(s.into_owned())),
                other => Err(ParseError::syntax(
                    format!("expected SEPARATOR string, found {other:?}"),
                    self.span(),
                )),
            }
        } else {
            Ok(None)
        }
    }

    fn fresh_agg_var(&mut self) -> Variable {
        let v = Variable::new(format!("__purrdf_agg_{}", self.agg_counter));
        self.agg_counter += 1;
        v
    }

    /// Mint a fresh, unique grouping variable for an expression-valued
    /// `GROUP BY (Expr)` condition with no explicit `AS`. Distinct namespace from
    /// `fresh_agg_var` so the two never collide.
    fn fresh_group_var(&mut self) -> Variable {
        let v = Variable::new(format!("__purrdf_group_{}", self.group_counter));
        self.group_counter += 1;
        v
    }

    /// Mint a fresh, unique label for an anonymous blank node (`[]`). Each
    /// occurrence is a distinct existential; reusing one label (e.g. `""`) would
    /// wrongly fuse separate blank nodes into a single AST node.
    fn fresh_anon(&mut self) -> BlankNode {
        let b = BlankNode::new(format!("__purrdf_anon_{}", self.anon_counter));
        self.anon_counter += 1;
        b
    }

    fn parse_arg_list(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Vec<Expression>> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        if self.eat(&Token::Star) {
            // e.g. COUNT(*) handled elsewhere; a bare `*` here is invalid.
            return Err(ParseError::syntax(
                "unexpected '*' in argument list",
                self.span(),
            ));
        }
        if !self.at(&Token::RParen) {
            self.eat_kw("DISTINCT");
            loop {
                args.push(self.parse_or(aggs)?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    fn parse_expression_list(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Vec<Expression>> {
        self.expect(&Token::LParen)?;
        let mut list = Vec::new();
        if !self.at(&Token::RParen) {
            loop {
                list.push(self.parse_or(aggs)?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(list)
    }
}

/// A parsed predicate: a simple verb (IRI/`a`/variable) yielding a triple, a
/// complex property path yielding a `GraphPattern::Path`, or a plain IRI under a
/// configured property-function namespace yielding a
/// `GraphPattern::PropertyFunction` (carrying that IRI byte-exact).
enum Verb {
    Simple(NamedNodePattern),
    Path(PropertyPathExpression),
    PropertyFn(String),
}

/// The subject a predicate-object list hangs off: either an ordinary term, or —
/// when a parenthesized subject group is followed by a property-function
/// predicate — that call's subject ARGUMENT VECTOR, taken structurally.
enum SubjectArgs {
    Term(TermPattern),
    Args(Vec<TermPattern>),
}

impl SubjectArgs {
    /// The subject as a single term. An argument vector has no term form: it is
    /// only meaningful to a property function, so pairing it with a data
    /// predicate is a hard error.
    fn as_term(&self, at: usize) -> Result<&TermPattern> {
        match self {
            Self::Term(t) => Ok(t),
            Self::Args(_) => Err(ParseError::syntax(
                "a property-function argument list `( … )` cannot be the subject of \
                 an ordinary triple pattern",
                at,
            )),
        }
    }

    /// The subject as an argument vector: a plain term is a ONE-element vector.
    fn as_args(&self) -> Vec<TermPattern> {
        match self {
            Self::Term(t) => vec![t.clone()],
            Self::Args(a) => a.clone(),
        }
    }
}

/// The accumulator for ONE triples block: the data triples of its BGP, the
/// property-path nodes it produced, and its property-function calls.
///
/// Each property-function call records the number of data triples that preceded
/// it, so [`Self::into_pattern`] can rebuild the block as a LEFT-DEEP `Lateral`
/// chain in TEXTUAL order — every call sees the triples written before it on its
/// left. With no property functions the assembly is exactly `Bgp { triples }`,
/// bit for bit what the block produced before the seam existed.
#[derive(Default)]
struct BlockSink {
    triples: Vec<TriplePattern>,
    paths: Vec<GraphPattern>,
    prop_fns: Vec<(usize, PropertyFunctionCall)>,
}

impl BlockSink {
    /// Record a property-function call at the current position in the block.
    fn push_property_function(&mut self, call: PropertyFunctionCall) {
        self.prop_fns.push((self.triples.len(), call));
    }

    /// Assemble the block: the data triples as a `Bgp`, each property-function
    /// call laterally joined onto everything written before it, then the
    /// property-path nodes joined on.
    fn into_pattern(self) -> GraphPattern {
        let mut triples = self.triples.into_iter();
        let mut taken = 0usize;
        let mut g = GraphPattern::Bgp { patterns: vec![] };
        for (at, call) in self.prop_fns {
            let residual: Vec<TriplePattern> = triples.by_ref().take(at - taken).collect();
            taken = at;
            g = GraphPattern::Lateral {
                left: Box::new(join(g, GraphPattern::Bgp { patterns: residual })),
                right: Box::new(GraphPattern::PropertyFunction(call)),
            };
        }
        g = join(
            g,
            GraphPattern::Bgp {
                patterns: triples.collect(),
            },
        );
        for path in self.paths {
            g = join(g, path);
        }
        g
    }
}

#[derive(Default)]
struct Modifiers {
    group_by: Vec<Variable>,
    /// `(Expr AS ?v)` / bare-expression `GROUP BY` conditions, lowered to
    /// `Extend(?v := Expr)` nodes inserted *under* the `Group` (SPARQL 1.1
    /// §18.2.4). Each synthetic/explicit `?v` minted here is also pushed to
    /// `group_by` as a grouping key.
    group_extends: Vec<(Variable, Expression)>,
    having: Vec<Expression>,
    order_by: Vec<OrderExpression>,
    limit: Option<usize>,
    offset: Option<usize>,
}

impl Modifiers {
    /// True when no solution modifier was parsed at all.
    fn is_empty(&self) -> bool {
        self.group_by.is_empty()
            && self.group_extends.is_empty()
            && self.having.is_empty()
            && self.order_by.is_empty()
            && self.limit.is_none()
            && self.offset.is_none()
    }
}

// ── free helpers ─────────────────────────────────────────────────────────────

/// Join two patterns, merging adjacent BGPs and absorbing the empty pattern (the
/// identity table `Z`) on either side so a group that opens with a non-triple
/// element (`UNION`, a property path, …) is not wrapped in a vacuous `Join`.
fn join(left: GraphPattern, right: GraphPattern) -> GraphPattern {
    if is_empty_bgp(&left) {
        return right;
    }
    if is_empty_bgp(&right) {
        return left;
    }
    match (left, right) {
        (GraphPattern::Bgp { mut patterns }, GraphPattern::Bgp { patterns: r }) => {
            patterns.extend(r);
            GraphPattern::Bgp { patterns }
        }
        (l, r) => GraphPattern::Join {
            left: Box::new(l),
            right: Box::new(r),
        },
    }
}

fn is_empty_bgp(p: &GraphPattern) -> bool {
    matches!(p, GraphPattern::Bgp { patterns } if patterns.is_empty())
}

/// Lift a run of template triples into quad patterns, all scoped to `graph`
/// (`None` = the default graph).
fn scope_triples(
    triples: Vec<TriplePattern>,
    graph: Option<&NamedNodePattern>,
) -> Vec<QuadPattern> {
    triples
        .into_iter()
        .map(|triple| QuadPattern {
            triple,
            graph: graph.cloned(),
        })
        .collect()
}

/// Apply the `CONSTRUCT GRAPH VarOrIri …` whole-template shorthand: it is the
/// DEFAULT graph for the template, so it fills only the slots that named no
/// graph of their own and an inner `GRAPH` block still wins. With no shorthand
/// (`graph: None`) this is the identity, so a template that never mentions a
/// graph is returned untouched.
fn scope_template(
    mut template: Vec<QuadPattern>,
    graph: Option<&NamedNodePattern>,
) -> Vec<QuadPattern> {
    let Some(graph) = graph else {
        return template;
    };
    for quad in &mut template {
        if quad.graph.is_none() {
            quad.graph = Some(graph.clone());
        }
    }
    template
}

/// Does a just-parsed triples block contain a property-function call? Walks only
/// the shapes [`BlockSink::into_pattern`] can build (`Bgp`/`Path` leaves under
/// `Join`/`Lateral` spines), which is all the template callers need to tell a
/// property-function refusal from a property-path one.
fn block_has_property_function(p: &GraphPattern) -> bool {
    match p {
        GraphPattern::PropertyFunction(_) => true,
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
            block_has_property_function(left) || block_has_property_function(right)
        }
        _ => false,
    }
}

/// If a property path is length-1 (a single predicate), return it as a triple
/// predicate; complex paths return `None` (they become `GraphPattern::Path`).
fn simple_predicate(path: &PropertyPathExpression) -> Option<NamedNodePattern> {
    match path {
        PropertyPathExpression::NamedNode(n) => Some(NamedNodePattern::NamedNode(n.clone())),
        _ => None,
    }
}

/// Lift a trailing `Filter` out of an `OPTIONAL` body so it becomes the
/// `LeftJoin` join condition (§18.2.2.3 "filter-in-optional").
fn split_trailing_filter(p: GraphPattern) -> (GraphPattern, Option<Expression>) {
    match p {
        GraphPattern::Filter { expr, inner } => (*inner, Some(expr)),
        other => (other, None),
    }
}

/// An order-preserving set of [`Variable`]s with O(log n) membership and
/// insertion — the group-parsing loop's incremental in-scope set, and
/// [`visible_variables`]'s own collection buffer.
///
/// Two structures move together: `order` is the first-appearance sequence
/// SPARQL's "in scope" (`SELECT *`'s projection order, the LATERAL scope
/// walk's deterministic first-conflict order) needs, and `seen` is what makes
/// membership and insertion O(log n) instead of the O(n) linear scan a bare
/// `Vec<Variable>::contains` forces. That scan is what made both this loop
/// (recomputing it from scratch on every `BIND`/`LATERAL`, discussed at this
/// module's group-loop) and a single [`visible_variables`] call over a
/// `BIND`-heavy pattern quadratic; the `BTreeSet` (not a hash set — this
/// crate is wasm32-clean and deliberately does not depend on `ahash`, and a
/// membership set that is never iterated for its OWN order — only `order`
/// ever is — needs no hash at all) removes both.
#[derive(Default)]
struct VarScope {
    order: Vec<Variable>,
    seen: std::collections::BTreeSet<Variable>,
}

impl VarScope {
    fn new() -> Self {
        Self::default()
    }

    /// Record `v` as in scope; a no-op if it already is (first-appearance
    /// order is preserved, so a later re-mention never moves it).
    fn note(&mut self, v: &Variable) {
        if self.seen.insert(v.clone()) {
            self.order.push(v.clone());
        }
    }

    fn contains(&self, v: &Variable) -> bool {
        self.seen.contains(v)
    }

    fn as_slice(&self) -> &[Variable] {
        &self.order
    }

    fn into_vec(self) -> Vec<Variable> {
        self.order
    }
}

/// Collect the in-scope variables of a pattern in first-appearance order
/// (used for `SELECT *` projection). `pub(crate)`: `crate::serialize` also
/// needs this, to recover every variable a bare (`Project`-less) modifier
/// chain's remaining WHERE body still makes visible when reconstructing a
/// `SELECT` clause that has no real `Project` to read a variable list from
/// (see `fmt_subselect`'s `no_project_vars`).
pub(crate) fn visible_variables(p: &GraphPattern) -> Vec<Variable> {
    let mut scope = VarScope::new();
    collect_vars(p, &mut scope);
    scope.into_vec()
}

fn collect_term_vars(t: &TermPattern, out: &mut VarScope) {
    match t {
        TermPattern::Variable(v) => out.note(v),
        TermPattern::Triple(tp) => {
            collect_term_vars(&tp.subject, out);
            if let NamedNodePattern::Variable(v) = &tp.predicate {
                out.note(v);
            }
            collect_term_vars(&tp.object, out);
        }
        _ => {}
    }
}

fn collect_triple_vars(tp: &TriplePattern, out: &mut VarScope) {
    collect_term_vars(&tp.subject, out);
    if let NamedNodePattern::Variable(v) = &tp.predicate {
        out.note(v);
    }
    collect_term_vars(&tp.object, out);
}

fn collect_vars(p: &GraphPattern, out: &mut VarScope) {
    match p {
        GraphPattern::Bgp { patterns } => {
            for tp in patterns {
                collect_triple_vars(tp, out);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            collect_term_vars(subject, out);
            collect_term_vars(object, out);
        }
        // Every argument variable of a property function — on either side — is
        // in scope in the enclosing group: the arguments are the call's inputs
        // AND its bindings.
        GraphPattern::PropertyFunction(call) => {
            for t in call.subject_args.iter().chain(&call.object_args) {
                collect_term_vars(t, out);
            }
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Lateral { left, right } => {
            collect_vars(left, out);
            collect_vars(right, out);
        }
        // SPARQL §18.2.1: variables occurring only in the right operand of
        // MINUS are not in scope in the enclosing group graph pattern, so we
        // descend into `left` only.
        GraphPattern::Minus { left, .. } => {
            collect_vars(left, out);
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            collect_vars(left, out);
            collect_vars(right, out);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => collect_vars(inner, out),
        GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
            if let NamedNodePattern::Variable(v) = name {
                out.note(v);
            }
            collect_vars(inner, out);
        }
        GraphPattern::Extend {
            inner, variable, ..
        } => {
            collect_vars(inner, out);
            out.note(variable);
        }
        GraphPattern::Values { variables, .. } => {
            for v in variables {
                out.note(v);
            }
        }
        GraphPattern::Project { variables, .. } => {
            for v in variables {
                out.note(v);
            }
        }
        GraphPattern::Group {
            variables,
            aggregates,
            ..
        } => {
            for v in variables {
                out.note(v);
            }
            for (v, _) in aggregates {
                out.note(v);
            }
        }
    }
}

/// An `EXISTS`/`NOT EXISTS` scope check deferred out of
/// [`Parser::parse_exists_body`] because it was reached while
/// `Parser::projection_scope_pending` was set — a `SELECT`'s projection list
/// is parsed BEFORE `WHERE`, so no correct in-scope set exists yet at the
/// point the body itself is parsed. Resolved in [`Parser::parse_select`]'s
/// post-`WHERE` block, once the correct basis (see [`ExistsScopeBasis`]) is
/// known.
struct PendingExistsScopeCheck {
    /// Everything already known to be in scope at the moment this `EXISTS`
    /// was parsed, MINUS the one thing that cannot be known yet (the
    /// projection's own root scope) — captured as
    /// [`Parser::exists_scope`]'s snapshot (which already carries every
    /// enclosing `EXISTS`/`MINUS` body's own introductions-so-far, for a
    /// nested occurrence) plus, for `ExistsScopeBasis::Projection` only, a
    /// snapshot of [`Parser::projection_seen_targets`] at that same moment.
    /// The missing root scope is uniform across every entry recorded during
    /// ONE `SELECT`'s projection-parsing window (it is that `SELECT`'s own),
    /// so resolving is exactly `local_scope ∪ root_scope` — no other
    /// adjustment is needed regardless of nesting depth.
    local_scope: Vec<Variable>,
    /// The already-parsed `EXISTS`/`NOT EXISTS` body, re-walked with
    /// [`find_scope_conflict`] once `local_scope` is completed by the root.
    body: GraphPattern,
    /// Where to anchor the syntax error, captured at the `EXISTS`/`NOT
    /// EXISTS` keyword — mirrors [`Parser::parse_exists_body`]'s own `at`.
    at: usize,
    /// Which root scope this entry resolves against.
    basis: ExistsScopeBasis,
}

/// Which root scope a [`PendingExistsScopeCheck`] resolves against, once its
/// enclosing `SELECT`'s post-`WHERE` state is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistsScopeBasis {
    /// A `(expr AS ?v)` SELECT-list target, or an `EXISTS` nested inside one
    /// (not inside an aggregate argument) — the row this checks against is
    /// exactly the one [`Parser::parse_select`]'s pre-existing §19.8 check
    /// already computes: when the query aggregates, the `GROUP BY` keys plus
    /// any expression-valued `GROUP BY (expr AS ?v)` targets (grouping
    /// hides the raw `WHERE` pattern behind them); otherwise the full
    /// `WHERE`-clause scope.
    Projection,
    /// An aggregate's own `(...)` argument expression. An aggregate folds
    /// over the UNGROUPED rows in its group — the parser builds
    /// expression-valued `GROUP BY (expr AS ?v)` targets as `Extend`s placed
    /// BENEATH `Group` (see the grouping-extend loop in
    /// [`Parser::parse_select`]), so they are already bound at the point an
    /// aggregate argument evaluates, but the grouping itself has not
    /// happened yet — the row is the full `WHERE`-clause scope PLUS those
    /// grouping-extend targets, never narrowed to bare `GROUP BY` keys the
    /// way `Projection`'s basis is.
    AggregateArgument,
}

/// Whether a construct that introduces a fresh binding inside the pattern
/// [`find_scope_conflict`] walks is a `BIND`/`(expr AS ?v)` target or a
/// `VALUES` variable — the two shapes it can report, matching the two
/// message forms each call site produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeIntro {
    /// `BIND(expr AS ?v)`, a sub-`SELECT`'s `(expr AS ?v)` projection target,
    /// a `GROUP BY (expr AS ?v)` condition, or a `GROUP BY` aggregate's output
    /// variable — all lower to an `Extend`/`Group` introduction and share one
    /// message form.
    Bind,
    /// A `VALUES` block's column variable.
    Values,
}

impl ScopeIntro {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bind => "BIND target",
            Self::Values => "VALUES variable",
        }
    }
}

/// Which restriction is consulting [`find_scope_conflict`]'s result —
/// `LATERAL`'s right-hand side (SEP-0006) or an `EXISTS`/`NOT EXISTS` group
/// graph pattern (SEP-0007 Part 3). This enum is used ONLY by the two call
/// sites, to format their own syntax error — it is not even a parameter of
/// `find_scope_conflict` itself, so the walk cannot branch on it, not even by
/// an unexercised branch: LABEL-ONLY, by construction, not by convention. If
/// a future client ever needs a walk that behaves differently from this one
/// (rather than merely naming itself differently in the error message it
/// builds from an identical walk), the walk functions split for that client
/// rather than this enum growing a match inside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeConstruct {
    Lateral,
    Exists,
}

impl ScopeConstruct {
    /// The keyword named in the syntax error.
    fn keyword(self) -> &'static str {
        match self {
            Self::Lateral => "LATERAL",
            Self::Exists => "EXISTS",
        }
    }

    /// The trailing clause describing what the colliding variable is already
    /// in scope ON.
    fn already_in_scope_clause(self) -> &'static str {
        match self {
            Self::Lateral => "the LATERAL left-hand side",
            Self::Exists => "the row being filtered",
        }
    }
}

/// The `LATERAL` left-hand side's in-scope variable set, as consulted by
/// [`find_scope_conflict`].
///
/// A thin delegating wrapper over [`visible_variables`] — same function,
/// different name — so that the one rule-specific consequence it carries
/// (below) is documented on the rule that owns it, rather than left as an
/// unexplained side effect of reusing `visible_variables` directly. If
/// `visible_variables` ever grows a caller-specific variant (e.g. a
/// `SELECT *`-shaped pass-down some other engine carries), this is the one
/// call site that would need to move to it — a change made for an unrelated
/// caller cannot silently widen or narrow the LATERAL rule.
///
/// # A `SERVICE ?g` endpoint variable is left-hand scope
///
/// `visible_variables` (via [`collect_vars`]) includes a `SERVICE`/`GRAPH`
/// name variable, pushed before it descends into the block — and that is
/// the correct scope for it, not an accident of reusing `visible_variables`
/// for this rule. `SERVICE ?g { ... }` with a variable endpoint requires
/// `?g` to already be bound in the incoming solution: this engine resolves
/// the endpoint IRI from that binding before the remote call is made
/// (`substitute_in_named_node_pattern` in `crates/sparql-eval`, the same
/// per-row substitution `LATERAL`'s own `inject` step uses), so by the time
/// a `LATERAL` right-hand side runs, `?g` already holds an observable,
/// per-row value contributed by the left-hand side — a USE of an existing
/// binding, exactly like any other variable reused from an ordinary triple
/// pattern, not a fresh binding `SERVICE` introduces. A `BIND`/`VALUES` on
/// the right giving `?g` a NEW value at the right-hand side's own scope
/// level is therefore exactly the class of observable rebinding
/// [`find_scope_conflict`]'s theorem rejects, applied to this
/// variable the same as any other left-bound one. Jena's `SyntaxVarScope`
/// omits the `SERVICE` endpoint variable from its own left-scope set; this
/// function does not follow it here, on the ground above. Pinned by
/// `lateral_left_scope_includes_a_service_endpoint_variable`.
fn compute_lateral_left_scope(p: &GraphPattern) -> Vec<Variable> {
    visible_variables(p)
}

/// Find the first variable `pattern` introduces (via `BIND`, a sub-`SELECT`'s
/// `(expr AS ?v)` projection target, a `GROUP BY` aggregate's output
/// variable, or `VALUES`) that collides with a variable already in `scope`.
///
/// Shared by two restrictions that turn out to be the SAME walk applied to
/// two different (`scope`, `pattern`) pairs: a `LATERAL` right-hand side
/// against its left-hand side's scope (SEP-0006; `scope` from
/// [`compute_lateral_left_scope`]), and an `EXISTS`/`NOT EXISTS` group graph
/// pattern against the row being filtered (SEP-0007 Part 3; `scope` from
/// [`Parser::exists_scope`], captured at the `EXISTS` keyword before its body
/// is parsed). Each call site names itself only in the [`ParseError`] it
/// builds from this function's result, via [`ScopeConstruct`] — this function
/// never sees which one is asking (see [`ScopeConstruct`]'s doc).
///
/// # The theorem
///
/// SEP-0006 defines `Lateral(Ω, P) = ⋃_{μ∈Ω} eval(inject(P, μ))`, where
/// `inject` — the corrected substitution the evaluator performs
/// (`crates/sparql-eval`) — exposes the left row's bindings to `P` as
/// ordinary, still-variable solutions rather than literal term substitution.
/// SPARQL's own `ExprEXISTS(P)(µ)` (§18.5) is the same shape one level down:
/// `P` is evaluated with the filtered row `µ`'s bindings already exposed to
/// it, not as a literal substitution either — SEP-0007 Part 3 is this SAME
/// injection theorem applied to `EXISTS`/`NOT EXISTS` rather than `LATERAL`.
/// This walker is the syntax half of the contract that makes either
/// injection sound: **it rejects exactly the programs, WRITTEN WITH THE
/// RESTRICTED KEYWORD, in which injecting an outer binding into the pattern
/// would be observable as a rebinding** — a fresh `BIND`/`VALUES`/aggregate
/// target at the pattern's own top scope level trying to give a NEW value to
/// a variable the outer row already gave one. A target confined to a `MINUS`
/// right operand is, by definition, never such an observable rebinding:
/// §18.5's evaluation uses the right operand only for the compatibility test
/// and discards its bindings, so nothing downstream of a `MINUS` can ever see
/// a value that operand introduced (see the `MINUS`-right paragraph in
/// "Scope-level argument" below) — "exactly" holds with that definition, not
/// as an exception carved out of it. The `SERVICE ?g { ... }` auto-wrap form
/// (see "A property of the surface form" below) also builds a
/// `GraphPattern::Lateral` node but is outside this theorem's `LATERAL`
/// domain — it is never walked, by construction, regardless of what its
/// right-hand side does. The evaluator's half of the same theorem is that
/// injection never crosses a `Project` boundary the projection does not
/// itself carry forward, so a sub-select that does not project a shadowed
/// name never observes the outer value, and rebinding it inside is legal.
/// The two halves are one design: this function decides parse-time legality
/// by walking the same scope boundary the evaluator's substitution respects
/// at run time.
///
/// # Scope-level argument
///
/// SPARQL §18.2.1 defines exactly one construct that opens a fresh variable
/// scope: a sub-`SELECT`'s projection (`Project`). Every other construct —
/// `OPTIONAL`, `UNION`, `GRAPH`, a nested group, even a nested `LATERAL` — is
/// transparent to scope: its introductions sit at the *same* scope level as
/// the group around it. That is why every binary/unary node below simply
/// recurses (a nested `LATERAL`'s own left AND right operands are both at the
/// outer scope level — they are not given a fresh boundary of their own),
/// while only `Project` narrows the scope being checked, and `Group`
/// (§18.2.4's aggregation boundary, which likewise hides the pattern it
/// aggregates over behind grouping keys and aggregate outputs) is checked
/// only for the fresh variables its OWN aggregates and expression-valued
/// `GROUP BY` conditions introduce.
///
/// `MINUS` is the one binary node that is NOT scope-transparent on its right
/// operand. §18.2.1 puts a `MINUS`-right-only variable out of scope
/// entirely, and §18.5 explains why: evaluation uses the right operand
/// solely to build the compatibility test against the left operand's rows,
/// then discards its bindings — nothing the right operand introduces ever
/// reaches a solution that survives `MINUS`. A `BIND`/`VALUES`/aggregate
/// target confined to a `MINUS` right operand therefore cannot be an
/// observable rebinding at ANY depth, so only the left operand is walked;
/// the right operand is skipped outright rather than merely narrowed.
///
/// # `Group`'s synthetic targets: aggregate outputs and grouping-expression keys
///
/// A `GROUP BY` query's aggregate output variables (e.g. `?n` in
/// `(COUNT(?x) AS ?n)`) are, like a `BIND` target, fresh bindings introduced
/// at the `Group` node's own scope level — checked here the same way (mapped
/// to [`ScopeIntro::Bind`]). So is an expression-valued `GROUP BY (expr AS
/// ?v)` condition's grouping variable: `?v`'s value is a computed
/// expression, never a matched term, so the parser lowers it to an `Extend`
/// placed directly beneath `Group` (see the grouping-extend loop right
/// before `GraphPattern::Group` is built). [`find_group_extend_conflict`]
/// walks exactly that lowered chain and nothing past it: the pattern
/// actually being grouped is never walked, because only the grouping keys
/// and aggregate outputs escape a `Group` (mirrors [`collect_vars`]'s own
/// non-descent into `Group`'s `inner`, which likewise only notes
/// `variables` and `aggregates`). A bare `GROUP BY ?y` grouping key that
/// merely names an already-bound variable produces no `Extend` at all — it
/// is a USE of an existing binding, not an introduction, so this walker
/// never has anything to find for it.
///
/// # Two points of disagreement with Jena
///
/// * **Laxer.** An introduction confined to a `MINUS` right operand is
///   ACCEPTED here at ANY depth (Jena rejects it) — not only under a
///   `SELECT *` sub-select, which is merely one route to this same shape.
///   §18.2.1 puts `MINUS`-right variables out of scope, and §18.5 explains
///   why nothing built on top of `MINUS` can ever observe them (see the
///   `MINUS`-right paragraph in "Scope-level argument" above), so there is
///   never anything observable for the rule to reject, whether the
///   `MINUS` sits directly under `LATERAL`'s keyword or beneath a `SELECT
///   *` sub-select above it. Jena's walk instead passes the full
///   unfiltered variable set through unconditionally — declined here as a
///   specification mismatch (§18.2.1/§18.5 win), not adopted. Pinned by
///   `lateral_rhs_minus_right_bind_is_accepted` (the bare form) and
///   `lateral_rhs_minus_right_bind_under_select_star_is_accepted` (the
///   `SELECT *`-wrapped form that first surfaced the shape).
/// * **Stricter.** A `SERVICE ?g` endpoint variable counts as left-hand scope
///   here (ground documented on [`compute_lateral_left_scope`]: the variable
///   is required to already be bound before the endpoint is resolved, so it
///   is left-hand USE, not introduction); Jena omits it from its own
///   left-scope set.
///
/// # Never descends into `Expression`
///
/// `Filter`'s and `Extend`'s expression operands are never visited — an
/// `EXISTS { ... }` nested inside one binds nothing at the OUTER pattern's
/// OWN scope level (it is its own nested query pattern), so a `BIND` inside
/// a NESTED `EXISTS` can never trigger the OUTER `LATERAL`/`EXISTS` check
/// that is walking the pattern around it. Pinned live, not just documented,
/// by `lateral_rhs_exists_is_not_walked`. This is exactly why `EXISTS`
/// needs its OWN call to this function, at its OWN parse site (the parser's
/// `EXISTS`/`NOT EXISTS` production, via [`Parser::exists_scope`]): nothing
/// else ever walks into a nested `EXISTS` body to find a collision inside
/// it.
///
/// # A property of the surface form, not of the algebra node
///
/// This function is only ever called from the parser's `LATERAL`-keyword
/// dispatch arm and its `EXISTS`/`NOT EXISTS` expression production. The
/// pre-existing `SERVICE ?g { ... }` auto-wrap elsewhere in this module also
/// produces a `GraphPattern::Lateral` node — for the unrelated reason that a
/// variable-endpoint federated call must see the left row's bindings — but
/// it is not user-written `LATERAL` syntax, so the `LATERAL` restriction
/// never runs over it. Pinned by
/// `service_variable_endpoint_rhs_bind_is_not_scope_checked`.
///
/// # Determinism
///
/// `scope` is an order-preserving slice, never a hash container: the
/// first collision in a pre-order, left-to-right walk (and, within one node,
/// the introducing construct's own declaration order) is reported, so the
/// variable named in the error is reproducible across runs.
fn find_scope_conflict<'a>(
    scope: &[Variable],
    pattern: &'a GraphPattern,
) -> Option<(&'a Variable, ScopeIntro)> {
    match pattern {
        // Leaves: nothing is introduced.
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::PropertyFunction(_) => None,
        // Binary nodes are transparent to scope: recurse both operands at the
        // SAME scope level (see the scope-level argument above).
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            find_scope_conflict(scope, left).or_else(|| find_scope_conflict(scope, right))
        }
        // `MINUS` is the one binary node that is NOT scope-transparent on its
        // right operand: §18.2.1 puts a MINUS-right-only variable out of
        // scope, and §18.5's evaluation only ever uses the right side for the
        // compatibility test — its bindings are discarded, never carried
        // forward — so a `BIND`/`VALUES`/aggregate introduction confined to a
        // MINUS right operand can never be observed as a rebinding, at ANY
        // depth, not only under a `SELECT *` sub-select (which was one route
        // to this same shape, not a separate rule). Only the left operand is
        // walked.
        GraphPattern::Minus { left, .. } => find_scope_conflict(scope, left),
        // Unary wrappers are transparent to scope; any expression operand is
        // never visited.
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => find_scope_conflict(scope, inner),
        // `BIND`, a sub-SELECT's `(expr AS ?v)`, and a `GROUP BY (expr AS ?v)`
        // condition all lower to `Extend` — a fresh binding at this scope
        // level.
        GraphPattern::Extend {
            inner, variable, ..
        } => {
            if scope.contains(variable) {
                Some((variable, ScopeIntro::Bind))
            } else {
                find_scope_conflict(scope, inner)
            }
        }
        // `VALUES`: the first declared column that collides, in declaration
        // order.
        GraphPattern::Values { variables, .. } => {
            for v in variables {
                if scope.contains(v) {
                    return Some((v, ScopeIntro::Values));
                }
            }
            None
        }
        // A sub-SELECT's projection is the one scope boundary in the
        // grammar: narrow to the variables it actually carries out,
        // preserving `scope`'s own order, and stop once nothing survives
        // the narrowing — nothing beneath an empty narrowed scope could ever
        // be observed as a rebinding of an outer variable. Projecting is not
        // introducing: the projection's own `(expr AS ?v)` extends live
        // beneath it and are caught, narrowed, by the `Extend` arm above.
        GraphPattern::Project { inner, variables } => {
            let narrowed: Vec<Variable> = scope
                .iter()
                .filter(|v| variables.contains(*v))
                .cloned()
                .collect();
            if narrowed.is_empty() {
                None
            } else {
                find_scope_conflict(&narrowed, inner)
            }
        }
        // `GROUP BY`'s aggregate output variables are fresh bindings at this
        // scope level (see "Group's synthetic targets" above); then the
        // lowered chain of expression-valued `GROUP BY (expr AS ?v)`
        // `Extend`s directly beneath `Group` — and nothing past it, the
        // pattern being grouped is never walked.
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            for (v, _) in aggregates {
                if scope.contains(v) {
                    return Some((v, ScopeIntro::Bind));
                }
            }
            find_group_extend_conflict(inner, variables, scope)
        }
    }
}

/// Walk the chain of `Extend` nodes the parser lowers each expression-valued
/// `GROUP BY (expr AS ?v)` condition to, which the query builder places
/// directly beneath `Group` (one `Extend` per condition, innermost-first —
/// see the grouping-extend loop immediately before `GraphPattern::Group` is
/// constructed). Stops the instant a node is not one of these lowered
/// `Extend`s: `variables` is `Group`'s full grouping-key list, so an
/// `Extend` whose target is not in it cannot be a grouping-extend the parser
/// produced — it is the top of the pattern actually being grouped, which
/// this walker never descends into (mirrors [`collect_vars`]'s own
/// non-descent into `Group`'s `inner`).
///
/// Recurses before checking the current node, so the first conflict
/// reported is the earliest-DECLARED `GROUP BY (expr AS ?v)` condition
/// (the innermost `Extend`, closest to the ungrouped pattern), preserving
/// the walker's left-to-right determinism contract.
fn find_group_extend_conflict<'a>(
    inner: &'a GraphPattern,
    variables: &[Variable],
    lhs_scope: &[Variable],
) -> Option<(&'a Variable, ScopeIntro)> {
    let GraphPattern::Extend {
        inner: next,
        variable,
        ..
    } = inner
    else {
        return None;
    };
    if !variables.contains(variable) {
        return None;
    }
    find_group_extend_conflict(next, variables, lhs_scope).or_else(|| {
        if lhs_scope.contains(variable) {
            Some((variable, ScopeIntro::Bind))
        } else {
            None
        }
    })
}

/// Collect the labels of every blank node in a run of quad patterns, descending
/// into RDF-1.2 quoted triples. Used to enforce the §19.6 rule that a blank node
/// label may not be shared across two operations of one update request.
fn collect_quad_bnode_labels(quads: &[QuadPattern], out: &mut std::collections::HashSet<String>) {
    for q in quads {
        collect_triple_bnode_labels(&q.triple, out);
    }
}

fn collect_triple_bnode_labels(t: &TriplePattern, out: &mut std::collections::HashSet<String>) {
    collect_term_bnode_labels(&t.subject, out);
    collect_term_bnode_labels(&t.object, out);
}

fn collect_term_bnode_labels(t: &TermPattern, out: &mut std::collections::HashSet<String>) {
    match t {
        TermPattern::BlankNode(b) => {
            out.insert(b.as_str().to_owned());
        }
        TermPattern::Triple(tp) => collect_triple_bnode_labels(tp, out),
        _ => {}
    }
}

/// Hard-fail if any subject/object position of a triple pattern (descending into
/// RDF 1.2 quoted triples) is a blank node. Blank nodes are disallowed in DELETE
/// templates and `DELETE WHERE` (SPARQL 1.1 Update §3.1.3 / §3.1.3.2).
fn reject_blank_in_triple_pattern(t: &TriplePattern, at: usize) -> Result<()> {
    reject_blank_in_term_pattern(&t.subject, at)?;
    reject_blank_in_term_pattern(&t.object, at)
}

fn reject_blank_in_term_pattern(t: &TermPattern, at: usize) -> Result<()> {
    match t {
        TermPattern::BlankNode(_) => Err(ParseError::syntax(
            "blank node in a DELETE template is not allowed",
            at,
        )),
        TermPattern::Triple(tp) => reject_blank_in_triple_pattern(tp, at),
        _ => Ok(()),
    }
}

fn is_absolute_iri(s: &str) -> bool {
    // A scheme followed by ':' — RFC-3986 §3.1 (cheap prefix test).
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, c)) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for (_, c) in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

/// Split a lang tag into the language and an optional RDF 1.2 base direction
/// (`en--ltr` → (`en`, Ltr)).
fn split_lang_dir(tag: &str) -> (String, Option<BaseDirection>) {
    if let Some((lang, dir)) = tag.split_once("--") {
        let dir = match dir.to_ascii_lowercase().as_str() {
            "ltr" => Some(BaseDirection::Ltr),
            "rtl" => Some(BaseDirection::Rtl),
            _ => None,
        };
        (lang.to_owned(), dir)
    } else {
        (tag.to_owned(), None)
    }
}

fn expect_arity(args: &[Expression], n: usize, name: &str, at: usize) -> Result<()> {
    if args.len() == n {
        Ok(())
    } else {
        Err(ParseError::syntax(
            format!("{name} expects {n} arguments, got {}", args.len()),
            at,
        ))
    }
}

fn aggregate_function(upper: &str) -> Option<AggregateFunction> {
    Some(match upper {
        "COUNT" => AggregateFunction::Count,
        "SUM" => AggregateFunction::Sum,
        "AVG" => AggregateFunction::Avg,
        "MIN" => AggregateFunction::Min,
        "MAX" => AggregateFunction::Max,
        "SAMPLE" => AggregateFunction::Sample,
        "GROUP_CONCAT" => AggregateFunction::GroupConcat,
        // SEP-0009 `[127+]` adds `FOLD` as a KEYWORD alternative of `Aggregate`,
        // exactly like the seven above — not through the `AGG(<iri>, …)` custom
        // surface, for which the spec defines no aggregate IRI.
        "FOLD" => AggregateFunction::Fold,
        _ => return None,
    })
}

fn builtin_function(upper: &str) -> Option<Function> {
    Some(match upper {
        "STR" => Function::Str,
        "LANG" => Function::Lang,
        "LANGDIR" => Function::LangDir,
        "STRLANGDIR" => Function::StrLangDir,
        "HASLANG" => Function::HasLang,
        "HASLANGDIR" => Function::HasLangDir,
        "LANGMATCHES" => Function::LangMatches,
        "DATATYPE" => Function::Datatype,
        "IRI" => Function::Iri,
        "URI" => Function::Uri,
        "BNODE" => Function::BNode,
        "RAND" => Function::Rand,
        "ABS" => Function::Abs,
        "CEIL" => Function::Ceil,
        "FLOOR" => Function::Floor,
        "ROUND" => Function::Round,
        "CONCAT" => Function::Concat,
        "SUBSTR" => Function::SubStr,
        "STRLEN" => Function::StrLen,
        "REPLACE" => Function::Replace,
        "UCASE" => Function::UCase,
        "LCASE" => Function::LCase,
        "ENCODE_FOR_URI" => Function::EncodeForUri,
        "CONTAINS" => Function::Contains,
        "STRSTARTS" => Function::StrStarts,
        "STRENDS" => Function::StrEnds,
        "STRBEFORE" => Function::StrBefore,
        "STRAFTER" => Function::StrAfter,
        "YEAR" => Function::Year,
        "MONTH" => Function::Month,
        "DAY" => Function::Day,
        "HOURS" => Function::Hours,
        "MINUTES" => Function::Minutes,
        "SECONDS" => Function::Seconds,
        "TIMEZONE" => Function::Timezone,
        "TZ" => Function::Tz,
        "ADJUST" => Function::Adjust,
        "NOW" => Function::Now,
        "UUID" => Function::Uuid,
        "STRUUID" => Function::StrUuid,
        "MD5" => Function::Md5,
        "SHA1" => Function::Sha1,
        "SHA256" => Function::Sha256,
        "SHA384" => Function::Sha384,
        "SHA512" => Function::Sha512,
        // SEP-0008, in BOTH spellings the proposal uses. The hyphenated names are the
        // only built-ins carrying a `-`; the lexer's PN_PREFIX scan admits `-`, so each
        // arrives here as ONE word (see `Function::Sha3_224`'s rustdoc and the parser
        // tests below). The underscored names are SEP-0008's own literal spelling of the
        // four functions, accepted here so a query written from the proposal text parses
        // rather than failing as an unsupported construct. Both spellings resolve to the
        // SAME `Function`, so the serializer has exactly one form to emit and output
        // stays byte-deterministic — see `sha3_serializes_to_one_canonical_spelling`.
        "SHA3-224" | "SHA3_224" => Function::Sha3_224,
        "SHA3-256" | "SHA3_256" => Function::Sha3_256,
        "SHA3-384" | "SHA3_384" => Function::Sha3_384,
        "SHA3-512" | "SHA3_512" => Function::Sha3_512,
        "STRLANG" => Function::StrLang,
        "STRDT" => Function::StrDt,
        "ISIRI" => Function::IsIri,
        "ISURI" => Function::IsUri,
        "ISBLANK" => Function::IsBlank,
        "ISLITERAL" => Function::IsLiteral,
        "ISNUMERIC" => Function::IsNumeric,
        "REGEX" => Function::Regex,
        "TRIPLE" => Function::Triple,
        "SUBJECT" => Function::Subject,
        "PREDICATE" => Function::Predicate,
        "OBJECT" => Function::Object,
        "ISTRIPLE" => Function::IsTriple,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{PurrdfCall, PurrdfFn};
    use pretty_assertions::assert_eq;

    const GM: &str =
        "PREFIX purrdf: <https://x/>\nPREFIX rdf: <http://r/>\nPREFIX rdfs: <http://s/>\n";

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("parse")
    }

    fn select_pattern(q: &str) -> GraphPattern {
        match parse(q) {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Strip the outer `Project` wrapper to reach the WHERE algebra.
    fn unproject(p: GraphPattern) -> GraphPattern {
        match p {
            GraphPattern::Project { inner, .. } => *inner,
            other => other,
        }
    }

    #[test]
    fn group_graph_pattern_nesting_has_a_typed_limit() {
        fn nested_query(depth: usize) -> String {
            format!(
                "SELECT * WHERE {} ?s ?p ?o {}",
                "{ ".repeat(depth),
                "} ".repeat(depth)
            )
        }

        SparqlParser::new()
            .parse_query(&nested_query(MAX_GRAPH_PATTERN_DEPTH))
            .expect("the documented maximum nesting depth parses");
        let error = SparqlParser::new()
            .parse_query(&nested_query(MAX_GRAPH_PATTERN_DEPTH + 1))
            .expect_err("one group beyond the safety limit must be refused");
        assert!(
            matches!(error, ParseError::Syntax { .. }),
            "the nesting refusal remains a typed syntax error: {error}"
        );
        assert!(error.to_string().contains("nesting exceeds"));
    }

    /// Locate the EXACT repetition count at which a monotonic spine generator
    /// — one more repetition of `spine` only ever charges MORE combinator
    /// nodes, never fewer, true of every generator this helper is applied to
    /// below — stops parsing, then assert the transition is exactly what
    /// [`MAX_GRAPH_PATTERN_NODES`] demands: one repetition short of it parses
    /// clean, and the very next repetition is refused with a typed
    /// [`ParseError`] naming the limit — never an abort (reaching this
    /// assertion at all, on either side, already demonstrates that this test
    /// PROCESS did not crash; a real stack overflow would have taken the
    /// whole process down before any assertion could run).
    fn assert_spine_bound(spine: impl Fn(usize) -> String) {
        assert!(
            SparqlParser::new().parse_query(&spine(1)).is_ok(),
            "the smallest spine must parse"
        );
        let (mut lo, mut hi) = (1usize, 2usize);
        while SparqlParser::new().parse_query(&spine(hi)).is_ok() {
            lo = hi;
            hi *= 2;
            assert!(
                hi < 1_000_000,
                "spine never reaches the safety limit up to {hi} repetitions"
            );
        }
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if SparqlParser::new().parse_query(&spine(mid)).is_ok() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        SparqlParser::new()
            .parse_query(&spine(lo))
            .expect("one repetition short of the safety limit must parse");
        let error = SparqlParser::new().parse_query(&spine(hi)).expect_err(
            "one repetition past the safety limit must be a typed refusal, never an abort",
        );
        assert!(
            matches!(error, ParseError::Syntax { .. }),
            "the spine refusal remains a typed syntax error: {error}"
        );
        assert!(
            error.to_string().contains("combinator count exceeds"),
            "the refusal should name the combinator-count limit, got: {error}"
        );
    }

    /// A run of SIBLING `OPTIONAL { }` elements at ONE brace depth: each
    /// keyword adds one `LeftJoin` level to a left-deep spine while
    /// `group_pattern_depth` never exceeds 1 — the exact shape
    /// `MAX_GRAPH_PATTERN_DEPTH` cannot see, and the shape
    /// [`MAX_GRAPH_PATTERN_NODES`] exists to bound instead.
    #[test]
    fn sibling_spine_length_is_bounded_by_a_typed_error() {
        assert_spine_bound(|n| {
            let mut body = String::from("SELECT * WHERE { ?s <https://example.org/p> ?o ");
            for _ in 0..n {
                body.push_str("OPTIONAL { ?s <https://example.org/q> ?r } ");
            }
            body.push('}');
            body
        });
    }

    /// The SAME left-deep-spine hazard, reachable with no `OPTIONAL`/`UNION`/
    /// `LATERAL` keyword at all: a long run of dot-separated COMPLEX
    /// property-path triples inside ONE triples block. `join` flattens
    /// adjacent plain `Bgp` triples into one node (so a triples-only spine of
    /// this length is harmless), but a property path with a modifier
    /// (`+`/`*`/`?`/a multi-step sequence) parses to its own
    /// `GraphPattern::Path` node, and `BlockSink::into_pattern` folds each one
    /// onto the block with its own `Join` — entirely inside what the
    /// group-parsing loop counts as ONE element.
    #[test]
    fn dot_separated_path_spine_length_is_bounded_by_a_typed_error() {
        use std::fmt::Write as _;
        assert_spine_bound(|n| {
            let mut body = String::from("SELECT * WHERE { ");
            for i in 0..n {
                let _ = write!(
                    body,
                    "<https://example.org/s{i}> <https://example.org/p>+ <https://example.org/o{i}> . "
                );
            }
            body.push('}');
            body
        });
    }

    /// A single bracketed group with a long run of `UNION` arms — hidden from
    /// the group loop's own per-element charge because the whole chain is
    /// consumed inside ONE loop iteration (the nested `while eat_kw("UNION")`
    /// loop), so it is charged separately, one unit per arm past the first.
    #[test]
    fn union_arm_spine_length_is_bounded_by_a_typed_error() {
        assert_spine_bound(|n| {
            let mut body = String::from("SELECT * WHERE { { ?s <https://example.org/p> ?o }");
            for _ in 0..n {
                body.push_str(" UNION { ?s <https://example.org/p> ?o }");
            }
            body.push('}');
            body
        });
    }

    /// A long `SELECT (e1 AS ?v1) … (eN AS ?vN)` projection list lowers to a
    /// chain of `Extend` nodes with no brace involved at all — a THIRD shape
    /// (alongside the group loop and its `UNION` arms) that
    /// `MAX_GRAPH_PATTERN_DEPTH` cannot see, closed by the same budget.
    #[test]
    fn select_expression_list_length_is_bounded_by_a_typed_error() {
        use std::fmt::Write as _;
        assert_spine_bound(|n| {
            let mut body = String::from("SELECT ");
            for i in 0..n {
                let _ = write!(body, "(1 AS ?v{i}) ");
            }
            body.push_str("WHERE { }");
            body
        });
    }

    #[test]
    fn inverse_in_negated_property_set_parses_with_direction() {
        // `!(^iri)` — the inverse element is preserved as a `NegatedPathElement`
        // with `inverse: true`, not silently degraded to the forward `!(iri)`.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x !(^purrdf:p) ?y }}");
        let pattern = unproject(select_pattern(&q));
        let GraphPattern::Path { path, .. } = pattern else {
            panic!("expected a Path pattern, got {pattern:?}");
        };
        match path {
            PropertyPathExpression::NegatedPropertySet(elems) => {
                assert_eq!(elems.len(), 1);
                assert!(elems[0].inverse, "^purrdf:p must set inverse: true");
                assert_eq!(elems[0].predicate.as_str(), "https://x/p");
            }
            other => panic!("expected NegatedPropertySet, got {other:?}"),
        }
    }

    #[test]
    fn distinct_anonymous_blank_nodes_do_not_collapse() {
        // Two `[]` are two distinct existentials; they must not fuse into one
        // AST node (which would wrongly merge the triples that mention them).
        let q = format!("{GM}SELECT ?x WHERE {{ [] purrdf:p ?x . [] purrdf:q ?x }}");
        let GraphPattern::Bgp { patterns } = unproject(select_pattern(&q)) else {
            panic!("expected BGP");
        };
        assert_eq!(patterns.len(), 2);
        let (TermPattern::BlankNode(a), TermPattern::BlankNode(b)) =
            (&patterns[0].subject, &patterns[1].subject)
        else {
            panic!("both subjects should be blank nodes");
        };
        assert_ne!(a, b, "distinct [] must produce distinct blank nodes");
    }

    #[test]
    fn quoted_triple_with_variable_predicate() {
        // The RDF-1.2 codec shape: `?r rdf:reifies <<( ?s ?p ?o )>>`.
        let q = format!("{GM}SELECT ?r WHERE {{ ?r rdf:reifies <<( ?s ?p ?o )>> . }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Bgp { patterns } = where_pat else {
            panic!("expected BGP, got {where_pat:?}");
        };
        assert_eq!(patterns.len(), 1);
        let TermPattern::Triple(inner) = &patterns[0].object else {
            panic!(
                "object should be a quoted triple, got {:?}",
                patterns[0].object
            );
        };
        assert_eq!(
            inner.predicate,
            NamedNodePattern::Variable(Variable::new("p"))
        );
        assert_eq!(inner.subject, TermPattern::Variable(Variable::new("s")));
    }

    #[test]
    fn optional_lifts_trailing_filter_to_leftjoin() {
        let q = format!(
            "{GM}SELECT ?a WHERE {{ ?a a purrdf:T . OPTIONAL {{ ?a purrdf:p ?b . FILTER(?b != ?a) }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::LeftJoin { expression, .. } = where_pat else {
            panic!("expected LeftJoin, got {where_pat:?}");
        };
        assert!(expression.is_some(), "FILTER should lift into the LeftJoin");
    }

    #[test]
    fn union_of_two_groups() {
        let q = format!("{GM}SELECT ?a WHERE {{ {{ ?a a purrdf:X }} UNION {{ ?a a purrdf:Y }} }}");
        let where_pat = unproject(select_pattern(&q));
        assert!(
            matches!(where_pat, GraphPattern::Union { .. }),
            "got {where_pat:?}"
        );
    }

    #[test]
    fn bind_becomes_extend() {
        let q = format!("{GM}SELECT ?k WHERE {{ ?a a purrdf:T . BIND(\"x\" AS ?k) }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { variable, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        assert_eq!(variable, Variable::new("k"));
    }

    #[test]
    fn property_path_zero_or_more() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x rdfs:subClassOf* purrdf:C . }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Path { path, .. } = where_pat else {
            panic!("expected Path, got {where_pat:?}");
        };
        assert!(matches!(path, PropertyPathExpression::ZeroOrMore(_)));
    }

    #[test]
    fn sequence_path_with_star() {
        // `owl:members/rdf:rest*/rdf:first` — Sequence containing a ZeroOrMore.
        let q = format!("{GM}SELECT ?x WHERE {{ ?d purrdf:members/rdf:rest*/rdf:first ?x . }}");
        let where_pat = unproject(select_pattern(&q));
        assert!(
            matches!(
                where_pat,
                GraphPattern::Path {
                    path: PropertyPathExpression::Sequence(..),
                    ..
                }
            ),
            "got {where_pat:?}"
        );
    }

    #[test]
    fn rdf_collection_in_object_desugars_to_first_rest_chain() {
        // `?s purrdf:members ( purrdf:a purrdf:b purrdf:c )` desugars to the standard
        // rdf:first/rdf:rest blank-node chain (SPARQL §19.5 Collection). The members
        // predicate binds to the HEAD blank; three rdf:first edges carry the elements;
        // three rdf:rest edges link the chain and terminate it with rdf:nil.
        let q =
            format!("{GM}SELECT ?s WHERE {{ ?s purrdf:members ( purrdf:a purrdf:b purrdf:c ) }}");
        let GraphPattern::Bgp { patterns } = unproject(select_pattern(&q)) else {
            panic!("expected BGP");
        };
        // 1 members edge + 3 rdf:first + 3 rdf:rest = 7 triples. The desugaring emits
        // the REAL rdf: IRIs (not the test's mock `rdf:` prefix binding).
        assert_eq!(patterns.len(), 7, "got {patterns:?}");
        let first = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
        let rest = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
        let nil = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
        let pred = |p: &TriplePattern| match &p.predicate {
            NamedNodePattern::NamedNode(n) => n.as_str().to_owned(),
            other @ NamedNodePattern::Variable(_) => panic!("unexpected predicate {other:?}"),
        };
        assert_eq!(patterns.iter().filter(|p| pred(p) == first).count(), 3);
        assert_eq!(patterns.iter().filter(|p| pred(p) == rest).count(), 3);
        assert_eq!(
            patterns
                .iter()
                .filter(|p| matches!(&p.object, TermPattern::NamedNode(n) if n.as_str() == nil))
                .count(),
            1,
            "exactly one rdf:nil terminator"
        );
        // The members triple's object is the chain head (a blank node).
        let members = patterns
            .iter()
            .find(|p| pred(p).ends_with("members"))
            .expect("members edge present");
        assert!(
            matches!(members.object, TermPattern::BlankNode(_)),
            "members object is the collection head blank"
        );
    }

    #[test]
    fn filter_not_exists() {
        let q = format!(
            "{GM}SELECT ?a WHERE {{ ?a a purrdf:T . FILTER NOT EXISTS {{ ?a purrdf:bad ?x }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Filter { expr, .. } = where_pat else {
            panic!("expected Filter, got {where_pat:?}");
        };
        assert!(matches!(expr, Expression::Not(inner) if matches!(*inner, Expression::Exists(_))));
    }

    #[test]
    fn group_by_with_count_aggregate() {
        let q = format!(
            "{GM}SELECT ?m (COUNT(?c) AS ?n) WHERE {{ ?c purrdf:vantage ?m . }} GROUP BY ?m"
        );
        let where_pat = unproject(select_pattern(&q));
        // After §18.2: ... Extend(?n = synth) over Group{aggregates:[(synth, COUNT ?c)]}.
        let GraphPattern::Extend {
            inner, variable, ..
        } = where_pat
        else {
            panic!("expected Extend, got {where_pat:?}");
        };
        assert_eq!(variable, Variable::new("n"));
        let GraphPattern::Group {
            variables,
            aggregates,
            ..
        } = *inner
        else {
            panic!("expected Group under Extend");
        };
        assert_eq!(variables, vec![Variable::new("m")]);
        assert_eq!(aggregates.len(), 1);
        assert!(matches!(
            aggregates[0].1,
            AggregateExpression {
                function: AggregateFunction::Count,
                ..
            }
        ));
    }

    #[test]
    fn order_by_desc_aggregate_lifts_into_group() {
        // SPARQL 1.1 §11.3: ORDER BY on an aggregate is legal inside a grouped
        // query. This was previously rejected as `Unsupported` because ORDER BY
        // used `parse_expression()` (aggregate-blind) instead of the agg-lifting
        // path.
        let q = format!(
            "{GM}SELECT ?t (COUNT(?x) AS ?c) WHERE {{ ?x a ?t }} GROUP BY ?t ORDER BY DESC(COUNT(?x))"
        );
        let where_pat = unproject(select_pattern(&q));
        // Expected algebra (outermost to innermost, modulo ORDER BY wrapper):
        //   OrderBy { order: [Desc(...)], inner: Extend { var: ?c, inner: Group { aggs: [...] } } }
        let GraphPattern::OrderBy {
            inner,
            expression: order,
        } = where_pat
        else {
            panic!("expected OrderBy at top of unproject'd pattern, got {where_pat:?}");
        };
        // The order key must be a Desc wrapping a Variable reference to the
        // synthetic aggregate variable (lifted COUNT(?x)).
        assert_eq!(order.len(), 1);
        assert!(
            matches!(order[0], OrderExpression::Desc(_)),
            "ORDER BY DESC must produce Desc variant, got {:?}",
            order[0]
        );
        // Walk down: Extend → Group.
        let GraphPattern::Extend {
            inner: group_inner,
            variable,
            ..
        } = *inner
        else {
            panic!("expected Extend under OrderBy, got {inner:?}");
        };
        assert_eq!(variable, Variable::new("c"));
        let GraphPattern::Group { aggregates, .. } = *group_inner else {
            panic!("expected Group under Extend");
        };
        // The aggregate lifted from ORDER BY DESC(COUNT(?x)) must appear in the
        // Group's aggregate list alongside the SELECT-projected one. There must
        // be at least one COUNT aggregate (the ?c projection); the ORDER BY
        // COUNT(?x) should either reuse or add another.
        assert!(
            !aggregates.is_empty(),
            "Group must have at least one aggregate"
        );
        assert!(aggregates.iter().any(|(_, ae)| matches!(
            ae,
            AggregateExpression {
                function: AggregateFunction::Count,
                ..
            }
        )));
    }

    // ── `HAVING`/`ORDER BY` bare `Constraint` forms ──────────────────────────
    //    SPARQL 1.1/1.2: `HAVING ::= 'HAVING' Constraint+`,
    //    `OrderCondition ::= (('ASC'|'DESC') BrackettedExpression) |
    //    (Constraint | Var)`, `Constraint ::= BrackettedExpression |
    //    BuiltInCall | FunctionCall`. Previously only the bracketed form
    //    (`HAVING (…)`, `ORDER BY ASC(…)`/`DESC(…)`, and a narrow set of
    //    bare builtins in `ORDER BY`) parsed; `EXISTS`/`NOT EXISTS`, `BOUND`,
    //    and a bare `FunctionCall` refused at both sites.

    #[test]
    fn having_accepts_a_bare_exists_constraint() {
        // `EXISTS` is a `BuiltInCall`, so a bare (non-parenthesized) `HAVING
        // EXISTS { … }` is REC-legal — only `HAVING (EXISTS { … })` parsed
        // before this fix.
        let q = format!(
            "{GM}SELECT ?m (COUNT(?c) AS ?n) WHERE {{ ?c purrdf:vantage ?m . }} GROUP BY ?m \
             HAVING EXISTS {{ ?w purrdf:q ?a }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Filter { expr, inner: group } = *inner else {
            panic!("expected Filter (HAVING), got {inner:?}");
        };
        assert!(
            matches!(expr, Expression::Exists(_)),
            "expected the bare EXISTS to parse as Expression::Exists, got {expr:?}"
        );
        assert!(
            matches!(*group, GraphPattern::Group { .. }),
            "expected the HAVING Filter to sit directly over Group, got {group:?}"
        );
    }

    #[test]
    fn having_accepts_multiple_bare_constraints() {
        // `HAVING`'s `Constraint+` is a SPACE-SEPARATED list with no
        // connective — each element may independently be bracketed or bare.
        // `HAVING (a) (b) …` lowers to a `Filter`-over-`Filter` chain (see
        // `serialize.rs`'s `extend_chain_reaches_group` doc); this proves the
        // same lowering for a bare-then-bare pair.
        let q = format!(
            "{GM}SELECT ?m (COUNT(?c) AS ?n) WHERE {{ ?c purrdf:vantage ?m . }} GROUP BY ?m \
             HAVING BOUND(?m) EXISTS {{ ?w purrdf:q ?a }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Filter {
            expr: outer_expr,
            inner: mid,
        } = *inner
        else {
            panic!("expected outer Filter (2nd HAVING condition), got {inner:?}");
        };
        assert!(
            matches!(outer_expr, Expression::Exists(_)),
            "expected the 2nd condition (EXISTS) outermost, got {outer_expr:?}"
        );
        let GraphPattern::Filter {
            expr: inner_expr,
            inner: group,
        } = *mid
        else {
            panic!("expected inner Filter (1st HAVING condition), got {mid:?}");
        };
        assert!(
            matches!(inner_expr, Expression::Bound(_)),
            "expected the 1st condition (BOUND) innermost, got {inner_expr:?}"
        );
        assert!(
            matches!(*group, GraphPattern::Group { .. }),
            "expected the innermost HAVING Filter to sit directly over Group, got {group:?}"
        );
    }

    #[test]
    fn order_by_accepts_a_bare_exists_constraint() {
        let q = format!(
            "{GM}SELECT ?x WHERE {{ ?x purrdf:p ?c }} ORDER BY EXISTS {{ ?w purrdf:q ?a }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::OrderBy { expression, .. } = where_pat else {
            panic!("expected OrderBy, got {where_pat:?}");
        };
        assert_eq!(expression.len(), 1);
        let OrderExpression::Asc(e) = &expression[0] else {
            panic!(
                "OrderCondition's bare Constraint alternative lowers to Asc, got {:?}",
                expression[0]
            );
        };
        assert!(
            matches!(e, Expression::Exists(_)),
            "expected the bare EXISTS to parse as Expression::Exists, got {e:?}"
        );
    }

    #[test]
    fn order_by_accepts_a_bare_builtin_call() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p ?c }} ORDER BY BOUND(?c)");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::OrderBy { expression, .. } = where_pat else {
            panic!("expected OrderBy, got {where_pat:?}");
        };
        assert_eq!(expression.len(), 1);
        let OrderExpression::Asc(e) = &expression[0] else {
            panic!(
                "OrderCondition's bare Constraint alternative lowers to Asc, got {:?}",
                expression[0]
            );
        };
        assert!(
            matches!(e, Expression::Bound(_)),
            "expected the bare BOUND(...) to parse as Expression::Bound, got {e:?}"
        );
    }

    #[test]
    fn order_by_accepts_a_bare_function_call() {
        // `purrdf:` here is the test-fixture prefix (`{GM}` → `<https://x/>`),
        // NOT a configured extension-function namespace — so
        // `purrdf:custom(?x)` parses as an ordinary `Function::Custom`
        // FunctionCall, exactly the `Constraint`'s `FunctionCall`
        // alternative.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p ?c }} ORDER BY purrdf:custom(?x)");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::OrderBy { expression, .. } = where_pat else {
            panic!("expected OrderBy, got {where_pat:?}");
        };
        assert_eq!(expression.len(), 1);
        let OrderExpression::Asc(e) = &expression[0] else {
            panic!(
                "OrderCondition's bare Constraint alternative lowers to Asc, got {:?}",
                expression[0]
            );
        };
        assert!(
            matches!(e, Expression::FunctionCall(Function::Custom(_), _)),
            "expected the bare purrdf:custom(?x) to parse as a custom FunctionCall, got {e:?}"
        );
    }

    #[test]
    fn subselect_having_bare_constraint_parses() {
        // The same `parse_solution_modifiers` call site backs a `SubSelect`
        // (`parse_select` is called recursively for `{ SELECT ... }` —
        // `parse_group_graph_pattern_inner`) — no separate production to fix.
        let q = format!(
            "{GM}SELECT * WHERE {{ {{ SELECT ?m (COUNT(?c) AS ?n) WHERE {{ ?c purrdf:vantage ?m . \
             }} GROUP BY ?m HAVING EXISTS {{ ?w purrdf:q ?a }} }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a bare HAVING EXISTS constraint inside a sub-SELECT must parse");
    }

    #[test]
    fn having_bare_exists_scope_collision_is_rejected() {
        // Proof that broadening HAVING's grammar did not bypass SEP-0007
        // Part 3: `Parser::parse_exists_body`'s doc states a solution-modifier
        // expression's (`GROUP BY`/`HAVING`/`ORDER BY`) in-scope set is the
        // complete `WHERE` clause's scope — so `?c`, bound by WHERE, cannot be
        // rebound inside a bare `HAVING EXISTS { … }` body either, exactly as
        // it cannot inside the bracketed form.
        let q = format!(
            "{GM}SELECT ?m (COUNT(?c) AS ?n) WHERE {{ ?c purrdf:vantage ?m . }} GROUP BY ?m \
             HAVING EXISTS {{ BIND(1 AS ?c) }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a BIND target inside a bare HAVING EXISTS colliding with the WHERE row must fail",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?c inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn filter_in_list() {
        let q = format!(
            "{GM}SELECT ?p WHERE {{ ?f purrdf:pol ?p . FILTER(?p IN (purrdf:a, purrdf:b)) }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Filter { expr, .. } = where_pat else {
            panic!("expected Filter, got {where_pat:?}");
        };
        let Expression::In(_, list) = expr else {
            panic!("expected IN, got {expr:?}");
        };
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn construct_form() {
        let q = format!("{GM}CONSTRUCT {{ ?s a purrdf:Out }} WHERE {{ ?s a purrdf:In }}");
        let Query::Construct { template, .. } = parse(&q) else {
            panic!("expected CONSTRUCT");
        };
        assert_eq!(template.len(), 1);
        assert_eq!(
            template_graphs(&template),
            vec![None],
            "a plain CONSTRUCT names no graph — it is the triple-producing form"
        );
    }

    // ── The quad-producing CONSTRUCT ─────────────────────────────────────────

    /// Every template quad's graph term, spelled the way the query wrote it
    /// (`<iri>` / `?var`), or `None` for an unscoped (default-graph) statement.
    fn template_graphs(template: &[QuadPattern]) -> Vec<Option<String>> {
        template
            .iter()
            .map(|quad| {
                quad.graph.as_ref().map(|g| match g {
                    NamedNodePattern::NamedNode(n) => format!("<{}>", n.as_str()),
                    NamedNodePattern::Variable(v) => format!("?{}", v.as_str()),
                })
            })
            .collect()
    }

    fn construct_template(query: &str) -> Vec<QuadPattern> {
        let Query::Construct { template, .. } = parse(query) else {
            panic!("expected CONSTRUCT for `{query}`");
        };
        template
    }

    /// The upstream form: a `GRAPH <iri> { … }` block INSIDE the template.
    #[test]
    fn construct_template_graph_block_scopes_its_statements() {
        let template = construct_template(
            "CONSTRUCT { GRAPH <http://example.org/g> { ?s ?p ?o } } WHERE { ?s ?p ?o }",
        );
        assert_eq!(
            template_graphs(&template),
            vec![Some("<http://example.org/g>".to_owned())]
        );
    }

    /// A VARIABLE graph name — the first of the three upstream forms the
    /// whole-template shorthand alone could not express.
    #[test]
    fn construct_admits_a_variable_graph_name() {
        // Inside the template, as Jena spells it.
        let template = construct_template(
            "CONSTRUCT { GRAPH ?g { ?s ?p ?o } } WHERE { GRAPH ?g { ?s ?p ?o } }",
        );
        assert_eq!(template_graphs(&template), vec![Some("?g".to_owned())]);

        // And as the whole-template shorthand, which takes a `VarOrIri` too.
        let template =
            construct_template("CONSTRUCT GRAPH ?g { ?s ?p ?o } WHERE { GRAPH ?g { ?s ?p ?o } }");
        assert_eq!(template_graphs(&template), vec![Some("?g".to_owned())]);
    }

    /// MULTIPLE target graphs in ONE template — the second upstream form.
    #[test]
    fn construct_admits_multiple_graph_blocks_in_one_template() {
        let template = construct_template(
            "CONSTRUCT { GRAPH <http://example.org/g> { ?s ?p ?o } \
             GRAPH <http://example.org/h> { ?o ?p ?s } \
             GRAPH ?w { ?s ?s ?s } } WHERE { ?s ?p ?o }",
        );
        assert_eq!(
            template_graphs(&template),
            vec![
                Some("<http://example.org/g>".to_owned()),
                Some("<http://example.org/h>".to_owned()),
                Some("?w".to_owned()),
            ]
        );
    }

    /// Default-graph triples MIXED with named-graph quads in one template — the
    /// third upstream form. The unscoped runs before, between and after the
    /// blocks all stay unscoped, and the optional `.` after a block is admitted.
    #[test]
    fn construct_admits_default_graph_triples_mixed_with_graph_blocks() {
        let template = construct_template(
            "CONSTRUCT { ?s <http://example.org/a> ?o . \
             GRAPH <http://example.org/g> { ?s <http://example.org/b> ?o } . \
             ?s <http://example.org/c> ?o . \
             GRAPH ?w { ?s <http://example.org/d> ?o } \
             ?s <http://example.org/e> ?o } WHERE { ?s ?p ?o }",
        );
        assert_eq!(
            template_graphs(&template),
            vec![
                None,
                Some("<http://example.org/g>".to_owned()),
                None,
                Some("?w".to_owned()),
                None,
            ]
        );
        // The statements themselves keep their template order and content.
        let predicates: Vec<String> = template
            .iter()
            .map(|quad| match &quad.triple.predicate {
                NamedNodePattern::NamedNode(n) => n.as_str().to_owned(),
                NamedNodePattern::Variable(v) => format!("?{}", v.as_str()),
            })
            .collect();
        assert_eq!(
            predicates,
            [
                "http://example.org/a",
                "http://example.org/b",
                "http://example.org/c",
                "http://example.org/d",
                "http://example.org/e",
            ]
        );
    }

    /// `GRAPH` is optional in `ConstructQuadsNotTriples`: a bare nested `{ … }`
    /// block is the DEFAULT graph, spelled as a block.
    #[test]
    fn construct_admits_a_bare_nested_block_as_the_default_graph() {
        let template = construct_template(
            "CONSTRUCT { { ?s <http://example.org/a> ?o } GRAPH <http://example.org/g> \
             { ?s <http://example.org/b> ?o } } WHERE { ?s ?p ?o }",
        );
        assert_eq!(
            template_graphs(&template),
            vec![None, Some("<http://example.org/g>".to_owned())]
        );
    }

    // ── The `CONSTRUCT GRAPH …` whole-template shorthand ─────────────────────

    #[test]
    fn construct_graph_shorthand_scopes_the_whole_template() {
        let q = format!(
            "{GM}CONSTRUCT GRAPH <http://example.org/g> {{ ?s a purrdf:Out . ?s a purrdf:In }} \
             WHERE {{ ?s a purrdf:In }}"
        );
        let template = construct_template(&q);
        assert_eq!(template.len(), 2);
        assert_eq!(
            template_graphs(&template),
            vec![
                Some("<http://example.org/g>".to_owned()),
                Some("<http://example.org/g>".to_owned())
            ]
        );
    }

    /// A prefixed name and a `BASE`-relative reference are both legal graph
    /// names, resolved by the ordinary IRI production.
    #[test]
    fn construct_graph_accepts_prefixed_and_relative_graph_names() {
        let q = format!("{GM}CONSTRUCT GRAPH purrdf:g {{ ?s ?p ?o }} WHERE {{ ?s ?p ?o }}");
        // `GM` binds `purrdf:` to `<https://x/>`.
        assert_eq!(
            template_graphs(&construct_template(&q)),
            vec![Some("<https://x/g>".to_owned())]
        );

        let q = "BASE <http://example.org/> CONSTRUCT GRAPH <g> { ?s ?p ?o } WHERE { ?s ?p ?o }";
        assert_eq!(
            template_graphs(&construct_template(q)),
            vec![Some("<http://example.org/g>".to_owned())]
        );
    }

    /// The shorthand is a DEFAULT, not an override: an inner `GRAPH` block wins
    /// over it, and only the slots that named no graph take the shorthand's.
    #[test]
    fn construct_graph_shorthand_yields_to_an_inner_graph_block() {
        let template = construct_template(
            "CONSTRUCT GRAPH <http://example.org/outer> { ?s <http://example.org/a> ?o . \
             GRAPH <http://example.org/inner> { ?s <http://example.org/b> ?o } } \
             WHERE { ?s ?p ?o }",
        );
        assert_eq!(
            template_graphs(&template),
            vec![
                Some("<http://example.org/outer>".to_owned()),
                Some("<http://example.org/inner>".to_owned()),
            ]
        );
    }

    /// The short form (§16.2.1, template ≡ WHERE block) takes the shorthand
    /// too: `GRAPH VarOrIri` is read before the short/long fork, so both
    /// spellings reach the same algebra node.
    #[test]
    fn construct_graph_works_with_the_short_form() {
        let template =
            construct_template("CONSTRUCT GRAPH <http://example.org/g> WHERE { ?s ?p ?o }");
        assert_eq!(template.len(), 1, "the short form's template IS the block");
        assert_eq!(
            template_graphs(&template),
            vec![Some("<http://example.org/g>".to_owned())]
        );

        // Including with a variable graph name. (The short form's block is a
        // `TriplesTemplate`, so `?g` can only be bound by a solution modifier's
        // scope here — the parse is what this pins.)
        let template = construct_template("CONSTRUCT GRAPH ?g WHERE { ?s ?p ?o }");
        assert_eq!(template_graphs(&template), vec![Some("?g".to_owned())]);
    }

    // ── Negative syntax pins ─────────────────────────────────────────────────

    /// The shorthand's `GRAPH` must be followed by a graph NAME; a bare
    /// `CONSTRUCT GRAPH { … }` is a syntax error rather than a
    /// silently-default-graph CONSTRUCT. (The optional-`GRAPH` bare block form
    /// lives INSIDE the template braces, not before them, so this spelling
    /// stays unambiguous.)
    #[test]
    fn construct_graph_shorthand_without_a_name_is_refused() {
        SparqlParser::new()
            .parse_query("CONSTRUCT GRAPH { ?s ?p ?o } WHERE { ?s ?p ?o }")
            .expect_err("the CONSTRUCT GRAPH shorthand requires a graph name");
    }

    /// The upstream grammar's graph name is a `VarOrIri`: a literal, a blank
    /// node and a triple term are all refused.
    #[test]
    fn construct_template_graph_name_must_be_a_var_or_iri() {
        for q in [
            r#"CONSTRUCT { GRAPH "g" { ?s ?p ?o } } WHERE { ?s ?p ?o }"#,
            "CONSTRUCT { GRAPH _:g { ?s ?p ?o } } WHERE { ?s ?p ?o }",
            "CONSTRUCT { GRAPH <<( ?a ?b ?c )>> { ?s ?p ?o } } WHERE { ?s ?p ?o }",
            r#"CONSTRUCT GRAPH "g" { ?s ?p ?o } WHERE { ?s ?p ?o }"#,
        ] {
            SparqlParser::new()
                .parse_query(q)
                .expect_err("a CONSTRUCT template graph name must be an IRI or a variable");
        }
    }

    /// `ConstructQuadsNotTriples` blocks do not nest, and the short form's
    /// block is a `TriplesTemplate` — it is read twice, once as the template and
    /// once as the `WHERE` BGP, which has no graph slot to carry a scope into.
    /// Both are refused, the short form by name.
    #[test]
    fn construct_graph_blocks_are_refused_where_the_grammar_has_none() {
        let err = SparqlParser::new()
            .parse_query("CONSTRUCT WHERE { GRAPH <http://example.org/g> { ?s ?p ?o } }")
            .expect_err("the CONSTRUCT short form admits no GRAPH block");
        assert!(
            format!("{err}").contains("short form"),
            "the diagnostic must name the short form, got: {err}"
        );

        SparqlParser::new()
            .parse_query(
                "CONSTRUCT { GRAPH <http://example.org/g> { GRAPH <http://example.org/h> \
                 { ?s ?p ?o } } } WHERE { ?s ?p ?o }",
            )
            .expect_err("a graph block does not nest inside another graph block");
    }

    /// A property path and a property function are no more assertable inside a
    /// `GRAPH` block than they are at the template's top level.
    #[test]
    fn construct_graph_block_still_refuses_paths() {
        let err = SparqlParser::new()
            .parse_query(
                "CONSTRUCT { GRAPH <http://example.org/g> { ?s <http://example.org/p>+ ?o } } \
                 WHERE { ?s ?p ?o }",
            )
            .expect_err("a property path is not assertable in a template");
        assert!(format!("{err}").contains("property paths"), "got: {err}");
    }

    /// Adding the quad template must not perturb any OTHER CONSTRUCT spelling:
    /// the dataset clause, the WHERE-less short form, and the solution
    /// modifiers all parse to exactly the algebra they did before, with every
    /// template slot unscoped.
    #[test]
    fn plain_construct_spellings_are_unchanged_and_name_no_graph() {
        for q in [
            "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
            "CONSTRUCT { ?s ?p ?o } FROM <http://example.org/d> WHERE { ?s ?p ?o }",
            "CONSTRUCT WHERE { ?s ?p ?o }",
            "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o } ORDER BY ?s LIMIT 3",
        ] {
            let template = construct_template(q);
            assert!(
                template.iter().all(|quad| quad.graph.is_none()),
                "`{q}` must name no graph on any template slot"
            );
        }
    }

    #[test]
    fn distinct_and_order_by_and_slice() {
        let q = format!(
            "{GM}SELECT DISTINCT ?a WHERE {{ ?a a purrdf:T }} ORDER BY ?a LIMIT 5 OFFSET 2"
        );
        let p = select_pattern(&q);
        // Distinct wraps Project; Slice is the outermost? Order: Project → Distinct → Slice.
        let GraphPattern::Slice {
            inner,
            start,
            length,
        } = p
        else {
            panic!("expected Slice outermost, got {p:?}");
        };
        assert_eq!(start, 2);
        assert_eq!(length, Some(5));
        assert!(matches!(*inner, GraphPattern::Distinct { .. }));
    }

    #[test]
    fn select_star_collects_visible_vars() {
        let q = format!("{GM}SELECT * WHERE {{ ?a purrdf:p ?b . }}");
        let GraphPattern::Project { variables, .. } = select_pattern(&q) else {
            panic!("expected Project");
        };
        assert_eq!(variables, vec![Variable::new("a"), Variable::new("b")]);
    }

    #[test]
    fn from_clause_parses_into_query_dataset() {
        let q = format!(
            "{GM}SELECT ?a FROM <http://g/> FROM NAMED <http://n/> WHERE {{ ?a a purrdf:T }}"
        );
        let Query::Select { dataset, .. } = parse(&q) else {
            panic!("expected SELECT");
        };
        assert_eq!(dataset.default.len(), 1);
        assert_eq!(dataset.default[0].as_str(), "http://g/");
        assert_eq!(dataset.named.len(), 1);
        assert_eq!(dataset.named[0].as_str(), "http://n/");
    }

    #[test]
    fn no_dataset_clause_is_empty() {
        let q = format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }}");
        let Query::Select { dataset, .. } = parse(&q) else {
            panic!("expected SELECT");
        };
        assert!(dataset.default.is_empty() && dataset.named.is_empty());
    }

    #[test]
    fn version_basic_parses_to_typed_and_byte_exact_raw() {
        let q = "PREFIX : <http://example/>\nVERSION \"1.2-basic\"\n\nSELECT * { ?s ?p ?o . }";
        let query = SparqlParser::new().parse_query(q).expect("parse");
        let version = query.version().expect("VERSION declared");
        assert_eq!(*version, SparqlVersion::V12Basic);
        assert_eq!(version.raw(), "1.2-basic");
        assert!(version.is_recognized());
    }

    #[test]
    fn version_repeated_declarations_last_wins() {
        // Mirrors the vendored W3C `w3c-sparql12` `version-06.rq` shape: three
        // `VERSION` declarations interleaved with `PREFIX`.
        let q = "VERSION \"1.2\"\nPREFIX : <http://example/>\nVERSION \"1.2-basic\"\nVERSION \"1.2\"\n\nSELECT * { ?s ?p ?o . }";
        let query = SparqlParser::new().parse_query(q).expect("parse");
        assert_eq!(query.version(), Some(&SparqlVersion::V12));
    }

    #[test]
    fn version_arbitrary_string_is_a_syntax_only_accept() {
        // Mirrors the vendored W3C `w3c-sparql12` `version-04.rq` shape: an
        // unrecognized version string is a `PositiveSyntaxTest` — parsing accepts
        // any string; recognition is enforced only at evaluation admission.
        let q = "PREFIX : <http://example/>\nVERSION \"1.1\"\n\nSELECT * { ?s ?p ?o . }";
        let query = SparqlParser::new().parse_query(q).expect("parse");
        let version = query.version().expect("VERSION declared");
        assert_eq!(*version, SparqlVersion::Other("1.1".to_owned()));
        assert_eq!(version.raw(), "1.1");
        assert!(!version.is_recognized());
    }

    #[test]
    fn version_absent_is_none() {
        let q = format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }}");
        assert_eq!(parse(&q).version(), None);
    }

    #[test]
    fn undeclared_prefix_is_syntax_error() {
        let err = SparqlParser::new()
            .parse_query("SELECT ?a WHERE { ?a a nope:T }")
            .unwrap_err();
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn trailing_tokens_rejected() {
        let q =
            format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }} SELECT ?b WHERE {{ ?b a purrdf:U }}");
        assert!(SparqlParser::new().parse_query(&q).is_err());
    }

    #[test]
    fn trailing_values_clause_is_accepted() {
        // §18.2.4.3: a `VALUES DataBlock` after the WHERE / solution modifiers,
        // both at the top level and on a SubSelect.
        let q =
            format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }} VALUES ?a {{ purrdf:x purrdf:y }}");
        assert!(
            matches!(parse(&q), Query::Select { .. }),
            "trailing top-level VALUES must parse"
        );
        let q2 = format!(
            "{GM}SELECT ?s ?o WHERE {{ {{ SELECT * WHERE {{ ?s ?p ?o }} VALUES (?o) {{ (purrdf:b) }} }} }}"
        );
        assert!(
            matches!(parse(&q2), Query::Select { .. }),
            "trailing VALUES on a sub-select must parse"
        );
    }
    #[test]
    fn custom_function_arg_aggregate_reaches_group() {
        // `purrdf:fn(COUNT(?x))` was discarding the COUNT into a
        // throwaway Vec rather than threading it through to the Group.  The
        // algebra must have a Group whose aggregates list is non-empty.
        let q =
            format!("{GM}SELECT ?t (purrdf:fn(COUNT(?x)) AS ?n) WHERE {{ ?x a ?t }} GROUP BY ?t");
        let where_pat = unproject(select_pattern(&q));
        // Outermost is Extend (for the AS ?n binding).
        let GraphPattern::Extend {
            inner, variable, ..
        } = where_pat
        else {
            panic!("expected Extend, got {where_pat:?}");
        };
        assert_eq!(variable, Variable::new("n"));
        // Inner is the Group node.
        let GraphPattern::Group {
            variables,
            aggregates,
            ..
        } = *inner
        else {
            panic!("expected Group under Extend, got {inner:?}");
        };
        assert_eq!(variables, vec![Variable::new("t")]);
        // The COUNT aggregate must have been collected — not discarded.
        assert_eq!(
            aggregates.len(),
            1,
            "COUNT aggregate was silently discarded (G3); aggregates = {aggregates:?}"
        );
        assert!(
            matches!(
                &aggregates[0].1,
                AggregateExpression {
                    function: AggregateFunction::Count,
                    ..
                }
            ),
            "expected COUNT aggregate, got {:?}",
            aggregates[0].1
        );
    }

    #[test]
    fn aggregate_in_no_group_position_is_unsupported() {
        // An aggregate in a plain FILTER (no GROUP BY) must still be rejected.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x a purrdf:T . FILTER(COUNT(?x) > 0) }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Unsupported(_)),
            "expected Unsupported for aggregate in filter position, got {err:?}"
        );
    }

    // ── AGG(<iri>, …) custom-aggregate surface ──────────────────────────────

    #[test]
    fn agg_call_single_arg_parses_to_custom_aggregate() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, ?x) AS ?a) WHERE {{ ?x a ?t }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates.len(), 1);
        let agg = &aggregates[0].1;
        assert!(
            matches!(&agg.function, AggregateFunction::Custom(n) if n.as_str() == "http://ex/myAgg"),
            "expected Custom(<http://ex/myAgg>), got {:?}",
            agg.function
        );
        assert_eq!(agg.args.len(), 1);
        assert!(!agg.distinct);
        assert!(agg.scalarvals.is_empty());
    }

    #[test]
    fn agg_call_distinct_multi_arg_parses() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, DISTINCT ?x, ?y) AS ?a) \
             WHERE {{ ?x a ?t . ?x purrdf:vantage ?y }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates.len(), 1);
        let agg = &aggregates[0].1;
        assert!(
            matches!(&agg.function, AggregateFunction::Custom(n) if n.as_str() == "http://ex/myAgg")
        );
        assert_eq!(agg.args.len(), 2);
        assert!(agg.distinct);
    }

    #[test]
    fn agg_call_accepts_a_prefixed_name_iri() {
        // `<iri>` may be any IRI, including a prefixed name resolved against the
        // query's prologue, retained byte-exact via `expect_iri_node`.
        let q =
            format!("{GM}SELECT ?t (AGG(purrdf:myAgg, ?x) AS ?a) WHERE {{ ?x a ?t }} GROUP BY ?t");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert!(matches!(
            &aggregates[0].1.function,
            AggregateFunction::Custom(n) if n.as_str() == "https://x/myAgg"
        ));
    }

    #[test]
    fn agg_call_requires_at_least_one_argument() {
        let q =
            format!("{GM}SELECT ?t (AGG(<http://ex/myAgg>) AS ?a) WHERE {{ ?x a ?t }} GROUP BY ?t");
        assert!(SparqlParser::new().parse_query(&q).is_err());
    }

    /// `AGG(<iri>, arg; NAME=value)` — the named scalarval clause — populates
    /// [`AggregateExpression::scalarvals`] with the upper-cased name and the
    /// literal's natural (here, decimal) datatype, exactly the surface
    /// `PERCENTILE`'s `p`/`TOPK`'s `k` are meant to reach the evaluator through.
    #[test]
    fn agg_call_named_scalarval_populates_scalarvals() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, ?v; PERCENTILE = 0.95) AS ?a) \
             WHERE {{ ?x a ?t ; purrdf:vantage ?v }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates.len(), 1);
        let agg = &aggregates[0].1;
        assert_eq!(
            agg.args.len(),
            1,
            "PERCENTILE=0.95 is a scalarval, not a positional arg"
        );
        assert_eq!(agg.scalarvals.len(), 1);
        assert_eq!(agg.scalarvals[0].0, "PERCENTILE");
        assert_eq!(agg.scalarvals[0].1.value(), "0.95");
        assert_eq!(
            agg.scalarvals[0].1.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#decimal"
        );
    }

    /// The scalarval NAME is matched case-insensitively and stored upper-cased —
    /// mirroring `SEPARATOR`'s own case-insensitive keyword match.
    #[test]
    fn agg_call_scalarval_name_is_upper_cased() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, ?v; p=1) AS ?a) \
             WHERE {{ ?x a ?t ; purrdf:vantage ?v }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates[0].1.scalarvals[0].0, "P");
    }

    /// Multiple `; NAME=value` clauses all parse, in the order written.
    #[test]
    fn agg_call_multiple_scalarvals_parse_in_order() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, ?v; K=3; LABEL=\"x\") AS ?a) \
             WHERE {{ ?x a ?t ; purrdf:vantage ?v }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        let scalarvals = &aggregates[0].1.scalarvals;
        assert_eq!(scalarvals.len(), 2);
        assert_eq!(scalarvals[0].0, "K");
        assert_eq!(scalarvals[0].1.value(), "3");
        assert_eq!(scalarvals[1].0, "LABEL");
        assert_eq!(scalarvals[1].1.value(), "x");
    }

    /// `value` in `; NAME=value` is any SPARQL literal (§20.3), including the
    /// signed halves of the numeric tower and the boolean literals — not just
    /// the unsigned-numeral/string subset `parse_literal` used to accept.
    #[test]
    fn agg_call_scalarval_accepts_signed_numerals_and_booleans() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, ?v; Q=-1; P=+0.5; B=true) AS ?a) \
             WHERE {{ ?x a ?t ; purrdf:vantage ?v }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        let scalarvals = &aggregates[0].1.scalarvals;
        assert_eq!(scalarvals.len(), 3);
        assert_eq!(scalarvals[0].0, "Q");
        assert_eq!(scalarvals[0].1.value(), "-1");
        assert_eq!(
            scalarvals[0].1.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#integer"
        );
        assert_eq!(scalarvals[1].0, "P");
        assert_eq!(scalarvals[1].1.value(), "+0.5");
        assert_eq!(
            scalarvals[1].1.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#decimal"
        );
        assert_eq!(scalarvals[2].0, "B");
        assert_eq!(scalarvals[2].1.value(), "true");
        assert_eq!(
            scalarvals[2].1.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#boolean"
        );
    }

    /// A bare sign with nothing numeric behind it is refused, not silently
    /// dropped or mis-parsed.
    #[test]
    fn agg_call_scalarval_bare_sign_is_a_syntax_error() {
        let q = format!(
            "{GM}SELECT ?t (AGG(<http://ex/myAgg>, ?v; Q=-true) AS ?a) \
             WHERE {{ ?x a ?t ; purrdf:vantage ?v }} GROUP BY ?t"
        );
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(matches!(err, ParseError::Syntax { .. }));
    }

    /// `VALUES` ground terms admit the same signed-numeral/boolean grammar as
    /// any other literal position — the class this fix closes, not just the
    /// `AGG` scalarval instance of it.
    #[test]
    fn values_ground_terms_accept_signed_numerals_and_booleans() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?s ?p ?o }} VALUES ?x {{ -1 +0.5 true false }}");
        let where_pat = select_pattern(&q);
        let GraphPattern::Project { inner, .. } = where_pat else {
            panic!("expected Project, got {where_pat:?}");
        };
        let GraphPattern::Join { right, .. } = *inner else {
            panic!("expected the trailing VALUES joined in, got {inner:?}");
        };
        let GraphPattern::Values { bindings, .. } = *right else {
            panic!("expected Values, got {right:?}");
        };
        assert_eq!(bindings.len(), 4);
        let Some(GroundTerm::Literal(l0)) = &bindings[0][0] else {
            panic!("expected a literal binding, got {:?}", bindings[0][0]);
        };
        assert_eq!(l0.value(), "-1");
        let Some(GroundTerm::Literal(l1)) = &bindings[1][0] else {
            panic!("expected a literal binding, got {:?}", bindings[1][0]);
        };
        assert_eq!(l1.value(), "+0.5");
        let Some(GroundTerm::Literal(l2)) = &bindings[2][0] else {
            panic!("expected a literal binding, got {:?}", bindings[2][0]);
        };
        assert_eq!(l2.value(), "true");
        assert_eq!(
            l2.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#boolean"
        );
        let Some(GroundTerm::Literal(l3)) = &bindings[3][0] else {
            panic!("expected a literal binding, got {:?}", bindings[3][0]);
        };
        assert_eq!(l3.value(), "false");
    }

    /// A triple pattern's object is the same ground-literal grammar too: a
    /// signed numeral parses exactly as it does inside `VALUES`.
    #[test]
    fn triple_pattern_object_accepts_a_signed_numeral() {
        let q = format!("{GM}SELECT ?s WHERE {{ ?s purrdf:p -3 }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Bgp { patterns } = where_pat else {
            panic!("expected a BGP, got {where_pat:?}");
        };
        assert_eq!(patterns.len(), 1);
        let TermPattern::Literal(l) = &patterns[0].object else {
            panic!("expected a literal object, got {:?}", patterns[0].object);
        };
        assert_eq!(l.value(), "-3");
        assert_eq!(
            l.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#integer"
        );
    }

    #[test]
    fn count_star_has_empty_args() {
        let q = format!("{GM}SELECT ?t (COUNT(*) AS ?c) WHERE {{ ?x a ?t }} GROUP BY ?t");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates.len(), 1);
        assert!(
            aggregates[0].1.args.is_empty(),
            "COUNT(*) must have the spec's empty exprlist, got {:?}",
            aggregates[0].1.args
        );
        assert!(matches!(aggregates[0].1.function, AggregateFunction::Count));
    }

    /// `'*'` is the spec's empty exprlist, and the grammar admits it in exactly one
    /// production: `Count` (SPARQL 1.1 §18.5.1 / SPARQL 1.2 §19.8). Every other
    /// built-in aggregate is fixed-arity one, so `SUM(*)`/`AVG(*)`/`MIN(*)`/`MAX(*)`/
    /// `SAMPLE(*)`/`GROUP_CONCAT(*)` must all be hard syntax errors — never a silent
    /// row count (the shipped-CLI regression this guards against).
    #[test]
    fn star_is_rejected_for_every_non_count_aggregate() {
        for name in ["SUM", "AVG", "MIN", "MAX", "SAMPLE", "GROUP_CONCAT"] {
            let q = format!("{GM}SELECT ?t ({name}(*) AS ?a) WHERE {{ ?x a ?t }} GROUP BY ?t");
            let err = SparqlParser::new().parse_query(&q).unwrap_err();
            assert!(
                matches!(err, ParseError::Syntax { .. }),
                "{name}(*) must be a syntax error, got {err:?}"
            );
        }
    }

    /// `COUNT(DISTINCT *)` is the star form; `COUNT(*)` is covered by
    /// `count_star_has_empty_args`.
    #[test]
    fn count_distinct_star_still_parses() {
        let q = format!("{GM}SELECT ?t (COUNT(DISTINCT *) AS ?c) WHERE {{ ?x a ?t }} GROUP BY ?t");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates.len(), 1);
        assert!(aggregates[0].1.args.is_empty());
        assert!(aggregates[0].1.distinct);
        assert!(matches!(aggregates[0].1.function, AggregateFunction::Count));
    }

    #[test]
    fn group_concat_separator_is_a_scalarval() {
        let q = format!(
            "{GM}SELECT ?t (GROUP_CONCAT(?x; SEPARATOR=\"|\") AS ?g) WHERE {{ ?x a ?t }} GROUP BY ?t"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { inner, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        let GraphPattern::Group { aggregates, .. } = *inner else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates[0].1.separator(), Some("|"));
    }

    // ── Bounded repetition {n,m} + predicate wildcard (PurRDF extensions) ──

    fn path_of(q: &str) -> PropertyPathExpression {
        match unproject(select_pattern(q)) {
            GraphPattern::Path { path, .. } => path,
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn property_path_bounded_range() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,3}} ?y . }}");
        assert!(matches!(
            path_of(&q),
            PropertyPathExpression::Range {
                min: 1,
                max: Some(3),
                ..
            }
        ));
    }

    #[test]
    fn property_path_exact_repetition() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2}} ?y . }}");
        assert!(matches!(
            path_of(&q),
            PropertyPathExpression::Range {
                min: 2,
                max: Some(2),
                ..
            }
        ));
    }

    #[test]
    fn property_path_at_least_n() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2,}} ?y . }}");
        assert!(matches!(
            path_of(&q),
            PropertyPathExpression::Range {
                min: 2,
                max: None,
                ..
            }
        ));
    }

    #[test]
    fn property_path_range_round_trips_through_display() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,3}} ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "<https://x/p>{1,3}");
        // Re-parse the serialized surface → the same algebra node.
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn property_path_inverted_range_is_a_hard_error() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2,1}} ?y . }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            err.to_string().contains("exceeds upper bound"),
            "expected a min>max hard error, got {err}"
        );
    }

    #[test]
    fn property_path_empty_range_is_a_hard_error() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{}} ?y . }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            err.to_string().contains("empty path range"),
            "expected an empty-range hard error, got {err}"
        );
    }

    #[test]
    fn property_path_both_bounds_absent_range_is_a_hard_error() {
        // `{,}` with BOTH bounds absent must hard-fail — it is NOT a silent `*`.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{,}} ?y . }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            err.to_string().contains("empty path range {,}"),
            "expected a {{,}} hard error, got {err}"
        );
    }

    #[test]
    fn property_path_partial_bounds_still_parse() {
        // `{n}`, `{n,}`, `{,m}`, `{n,m}` must all still succeed.
        let cases = [
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2}} ?y . }}"),
                "<https://x/p>{2}",
            ),
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,}} ?y . }}"),
                "<https://x/p>{1,}",
            ),
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{,2}} ?y . }}"),
                "<https://x/p>{0,2}",
            ),
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,3}} ?y . }}"),
                "<https://x/p>{1,3}",
            ),
        ];
        for (q, expected_display) in &cases {
            let path = path_of(q);
            assert_eq!(
                path.to_string(),
                *expected_display,
                "path range failed to parse correctly for input: {q}"
            );
        }
    }

    #[test]
    fn property_path_unterminated_range_is_a_hard_error() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1 ?y . }}");
        assert!(
            SparqlParser::new().parse_query(&q).is_err(),
            "an unterminated path range must hard-fail"
        );
    }

    #[test]
    fn predicate_wildcard_serializes_emit_only() {
        // The wildcard is emit-only: the grammar has no production for `<any>` /
        // `<any:ns>`, so it can only be built through the algebra API (as here)
        // and serialized via `Display`, never produced by parsing query text.
        let any = PropertyPathExpression::Wildcard { namespace: None };
        assert_eq!(any.to_string(), "<any>");
        let scoped = PropertyPathExpression::Wildcard {
            namespace: Some(NamedNode::new_unchecked("https://x/org/")),
        };
        assert_eq!(scoped.to_string(), "<any:https://x/org/>");
    }

    #[test]
    fn star_over_grouped_sequence_round_trips_with_parens() {
        // Display must re-parenthesize a compound operand under a postfix operator.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (purrdf:p/purrdf:q){{1,2}} ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(<https://x/p>/<https://x/q>){1,2}");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    // CR6: postfix quantifier over an inverse path must parenthesize the inverse
    // so that Display + re-parse preserves the original AST.
    //
    // Before the fix `ZeroOrMore(Reverse(p))` serialised as `^<p>*`, which
    // reparses as `Reverse(ZeroOrMore(p))` — the nesting is inverted.  The
    // corrected form is `(^<p>)*`.

    #[test]
    fn zero_or_more_over_inverse_round_trips_with_parens() {
        // Parse `(^purrdf:p)*`  →  ZeroOrMore(Reverse(NamedNode(p)))
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p)* ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>)*");
        // Re-parse the serialised surface — must give the identical algebra node.
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn one_or_more_over_inverse_round_trips_with_parens() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p)+ ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>)+");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn zero_or_one_over_inverse_round_trips_with_parens() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p)? ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>)?");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn range_over_inverse_round_trips_with_parens() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p){{1,2}} ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>){1,2}");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    // ── SPARQL 1.1 Update parsing ─────────────────────────────────────────────

    fn parse_update(u: &str) -> Update {
        SparqlParser::new()
            .parse_update(&format!("{GM}{u}"))
            .expect("update parse")
    }

    fn update_err(u: &str) -> ParseError {
        SparqlParser::new()
            .parse_update(&format!("{GM}{u}"))
            .expect_err("update should fail")
    }

    #[test]
    fn update_retains_version_declaration() {
        // The prologue-parsing path is shared with queries (`parse_prologue`); an
        // Update request's own `VERSION` declaration is retained the same way.
        let u = SparqlParser::new()
            .parse_update(&format!(
                "VERSION \"1.2-basic\"\n{GM}INSERT DATA {{ purrdf:s purrdf:p purrdf:o }}"
            ))
            .expect("update parse");
        assert_eq!(u.version(), Some(&SparqlVersion::V12Basic));
    }

    #[test]
    fn update_with_no_version_is_none() {
        let u = parse_update("INSERT DATA { purrdf:s purrdf:p purrdf:o }");
        assert_eq!(u.version(), None);
    }

    #[test]
    fn update_insert_data() {
        let u = parse_update("INSERT DATA { purrdf:s purrdf:p purrdf:o }");
        assert_eq!(u.operations.len(), 1);
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData, got {:?}", u.operations[0]);
        };
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].graph, None);
    }

    #[test]
    fn update_insert_data_with_graph() {
        let u = parse_update("INSERT DATA { GRAPH purrdf:g { purrdf:s purrdf:p purrdf:o } }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
        assert_eq!(
            data[0].graph,
            Some(NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                "https://x/g"
            )))
        );
    }

    #[test]
    fn update_insert_data_quoted_triple() {
        // RDF 1.2 INSERT DATA with a quoted-triple object survives as a TermPattern.
        let u =
            parse_update("INSERT DATA { purrdf:s rdf:reifies <<( purrdf:a purrdf:b purrdf:c )>> }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
        assert!(matches!(data[0].triple.object, TermPattern::Triple(_)));
    }

    #[test]
    fn update_insert_data_blank_node_is_allowed() {
        // Blank nodes ARE standard in INSERT DATA (§3.1.1, minted fresh per request).
        let u = parse_update("INSERT DATA { [] purrdf:p purrdf:o }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
        assert!(matches!(data[0].triple.subject, TermPattern::BlankNode(_)));
    }

    #[test]
    fn update_reused_blank_label_across_operations_is_rejected() {
        // §19.6: a blank node label is scoped to one operation — sharing `_:b1`
        // across two INSERT DATA operations of a request is illegal (vendored
        // W3C `syntax-update-1` `syntax-update-54`).
        let err = update_err(
            "INSERT DATA { _:b1 purrdf:p purrdf:o } ; INSERT DATA { _:b1 purrdf:p purrdf:o }",
        );
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "expected Syntax for reused blank label across operations, got {err:?}"
        );
        // The same label WITHIN one operation is fine (one blank node), and a
        // fresh label per operation is fine.
        parse_update("INSERT DATA { _:b1 purrdf:p _:b1 } ; INSERT DATA { _:b2 purrdf:p purrdf:o }");
    }

    #[test]
    fn update_reused_blank_label_inside_quoted_triple_across_operations_is_rejected() {
        // §19.6 still applies when the blank label is nested inside an RDF 1.2
        // quoted triple term: reusing `_:b` across two INSERT DATA operations is
        // illegal even though the label never appears at top level. This exercises
        // the `TermPattern::Triple` descent in `collect_term_bnode_labels`.
        let err = update_err(concat!(
            "INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p purrdf:o )>> } ; ",
            "INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p purrdf:o )>> }",
        ));
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "expected Syntax for reused blank label inside quoted triple across operations, got {err:?}"
        );
    }

    #[test]
    fn update_blank_label_inside_quoted_triple_within_one_operation_is_allowed() {
        // The same blank label confined to a single operation is one blank node —
        // nesting it inside a quoted triple must not trigger a false rejection.
        parse_update(concat!(
            "INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p _:b )>> } ; ",
            "INSERT DATA { purrdf:s rdf:reifies <<( _:c purrdf:p purrdf:o )>> }",
        ));
    }

    #[test]
    fn update_insert_data_labeled_blank_node_is_allowed() {
        let u = parse_update("INSERT DATA { _:b purrdf:p purrdf:o }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert!(matches!(data[0].triple.subject, TermPattern::BlankNode(_)));
    }

    #[test]
    fn update_blank_in_insert_data_quoted_triple_is_allowed() {
        // A blank node nested inside a quoted triple in INSERT DATA is still allowed.
        let u = parse_update("INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p purrdf:o )>> }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
    }

    #[test]
    fn update_delete_data() {
        let u = parse_update("DELETE DATA { purrdf:s purrdf:p purrdf:o }");
        assert!(matches!(
            u.operations[0],
            GraphUpdateOperation::DeleteData { .. }
        ));
    }

    #[test]
    fn update_delete_where() {
        let u = parse_update("DELETE WHERE { ?s purrdf:p ?o }");
        let GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            pattern,
            ..
        } = &u.operations[0]
        else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(delete.len(), 1);
        assert!(insert.is_empty());
        // The template IS the where pattern.
        assert!(matches!(**pattern, GraphPattern::Bgp { .. }));
    }

    #[test]
    fn delete_where_template_refuses_lateral() {
        // `DELETE WHERE { … }`'s braces are parsed TWICE: once as a quad
        // TEMPLATE (the delete side, via `parse_quad_pattern_block`) and once
        // as an ordinary group graph pattern (the WHERE side — see
        // `insert_where_lateral_parses_and_scope_checks`, where LATERAL is
        // legal). A template has no group-pattern operators at all — no
        // `OPTIONAL`/`FILTER`/`GRAPH`-as-a-group either — so `LATERAL` there
        // must be refused with a clear, named message (the same idiom already
        // used for property paths and property-function calls in
        // `parse_template_triple`) rather than a confusing "expected a term"
        // subject-parse error.
        let err = update_err("DELETE WHERE { ?s purrdf:p ?o LATERAL { ?o purrdf:q ?z } }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("LATERAL is not allowed in an update template")),
            "got {err:?}"
        );
    }

    #[test]
    fn insert_where_lateral_parses_and_scope_checks() {
        // `INSERT … WHERE`, `DELETE … WHERE` (the template form, not the
        // `DELETE WHERE` shorthand above), and `WITH … WHERE` all route their
        // WHERE clause through the SAME `parse_group_graph_pattern` dispatch
        // a SELECT's WHERE does — the shared arm that recognizes `LATERAL`
        // (`parse_insert`/`parse_delete`/`parse_with_modify` each call
        // `self.parse_group_graph_pattern()` for their WHERE, with no
        // template-only restriction). Positive: it parses to the same
        // `Lateral` node a SELECT's WHERE would.
        let u = parse_update(
            "INSERT { ?s purrdf:q ?label } \
             WHERE { ?s purrdf:p ?o LATERAL { ?o purrdf:label ?label } }",
        );
        let GraphUpdateOperation::DeleteInsert { pattern, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert!(
            matches!(**pattern, GraphPattern::Lateral { .. }),
            "an UPDATE WHERE clause must produce the same Lateral node a \
             SELECT's WHERE does: {pattern:?}"
        );

        // Negative: the SAME scope-conflict check (Jena's `SyntaxVarScope`,
        // which runs on UPDATE WHERE clauses too) must run here — a BIND
        // target already in scope on LATERAL's left is refused, naming the
        // variable, exactly as it would inside a SELECT.
        let err = update_err(
            "INSERT { ?s purrdf:q ?o } WHERE { ?s purrdf:p ?o LATERAL { BIND(1 AS ?o) } }",
        );
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("BIND target ?o inside LATERAL is already in scope")),
            "got {err:?}"
        );
    }

    #[test]
    fn update_delete_insert_modify() {
        let u = parse_update(
            "DELETE { ?s purrdf:p ?o } INSERT { ?s purrdf:q ?o } WHERE { ?s purrdf:p ?o }",
        );
        let GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            ..
        } = &u.operations[0]
        else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(delete.len(), 1);
        assert_eq!(insert.len(), 1);
        assert!(with.is_none());
        assert!(using.is_empty());
    }

    #[test]
    fn update_insert_only_modify() {
        let u = parse_update("INSERT { ?s purrdf:q purrdf:o } WHERE { ?s a purrdf:T }");
        let GraphUpdateOperation::DeleteInsert { delete, insert, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert!(delete.is_empty());
        assert_eq!(insert.len(), 1);
    }

    #[test]
    fn update_with_modify() {
        let u = parse_update(
            "WITH purrdf:g DELETE { ?s purrdf:p ?o } INSERT { ?s purrdf:q ?o } WHERE { ?s purrdf:p ?o }",
        );
        let GraphUpdateOperation::DeleteInsert { with, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(*with, Some(NamedNode::new_unchecked("https://x/g")));
    }

    #[test]
    fn update_using_clauses() {
        let u = parse_update(
            "DELETE { ?s purrdf:p ?o } USING purrdf:g1 USING NAMED purrdf:g2 WHERE { ?s purrdf:p ?o }",
        );
        let GraphUpdateOperation::DeleteInsert { using, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(using.len(), 2);
        // The NAMED modifier is preserved (USING <g1> vs USING NAMED <g2>).
        assert!(matches!(&using[0], UsingClause::Default(n) if n.as_str() == "https://x/g1"));
        assert!(matches!(&using[1], UsingClause::Named(n) if n.as_str() == "https://x/g2"));
    }

    #[test]
    fn update_load() {
        let u = parse_update("LOAD <http://src/data> INTO GRAPH purrdf:g");
        let GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        } = &u.operations[0]
        else {
            panic!("expected Load");
        };
        assert!(!silent);
        assert_eq!(source.as_str(), "http://src/data");
        assert_eq!(
            *destination,
            GraphTarget::Named(NamedNode::new_unchecked("https://x/g"))
        );
    }

    #[test]
    fn update_load_silent_default_destination() {
        let u = parse_update("LOAD SILENT <http://src/data>");
        let GraphUpdateOperation::Load {
            silent,
            destination,
            ..
        } = &u.operations[0]
        else {
            panic!("expected Load");
        };
        assert!(silent);
        assert_eq!(*destination, GraphTarget::Default);
    }

    #[test]
    fn update_clear_each_target() {
        for (text, expected) in [
            ("CLEAR DEFAULT", GraphTarget::Default),
            ("CLEAR NAMED", GraphTarget::NamedGraphs),
            ("CLEAR ALL", GraphTarget::All),
            (
                "CLEAR GRAPH purrdf:g",
                GraphTarget::Named(NamedNode::new_unchecked("https://x/g")),
            ),
        ] {
            let u = parse_update(text);
            let GraphUpdateOperation::Clear { target, .. } = &u.operations[0] else {
                panic!("expected Clear for {text}");
            };
            assert_eq!(*target, expected, "target mismatch for {text}");
        }
    }

    #[test]
    fn update_drop() {
        let u = parse_update("DROP SILENT GRAPH purrdf:g");
        let GraphUpdateOperation::Drop { silent, target } = &u.operations[0] else {
            panic!("expected Drop");
        };
        assert!(silent);
        assert_eq!(
            *target,
            GraphTarget::Named(NamedNode::new_unchecked("https://x/g"))
        );
    }

    #[test]
    fn update_create() {
        let u = parse_update("CREATE GRAPH purrdf:g");
        let GraphUpdateOperation::Create { graph, .. } = &u.operations[0] else {
            panic!("expected Create");
        };
        assert_eq!(graph.as_str(), "https://x/g");
    }

    #[test]
    fn update_add_move_copy() {
        let add = parse_update("ADD DEFAULT TO GRAPH purrdf:g");
        assert!(matches!(
            add.operations[0],
            GraphUpdateOperation::Add { .. }
        ));
        let mv = parse_update("MOVE GRAPH purrdf:a TO GRAPH purrdf:b");
        assert!(matches!(
            mv.operations[0],
            GraphUpdateOperation::Move { .. }
        ));
        let cp = parse_update("COPY GRAPH purrdf:a TO DEFAULT");
        let GraphUpdateOperation::Copy {
            source,
            destination,
            ..
        } = &cp.operations[0]
        else {
            panic!("expected Copy");
        };
        assert_eq!(
            *source,
            GraphTarget::Named(NamedNode::new_unchecked("https://x/a"))
        );
        assert_eq!(*destination, GraphTarget::Default);
    }

    #[test]
    fn update_sequence_of_operations() {
        let u = parse_update("CREATE GRAPH purrdf:g ; CLEAR DEFAULT ;");
        assert_eq!(u.operations.len(), 2, "trailing ; must be allowed");
    }

    #[test]
    fn update_empty_request_is_valid() {
        let u = SparqlParser::new()
            .parse_update("PREFIX ex: <http://e/>")
            .expect("prologue-only update");
        assert!(u.operations.is_empty());
    }

    #[test]
    fn update_base_iri_resolves_prologue() {
        let u = SparqlParser::new()
            .with_base_iri("http://base/")
            .parse_update("INSERT DATA { <s> <http://base/p> <o> }")
            .expect("base-resolved update");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(
            data[0].triple.subject,
            TermPattern::NamedNode(NamedNode::new_unchecked("http://base/s"))
        );
    }

    #[test]
    fn update_blank_in_delete_data_is_error() {
        let err = update_err("DELETE DATA { _:b purrdf:p purrdf:o }");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn update_blank_in_delete_template_is_error() {
        let err = update_err("DELETE { _:b purrdf:p ?o } WHERE { ?s purrdf:p ?o }");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn update_variable_in_insert_data_is_error() {
        let err = update_err("INSERT DATA { purrdf:s purrdf:p ?o }");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn update_unknown_keyword_is_error() {
        let err = update_err("FROBNICATE GRAPH purrdf:g");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn inverse_over_zero_or_more_stays_distinct_from_zero_or_more_over_inverse() {
        // `^purrdf:p*` parses as Reverse(ZeroOrMore(p)) — the star is inside.
        // Display of Reverse(ZeroOrMore(p)) must remain `^<p>*` (no extra parens
        // needed for Reverse; the inner `ZeroOrMore` is already a named-node-like
        // primary from the `^` perspective).
        let q = format!("{GM}SELECT ?x WHERE {{ ?x ^purrdf:p* ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "^<https://x/p>*");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    // ── expression-valued GROUP BY ───────────────────────────────────────────

    #[test]
    fn group_by_expr_as_lowers_to_extend_under_group() {
        // `GROUP BY (?a + ?a AS ?z)` → Extend(?z := ?a+?a) sits UNDER the Group,
        // whose grouping key is the explicit ?z (no algebra change).
        let q = format!(
            "{GM}SELECT ?z (COUNT(*) AS ?c) WHERE {{ ?r purrdf:a ?a }} GROUP BY (?a + ?a AS ?z)"
        );
        // Strip Project, then the select-expr Extend for ?c, to reach the Group.
        let group = match unproject(select_pattern(&q)) {
            GraphPattern::Extend { inner, .. } => *inner,
            other => other,
        };
        match group {
            GraphPattern::Group {
                inner, variables, ..
            } => {
                assert_eq!(variables, vec![Variable::new("z")]);
                match *inner {
                    GraphPattern::Extend { variable, .. } => {
                        assert_eq!(variable, Variable::new("z"));
                    }
                    other => panic!("expected Extend under Group, got {other:?}"),
                }
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn group_by_bare_builtin_synthesizes_a_group_var() {
        // `GROUP BY STR(?a)` (no AS) mints a synthetic grouping variable.
        let q = format!("{GM}SELECT (COUNT(*) AS ?c) WHERE {{ ?r purrdf:a ?a }} GROUP BY STR(?a)");
        let group = match unproject(select_pattern(&q)) {
            GraphPattern::Extend { inner, .. } => *inner,
            other => other,
        };
        match group {
            GraphPattern::Group {
                inner, variables, ..
            } => {
                assert_eq!(variables.len(), 1);
                assert!(variables[0].as_str().starts_with("__purrdf_group_"));
                assert!(matches!(*inner, GraphPattern::Extend { .. }));
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn aggregate_in_group_by_key_is_rejected() {
        // `GROUP BY (SUM(?x) AS ?z)` is illegal — an aggregate cannot be a
        // grouping key. The non-lifting expression parse surfaces it.
        let q = format!("{GM}SELECT ?z WHERE {{ ?r purrdf:a ?x }} GROUP BY (SUM(?x) AS ?z)");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Unsupported(_)),
            "expected Unsupported for aggregate in GROUP BY key, got {err:?}"
        );
    }

    #[test]
    fn select_star_with_group_by_is_rejected() {
        // §11.1: `SELECT *` is illegal in an aggregate query (vendored W3C
        // `syntax-query` `syn-bad-01`). Both an explicit GROUP BY and a bare
        // aggregate must trip it.
        let q = format!("{GM}SELECT * {{ ?s ?p ?o }} GROUP BY ?s");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "expected Syntax for SELECT * with GROUP BY, got {err:?}"
        );
    }

    #[test]
    fn bind_target_already_in_scope_is_rejected() {
        // §19.6: re-binding an in-scope variable via BIND is a hard error
        // (vendored W3C `syntax-query` `syntax-BINDscope6/7/8`). Cover the flat
        // BGP, a preceding nested group, and a preceding UNION.
        for body in [
            "?s purrdf:p ?o . ?s purrdf:q ?o1 . BIND((1 + ?o) AS ?o1)",
            "{ ?s purrdf:p ?o . ?s purrdf:q ?o1 . } BIND((1 + ?o) AS ?o1)",
            "{ { ?s purrdf:p ?Y } UNION { ?s purrdf:p ?Z } } BIND(1 AS ?Y)",
        ] {
            let q = format!("{GM}SELECT * WHERE {{ {body} }}");
            let err = SparqlParser::new()
                .parse_query(&q)
                .expect_err("BIND over in-scope var must fail");
            assert!(
                matches!(err, ParseError::Syntax { .. }),
                "expected Syntax for BIND scope violation in {body:?}, got {err:?}"
            );
        }
        // A BIND target that is genuinely fresh still parses.
        let ok = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o . BIND((1 + ?o) AS ?o1) }}");
        SparqlParser::new()
            .parse_query(&ok)
            .expect("fresh BIND target parses");
    }

    /// The group-parsing loop's scope set is a genuinely incremental structure,
    /// not a per-element recompute wearing an O(log n) membership test: a
    /// query's count of PRODUCTION scope-snapshot consultations
    /// ([`Parser::note_scope_consultation`]) is fixed by its STRUCTURE — one
    /// `SELECT *`, one `LATERAL` keyword — and invariant under how many `BIND`s
    /// sit inside it. Falsified by a SCALE-INVARIANT COUNT, deliberately never
    /// a clock (benches report, never assert; wall-clock varies with machine
    /// load, a call count reached by fixed code paths does not).
    ///
    /// Named without a `lateral_`/`sep0006_`/`service_variable_` prefix so it
    /// stays outside `rg -c '^\s*fn (lateral_|sep0006_|service_variable_)'`'s
    /// count of the `LATERAL` scope-rule tests (still 25 — untouched by this
    /// test).
    #[cfg(debug_assertions)]
    #[test]
    fn scope_set_stays_linear_over_two_thousand_binds() {
        fn binds(n: usize) -> String {
            use std::fmt::Write as _;
            (1..=n).fold(String::new(), |mut acc, i| {
                let _ = write!(acc, "BIND({i} AS ?x{i}) ");
                acc
            })
        }

        /// Parse `body` (already wrapped in a full query) and return the
        /// number of production scope-snapshot consultations it took —
        /// reading the counter is itself a plain field read, not a
        /// consultation, so this helper cannot inflate what it measures.
        fn consultations(query: &str) -> u64 {
            let options = ParserOptions::default();
            let mut p = SparqlParser::new()
                .parser_for(query, &options)
                .expect("tokenize");
            p.parse_query().expect("parse");
            p.expect_eof().expect("a full query consumes every token");
            p.debug_scope_consultations()
        }

        // Plain: N sequential BINDs directly in the WHERE group, under a
        // `SELECT *` (one production consultation: the projection build).
        let plain = |n: usize| {
            consultations(&format!(
                "{GM}SELECT * WHERE {{ ?s purrdf:p ?o {} }}",
                binds(n)
            ))
        };
        let plain_200 = plain(200);
        let plain_2000 = plain(2000);
        assert_eq!(
            plain_200, plain_2000,
            "the count must be invariant under how many BINDs the group holds"
        );
        assert!(
            plain_2000 <= 2,
            "count={plain_2000} — a per-BIND recompute would read 200 vs 2,000 here (the \
             pre-fix revert-check), not a value fixed at <= 2"
        );

        // The same N BINDs, inside a LATERAL right-hand side (two production
        // consultations: the outer `SELECT *` and the LATERAL's own
        // left-scope read — each fires exactly ONCE regardless of N).
        let inside_lateral = |n: usize| {
            consultations(&format!(
                "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ {} }} }}",
                binds(n)
            ))
        };
        let lateral_200 = inside_lateral(200);
        let lateral_2000 = inside_lateral(2000);
        assert_eq!(
            lateral_200, lateral_2000,
            "the count must be invariant under how many BINDs the LATERAL right-hand side holds"
        );
        assert!(lateral_2000 <= 2, "count={lateral_2000}");
    }

    #[test]
    fn bind_target_only_in_minus_right_is_allowed() {
        // §18.2.1: a variable occurring only in the right operand of MINUS is
        // NOT in scope in the enclosing group, so binding it via BIND is legal.
        // `?v` appears solely inside the MINUS-right, so `BIND(1 AS ?v)` is fresh.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o MINUS {{ ?x purrdf:q ?v }} BIND(1 AS ?v) }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("BIND over a MINUS-right-only var must parse");
    }

    #[test]
    fn select_star_excludes_minus_right_only_vars() {
        // §18.2.1: `SELECT *` must not project variables that occur only in the
        // right operand of MINUS. `?v` is MINUS-right-only, so the projection is
        // exactly {?s, ?o}.
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o MINUS {{ ?x purrdf:q ?v }} }}");
        let GraphPattern::Project { variables, .. } = select_pattern(&q) else {
            panic!("expected a Project wrapper for SELECT *");
        };
        let names: Vec<&str> = variables.iter().map(Variable::as_str).collect();
        assert!(
            names.contains(&"s") && names.contains(&"o"),
            "expected ?s and ?o in projection, got {names:?}"
        );
        assert!(
            !names.contains(&"v") && !names.contains(&"x"),
            "MINUS-right-only vars must not be projected, got {names:?}"
        );
    }

    // ── LATERAL (SEP-0006 surface syntax) ───────────────────────────────────

    #[test]
    fn lateral_takes_the_preceding_pattern_as_its_left() {
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ ?o purrdf:q ?z }} }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Lateral { left, right } = where_pat else {
            panic!("expected Lateral, got {where_pat:?}");
        };
        let GraphPattern::Bgp { patterns: lp } = *left else {
            panic!("expected the left to be the preceding BGP");
        };
        assert_eq!(lp.len(), 1);
        assert_eq!(lp[0].subject, TermPattern::Variable(Variable::new("s")));
        assert_eq!(lp[0].object, TermPattern::Variable(Variable::new("o")));
        let GraphPattern::Bgp { patterns: rp } = *right else {
            panic!("expected the right to be the LATERAL body's BGP");
        };
        assert_eq!(rp.len(), 1);
        assert_eq!(rp[0].subject, TermPattern::Variable(Variable::new("o")));
    }

    #[test]
    fn lateral_chains_left_deep_in_textual_order() {
        // Two consecutive `LATERAL`s at the same nesting level must chain
        // LEFT-DEEP, each new one absorbing everything written before it.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?a purrdf:p ?b LATERAL {{ ?b purrdf:q ?c }} LATERAL {{ ?c purrdf:r ?d }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Lateral { left, right } = where_pat else {
            panic!("expected the outermost node to be Lateral, got {where_pat:?}");
        };
        let GraphPattern::Bgp { patterns } = *right else {
            panic!("expected the outermost right to be `?c purrdf:r ?d`");
        };
        assert_eq!(
            patterns[0].subject,
            TermPattern::Variable(Variable::new("c"))
        );
        let GraphPattern::Lateral {
            left: inner_left,
            right: inner_right,
        } = *left
        else {
            panic!("expected the outermost left to itself be a Lateral");
        };
        assert!(matches!(*inner_left, GraphPattern::Bgp { .. }));
        let GraphPattern::Bgp { patterns: irp } = *inner_right else {
            panic!("expected the inner right to be `?b purrdf:q ?c`");
        };
        assert_eq!(irp[0].subject, TermPattern::Variable(Variable::new("b")));
    }

    #[test]
    fn lateral_nests_on_the_right_when_written_nested() {
        // A `LATERAL` written INSIDE another `LATERAL`'s body nests on the
        // right, rather than flattening into the outer chain.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?a purrdf:p ?b LATERAL {{ ?b purrdf:q ?c LATERAL {{ ?c purrdf:r ?d }} }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Lateral { left, right } = where_pat else {
            panic!("expected Lateral, got {where_pat:?}");
        };
        assert!(matches!(*left, GraphPattern::Bgp { .. }));
        let GraphPattern::Lateral { .. } = *right else {
            panic!("expected the outer right to itself be a Lateral");
        };
    }

    #[test]
    fn lateral_after_a_dot_separated_triples_block() {
        // A `.` between the preceding triples and `LATERAL` must not confuse
        // the triples-block loop into trying to parse `LATERAL` as a fresh
        // subject (the `block_boundary` fix under test).
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o . LATERAL {{ ?o purrdf:q ?z }} }}");
        let where_pat = unproject(select_pattern(&q));
        assert!(
            matches!(where_pat, GraphPattern::Lateral { .. }),
            "got {where_pat:?}"
        );
    }

    #[test]
    fn lateral_with_no_left_pattern_is_the_unit_table() {
        // `LATERAL` as the first element of a group has no preceding pattern;
        // its left stays the identity table `Bgp { patterns: [] }`, exactly
        // like `OPTIONAL`/`MINUS` written first.
        let q = format!("{GM}SELECT * WHERE {{ LATERAL {{ ?s purrdf:p ?o }} }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Lateral { left, .. } = where_pat else {
            panic!("expected Lateral, got {where_pat:?}");
        };
        assert_eq!(*left, GraphPattern::Bgp { patterns: vec![] });
    }

    #[test]
    fn lateral_binds_looser_than_union() {
        // `{A} UNION {B} LATERAL {C}` must attach `LATERAL` to the WHOLE
        // union, not just `{B}` — the same looser-than-UNION precedence
        // `OPTIONAL`/`MINUS` already have (structural: the outer loop only
        // reaches the `LATERAL` arm after the `{...} UNION {...}` element has
        // already been folded into `g`).
        let q = format!(
            "{GM}SELECT * WHERE {{ {{ ?a purrdf:p ?x }} UNION {{ ?a purrdf:q ?x }} LATERAL {{ ?x purrdf:r ?y }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Lateral { left, .. } = where_pat else {
            panic!("expected Lateral, got {where_pat:?}");
        };
        assert!(
            matches!(*left, GraphPattern::Union { .. }),
            "LATERAL's left must be the whole UNION, got {left:?}"
        );
    }

    #[test]
    fn lateral_right_hand_side_inherits_the_depth_limit() {
        // The RHS parses via `parse_group_graph_pattern` (not `_inner`), so it
        // counts toward `MAX_GRAPH_PATTERN_DEPTH` exactly like every other
        // braced construct.
        fn nested_query(extra_depth: usize) -> String {
            format!(
                "SELECT * WHERE {{ ?s ?p ?o LATERAL {} ?x ?y ?z {} }}",
                "{ ".repeat(extra_depth),
                "} ".repeat(extra_depth)
            )
        }
        SparqlParser::new()
            .parse_query(&nested_query(MAX_GRAPH_PATTERN_DEPTH - 1))
            .expect("depth budget reached exactly through a LATERAL right-hand side must parse");
        let error = SparqlParser::new()
            .parse_query(&nested_query(MAX_GRAPH_PATTERN_DEPTH))
            .expect_err("one level beyond the limit through a LATERAL right-hand side must fail");
        assert!(matches!(error, ParseError::Syntax { .. }));
        assert!(error.to_string().contains("nesting exceeds"));
    }

    #[test]
    fn lateral_is_positional_not_reserved() {
        // `LATERAL` is a keyword only POSITIONALLY: it must not shadow
        // `lateral` as an ordinary prefixed-name local part or variable name.
        // Both dotted positions below are the ones that exercise the changed
        // `block_boundary` arm — without it, the triples-block loop would
        // continue past the `.` and try to parse a fresh subject where the
        // keyword sits, since `peek_kw` alone (used by the outer dispatch
        // loop) already never confuses a `PrefixedName`/`Variable` token with
        // a `Word` keyword token.
        let q = format!(
            "{GM}PREFIX lateral: <https://l/>\nSELECT * WHERE {{ ?s purrdf:p ?o . lateral:x purrdf:p ?o }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Bgp { patterns } = where_pat else {
            panic!(
                "a prefixed name in the `lateral:` namespace must parse as an \
                 ordinary triple, not the LATERAL keyword: {where_pat:?}"
            );
        };
        assert_eq!(patterns.len(), 2);

        let q2 = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o . ?lateral purrdf:p ?o }}");
        let where_pat2 = unproject(select_pattern(&q2));
        let GraphPattern::Bgp { patterns: p2 } = where_pat2 else {
            panic!(
                "a variable named `lateral` must parse as an ordinary variable, \
                 not the LATERAL keyword: {where_pat2:?}"
            );
        };
        assert_eq!(p2.len(), 2);
    }

    #[test]
    fn sep0006_illegal_bind_example_is_rejected() {
        // SEP-0006's own illegal example: `LATERAL { BIND(123 AS ?o) }` after
        // `?s ?p ?o` — `?o` is already bound on the left.
        let q = format!("{GM}SELECT * {{ ?s ?p ?o LATERAL {{ BIND(123 AS ?o) }} }}");
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("SEP-0006's own illegal example must be rejected");
        assert!(
            err.to_string()
                .contains("BIND target ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn sep0006_legal_subselect_example_is_accepted() {
        // SEP-0006's own legal example: a sub-`SELECT` that reuses the outer
        // `?s` internally but projects only `?label` — the reused name is not
        // an introduction, so it never collides.
        let q = format!(
            "{GM}SELECT * {{ ?s rdf:type purrdf:T LATERAL {{ SELECT ?label {{ ?s rdfs:label ?label }} LIMIT 1 }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("SEP-0006's own legal sub-select example must parse");
    }

    #[test]
    fn sep0006_select_star_subselect_examples_are_accepted() {
        // SEP-0006's own two `SELECT *` sub-select examples: a bare form and
        // one wrapped in `OPTIONAL`. `SELECT *` projects the sub-select's own
        // visible variables (here including the reused `?s`), so `Project`
        // narrows to a non-empty scope but the sub-select body is a plain
        // BGP — a leaf, never an introduction — so both accept.
        for q in [
            format!(
                "{GM}SELECT * {{ ?s rdf:type purrdf:T LATERAL {{ SELECT * {{ ?s rdfs:label ?label }} LIMIT 1 }} }}"
            ),
            format!(
                "{GM}SELECT * {{ ?s ?p ?o LATERAL {{ OPTIONAL {{ SELECT * {{ ?s rdfs:label ?label }} LIMIT 1 }} }} }}"
            ),
        ] {
            SparqlParser::new()
                .parse_query(&q)
                .unwrap_or_else(|e| panic!("SEP-0006's own SELECT * example must parse: {e}\n{q}"));
        }
    }

    #[test]
    fn lateral_rhs_values_collision_is_rejected() {
        let q =
            format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ VALUES ?o {{ 1 2 }} }} }}");
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a VALUES column colliding with the left scope must be rejected");
        assert!(
            err.to_string()
                .contains("VALUES variable ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn lateral_rhs_subselect_as_target_collision_is_rejected() {
        // The sub-select projects exactly the colliding `(1+1 AS ?o)` target,
        // so `Project`'s narrowing does not filter it away.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ SELECT (1 + 1 AS ?o) {{ ?x purrdf:q ?y }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a projected (expr AS ?v) target colliding with the left scope must fail");
        assert!(
            err.to_string()
                .contains("BIND target ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn lateral_rhs_group_by_expr_target_collision_is_rejected() {
        // The parser lowers `GROUP BY (?y AS ?o)` to an `Extend` directly
        // beneath `Group`, at the RHS's own top scope level — a fresh
        // (computed) binding for `?o` just like a `BIND` target, so it must
        // collide with the LHS's `?o` the same way.
        let q = format!(
            "{GM}SELECT * {{ ?s purrdf:p ?o LATERAL {{ SELECT ?o WHERE {{ ?x purrdf:q ?y }} GROUP BY (?y AS ?o) }} }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a GROUP BY (expr AS ?v) grouping target colliding with the left scope must fail",
        );
        assert!(
            err.to_string()
                .contains("BIND target ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn lateral_rhs_group_by_expr_fresh_target_is_accepted() {
        // Control: a genuinely fresh grouping target must still parse.
        let q = format!(
            "{GM}SELECT * {{ ?s purrdf:p ?o LATERAL {{ SELECT ?fresh WHERE {{ ?x purrdf:q ?y }} GROUP BY (?y AS ?fresh) }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a genuinely fresh GROUP BY (expr AS ?v) target must parse");
    }

    #[test]
    fn lateral_rhs_collision_below_optional_and_union_is_rejected() {
        // `OPTIONAL`/`UNION` are transparent to scope: a BIND collision
        // beneath either must still be found.
        for body in [
            "OPTIONAL { BIND(1 AS ?o) }",
            "{ BIND(1 AS ?o) } UNION { ?x purrdf:q ?y }",
        ] {
            let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ {body} }} }}");
            let err = SparqlParser::new()
                .parse_query(&q)
                .expect_err(&format!("expected a scope-conflict rejection for {body:?}"));
            assert!(
                err.to_string()
                    .contains("BIND target ?o inside LATERAL is already in scope"),
                "unexpected message for {body:?}: {err}"
            );
        }
    }

    #[test]
    fn lateral_rhs_collision_inside_a_nested_lateral_is_rejected() {
        // A nested `LATERAL`'s own operands sit at the OUTER scope level —
        // the walker must keep recursing into a nested Lateral rather than
        // treating it as its own boundary.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ ?x purrdf:q ?y LATERAL {{ BIND(1 AS ?o) }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a collision inside a nested LATERAL must still be found");
        assert!(
            err.to_string()
                .contains("BIND target ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn lateral_rhs_fresh_bind_is_accepted() {
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ BIND(1 AS ?fresh) }} }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("a genuinely fresh BIND target must parse");
    }

    #[test]
    fn lateral_rhs_reusing_a_left_variable_in_a_triple_is_accepted() {
        // Using a left-hand variable inside an ordinary RHS triple is
        // correlated USE, not an introduction — always legal.
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ ?o purrdf:q ?z }} }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("reusing a left variable inside a triple must parse");
    }

    #[test]
    fn lateral_rhs_subselect_projecting_a_left_variable_is_accepted() {
        // Projecting an EXISTING left variable back out is not introducing a
        // fresh one.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ SELECT ?o {{ ?o purrdf:q ?z }} }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("projecting a left variable back out must parse");
    }

    #[test]
    fn lateral_rhs_nested_subselect_rescopes_the_check() {
        // Two nested sub-selects, each projecting exactly `?o`: the
        // narrowing must be re-derived at EACH `Project` boundary (not
        // computed once at the outer level) to still find the innermost
        // `BIND(1 AS ?o)`.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ SELECT ?o {{ SELECT ?o {{ ?x purrdf:q ?y . BIND(1 AS ?o) }} }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("the collision must survive two nested Project boundaries");
        assert!(
            err.to_string()
                .contains("BIND target ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn lateral_left_scope_excludes_minus_right_only_variables() {
        // §18.2.1: a variable occurring only in the right operand of MINUS is
        // not in scope on the LATERAL left-hand side, so binding it inside
        // LATERAL is fresh.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o MINUS {{ ?x purrdf:q ?v }} LATERAL {{ BIND(1 AS ?v) }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a MINUS-right-only variable must not be in LATERAL's left scope");
    }

    #[test]
    fn lateral_left_scope_excludes_filter_only_variables() {
        // A variable occurring only inside a FILTER expression is never
        // collected by `collect_vars` (FILTER's expression is not walked),
        // so it is not part of the LATERAL left scope either.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o FILTER(?w > 5) LATERAL {{ BIND(1 AS ?w) }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a FILTER-only variable must not be in LATERAL's left scope");
    }

    #[test]
    fn lateral_rhs_minus_right_bind_under_select_star_is_accepted() {
        // Deliberate divergence from Jena: `SELECT *`'s own projection list is
        // computed from `visible_variables`, which already excludes a
        // MINUS-right-only BIND target, so the narrowed scope is empty before
        // the walker ever reaches it — nothing observable can shadow.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ SELECT * {{ ?x purrdf:q ?y MINUS {{ ?z purrdf:r ?w . BIND(1 AS ?o) }} }} }} }}"
        );
        SparqlParser::new().parse_query(&q).expect(
            "a MINUS-right BIND under a SELECT * sub-select must be accepted (Jena diverges here)",
        );
    }

    #[test]
    fn lateral_rhs_minus_right_bind_is_accepted() {
        // The bare form — a `MINUS` written directly under `LATERAL`'s
        // keyword, no `SELECT *` sub-select in between. Same ground as
        // `lateral_rhs_minus_right_bind_under_select_star_is_accepted`: the
        // `MINUS` right operand's `BIND(1 AS ?o)` is discarded by §18.5's
        // evaluation before it could ever be observed as a rebinding of the
        // left-hand `?o`, so the `Minus` arm skips the right operand
        // outright rather than needing a `Project` boundary to narrow it
        // away. This generalizes the `SELECT *` case above rather than
        // adding a second rule: the `SELECT *` form was one route to this
        // same shape, not a separate one.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ ?a purrdf:p ?b MINUS {{ ?a purrdf:p ?b BIND(1 AS ?o) }} }} }}"
        );
        SparqlParser::new().parse_query(&q).expect(
            "a bare MINUS-right BIND directly under LATERAL must be accepted (Jena diverges here)",
        );
    }

    #[test]
    fn lateral_rhs_minus_left_bind_is_rejected() {
        // Control: a collision in the MINUS LEFT operand is an ordinary
        // observable rebinding — the left operand's own bindings survive
        // `MINUS` (only the right operand is used-then-discarded by §18.5's
        // compatibility test) — so it must still be rejected. Confirms the
        // `Minus` arm's narrowing to skip the right operand did not
        // accidentally stop walking the left operand too.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ BIND(1 AS ?o) MINUS {{ ?a purrdf:p ?b }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a MINUS-left collision must still be rejected");
        assert!(
            err.to_string()
                .contains("BIND target ?o inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn lateral_left_scope_includes_a_service_endpoint_variable() {
        // Divergence from Jena: `visible_variables` (the parser's one
        // definition of "in scope") includes a SERVICE endpoint variable, so
        // LATERAL's left scope does too, even though Jena's SyntaxVarScope
        // omits it.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?g . SERVICE ?g {{ ?x purrdf:q ?y }} LATERAL {{ BIND(1 AS ?g) }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a SERVICE ?g endpoint variable must be in LATERAL's left scope here");
        assert!(
            err.to_string()
                .contains("BIND target ?g inside LATERAL is already in scope"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn service_variable_endpoint_rhs_bind_is_not_scope_checked() {
        // `SERVICE ?g { ... }` is auto-wrapped into a `Lateral` node by the
        // pre-existing SERVICE dispatch arm (a representation detail of
        // variable-endpoint federation), not by the user writing the
        // `LATERAL` keyword — so SEP-0006's scope restriction, which only
        // runs from the `LATERAL`-keyword dispatch arm, never walks its body.
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?g . SERVICE ?g {{ BIND(1 AS ?g) }} }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("a SERVICE ?g body is not scope-checked by the LATERAL restriction");
    }

    #[test]
    fn lateral_rhs_exists_is_not_walked() {
        // `Filter`'s expression operand is never visited by the WALKER
        // (`find_scope_conflict`), so an `EXISTS { BIND(1 AS ?fresh) }`
        // nested inside a FILTER cannot trigger the LATERAL scope
        // restriction, even in principle — `?fresh` collides with nothing,
        // so this also stays accepted under the SEP-0007 EXISTS-site check
        // (`parse_exists_body`), which independently sees an empty
        // collision here too. See
        // `exists_scope_bind_collision_inside_lateral_is_caught_at_the_exists_site`
        // for the ORIGINAL (`?o`-colliding) form of this query, which the
        // EXISTS-site check — not the LATERAL walker — now refuses.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ FILTER EXISTS {{ BIND(1 AS ?fresh) }} }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a BIND nested inside an EXISTS expression must not be walked");
    }

    #[test]
    fn exists_scope_bind_collision_inside_lateral_is_caught_at_the_exists_site() {
        // The ORIGINAL text of `lateral_rhs_exists_is_not_walked` before its
        // `?o` target was changed to the fresh `?fresh`: `find_scope_conflict`
        // still never descends into `Expression`, so the LATERAL walker
        // itself does not catch this — but SEP-0007 Part 3's EXISTS-site
        // check does, since `?o` is on the LATERAL left-hand side, which is
        // scope-transparent into the LATERAL right-hand side block the
        // `FILTER EXISTS` sits in (`Parser::exists_scope_stack`'s doc).
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o LATERAL {{ FILTER EXISTS {{ BIND(1 AS ?o) }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a BIND inside EXISTS colliding with the LATERAL left-hand side must fail");
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    // ── EXISTS in-scope-set restriction (SEP-0007 Part 3) ────────────────────

    #[test]
    fn exists_scope_bind_collision_is_rejected() {
        let q =
            format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ BIND(1 AS ?o) }} }}");
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a BIND target colliding with the row being filtered must fail");
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn exists_scope_values_collision_is_rejected() {
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ VALUES ?o {{ 1 }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a VALUES column colliding with the row being filtered must fail");
        assert!(
            err.to_string().contains(
                "VALUES variable ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn exists_scope_subselect_as_target_collision_is_rejected() {
        // The sub-select projects exactly the colliding `(1 AS ?o)` target,
        // so `Project`'s narrowing does not filter it away.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ SELECT (1 AS ?o) WHERE {{ ?x purrdf:q ?y }} }} }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a projected (expr AS ?v) target colliding with the row being filtered must fail",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn exists_scope_group_by_expr_target_collision_is_rejected() {
        // `GROUP BY (?y AS ?o)` lowers to an `Extend` directly beneath
        // `Group`, at the EXISTS body's own top scope level — a fresh
        // (computed) binding for `?o`, exactly like a `BIND` target.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ SELECT ?o WHERE {{ ?x purrdf:q ?y }} GROUP BY (?y AS ?o) }} }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a GROUP BY (expr AS ?v) grouping target colliding with the row being filtered must fail",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn exists_scope_fresh_bind_is_accepted() {
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ BIND(1 AS ?fresh) }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a genuinely fresh BIND target inside EXISTS must parse");
    }

    #[test]
    fn exists_scope_projected_away_collision_is_accepted() {
        // The sub-select inside EXISTS projects only `?x` — `?o`'s `BIND`
        // never escapes the sub-select's own `Project` boundary, so the
        // narrowed scope the walker checks against is empty by the time it
        // reaches the `BIND`.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ SELECT ?x WHERE {{ ?x purrdf:q ?y . BIND(1 AS ?o) }} }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a BIND target projected away by an inner sub-select must parse");
    }

    #[test]
    fn exists_scope_minus_right_introduction_is_accepted() {
        // §18.2.1: a MINUS right operand's own introductions never escape
        // it, so a `BIND` confined there cannot be an observable rebinding
        // of the row being filtered, at any depth.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ ?a purrdf:q ?b MINUS {{ ?a purrdf:q ?b BIND(1 AS ?o) }} }} }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a MINUS-right introduction inside EXISTS must be accepted");
    }

    #[test]
    fn exists_scope_nested_exists_checks_its_own_site() {
        // `?a` is bound by the OUTER EXISTS's own body (`?a purrdf:q ?b`),
        // not by the top-level WHERE clause — the INNER (doubly-nested)
        // EXISTS's `BIND(1 AS ?a)` still collides, because the inner
        // EXISTS's own in-scope set is seeded from its immediately
        // enclosing frame (the outer EXISTS's own, isolated one), not just
        // the outermost row.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ ?a purrdf:q ?b . FILTER EXISTS {{ BIND(1 AS ?a) }} }} }}"
        );
        let err = SparqlParser::new()
            .parse_query(&q)
            .expect_err("a collision against the outer EXISTS's own row must still be found");
        assert!(
            err.to_string().contains(
                "BIND target ?a inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn not_exists_scope_bind_collision_is_rejected() {
        // `NOT EXISTS` shares the SAME production (and hence the SAME check)
        // as `EXISTS` — there is no separate "NOT EXISTS" wording.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o . FILTER NOT EXISTS {{ BIND(1 AS ?o) }} }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a BIND target colliding with the row being filtered must fail under NOT EXISTS too",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn update_where_exists_scope_collision_is_rejected() {
        // `INSERT … WHERE` routes its WHERE clause through the SAME
        // `parse_group_graph_pattern` (and hence the SAME EXISTS
        // production) a SELECT's WHERE does.
        let u = format!(
            "{GM}INSERT {{ ?s purrdf:tag 1 }} WHERE {{ ?s purrdf:p ?o . FILTER EXISTS {{ BIND(1 AS ?o) }} }}"
        );
        let err = SparqlParser::new().parse_update(&u).expect_err(
            "a BIND target colliding with the row being filtered must fail in an UPDATE WHERE too",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    // ── EXISTS in-scope-set restriction: SELECT-list position (SEP-0007
    //    Part 3 — the projection list is parsed BEFORE `WHERE`, so
    //    the check above (`Parser::parse_exists_body`) cannot run there
    //    immediately; see `Parser::pending_exists_scope_checks`'s doc for
    //    the deferred mechanism that closes it) ─────────────────────────────

    #[test]
    fn select_expression_exists_scope_collision_is_rejected() {
        let q = format!(
            "{GM}SELECT ?x (EXISTS {{ BIND(purrdf:e AS ?x) }} AS ?z) WHERE {{ ?x purrdf:p purrdf:c }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a BIND target inside a SELECT-list EXISTS colliding with the row being filtered must fail",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?x inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn subselect_expression_exists_scope_collision_is_rejected() {
        let q = format!(
            "{GM}SELECT * WHERE {{ {{ SELECT ?o (EXISTS {{ BIND(1 AS ?o) }} AS ?e) WHERE {{ ?s purrdf:p ?o }} }} }}"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a BIND target inside a sub-SELECT's SELECT-list EXISTS colliding with its own WHERE row must fail",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?o inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn aggregate_argument_exists_scope_collision_is_rejected() {
        // `?x` is both a WHERE variable and the GROUP BY key: an aggregate's
        // argument folds over the raw (ungrouped) rows, so it sees `?x`
        // bound either way (see `ExistsScopeBasis::AggregateArgument`'s
        // doc).
        let q = format!(
            "{GM}SELECT ?x (SUM(IF(EXISTS {{ BIND(purrdf:e AS ?x) }}, 1, 0)) AS ?n) WHERE {{ ?x purrdf:p purrdf:c }} GROUP BY ?x"
        );
        let err = SparqlParser::new().parse_query(&q).expect_err(
            "a BIND target inside an aggregate argument's EXISTS colliding with the ungrouped row must fail",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?x inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn update_subselect_expression_exists_collision_is_rejected() {
        let u = format!(
            "{GM}INSERT {{ ?x purrdf:tag 1 }} WHERE {{ {{ SELECT ?x (EXISTS {{ BIND(purrdf:e AS ?x) }} AS ?z) WHERE {{ ?x purrdf:p purrdf:c }} }} }}"
        );
        let err = SparqlParser::new().parse_update(&u).expect_err(
            "a BIND target inside an UPDATE-embedded sub-SELECT's SELECT-list EXISTS must fail the same way",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?x inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn select_expression_exists_fresh_bind_is_accepted() {
        let q = format!(
            "{GM}SELECT ?x (EXISTS {{ BIND(1 AS ?fresh) }} AS ?z) WHERE {{ ?x purrdf:p purrdf:c }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("a genuinely fresh BIND target inside a SELECT-list EXISTS must parse");
    }

    #[test]
    fn select_expression_exists_after_aggregation_uses_the_rescope() {
        // The aggregating rescope rule this reuses (from the pre-existing
        // §19.8 direct-target check, right above the deferred-check
        // resolution point in `Parser::parse_select`): when the query
        // aggregates, only the GROUP BY keys and grouping-extend targets
        // stay visible to the projection — the raw WHERE pattern is hidden
        // behind grouping, so a variable that WHERE bound but grouping does
        // NOT expose is fresh again, from the projection's point of view.
        //
        // `?y` is bound by WHERE but is not a GROUP BY key: grouping hides
        // it, so rebinding it inside a SELECT-list EXISTS is legal.
        let q = format!(
            "{GM}SELECT ?x (EXISTS {{ BIND(1 AS ?y) }} AS ?z) WHERE {{ ?x purrdf:p ?y }} GROUP BY ?x"
        );
        SparqlParser::new().parse_query(&q).expect(
            "a WHERE-only variable hidden by grouping may be rebound inside a SELECT-list EXISTS",
        );

        // `?x` IS the GROUP BY key — still visible to the projection after
        // grouping, so rebinding it must still fail.
        let q2 = format!(
            "{GM}SELECT ?x (EXISTS {{ BIND(1 AS ?x) }} AS ?z) WHERE {{ ?x purrdf:p ?y }} GROUP BY ?x"
        );
        let err = SparqlParser::new().parse_query(&q2).expect_err(
            "a GROUP BY key stays in scope on the row an aggregating SELECT-list EXISTS filters",
        );
        assert!(
            err.to_string().contains(
                "BIND target ?x inside EXISTS is already in scope on the row being filtered"
            ),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn nested_aggregate_stays_rejected() {
        // `SUM(COUNT(?x))` is illegal SPARQL 1.1 (no direct aggregate nesting) and
        // must remain a hard error — a regression guard.
        let q = format!("{GM}SELECT (SUM(COUNT(?x)) AS ?y) WHERE {{ ?r purrdf:a ?x }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Unsupported(_)),
            "expected Unsupported for nested aggregate, got {err:?}"
        );
    }

    // ── blank-node property lists ─────────────────────────────────────────────

    /// Count the triples in a (possibly Join-wrapped) BGP-only WHERE body.
    fn bgp_triple_count(p: &GraphPattern) -> usize {
        match p {
            GraphPattern::Bgp { patterns } => patterns.len(),
            GraphPattern::Join { left, right } => bgp_triple_count(left) + bgp_triple_count(right),
            _ => 0,
        }
    }

    #[test]
    fn blank_node_property_list_in_object_position() {
        // `?o :hasItem [ rdfs:label ?l ]` → two triples: (?o :hasItem _:b) and
        // (_:b rdfs:label ?l), with a fresh blank node linking them.
        let q = format!("{GM}SELECT * WHERE {{ ?o purrdf:hasItem [ rdfs:label ?l ] }}");
        let body = unproject(select_pattern(&q));
        assert_eq!(bgp_triple_count(&body), 2, "got {body:?}");
    }

    #[test]
    fn blank_node_property_list_standalone_subject() {
        // `[ :p ?o ] .` is a valid standalone subject — one triple (_:b :p ?o).
        let q = format!("{GM}SELECT * WHERE {{ [ purrdf:p ?o ] . }}");
        let body = unproject(select_pattern(&q));
        assert_eq!(bgp_triple_count(&body), 1, "got {body:?}");
    }

    #[test]
    fn blank_node_property_list_multiple_predicates() {
        // `[ :a 1 ; :b 2 ]` emits two triples sharing the fresh blank node.
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:has [ purrdf:a 1 ; purrdf:b 2 ] }}");
        let body = unproject(select_pattern(&q));
        // (?s :has _:b), (_:b :a 1), (_:b :b 2) = three triples.
        assert_eq!(bgp_triple_count(&body), 3, "got {body:?}");
    }

    // ── empty anonymous blank node [] ─────────────────────────────────────────

    #[test]
    fn empty_blank_node_in_subject_position_parses() {
        // `[] <p> <o>` — SPARQL ANON with no property list in subject position.
        let q = format!("{GM}ASK {{ [] purrdf:p <http://ex/o> }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("[] in subject position should parse without error");
    }

    #[test]
    fn empty_blank_node_in_object_position_parses() {
        // `<s> <p> []` — SPARQL ANON with no property list in object position.
        let q = format!("{GM}ASK {{ <http://ex/s> purrdf:p [] }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("[] in object position should parse without error");
    }

    #[test]
    fn non_empty_blank_node_property_list_still_parses() {
        // Regression guard: a non-empty `[ :p :o ]` must continue to work after
        // the empty-[] fix.
        let q = format!("{GM}ASK {{ <http://ex/s> purrdf:p [ purrdf:q <http://ex/o> ] }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("non-empty blank-node property list should still parse");
    }

    // ── extension-function seam (caller-configured; OFF by default) ───────────

    /// A caller-configured extension-function namespace for these tests (a
    /// neutral example.org name — purrdf itself mints no vocabulary IRIs).
    const EXT_NS: &str = "https://example.org/ext/";

    /// A prologue binding `g:` to the test extension namespace.
    const EXTP: &str = "PREFIX g: <https://example.org/ext/>\n";

    /// Options with only [`EXT_NS`] configured.
    fn ext_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: vec![EXT_NS.to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        }
    }

    /// Parse a SELECT with explicit options and return its root pattern.
    fn select_pattern_with(q: &str, options: &ParserOptions) -> GraphPattern {
        match SparqlParser::new()
            .parse_query_with(q, options)
            .expect("parse")
        {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Pull the single `BIND(... AS ?v)` expression out of a parsed SELECT.
    fn bound_expr_with(q: &str, options: &ParserOptions) -> Expression {
        let GraphPattern::Extend { expression, .. } = unproject(select_pattern_with(q, options))
        else {
            panic!("expected Extend");
        };
        expression
    }

    /// [`bound_expr_with`] under the default (no extension namespaces) options.
    fn bound_expr(q: &str) -> Expression {
        bound_expr_with(q, &ParserOptions::default())
    }

    /// The expected `heldIn` call node for a given namespace spelling.
    fn held_in_call(ns: &str) -> Function {
        Function::Purrdf(PurrdfCall {
            fn_kind: PurrdfFn::HeldIn,
            iri: format!("{ns}heldIn"),
        })
    }

    #[test]
    fn configured_extension_iri_dispatches_to_the_closed_fn_set() {
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:heldIn(?r, ?s) AS ?h) }}");
        let Expression::FunctionCall(func, args) = bound_expr_with(&q, &ext_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, held_in_call(EXT_NS));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn extension_full_iri_dispatches() {
        // The same dispatch via a full (non-prefixed) IRI under the configured
        // namespace.
        let q =
            "SELECT ?h WHERE { ?r ?p ?o . BIND(<https://example.org/ext/heldIn>(?r, ?s) AS ?h) }";
        let Expression::FunctionCall(func, _) = bound_expr_with(q, &ext_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, held_in_call(EXT_NS));
    }

    #[test]
    fn unknown_extension_function_is_hard_parse_error() {
        let q = format!("{EXTP}SELECT ?x WHERE {{ ?r ?p ?o . BIND(g:bogus(?r) AS ?x) }}");
        let err = SparqlParser::new()
            .parse_query_with(&q, &ext_options())
            .unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "unknown g:bogus(...) under a configured namespace must be a hard parse error, got {err:?}"
        );
    }

    #[test]
    fn extension_iri_without_call_is_plain_named_node() {
        // A configured-namespace IRI NOT in call position stays an ordinary IRI term.
        let q = format!("{EXTP}SELECT ?x WHERE {{ ?x a g:heldIn }}");
        let GraphPattern::Bgp { patterns } = unproject(select_pattern_with(&q, &ext_options()))
        else {
            panic!("expected BGP");
        };
        assert_eq!(patterns.len(), 1);
        let TermPattern::NamedNode(n) = &patterns[0].object else {
            panic!("expected a NamedNode object");
        };
        assert_eq!(n.as_str(), "https://example.org/ext/heldIn");
    }

    #[test]
    fn default_options_have_no_extension_namespaces() {
        // With NO configured namespace (the default) the extension seam is OFF:
        // a call-position IRI is an ordinary custom function — no error, no
        // special-casing, regardless of its local name.
        assert!(ParserOptions::default().extension_fn_namespaces.is_empty());
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:heldIn(?r, ?s) AS ?h) }}");
        let Expression::FunctionCall(func, _) = bound_expr(&q) else {
            panic!("expected a FunctionCall");
        };
        assert!(
            matches!(&func, Function::Custom(n) if n.as_str() == format!("{EXT_NS}heldIn")),
            "got {func:?}"
        );
    }

    // ── SEP-0008 SHA-3 built-ins (the hyphenated keyword surface) ────────────

    #[test]
    fn sha3_builtins_parse_under_their_hyphenated_names() {
        for (name, expected) in [
            ("SHA3-224", Function::Sha3_224),
            ("SHA3-256", Function::Sha3_256),
            ("SHA3-384", Function::Sha3_384),
            ("SHA3-512", Function::Sha3_512),
        ] {
            let q = format!("SELECT ?h WHERE {{ ?s ?p ?o . BIND({name}(STR(?o)) AS ?h) }}");
            let Expression::FunctionCall(func, args) = bound_expr(&q) else {
                panic!("expected a FunctionCall for {name}");
            };
            assert_eq!(func, expected, "{name} dispatched to the wrong builtin");
            assert_eq!(args.len(), 1, "{name} takes one argument");
        }
    }

    /// Case-insensitivity is the whole `BuiltInCall` keyword rule, and the
    /// hyphen must not break it (`upper` is an ASCII uppercase of the WHOLE
    /// token, hyphen included).
    #[test]
    fn sha3_builtin_names_are_case_insensitive() {
        let q = "SELECT ?h WHERE { ?s ?p ?o . BIND(sha3-512(STR(?o)) AS ?h) }";
        let Expression::FunctionCall(func, _) = bound_expr(q) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, Function::Sha3_512);
    }

    /// THE hyphen trap, pinned from the parser side: `SHA3-224(…)` is ONE
    /// built-in call, while a SPACED `SHA3 - 224` is a different token sequence
    /// that must NOT resolve to it. `SHA3` alone is not a function, so the
    /// spaced form is a hard parse error rather than a silently different
    /// meaning — which is exactly the outcome that keeps the two unambiguous.
    #[test]
    fn sha3_hyphen_is_one_token_not_a_subtraction() {
        // The joined form is the built-in.
        let q = "SELECT ?h WHERE { ?s ?p ?o . BIND(SHA3-224(STR(?o)) AS ?h) }";
        assert!(matches!(
            bound_expr(q),
            Expression::FunctionCall(Function::Sha3_224, _)
        ));

        // The spaced form is not: `SHA3` is no function or keyword.
        let spaced = "SELECT ?h WHERE { ?s ?p ?o . BIND(SHA3 - 224 AS ?h) }";
        let err = SparqlParser::new()
            .parse_query(spaced)
            .expect_err("`SHA3 - 224` must not resolve to the SHA3-224 builtin");
        assert!(
            format!("{err:?}").contains("SHA3"),
            "the diagnostic must name the unresolved token, got {err:?}"
        );

        // And subtraction against a real hash call still parses as subtraction:
        // the `-` there follows `)`, not a word character.
        let sub = "SELECT ?h WHERE { ?s ?p ?o . BIND(STRLEN(SHA3-256(STR(?o))) - 4 AS ?h) }";
        assert!(
            matches!(bound_expr(sub), Expression::Subtract(_, _)),
            "an ordinary subtraction beside a SHA-3 call must stay a subtraction"
        );
    }

    /// SEP-0008 writes its four functions UNDERSCORED (`sha3_256`), so a query
    /// copied out of the proposal must parse. Both spellings are the same call:
    /// each `SHA3_NNN` pins to the SAME [`Function`] as its `SHA3-NNN` twin, and
    /// the two parse to an identical expression tree, so nothing downstream can
    /// tell which spelling the author typed.
    #[test]
    fn sha3_underscored_sep_spelling_pins_to_the_same_function() {
        for (hyphen, underscore, expected) in [
            ("SHA3-224", "SHA3_224", Function::Sha3_224),
            ("SHA3-256", "SHA3_256", Function::Sha3_256),
            ("SHA3-384", "SHA3_384", Function::Sha3_384),
            ("SHA3-512", "SHA3_512", Function::Sha3_512),
        ] {
            let q = |name: &str| {
                format!("SELECT ?h WHERE {{ ?s ?p ?o . BIND({name}(STR(?o)) AS ?h) }}")
            };
            let under = bound_expr(&q(underscore));
            let Expression::FunctionCall(func, args) = &under else {
                panic!("expected a FunctionCall for {underscore}");
            };
            assert_eq!(
                func, &expected,
                "{underscore} dispatched to the wrong builtin"
            );
            assert_eq!(args.len(), 1, "{underscore} takes one argument");
            assert_eq!(
                under,
                bound_expr(&q(hyphen)),
                "{underscore} and {hyphen} must parse to the same expression"
            );
        }

        // Case-insensitive, exactly like the hyphenated spelling — and this is
        // SEP-0008's own lower-case rendering of the name.
        let q = "SELECT ?h WHERE { ?s ?p ?o . BIND(sha3_256(STR(?o)) AS ?h) }";
        assert!(matches!(
            bound_expr(q),
            Expression::FunctionCall(Function::Sha3_256, _)
        ));
    }

    /// Accepting two spellings must NOT put two spellings on the wire. The
    /// serializer has one arm per [`Function`], so a query written in either
    /// spelling serializes to the HYPHENATED canonical form, byte-identically —
    /// which is what keeps a spelling choice at the input from reaching the
    /// output at all.
    #[test]
    fn sha3_serializes_to_one_canonical_spelling() {
        for (hyphen, underscore) in [
            ("SHA3-224", "SHA3_224"),
            ("SHA3-256", "SHA3_256"),
            ("SHA3-384", "SHA3_384"),
            ("SHA3-512", "SHA3_512"),
        ] {
            let text = |name: &str| {
                let q = format!("SELECT ?h WHERE {{ ?s ?p ?o . BIND({name}(STR(?o)) AS ?h) }}");
                crate::serialize::pattern_to_select_query(&unproject(select_pattern(&q)))
            };
            let from_underscore = text(underscore);
            assert_eq!(
                from_underscore,
                text(hyphen),
                "{underscore} and {hyphen} must serialize byte-identically"
            );
            assert!(
                from_underscore.contains(hyphen),
                "the canonical spelling is `{hyphen}`, got: {from_underscore}"
            );
            assert!(
                !from_underscore.contains(underscore),
                "`{underscore}` must never be emitted, got: {from_underscore}"
            );
            // And the canonical text re-parses to the same call, so the
            // round trip from the SEP spelling is closed.
            assert_eq!(
                unproject(select_pattern(&from_underscore)),
                unproject(select_pattern(&format!(
                    "SELECT ?h WHERE {{ ?s ?p ?o . BIND({underscore}(STR(?o)) AS ?h) }}"
                ))),
                "the canonical form must re-parse to the {underscore} call"
            );
        }
    }

    /// Parse → serialize → parse must be stable for the hyphenated names: the
    /// serializer re-emits `SHA3-224`, which the lexer must read back as the
    /// same single word (a serializer that emitted `SHA3 - 224`, or a lexer that
    /// split it, would make the round trip lose the call).
    #[test]
    fn sha3_builtins_round_trip_through_the_serializer() {
        for name in ["SHA3-224", "SHA3-256", "SHA3-384", "SHA3-512"] {
            let q = format!("SELECT ?h WHERE {{ ?s ?p ?o . BIND({name}(STR(?o)) AS ?h) }}");
            let pattern = unproject(select_pattern(&q));
            let text = crate::serialize::pattern_to_select_query(&pattern);
            assert!(
                text.contains(name),
                "the serializer must re-emit `{name}` verbatim, got: {text}"
            );
            let reparsed = unproject(select_pattern(&text));
            assert_eq!(reparsed, pattern, "round-trip mismatch for {name}");
        }
    }

    #[test]
    fn non_extension_function_remains_custom() {
        // An IRI outside every configured namespace in call position is
        // Function::Custom even when a namespace IS configured.
        let q = format!("{GM}SELECT ?x WHERE {{ ?r ?p ?o . BIND(purrdf:fn(?r) AS ?x) }}");
        let Expression::FunctionCall(func, _) = bound_expr_with(&q, &ext_options()) else {
            panic!("expected a FunctionCall");
        };
        // `GM` binds `purrdf:` to `<https://x/>`, so this is an external custom IRI.
        assert!(matches!(func, Function::Custom(_)), "got {func:?}");
    }

    // ── configurable extension-function namespaces (ParserOptions) ────────────

    /// The gmeow ontology namespace — the original consumer's spelling of the
    /// same closed extension-function set.
    const GMEOW_NS: &str = "https://blackcatinformatics.ca/gmeow/";

    /// Options with the gmeow namespace configured ALONGSIDE the example one.
    fn gmeow_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: vec![EXT_NS.to_owned(), GMEOW_NS.to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        }
    }

    #[test]
    fn configured_namespace_alias_dispatches_to_purrdf_fn() {
        // gmeow:heldIn(...) dispatches to the SAME closed PurrdfFn set when the gmeow
        // namespace is supplied via ParserOptions.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?h WHERE {{ ?r ?p ?o . BIND(gmeow:heldIn(?r, ?s) AS ?h) }}"
        );
        let Expression::FunctionCall(func, args) = bound_expr_with(&q, &gmeow_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, held_in_call(GMEOW_NS));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn every_configured_namespace_dispatches() {
        // Configuring several namespaces recognizes each of them.
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:listLength(?r) AS ?h) }}");
        let Expression::FunctionCall(func, _) = bound_expr_with(&q, &gmeow_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(
            func,
            Function::Purrdf(PurrdfCall {
                fn_kind: PurrdfFn::ListLength,
                iri: format!("{EXT_NS}listLength"),
            })
        );
    }

    #[test]
    fn unknown_local_under_configured_alias_is_hard_parse_error() {
        // The closed-set contract applies to EVERY configured namespace: an unknown
        // local name under the gmeow namespace hard-fails, no Custom fallthrough.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?x WHERE {{ ?r ?p ?o . BIND(gmeow:bogus(?r) AS ?x) }}"
        );
        let err = SparqlParser::new()
            .parse_query_with(&q, &gmeow_options())
            .unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "unknown gmeow:bogus(...) must be a hard parse error, got {err:?}"
        );
    }

    #[test]
    fn unconfigured_namespace_stays_a_custom_function() {
        // WITHOUT the namespace configured (the default is empty), a gmeow IRI in
        // call position is an ordinary custom function — never an implicit
        // extension dispatch.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?h WHERE {{ ?r ?p ?o . BIND(gmeow:heldIn(?r, ?s) AS ?h) }}"
        );
        let Expression::FunctionCall(func, _) = bound_expr(&q) else {
            panic!("expected a FunctionCall");
        };
        assert!(
            matches!(&func, Function::Custom(n) if n.as_str() == format!("{GMEOW_NS}heldIn")),
            "got {func:?}"
        );
    }

    #[test]
    fn serialization_round_trips_the_original_iri() {
        // ROUND-TRIP: an extension call parsed under the gmeow namespace
        // re-serializes as the ORIGINAL gmeow IRI (no namespace is fabricated on
        // output), and a re-parse with the same options reproduces the same node.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?h WHERE {{ ?r ?p ?o . BIND(gmeow:heldIn(?r, ?s) AS ?h) }}"
        );
        let pattern = select_pattern_with(&q, &gmeow_options());
        let text = crate::serialize::pattern_to_select_query(&pattern);
        assert!(
            text.contains(&format!("<{GMEOW_NS}heldIn>")),
            "serialization must emit the original IRI; text = {text}"
        );
        assert!(
            !text.contains(EXT_NS),
            "no other configured namespace may leak into serialized output; text = {text}"
        );
        let reparsed = find_held_in(&select_pattern_with(&text, &gmeow_options()))
            .unwrap_or_else(|| panic!("re-parse lost the extension dispatch; text = {text}"));
        assert_eq!(reparsed, held_in_call(GMEOW_NS));
    }

    #[test]
    fn extension_serialize_round_trips() {
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:heldIn(?r, ?s) AS ?h) }}");
        let pattern = select_pattern_with(&q, &ext_options());
        let text = crate::serialize::pattern_to_select_query(&pattern);
        // The serialized query must still re-parse to the same HeldIn dispatch.
        let reparsed_expr = find_held_in(&select_pattern_with(&text, &ext_options()))
            .unwrap_or_else(|| panic!("round-trip lost the extension dispatch; text = {text}"));
        assert_eq!(reparsed_expr, held_in_call(EXT_NS));
    }

    /// Walk a graph pattern for the first `FunctionCall(Function::Purrdf(_), …)`,
    /// returning its `Function`. Tolerant of the exact `Extend`/`Project` nesting the
    /// serializer round-trip produces.
    fn find_held_in(p: &GraphPattern) -> Option<Function> {
        match p {
            GraphPattern::Extend {
                inner, expression, ..
            } => {
                if let Expression::FunctionCall(f @ Function::Purrdf(_), _) = expression {
                    return Some(f.clone());
                }
                find_held_in(inner)
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Filter { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::OrderBy { inner, .. } => find_held_in(inner),
            _ => None,
        }
    }

    // ── SEP-0009 composite-datatype functions (spec-fixed; ALWAYS on) ─────────

    /// The SEP-0009 namespace, as the spec defines it. Third-party and fixed —
    /// this crate reads it, exactly as it reads the `xsd:` namespace.
    const CDT_NS: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/";

    /// A prologue binding `cdt:` to the SEP-0009 namespace.
    fn cdt_prologue() -> String {
        format!("PREFIX cdt: <{CDT_NS}>\n")
    }

    #[test]
    fn every_cdt_function_is_recognized_in_call_position() {
        // The registry is closed and the parser must recognize ALL of it, so this
        // enumerates `CDT_FUNCTIONS` rather than transcribing a list that can drift.
        for fn_kind in purrdf_cdt::CDT_FUNCTIONS {
            // The smallest admissible call for this signature.
            let argc = match fn_kind.arity() {
                crate::algebra::CdtArity::Fixed(n) => n,
                crate::algebra::CdtArity::Range { min, .. }
                | crate::algebra::CdtArity::AtLeast(min) => min,
                crate::algebra::CdtArity::Pairs => 2,
            };
            let args = vec!["1"; argc].join(", ");
            let q = format!(
                "{}SELECT ?x WHERE {{ BIND(cdt:{}({args}) AS ?x) }}",
                cdt_prologue(),
                fn_kind.local_name()
            );
            let Expression::FunctionCall(func, parsed) = bound_expr(&q) else {
                panic!("expected a FunctionCall for cdt:{}", fn_kind.local_name());
            };
            assert_eq!(
                func,
                Function::Cdt(crate::algebra::CdtCall {
                    fn_kind,
                    iri: fn_kind.iri().to_owned(),
                })
            );
            assert_eq!(parsed.len(), argc);
        }
    }

    #[test]
    fn cdt_recognition_needs_no_parser_options() {
        // SEP-0009 fixes the namespace, so recognition is unconditional: the DEFAULT
        // options (no configured extension namespace at all) still dispatch.
        assert!(ParserOptions::default().extension_fn_namespaces.is_empty());
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND(cdt:size(\"[]\"^^cdt:List) AS ?x) }}",
            cdt_prologue()
        );
        let Expression::FunctionCall(func, _) = bound_expr(&q) else {
            panic!("expected a FunctionCall");
        };
        assert!(matches!(func, Function::Cdt(_)), "got {func:?}");
    }

    #[test]
    fn a_configured_extension_namespace_cannot_shadow_a_cdt_function() {
        // Configuring the SEP-0009 namespace as an extension-function namespace must
        // NOT reroute `cdt:get` into the `PurrdfFn` seam (where its local name is
        // unknown and would hard-fail): the CDT check runs first, unconditionally.
        let options = ParserOptions {
            extension_fn_namespaces: vec![CDT_NS.to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        };
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND(cdt:get(\"[1]\"^^cdt:List, 1) AS ?x) }}",
            cdt_prologue()
        );
        let Expression::FunctionCall(func, _) = bound_expr_with(&q, &options) else {
            panic!("expected a FunctionCall");
        };
        assert!(
            matches!(&func, Function::Cdt(call) if call.fn_kind == purrdf_cdt::CdtFn::Get),
            "got {func:?}"
        );
    }

    #[test]
    fn cdt_iri_outside_call_position_is_a_plain_named_node() {
        // `cdt:List` is also the DATATYPE IRI. Outside call position it is an
        // ordinary term, never a function.
        let q = format!("{}SELECT ?x WHERE {{ ?x a cdt:List }}", cdt_prologue());
        let GraphPattern::Bgp { patterns } =
            unproject(select_pattern_with(&q, &ParserOptions::default()))
        else {
            panic!("expected BGP");
        };
        let TermPattern::NamedNode(n) = &patterns[0].object else {
            panic!("expected a NamedNode object");
        };
        assert_eq!(n.as_str(), format!("{CDT_NS}List"));
    }

    #[test]
    fn a_wrong_arity_cdt_call_is_a_typed_parse_error() {
        // SPARQL has no overloading on argument count, so this is a STATIC error.
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND(cdt:head(\"[1]\"^^cdt:List, 2) AS ?x) }}",
            cdt_prologue()
        );
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(&err, ParseError::CdtArity { iri, found: 2, .. } if iri == &format!("{CDT_NS}head")),
            "got {err:?}"
        );
    }

    #[test]
    fn an_odd_argument_count_to_the_map_constructor_is_a_parse_error() {
        // `cdt:Map` takes key/value PAIRS; an odd count would silently drop the
        // trailing key, so it is refused outright.
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND(cdt:Map(1, 2, 3) AS ?x) }}",
            cdt_prologue()
        );
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(&err, ParseError::CdtArity { found: 3, .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains("even number"), "got {err}");
        // An even count is admitted, including zero.
        for args in ["", "1, 2", "1, 2, 3, 4"] {
            let q = format!(
                "{}SELECT ?x WHERE {{ BIND(cdt:Map({args}) AS ?x) }}",
                cdt_prologue()
            );
            assert!(
                SparqlParser::new().parse_query(&q).is_ok(),
                "cdt:Map({args})"
            );
        }
    }

    #[test]
    fn cdt_subseq_admits_two_or_three_arguments_and_nothing_else() {
        let call = |args: &str| {
            let q = format!(
                "{}SELECT ?x WHERE {{ BIND(cdt:subseq({args}) AS ?x) }}",
                cdt_prologue()
            );
            SparqlParser::new().parse_query(&q)
        };
        assert!(call("\"[1]\"^^cdt:List, 1").is_ok());
        assert!(call("\"[1]\"^^cdt:List, 1, 1").is_ok());
        assert!(matches!(
            call("\"[1]\"^^cdt:List").unwrap_err(),
            ParseError::CdtArity { found: 1, .. }
        ));
        assert!(matches!(
            call("\"[1]\"^^cdt:List, 1, 1, 1").unwrap_err(),
            ParseError::CdtArity { found: 4, .. }
        ));
    }

    #[test]
    fn a_cdt_call_serializes_to_its_original_iri_and_round_trips() {
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND(cdt:concat(\"[1]\"^^cdt:List, \"[2]\"^^cdt:List) AS ?x) }}",
            cdt_prologue()
        );
        let pattern = select_pattern_with(&q, &ParserOptions::default());
        let text = crate::serialize::pattern_to_select_query(&pattern);
        assert!(
            text.contains(&format!("<{CDT_NS}concat>")),
            "serialization must emit the spec IRI verbatim; text = {text}"
        );
        // A re-parse of the serialized text reproduces the SAME algebra, byte for
        // byte — the round trip is the identity on this node.
        let reparsed = select_pattern_with(&text, &ParserOptions::default());
        assert_eq!(
            crate::serialize::pattern_to_select_query(&reparsed),
            text,
            "re-serializing the re-parse must be a fixpoint"
        );
    }

    #[test]
    fn an_ill_formed_cdt_literal_parses_and_is_left_to_evaluation() {
        // `list-functions/list-less-than-error-03.rq` writes `"1"^^cdt:List` — the
        // manifest calls it an "ill-formed literal" — and requires the COMPARISON to
        // raise a SPARQL error (an unbound `BIND`), not the query to fail to parse.
        // A datatype IRI does not constrain what the parser accepts in a literal, for
        // `cdt:List` any more than for `xsd:integer`; ill-typedness is an evaluation-
        // time property of the term. So this must parse, and the lexical form must
        // survive byte-for-byte.
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND((\"1\"^^cdt:List < \"[2]\"^^cdt:List) AS ?x) }}",
            cdt_prologue()
        );
        let Expression::Less(left, _) = bound_expr(&q) else {
            panic!("expected a `<` comparison");
        };
        let Expression::Literal(literal) = *left else {
            panic!("expected a literal operand");
        };
        assert_eq!(literal.value(), "1");
        assert_eq!(literal.datatype().as_str(), format!("{CDT_NS}List"));
        // The same holds for a wholly unparseable form, and for `cdt:Map`.
        for lexical in ["[1,", "not a list at all"] {
            let q = format!(
                "{}ASK {{ FILTER(\"{lexical}\"^^cdt:Map = \"{{}}\"^^cdt:Map) }}",
                cdt_prologue()
            );
            assert!(
                SparqlParser::new().parse_query(&q).is_ok(),
                "an ill-formed cdt:Map literal must still parse: {lexical}"
            );
        }
    }

    #[test]
    fn a_well_formed_cdt_literal_keeps_its_lexical_form_verbatim() {
        // `list-functions/sameterm-04.rq` requires `cdt:List(1,2,3)` NOT to be
        // `sameTerm` with `"[  1 ,  2  ,   3   ]"^^cdt:List`, which is only possible
        // if the parser leaves an authored lexical form alone. No canonicalization
        // happens here — the byte-fidelity rule for literals is not suspended for
        // a datatype PurRDF happens to model.
        let spelling = "[  1 ,  2  ,   3   ]";
        let q = format!(
            "{}SELECT ?x WHERE {{ BIND(\"{spelling}\"^^cdt:List AS ?x) }}",
            cdt_prologue()
        );
        let Expression::Literal(literal) = bound_expr(&q) else {
            panic!("expected a literal");
        };
        assert_eq!(literal.value(), spelling);
    }

    // ── SEP-0009 FOLD (grammar `[127+]`, a KEYWORD Aggregate alternative) ─────

    /// The single `(variable, AggregateExpression)` a `SELECT (FOLD(…) AS ?x)`
    /// query lifts, asserting there is EXACTLY one.
    fn only_aggregate(q: &str) -> AggregateExpression {
        let mut pattern = select_pattern_with(q, &ParserOptions::default());
        loop {
            match pattern {
                GraphPattern::Group { aggregates, .. } => {
                    assert_eq!(
                        aggregates.len(),
                        1,
                        "expected exactly one lifted aggregate, got {}",
                        aggregates.len()
                    );
                    return aggregates.into_iter().next().expect("one aggregate").1;
                }
                GraphPattern::Project { inner, .. }
                | GraphPattern::Extend { inner, .. }
                | GraphPattern::Filter { inner, .. }
                | GraphPattern::Distinct { inner }
                | GraphPattern::Reduced { inner }
                | GraphPattern::Slice { inner, .. }
                | GraphPattern::OrderBy { inner, .. } => pattern = *inner,
                other => panic!("no Group node under {other:?}"),
            }
        }
    }

    #[test]
    fn fold_is_a_keyword_aggregate_not_the_agg_iri_surface() {
        // SEP-0009 `[127+]` adds FOLD beside SUM and GROUP_CONCAT. It must NOT be
        // spelled through `AGG(<iri>, …)`: the spec defines no aggregate IRI, and
        // inventing one would mint a vocabulary IRI.
        let agg = only_aggregate(&format!(
            "{}SELECT (FOLD(?v) AS ?l) WHERE {{ ?s ?p ?v }}",
            cdt_prologue()
        ));
        assert_eq!(agg.function(), &AggregateFunction::Fold);
        assert_eq!(agg.args().len(), 1);
        assert!(agg.order_by().is_empty());
        assert!(!agg.distinct);
    }

    #[test]
    fn fold_takes_a_second_argument_for_the_map_form() {
        let agg = only_aggregate(&format!(
            "{}SELECT (FOLD(?k, ?v) AS ?m) WHERE {{ ?k ?p ?v }}",
            cdt_prologue()
        ));
        assert_eq!(agg.function(), &AggregateFunction::Fold);
        assert_eq!(agg.args().len(), 2);
    }

    #[test]
    fn fold_parses_distinct_and_every_order_condition_shape() {
        // `DISTINCT` precedes the arguments; the ORDER BY tail admits the bare,
        // `ASC(…)` and `DESC(…)` forms, one or more of them.
        let agg = only_aggregate(&format!(
            "{}SELECT (FOLD(DISTINCT ?v ORDER BY ?a ASC(?b) DESC(?c)) AS ?l) \
             WHERE {{ ?a ?b ?c . ?a ?b ?v }}",
            cdt_prologue()
        ));
        assert!(agg.distinct);
        assert_eq!(agg.args().len(), 1);
        assert_eq!(agg.order_by().len(), 3);
        assert!(matches!(agg.order_by()[0], OrderExpression::Asc(_)));
        assert!(matches!(agg.order_by()[1], OrderExpression::Asc(_)));
        assert!(matches!(agg.order_by()[2], OrderExpression::Desc(_)));
    }

    #[test]
    fn fold_order_by_belongs_to_the_map_form_too() {
        let agg = only_aggregate(&format!(
            "{}SELECT (FOLD(?k, ?v ORDER BY DESC(?s)) AS ?m) WHERE {{ ?k ?s ?v }}",
            cdt_prologue()
        ));
        assert_eq!(agg.args().len(), 2);
        assert_eq!(agg.order_by().len(), 1);
    }

    #[test]
    fn fold_rejects_the_star_exprlist_and_still_accepts_one_argument() {
        // `FOLD(*)` names nothing: the `'*'` shorthand is COUNT's alone.
        let star = format!(
            "{}SELECT (FOLD(*) AS ?l) WHERE {{ ?s ?p ?v }}",
            cdt_prologue()
        );
        let err = SparqlParser::new()
            .parse_query(&star)
            .expect_err("FOLD(*) is not SPARQL");
        assert!(err.to_string().contains('*'), "got {err}");
        // The neighbouring VALID case still parses.
        let ok = format!(
            "{}SELECT (FOLD(?v) AS ?l) WHERE {{ ?s ?p ?v }}",
            cdt_prologue()
        );
        assert!(SparqlParser::new().parse_query(&ok).is_ok());
    }

    #[test]
    fn fold_rejects_a_third_argument_and_still_accepts_two() {
        let three = format!(
            "{}SELECT (FOLD(?a, ?b, ?c) AS ?m) WHERE {{ ?a ?b ?c }}",
            cdt_prologue()
        );
        assert!(
            SparqlParser::new().parse_query(&three).is_err(),
            "FOLD has no three-argument form"
        );
        // The neighbouring VALID case still parses.
        let two = format!(
            "{}SELECT (FOLD(?a, ?b) AS ?m) WHERE {{ ?a ?p ?b }}",
            cdt_prologue()
        );
        assert!(SparqlParser::new().parse_query(&two).is_ok());
    }

    #[test]
    fn fold_rejects_an_empty_order_by_and_still_accepts_a_populated_one() {
        // `OrderCondition+` is one or more: a bare `ORDER BY` names no sort key
        // and must be a hard syntax error, never a silent unordered fold.
        let empty = format!(
            "{}SELECT (FOLD(?v ORDER BY) AS ?l) WHERE {{ ?s ?p ?v }}",
            cdt_prologue()
        );
        let err = SparqlParser::new()
            .parse_query(&empty)
            .expect_err("FOLD's ORDER BY requires a condition");
        assert!(
            err.to_string().contains("at least one sort condition"),
            "the refusal must say what was missing, got {err}"
        );
        // The neighbouring VALID case still parses.
        let ok = format!(
            "{}SELECT (FOLD(?v ORDER BY ?v) AS ?l) WHERE {{ ?s ?p ?v }}",
            cdt_prologue()
        );
        assert!(SparqlParser::new().parse_query(&ok).is_ok());
    }

    #[test]
    fn fold_round_trips_through_the_serializer() {
        for query in [
            "SELECT (FOLD(?v) AS ?l) WHERE { ?s ?p ?v }",
            "SELECT (FOLD(DISTINCT ?v) AS ?l) WHERE { ?s ?p ?v }",
            "SELECT (FOLD(?k, ?v) AS ?m) WHERE { ?k ?p ?v }",
            "SELECT (FOLD(?v ORDER BY ?v) AS ?l) WHERE { ?s ?p ?v }",
            "SELECT (FOLD(DISTINCT ?k, ?v ORDER BY ASC(?k) DESC(?v)) AS ?m) WHERE { ?k ?p ?v }",
        ] {
            let q = format!("{}{query}", cdt_prologue());
            let pattern = select_pattern_with(&q, &ParserOptions::default());
            let text = crate::serialize::pattern_to_select_query(&pattern);
            assert!(
                text.contains("FOLD("),
                "the serializer must emit the FOLD keyword; text = {text}"
            );
            let reparsed = select_pattern_with(&text, &ParserOptions::default());
            assert_eq!(
                crate::serialize::pattern_to_select_query(&reparsed),
                text,
                "re-serializing the re-parse must be a fixpoint for: {query}"
            );
            // The aggregate itself must survive byte-identically. The whole
            // PATTERN cannot be compared: the serializer wraps every aggregate
            // query in `SELECT * WHERE { { … } }` (`SUM` no less than `FOLD`),
            // so a re-parse gains one `Project` node — a pre-existing property
            // of the renderer, not of this production.
            assert_eq!(
                only_aggregate(&text),
                only_aggregate(&q),
                "the round trip must preserve the aggregate exactly for: {query}"
            );
        }
    }

    #[test]
    fn only_fold_may_carry_order_by_sort_keys() {
        // The checked constructor is the gate: no other aggregate has an ORDER BY
        // surface to have parsed one from, so attaching one is refused rather than
        // rendered as text no parser accepts.
        let keys = vec![OrderExpression::Asc(Expression::Variable(Variable::new(
            "k",
        )))];
        let args = vec![Expression::Variable(Variable::new("v"))];
        let err = AggregateExpression::rebuild(
            AggregateFunction::Sum,
            args.clone(),
            Vec::new(),
            keys.clone(),
            false,
        )
        .expect_err("SUM admits no ORDER BY");
        assert!(matches!(
            err,
            crate::algebra::AggregateExpressionError::OrderBy(_)
        ));
        // The neighbouring VALID cases: the same keys on FOLD, and the same SUM
        // with no keys.
        assert!(AggregateExpression::new_fold(args.clone(), keys, false).is_ok());
        assert!(AggregateExpression::new(AggregateFunction::Sum, args, Vec::new(), false).is_ok());
    }

    #[test]
    fn a_unary_builtin_aggregate_refuses_a_second_argument() {
        // A second argument no evaluator reads is a silently discarded value, so
        // the constructor refuses it outright.
        let two = vec![
            Expression::Variable(Variable::new("a")),
            Expression::Variable(Variable::new("b")),
        ];
        for function in [
            AggregateFunction::Sum,
            AggregateFunction::Avg,
            AggregateFunction::Min,
            AggregateFunction::Max,
            AggregateFunction::Sample,
            AggregateFunction::GroupConcat,
            AggregateFunction::Count,
        ] {
            let err = AggregateExpression::new(function.clone(), two.clone(), Vec::new(), false)
                .expect_err("a unary built-in admits exactly one argument");
            assert!(
                matches!(&err, crate::algebra::AggregateExpressionError::Arity(e) if e.arity() == 2),
                "{function:?}: got {err:?}"
            );
            // The neighbouring VALID case: the same function, one argument.
            assert!(
                AggregateExpression::new(function.clone(), vec![two[0].clone()], Vec::new(), false)
                    .is_ok(),
                "{function:?} must still admit one argument"
            );
        }
        // FOLD is the one built-in that DOES admit two.
        assert!(AggregateExpression::new_fold(two, Vec::new(), false).is_ok());
    }

    // ── property-function seam (caller-configured; OFF by default) ────────────

    /// A caller-configured property-function namespace for these tests (a
    /// neutral example.org name — purrdf itself mints no vocabulary IRIs).
    const PF_NS: &str = "https://example.org/pf/";

    /// A prologue binding `pf:` to the test property-function namespace and `ex:`
    /// to an ordinary data namespace outside it.
    const PFP: &str = "PREFIX pf: <https://example.org/pf/>\nPREFIX ex: <https://example.org/d/>\n";

    /// Options with only [`PF_NS`] configured as a property-function namespace.
    fn pf_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: vec![PF_NS.to_owned()],
            property_fn_iris: Vec::new(),
        }
    }

    /// Parse a SELECT (with the `PFP` prologue) under [`pf_options`] and return
    /// its WHERE algebra.
    fn pf_pattern(q: &str) -> GraphPattern {
        unproject(select_pattern_with(&format!("{PFP}{q}"), &pf_options()))
    }

    /// [`pf_pattern`] under the DEFAULT options (the seam off).
    fn pf_pattern_off(q: &str) -> GraphPattern {
        unproject(select_pattern_with(
            &format!("{PFP}{q}"),
            &ParserOptions::default(),
        ))
    }

    /// The parse error of a SELECT parsed under [`pf_options`].
    fn pf_err(q: &str) -> ParseError {
        SparqlParser::new()
            .parse_query_with(&format!("{PFP}{q}"), &pf_options())
            .expect_err("query should fail to parse")
    }

    fn pf_var(n: &str) -> TermPattern {
        TermPattern::Variable(Variable::new(n))
    }

    fn pf_iri(s: &str) -> TermPattern {
        TermPattern::NamedNode(NamedNode::new_unchecked(s))
    }

    /// Collect every property-function call of a pattern, in left-to-right order.
    fn pf_calls(p: &GraphPattern) -> Vec<&PropertyFunctionCall> {
        let mut out = Vec::new();
        fn walk<'a>(p: &'a GraphPattern, out: &mut Vec<&'a PropertyFunctionCall>) {
            match p {
                GraphPattern::PropertyFunction(c) => out.push(c),
                GraphPattern::Join { left, right }
                | GraphPattern::Lateral { left, right }
                | GraphPattern::Union { left, right }
                | GraphPattern::Minus { left, right } => {
                    walk(left, out);
                    walk(right, out);
                }
                GraphPattern::LeftJoin { left, right, .. } => {
                    walk(left, out);
                    walk(right, out);
                }
                GraphPattern::Filter { inner, .. }
                | GraphPattern::Graph { inner, .. }
                | GraphPattern::Project { inner, .. }
                | GraphPattern::Extend { inner, .. } => walk(inner, out),
                _ => {}
            }
        }
        walk(p, &mut out);
        out
    }

    /// The one property-function call of a pattern.
    fn pf_only_call(p: &GraphPattern) -> &PropertyFunctionCall {
        let calls = pf_calls(p);
        assert_eq!(calls.len(), 1, "expected exactly one call in {p:?}");
        calls[0]
    }

    /// Every triple pattern of a parsed block, in order.
    fn pf_triples(p: &GraphPattern) -> Vec<TriplePattern> {
        let mut out = Vec::new();
        fn walk(p: &GraphPattern, out: &mut Vec<TriplePattern>) {
            match p {
                GraphPattern::Bgp { patterns } => out.extend(patterns.iter().cloned()),
                GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                    walk(left, out);
                    walk(right, out);
                }
                _ => {}
            }
        }
        walk(p, &mut out);
        out
    }

    #[test]
    fn default_options_have_no_property_fn_namespaces() {
        // The seam is OFF by default: with no configured namespace the very same
        // query text parses to the ordinary BGP triple pattern it always did.
        assert!(ParserOptions::default().property_fn_namespaces.is_empty());
        let q = "SELECT * WHERE { ?s pf:related ?o }";
        assert_eq!(
            pf_pattern_off(q),
            GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: pf_var("s"),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                        "https://example.org/pf/related"
                    )),
                    object: pf_var("o"),
                }]
            }
        );
        assert!(pf_calls(&pf_pattern_off(q)).is_empty());
    }

    #[test]
    fn seam_off_keeps_collection_desugaring_byte_identical() {
        // With no configured namespace a collection in either position is still
        // the standard rdf:first/rdf:rest cons-cell chain — unchanged, including
        // the synthetic blank-node numbering.
        let q = "SELECT * WHERE { ( ?a ?b ) pf:related ( ?c ) }";
        let GraphPattern::Bgp { patterns } = pf_pattern_off(q) else {
            panic!("expected a plain BGP with the seam off");
        };
        // 2-element list (4 triples) + 1-element list (2 triples) + the triple.
        assert_eq!(patterns.len(), 7);
        assert!(patterns.iter().any(
            |t| t.predicate == NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_FIRST))
        ));
    }

    #[test]
    fn seam_on_leaves_non_matching_predicates_untouched() {
        // Configuring a namespace changes NOTHING for a predicate outside it —
        // the collection subject still desugars, blank numbering and all.
        let q = "SELECT * WHERE { ( ?a ?b ) ex:data ?o . ?s ex:p ( ?c ) }";
        assert_eq!(pf_pattern(q), pf_pattern_off(q));
        // The same for every other blank-minting form in one block (nested
        // collections, blank-node property lists, reifiers and annotations):
        // the synthetic blank-node numbering is untouched by the seam.
        let rich = "SELECT * WHERE { \
                    ( ?a ( ?b ) [ ex:q ?c ] ) ex:data [ ex:r ?d ] . \
                    ?s ex:p ?o ~ ?r {| ex:note \"n\" |} . \
                    << ?x ex:y ?z >> ex:p ?w }";
        assert_eq!(pf_pattern(rich), pf_pattern_off(rich));
    }

    #[test]
    fn a_call_may_hang_off_a_blank_node_property_list() {
        // The seam lives in the shared predicate-object-list path, so a call
        // inside `[ … ]` works and its subject argument is that blank node.
        let p = pf_pattern("SELECT * WHERE { [ pf:solve ?x ] ex:p ?o }");
        let call = pf_only_call(&p);
        assert!(matches!(call.subject_args[0], TermPattern::BlankNode(_)));
        assert_eq!(call.object_args, vec![pf_var("x")]);
        let triples = pf_triples(&p);
        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].subject, call.subject_args[0]);
    }

    #[test]
    fn configured_predicate_mints_a_property_function() {
        let p = pf_pattern("SELECT * WHERE { ?s pf:related ?o }");
        assert_eq!(
            p,
            GraphPattern::Lateral {
                left: Box::new(GraphPattern::Bgp { patterns: vec![] }),
                right: Box::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
                    iri: format!("{PF_NS}related"),
                    subject_args: vec![pf_var("s")],
                    object_args: vec![pf_var("o")],
                })),
            }
        );
    }

    #[test]
    fn full_iri_predicate_mints_a_property_function() {
        // The same recognition via a full (non-prefixed) IRI, retained byte-exact.
        let p = pf_pattern("SELECT * WHERE { ?s <https://example.org/pf/related> ?o }");
        assert_eq!(pf_only_call(&p).iri, format!("{PF_NS}related"));
    }

    #[test]
    fn object_collection_is_an_argument_vector_not_cons_cells() {
        let p = pf_pattern("SELECT * WHERE { ?s pf:solve ( ?a ?b ?c ) }");
        let call = pf_only_call(&p);
        assert_eq!(call.subject_args, vec![pf_var("s")]);
        assert_eq!(
            call.object_args,
            vec![pf_var("a"), pf_var("b"), pf_var("c")]
        );
        // NO cons cells were emitted for the argument list.
        assert!(
            pf_triples(&p).is_empty(),
            "no rdf:first/rdf:rest desugaring"
        );
    }

    #[test]
    fn subject_collection_is_an_argument_vector_not_cons_cells() {
        let p = pf_pattern("SELECT * WHERE { ( ?a ?b ) pf:solve ?o }");
        let call = pf_only_call(&p);
        assert_eq!(call.subject_args, vec![pf_var("a"), pf_var("b")]);
        assert_eq!(call.object_args, vec![pf_var("o")]);
        assert!(
            pf_triples(&p).is_empty(),
            "no rdf:first/rdf:rest desugaring"
        );
    }

    #[test]
    fn empty_collection_is_a_zero_length_argument_vector() {
        // `()` denotes NO arguments on that side …
        let p = pf_pattern("SELECT * WHERE { () pf:solve ( ) }");
        let call = pf_only_call(&p);
        assert!(call.subject_args.is_empty());
        assert!(call.object_args.is_empty());
    }

    #[test]
    fn bare_rdf_nil_is_a_one_element_argument_vector() {
        // … whereas an explicitly-spelled rdf:nil IRI is a one-element vector
        // holding that IRI — the two spellings are deliberately distinct.
        let p = pf_pattern(&format!("SELECT * WHERE {{ <{RDF_NIL}> pf:solve ?o }}"));
        let call = pf_only_call(&p);
        assert_eq!(call.subject_args, vec![pf_iri(RDF_NIL)]);
        let p2 = pf_pattern("SELECT * WHERE { () pf:solve ?o }");
        assert!(pf_only_call(&p2).subject_args.is_empty());
        assert_ne!(call.subject_args, pf_only_call(&p2).subject_args);
    }

    #[test]
    fn nested_collection_in_an_object_argument_list_is_a_hard_error() {
        let err = pf_err("SELECT * WHERE { ?s pf:solve ( ?a ( ?b ) ) }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("nested collection in property-function argument list")),
            "got {err:?}"
        );
    }

    #[test]
    fn nested_collection_in_a_subject_argument_list_is_a_hard_error() {
        let err = pf_err("SELECT * WHERE { ( ( ?a ) ?b ) pf:solve ?o }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("nested collection in property-function argument list")),
            "got {err:?}"
        );
    }

    #[test]
    fn blank_nodes_are_admitted_as_arguments() {
        // A blank node in an argument position is a non-distinguished variable;
        // the parser passes it through unchanged (labelled and anonymous alike).
        let p = pf_pattern("SELECT * WHERE { _:b pf:solve ( [] ?o ) }");
        let call = pf_only_call(&p);
        assert_eq!(
            call.subject_args,
            vec![TermPattern::BlankNode(BlankNode::new("b"))]
        );
        assert!(matches!(call.object_args[0], TermPattern::BlankNode(_)));
        assert_eq!(call.object_args[1], pf_var("o"));
    }

    #[test]
    fn populated_blank_node_property_list_argument_is_a_hard_error() {
        let err = pf_err("SELECT * WHERE { ?s pf:solve ( [ ex:p ?x ] ) }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("blank-node property list")),
            "got {err:?}"
        );
    }

    #[test]
    fn repeated_variables_pass_through_unchanged() {
        // Within one side and across both: the parser never de-duplicates or
        // rewrites — the equality semantics belong to the evaluator.
        let p = pf_pattern("SELECT * WHERE { ( ?x ?x ) pf:solve ( ?x ?y ) }");
        let call = pf_only_call(&p);
        assert_eq!(call.subject_args, vec![pf_var("x"), pf_var("x")]);
        assert_eq!(call.object_args, vec![pf_var("x"), pf_var("y")]);
    }

    #[test]
    fn literal_and_quoted_triple_arguments_are_ordinary_terms() {
        let p = pf_pattern("SELECT * WHERE { ?s pf:solve ( \"purr\" 42 <<( ?a ex:p ?b )>> ) }");
        let call = pf_only_call(&p);
        assert_eq!(call.object_args.len(), 3);
        assert!(matches!(call.object_args[0], TermPattern::Literal(_)));
        assert!(matches!(call.object_args[1], TermPattern::Literal(_)));
        let TermPattern::Triple(t) = &call.object_args[2] else {
            panic!(
                "expected a quoted triple argument, got {:?}",
                call.object_args[2]
            );
        };
        assert_eq!(t.subject, pf_var("a"));
    }

    #[test]
    fn data_triples_before_a_call_become_its_lateral_left() {
        let p = pf_pattern("SELECT * WHERE { ?s ex:name ?n . ?s pf:related ?o . ?o ex:name ?m }");
        // Textual order: Bgp(before) LATERAL PropertyFunction, then the residual
        // Bgp(after) joined on.
        let GraphPattern::Join { left, right } = &p else {
            panic!("expected the trailing data triple to join on, got {p:?}");
        };
        let GraphPattern::Lateral {
            left: inner_left,
            right: inner_right,
        } = &**left
        else {
            panic!("expected a Lateral chain, got {left:?}");
        };
        let GraphPattern::Bgp { patterns } = &**inner_left else {
            panic!("expected the preceding triples as a BGP");
        };
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].object, pf_var("n"));
        assert!(matches!(**inner_right, GraphPattern::PropertyFunction(_)));
        let GraphPattern::Bgp { patterns } = &**right else {
            panic!("expected the trailing triples as a BGP");
        };
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].object, pf_var("m"));
    }

    #[test]
    fn multiple_calls_chain_left_deep_in_textual_order() {
        let p = pf_pattern(
            "SELECT * WHERE { ?s ex:name ?n . ?s pf:first ?a . ?a ex:p ?q . ?a pf:second ?b }",
        );
        let GraphPattern::Lateral { left, right } = &p else {
            panic!("expected the outermost Lateral to be the LAST call, got {p:?}");
        };
        let GraphPattern::PropertyFunction(second) = &**right else {
            panic!("expected the second call outermost");
        };
        assert_eq!(second.iri, format!("{PF_NS}second"));
        // Its left is the first call's Lateral joined with the triples between.
        let GraphPattern::Join {
            left: chain,
            right: between,
        } = &**left
        else {
            panic!("expected the intervening triples joined onto the first call, got {left:?}");
        };
        let GraphPattern::Lateral { right: first, .. } = &**chain else {
            panic!("expected the first call's Lateral innermost");
        };
        let GraphPattern::PropertyFunction(first) = &**first else {
            panic!("expected a PropertyFunction node");
        };
        assert_eq!(first.iri, format!("{PF_NS}first"));
        let GraphPattern::Bgp { patterns } = &**between else {
            panic!("expected the intervening data triples as a BGP");
        };
        assert_eq!(patterns.len(), 1);
        // Order is the order the author wrote them.
        let calls = pf_calls(&p);
        assert_eq!(
            calls.iter().map(|c| c.iri.as_str()).collect::<Vec<_>>(),
            vec![format!("{PF_NS}first"), format!("{PF_NS}second")]
        );
    }

    #[test]
    fn object_list_gives_each_object_its_own_call() {
        // `?s pf:solve (…) , (…) ; ex:data ?o` — one call per object; the other
        // predicate of the same predicate-object list stays a data triple.
        let p = pf_pattern("SELECT * WHERE { ?s pf:solve ( ?a ) , ( ?b ?c ) ; ex:data ?o }");
        let calls = pf_calls(&p);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].object_args, vec![pf_var("a")]);
        assert_eq!(calls[1].object_args, vec![pf_var("b"), pf_var("c")]);
        assert_eq!(calls[0].subject_args, vec![pf_var("s")]);
        let triples = pf_triples(&p);
        assert_eq!(
            triples.len(),
            1,
            "the ex:data triple stays in the residual BGP"
        );
        assert_eq!(triples[0].object, pf_var("o"));
    }

    #[test]
    fn variable_predicate_is_never_a_property_function() {
        // Even when the variable would bind to a configured-namespace IRI: only a
        // plain IRI predicate is recognized, at parse time.
        let p = pf_pattern("SELECT * WHERE { ?s ?p ?o }");
        assert!(pf_calls(&p).is_empty());
        assert_eq!(pf_triples(&p).len(), 1);
    }

    #[test]
    fn property_path_predicate_is_never_a_property_function() {
        for q in [
            "SELECT * WHERE { ?s pf:related+ ?o }",
            "SELECT * WHERE { ?s pf:related/ex:p ?o }",
            "SELECT * WHERE { ?s ^pf:related ?o }",
            "SELECT * WHERE { ?s !(pf:related) ?o }",
        ] {
            let p = pf_pattern(q);
            assert!(pf_calls(&p).is_empty(), "`{q}` must stay a property path");
            assert!(matches!(p, GraphPattern::Path { .. }), "`{q}` → {p:?}");
        }
        // …so a collection subject in front of one is still an ordinary
        // collection, cons cells and all — identical to the seam-off parse.
        let q = "SELECT * WHERE { ( ?a ?b ) pf:related+ ?o }";
        assert_eq!(pf_pattern(q), pf_pattern_off(q));
        assert!(pf_calls(&pf_pattern(q)).is_empty());
    }

    #[test]
    fn argument_variables_are_visible_in_the_enclosing_group() {
        // SELECT * projects every argument variable, both sides, in order.
        let q = format!("{PFP}SELECT * WHERE {{ ( ?a ?b ) pf:solve ( ?c ?d ) }}");
        let GraphPattern::Project { variables, .. } = select_pattern_with(&q, &pf_options()) else {
            panic!("expected a Project wrapper");
        };
        assert_eq!(
            variables,
            vec![
                Variable::new("a"),
                Variable::new("b"),
                Variable::new("c"),
                Variable::new("d"),
            ]
        );
    }

    #[test]
    fn a_filter_in_the_group_sees_an_argument_variable() {
        let p = pf_pattern("SELECT * WHERE { ?s pf:solve ?o FILTER(?o > 2) }");
        let GraphPattern::Filter { inner, .. } = &p else {
            panic!("expected a Filter, got {p:?}");
        };
        assert_eq!(pf_calls(inner).len(), 1);
        // §19.6: BIND may not re-bind a variable already in scope — proof that the
        // scope walker really does see the call's argument variables.
        let err = pf_err("SELECT * WHERE { ?s pf:solve ?o BIND(1 AS ?o) }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. } if reason.contains("already in scope")),
            "got {err:?}"
        );
    }

    #[test]
    fn annotation_syntax_cannot_annotate_a_call() {
        let err = pf_err("SELECT * WHERE { ?s pf:solve ?o {| ex:p ?x |} }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("cannot annotate a property-function call")),
            "got {err:?}"
        );
    }

    #[test]
    fn an_argument_list_cannot_head_a_data_triple() {
        // `( ?a ?b ) pf:solve ?o ; ex:data ?x` — the subject group is an argument
        // vector, which has no term form to hang the second predicate off.
        let err = pf_err("SELECT * WHERE { ( ?a ?b ) pf:solve ?o ; ex:data ?x }");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("cannot be the subject of an ordinary triple pattern")),
            "got {err:?}"
        );
    }

    #[test]
    fn property_functions_are_rejected_in_templates() {
        // A template asserts triples; a relation call has nothing to assert.
        let update = format!("{PFP}INSERT {{ ?s pf:solve ?o }} WHERE {{ ?s ex:p ?o }}");
        let err = SparqlParser::new()
            .parse_update_with(&update, &pf_options())
            .expect_err("a property function in an INSERT template must be refused");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("property functions are not allowed in an update template")),
            "got {err:?}"
        );
        let construct = format!("{PFP}CONSTRUCT {{ ?s pf:solve ?o }} WHERE {{ ?s ex:p ?o }}");
        let err = SparqlParser::new()
            .parse_query_with(&construct, &pf_options())
            .expect_err("a property function in a CONSTRUCT template must be refused");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("property functions are not allowed in a CONSTRUCT template")),
            "got {err:?}"
        );
    }

    #[test]
    fn a_call_is_recognized_inside_every_group_construct() {
        // The seam lives in the triples-block path, so it fires wherever a
        // triples block may appear.
        for q in [
            "SELECT * WHERE { GRAPH ?g { ?s pf:solve ?o } }",
            "SELECT * WHERE { OPTIONAL { ?s pf:solve ?o } }",
            "SELECT * WHERE { { ?s pf:solve ?o } UNION { ?s ex:p ?o } }",
            "SELECT * WHERE { ?s ex:p ?o . { SELECT * WHERE { ?s pf:solve ?o } } }",
        ] {
            assert_eq!(pf_calls(&pf_pattern(q)).len(), 1, "`{q}`");
        }
        // …including the group graph pattern of a FILTER EXISTS.
        let p = pf_pattern("SELECT * WHERE { ?s ex:p ?o FILTER EXISTS { ?s pf:solve ?o } }");
        let GraphPattern::Filter { expr, .. } = &p else {
            panic!("expected a Filter, got {p:?}");
        };
        let Expression::Exists(inner) = expr else {
            panic!("expected an EXISTS expression, got {expr:?}");
        };
        assert_eq!(pf_calls(inner).len(), 1);
    }

    #[test]
    fn every_configured_namespace_is_recognized_independently() {
        // Several namespaces may be configured; each is recognized, and the IRI
        // is retained exactly as spelled under whichever matched — recognition is
        // order-independent, unlike the extension-function seam's prefix stripping.
        let options = ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: vec![PF_NS.to_owned(), "https://example.org/other/".to_owned()],
            property_fn_iris: Vec::new(),
        };
        let q = "PREFIX o: <https://example.org/other/>\n\
                 PREFIX pf: <https://example.org/pf/>\n\
                 SELECT * WHERE { ?s pf:a ?x . ?s o:b ?y }";
        let calls = pf_calls(&select_pattern_with(q, &options))
            .iter()
            .map(|c| c.iri.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            calls,
            vec![
                format!("{PF_NS}a"),
                "https://example.org/other/b".to_owned()
            ]
        );
    }

    #[test]
    fn a_configured_namespace_iri_in_term_position_is_an_ordinary_iri() {
        // Only the PREDICATE position is the seam; the same IRI as a subject or
        // object is an ordinary term.
        let p = pf_pattern("SELECT * WHERE { pf:related ex:p pf:related }");
        assert!(pf_calls(&p).is_empty());
        let triples = pf_triples(&p);
        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].subject, pf_iri(&format!("{PF_NS}related")));
        assert_eq!(triples[0].object, pf_iri(&format!("{PF_NS}related")));
    }

    // ── property_fn_iris: EXACT-match seam (registry-derived) ─────────────────

    /// Options with only `a` under [`PF_NS`] configured as an EXACT
    /// property-function IRI — no namespace configured at all.
    fn pf_exact_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: Vec::new(),
            property_fn_iris: vec![format!("{PF_NS}a")],
        }
    }

    #[test]
    fn an_exact_registered_iri_is_recognized_with_no_namespace_configured() {
        // property_fn_iris alone (property_fn_namespaces empty) is enough to
        // recognize the exact call — this is the shape `prepare_for` uses.
        let q = format!("{PFP}SELECT * WHERE {{ ?s <{PF_NS}a> ?o }}");
        let p = unproject(select_pattern_with(&q, &pf_exact_options()));
        assert_eq!(pf_only_call(&p).iri, format!("{PF_NS}a"));
    }

    #[test]
    fn an_exact_iri_registration_does_not_hijack_a_sibling_data_predicate() {
        // THE regression: registering `.../pf/a` as an exact property-function
        // IRI must NOT reclassify the unrelated, longer, same-prefixed data
        // predicate `.../pf/ab` as a property-function call. Before this fix,
        // registering an IRI pushed it into the PREFIX set, so `ab` (which
        // starts with `a`) parsed as a call to an unregistered relation and
        // hard-errored — a category error, since a registry's keys are exact
        // IRIs, not namespaces.
        let q = format!("{PFP}SELECT * WHERE {{ ?s <{PF_NS}ab> ?o }}");
        let p = unproject(select_pattern_with(&q, &pf_exact_options()));
        assert!(
            pf_calls(&p).is_empty(),
            "a longer, merely-prefix-sharing IRI must stay an ordinary triple \
             pattern, got {p:?}"
        );
        let triples = pf_triples(&p);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].predicate,
            NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!("{PF_NS}ab")))
        );
    }

    #[test]
    fn property_fn_iris_and_property_fn_namespaces_recognition_is_a_union() {
        // A caller-declared namespace still prefix-matches, and an exact IRI
        // still exact-matches, when both are configured at once.
        let options = ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: vec!["https://example.org/other/".to_owned()],
            property_fn_iris: vec![format!("{PF_NS}a")],
        };
        let q = "PREFIX o: <https://example.org/other/>\n\
                 PREFIX pf: <https://example.org/pf/>\n\
                 SELECT * WHERE { ?s pf:a ?x . ?s o:anything ?y }";
        let calls = pf_calls(&select_pattern_with(q, &options))
            .iter()
            .map(|c| c.iri.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            calls,
            vec![
                format!("{PF_NS}a"),
                "https://example.org/other/anything".to_owned()
            ]
        );
    }

    // ---- ADJUST() ----------------------------------------------------------

    #[test]
    fn adjust_parses_as_a_two_arg_function_call() {
        let q = "SELECT ?h WHERE { ?s ?p ?o . \
                  BIND(ADJUST(?o, \"PT1H\"^^<http://www.w3.org/2001/XMLSchema#dayTimeDuration>) \
                  AS ?h) }";
        let Expression::FunctionCall(func, args) = bound_expr(q) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, Function::Adjust);
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn adjust_with_one_argument_is_a_syntax_error() {
        // ADJUST has exactly one documented (2-argument) signature (SEP-0002);
        // the generic builtin-call path does not arity-check on its own, so
        // this pins the dedicated `expect_arity` gate added for it.
        let q = "SELECT ?h WHERE { ?s ?p ?o . BIND(ADJUST(?o) AS ?h) }";
        let error = SparqlParser::new()
            .parse_query(q)
            .expect_err("ADJUST with one argument must be refused at parse time");
        assert!(
            matches!(error, ParseError::Syntax { .. }),
            "expected a typed syntax error, got {error:?}"
        );
        assert!(error.to_string().contains("ADJUST"));
    }

    #[test]
    fn adjust_with_three_arguments_is_a_syntax_error() {
        let q = "SELECT ?h WHERE { ?s ?p ?o . \
                  BIND(ADJUST(?o, \"PT1H\"^^<http://www.w3.org/2001/XMLSchema#dayTimeDuration>, ?o) \
                  AS ?h) }";
        let error = SparqlParser::new()
            .parse_query(q)
            .expect_err("ADJUST with three arguments must be refused at parse time");
        assert!(matches!(error, ParseError::Syntax { .. }));
    }
}
