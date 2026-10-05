// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The public parse entry point and the parser that turns a SPARQL 1.1/1.2 query
//! into the [`Query`] algebra.
//!
//! The parser translates *directly* into the W3C SPARQL algebra (§18.2) rather
//! than building a separate syntax tree: group graph patterns accumulate into
//! `Join`/`LeftJoin`/`Filter`/`Extend`/`Union`/`Minus`/`Graph`, solution
//! modifiers wrap the result as `Group`/`OrderBy`/`Project`/`Distinct`/`Slice`,
//! and aggregates are lifted to synthetic variables in a `Group` node (the
//! standard §18.2.4 mechanism). Anything outside the corpus-driven scope is a
//! hard [`ParseError::Unsupported`].
//!
//! # Nesting
//!
//! No production calls itself, directly or through another. Every construct that
//! nests — a group graph pattern and each element that holds one, a sub-`SELECT`, an
//! `EXISTS` body, a bracketted expression, a prefix operator, an argument or `IN`
//! list, an aggregate, an expression triple term, a property-path group, a blank-node
//! property list, a collection, a triple term, a reifying triple and an annotation
//! block — is read by a loop that keeps what encloses the cursor on an explicit,
//! heap-allocated stack: groups, sub-`SELECT`s, modifiers and expressions in one
//! pushdown machine (`parser/machine.rs`), triples, paths and terms in loops of their
//! own (`parser/triples.rs`). How deeply a request nests is therefore bounded by the
//! memory those stacks grow into and nothing else, the same on every host and on a
//! thread of any stack size; the tree it returns is walked, copied and dropped
//! without recursion too (see [`crate::walk`]).

mod machine;
mod triples;

use machine::Machine;
use triples::{PathLevel, TFrame};

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use purrdf_hash::fixed::FixedState;

use crate::algebra::{
    AggregateFunction, Expression, Function, GraphPattern, GraphTarget, GraphUpdateOperation,
    OrderExpression, PropertyFunctionCall, PropertyPathExpression, Query, QueryDataset,
    SparqlVersion, Update, UsingClause,
};
use crate::ast::{
    BaseDirection, BlankNode, GroundTerm, Literal, NamedNode, NamedNodePattern, QuadPattern,
    TermPattern, TriplePattern, Variable,
};
use crate::error::{ParseError, Result};
use crate::lexer::{Spanned, Token, tokenize};
use crate::tree::Child;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, IriError, LineIndex, langtag};

use purrdf_iri::vocab::rdf::{
    FIRST as RDF_FIRST, NIL as RDF_NIL, REIFIES as RDF_REIFIES, REST as RDF_REST, TYPE as RDF_TYPE,
};
use purrdf_xsd::datatype::{XSD_BOOLEAN, XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER};

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
/// [`crate::pattern_to_select_query`] protects data predicates from property-function
/// recognition under any configuration; [`crate::pattern_to_select_query_with_options`]
/// uses this configuration to protect only the predicates it claims. Both preserve
/// each [`Function::Purrdf`]'s original IRI (recorded in
/// [`crate::algebra::PurrdfCall::iri`]) and each [`GraphPattern::PropertyFunction`]'s
/// call IRI. Re-parsing with the same options therefore keeps data predicates and
/// relation calls distinct, and no namespace is fabricated on output.
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

impl ParserOptions {
    /// Whether a plain predicate IRI is claimed by the property-function seam: a
    /// namespace prefix match or an exact IRI match, independently of list order.
    pub(crate) fn is_property_fn(&self, iri: &str) -> bool {
        self.property_fn_namespaces
            .iter()
            .any(|ns| iri.starts_with(ns.as_str()))
            || self.property_fn_iris.iter().any(|exact| exact == iri)
    }
}

/// One parse of a query, with the two positions a layer that **re-wraps** the text
/// needs in order to move the parts the grammar will not let it carry.
///
/// A wrapper that puts a caller's query inside a sub-`SELECT` — `SubSelect ::=
/// SelectClause WhereClause SolutionModifier ValuesClause` — inherits that
/// production's two omissions. There is no `Prologue` inside a sub-select, and there
/// is no `DatasetClause` either; both are written once, on a whole query. A text
/// declaring either therefore cannot be wrapped as it stands, and neither position is
/// recoverable from the [`Query`] beside them: a prefixed name is resolved away at
/// parse time, and re-serializing the body would rewrite surface spellings the
/// caller's next reader depends on. So the positions are reported, the caller splits
/// its own bytes at them, and both clauses are re-written where the grammar does
/// accept them — in front of the wrapper, and on the wrapper's own `SELECT`, where
/// each scopes the whole query and therefore the body inside it, which is what the
/// caller wrote them to mean.
///
/// Everything here is positional. No field says anything the algebra does not already
/// say; they say *where it was written*.
///
/// Deliberately **not** `#[non_exhaustive]`. A wrapper reads every position here
/// precisely because a clause it fails to move is a clause the wrap destroys, and
/// `#[non_exhaustive]` would force each of those callers to destructure with `..` — so a
/// third position added later would be ignored by every one of them, silently, which is
/// the exact shape of the defect that made this type necessary. Adding a field here is a
/// breaking change, and it should be: it means there is a new thing a wrapper has to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuerySplit {
    /// The algebra, exactly as [`SparqlParser::parse_query_with`] returns it.
    pub query: Query,
    /// The byte offset the **prologue** ends at: the start of the query form's own
    /// first token, so `&text[..body_at]` is the `BASE`/`PREFIX`/`VERSION` run (plus
    /// any comment or whitespace between the directives) and `&text[body_at..]` is
    /// the form itself.
    ///
    /// `0` for a query with no prologue, where the whole text is the form.
    pub body_at: usize,
    /// The byte range the query form's `DatasetClause*` run occupies — from the
    /// first `FROM` keyword to the start of the token after the last clause, so the
    /// range carries its own trailing whitespace and excising it from the text leaves
    /// a query that still parses.
    ///
    /// `None` for a query that writes no `FROM`/`FROM NAMED`. The range is inside
    /// `&text[body_at..]`, because a dataset clause follows the query form's head.
    /// All four query forms carry one, and all four report it here.
    pub dataset_at: Option<Range<usize>>,
}

/// One parse of a query, with the position its **dataset clause** occupies — or,
/// for a query that writes none, the position where one would be written.
///
/// This is what a layer needs to *replace* a query's dataset as text: the SPARQL 1.1
/// Protocol's `default-graph-uri`/`named-graph-uri` parameters override every
/// `FROM`/`FROM NAMED` the query itself declares (Protocol §2.1.4), and applying them
/// without re-serializing the caller's text means cutting the query's own run out and
/// writing the parameters' clauses in the same place. [`QuerySplit::dataset_at`] cannot
/// serve that caller on its own: it is `None` for a query with no clause, and that is
/// exactly the query whose insertion point is still needed.
///
/// Positional only, like [`QuerySplit`], and deliberately not `#[non_exhaustive]` for
/// the same reason: a position a splicing caller fails to read is a clause its splice
/// gets wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryDatasetSlot {
    /// The algebra, exactly as [`SparqlParser::parse_query_with`] returns it.
    pub query: Query,
    /// The byte range of the query form's `DatasetClause*` run.
    ///
    /// For a query that writes one, exactly [`QuerySplit::dataset_at`]: from the first
    /// `FROM` to the start of the token after the last clause. For a query that writes
    /// none, the EMPTY range at the start of the token a dataset clause would precede
    /// (`WHERE`, or the `{` of a `WHERE`-less group), so text inserted there becomes the
    /// query's dataset clause. Always the whole query's own clause — never a
    /// sub-`SELECT`'s position, which has no dataset clause to hold.
    pub dataset_at: Range<usize>,
}

/// One parse of an update request, with the per-operation positions a layer needs in
/// order to give each operation's `WHERE` a dataset as **text**.
///
/// The SPARQL 1.1 Protocol's `using-graph-uri`/`using-named-graph-uri` parameters mean
/// what `USING`/`USING NAMED` clauses mean, applied to every operation that has a
/// `WHERE` (Protocol §2.2.3). Writing them into the request without re-serializing it
/// needs, per operation, where its `USING` run is (or would go), and whether it already
/// has a `WITH` or `USING` a parameter would conflict with.
///
/// Positional only, like [`QuerySplit`], and deliberately not `#[non_exhaustive]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateSplit {
    /// The algebra, exactly as [`SparqlParser::parse_update_with`] returns it.
    pub update: Update,
    /// One entry per operation, index-aligned with `update.operations`.
    pub operations: Vec<UpdateDatasetSlot>,
}

/// Where one update operation's dataset clause is written, by the production the
/// operation was read under (SPARQL 1.1 Update §3.1.3 and the §19 grammar).
///
/// Deliberately not `#[non_exhaustive]`: see [`UpdateSplit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateDatasetSlot {
    /// An operation with no `WHERE` clause (`INSERT DATA`, `DELETE DATA`, `LOAD`,
    /// `CLEAR`, `DROP`, `CREATE`, `ADD`, `MOVE`, `COPY`): there is no pattern for a
    /// dataset to scope.
    NoWhereClause,
    /// `[WITH <iri>] ( DELETE {…} [INSERT {…}] | INSERT {…} ) UsingClause* WHERE {…}`.
    Modify {
        /// The byte range of the `WITH <iri>` clause, from `WITH` to the start of the
        /// token after the IRI; `None` when the operation writes no `WITH`.
        with_at: Option<Range<usize>>,
        /// The byte range of the `UsingClause*` run, from the first `USING` to the start
        /// of the token after the last clause. EMPTY, at the start of the `WHERE`
        /// keyword, when the operation writes no `USING`.
        using_at: Range<usize>,
    },
    /// `DELETE WHERE QuadPattern` — the shorthand whose grammar has no `UsingClause`,
    /// defined (§3.1.3.3) as `DELETE QuadPattern WHERE QuadPattern` with the same
    /// pattern in both places. A dataset is given to it by writing that long form.
    DeleteWhere {
        /// The byte offset of the `WHERE` keyword.
        where_at: usize,
        /// The byte range of the braced `QuadPattern`, from its `{` to just past its
        /// matching `}` — the text the long form repeats as the `DELETE` template.
        pattern_at: Range<usize>,
    },
}

/// A reusable SPARQL query parser.
///
/// Consumers call `SparqlParser::new().parse_query(text)`.
/// Parse-time configuration (the extension-function namespace set) is passed per
/// call via [`SparqlParser::parse_query_with`] / [`SparqlParser::parse_update_with`];
/// the plain `parse_*` entries use [`ParserOptions::default`].
#[derive(Clone, Debug)]
pub struct SparqlParser {
    /// The caller-supplied base scope, or the failure that supplying it caused.
    ///
    /// [`SparqlParser::with_base_iri`] takes an arbitrary string and returns
    /// `Self`, so it has nowhere to report an unusable base. Recording the typed
    /// failure here and replaying it from the first fallible entry point
    /// ([`SparqlParser::parse_query_with`]/[`SparqlParser::parse_update_with`])
    /// keeps the hard-fail doctrine without silently dropping the base.
    base: core::result::Result<BaseScope, ParseError>,
}

purrdf_hash::default_from_new!(SparqlParser);

impl SparqlParser {
    /// Construct a parser with no implicit base IRI; [`Default`] delegates here.
    ///
    /// Written out rather than derived: the default base scope is an empty
    /// [`BaseScope`], not an error, and `Result` has no `Default`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: Ok(BaseScope::empty()),
        }
    }

    /// Set an implicit base IRI used to resolve relative IRI references that
    /// appear before any in-query `BASE` declaration, and against which a
    /// relative in-query `BASE` itself resolves (SPARQL 1.1 §4.1.1 → RFC-3986
    /// §5.1.1).
    ///
    /// The base must be absolute. Because this builder returns `Self`, a base
    /// that is not is reported by the subsequent `parse_query`/`parse_update`
    /// call as [`ParseError::Iri`] rather than being ignored.
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
        let base_iri = base_iri.into();
        self.base = BaseIri::parse(&base_iri)
            .map(|base| BaseScope::rooted(base, BaseOrigin::Caller))
            .map_err(|e| iri_error(&base_iri, &e));
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
        self.parse_query_split(query, options)
            .map(|split| split.query)
    }

    /// [`Self::parse_query_with`], also reporting where the two parts of the text a
    /// wrapping layer has to *move* are written: see [`QuerySplit`].
    ///
    /// Positional only. Both numbers are offsets the one parse below was already
    /// standing at, so every piece a caller cuts out of the text is the caller's own
    /// bytes rather than anything re-rendered, and nothing about the algebra returned
    /// beside them changes.
    ///
    /// # Errors
    ///
    /// Exactly [`Self::parse_query_with`]'s: a [`ParseError`] for an unusable base
    /// IRI, a tokenizer refusal, a syntax error or trailing tokens after the form.
    pub fn parse_query_split(&self, query: &str, options: &ParserOptions) -> Result<QuerySplit> {
        let mut p = self.parser_for(query, options)?;
        p.parse_prologue()?;
        let body_at = p.span();
        let q = p.parse_query_form()?;
        p.expect_eof()?;
        Ok(QuerySplit {
            query: q,
            body_at,
            dataset_at: p.dataset_at,
        })
    }

    /// [`Self::parse_query_with`], also reporting the position of the query's dataset
    /// clause, or of where one would be written: see [`QueryDatasetSlot`].
    ///
    /// # Errors
    ///
    /// Exactly [`Self::parse_query_with`]'s.
    pub fn parse_query_dataset_slot(
        &self,
        query: &str,
        options: &ParserOptions,
    ) -> Result<QueryDatasetSlot> {
        let mut p = self.parser_for(query, options)?;
        p.parse_prologue()?;
        let q = p.parse_query_form()?;
        p.expect_eof()?;
        // Every query form reads its `DatasetClause*` run through
        // `parse_dataset_clauses`, so a parsed query always has a slot. A form that
        // somehow did not would have nowhere for a dataset to go; that is reported as
        // the parse error it is rather than guessed at.
        let dataset_at = p.dataset_slot.ok_or_else(|| {
            ParseError::syntax("the query form carries no dataset-clause position", 0)
        })?;
        Ok(QueryDatasetSlot {
            query: q,
            dataset_at,
        })
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
        self.parse_update_split(update, options)
            .map(|split| split.update)
    }

    /// [`Self::parse_update_with`], also reporting where each operation's dataset
    /// clause is (or would be) written: see [`UpdateSplit`].
    ///
    /// # Errors
    ///
    /// Exactly [`Self::parse_update_with`]'s.
    pub fn parse_update_split(&self, update: &str, options: &ParserOptions) -> Result<UpdateSplit> {
        let mut p = self.parser_for(update, options)?;
        let u = p.parse_update()?;
        p.expect_eof()?;
        Ok(UpdateSplit {
            update: u,
            operations: p.update_slots,
        })
    }

    /// Tokenize `text` and assemble the internal parser state.
    fn parser_for<'a, 'o>(
        &self,
        text: &'a str,
        options: &'o ParserOptions,
    ) -> Result<Parser<'a, 'o>> {
        let base = self.base.clone()?;
        let tokens: Vec<Option<Spanned<'a>>> = tokenize(text)?.into_iter().map(Some).collect();
        let anon_prefix = anon_label_prefix(&tokens);
        Ok(Parser {
            tokens,
            pos: 0,
            src: text,
            end: text.len(),
            prefixes: HashMap::with_hasher(FixedState::new()),
            base,
            version: None,
            agg_counter: 0,
            anon_counter: 0,
            anon_prefix,
            group_counter: 0,
            exists_scope_stack: ExistsScopes::default(),
            dataset_at: None,
            dataset_slot: None,
            update_slots: Vec::new(),
            op_slot: UpdateDatasetSlot::NoWhereClause,
            projection_scope_pending: false,
            in_aggregate_argument: false,
            projection_seen_targets: Vec::new(),
            pending_exists_scope_checks: Vec::new(),
            #[cfg(debug_assertions)]
            scope_consultations: 0,
            blank_label_bgps: HashMap::with_hasher(FixedState::new()),
            bgp_counter: 0,
            bgp_scope: None,
            machine: Machine::default(),
            triple_frames: Vec::new(),
            path_levels: Vec::new(),
            options,
        })
    }
}

/// The prefix an anonymous blank node's minted label starts with: `__purrdf_anon_`,
/// lengthened one leading underscore at a time until no blank node label the request
/// text writes starts with it.
///
/// A minted label must never equal one the author wrote, or `[]` and `_:label` would
/// be read as one node. Any label is a legal `BLANK_NODE_LABEL`, so no fixed prefix
/// is out of an author's reach; one chosen against the text in hand is.
fn anon_label_prefix(tokens: &[Option<Spanned<'_>>]) -> String {
    let mut prefix = String::from("__purrdf_anon_");
    while tokens.iter().flatten().any(|spanned| {
        matches!(spanned.token, Token::BlankNodeLabel(label) if label.starts_with(prefix.as_str()))
    }) {
        prefix.insert(0, '_');
    }
    prefix
}

/// Which grammar production a `SELECT` is being read under.
///
/// The two positions are different productions with different contents, not one
/// production read in two moods: `Query ::= Prologue ( SelectQuery | … )` reaches
/// `SelectQuery ::= SelectClause DatasetClause* WhereClause SolutionModifier`, while a
/// group's `SubSelect ::= SelectClause WhereClause SolutionModifier ValuesClause` has no
/// `DatasetClause` at all. One function parses both, because everything else about them
/// is identical and two copies would drift; this says which one it is parsing, so the
/// clause the sub-select production omits is refused where it is read rather than
/// dropped where the `Query` is unwrapped.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectPosition {
    /// A whole query's `SelectQuery`, which carries a dataset clause.
    Query,
    /// A group graph pattern's `SubSelect`, which does not.
    SubSelect,
}

struct Parser<'a, 'o> {
    tokens: Vec<Option<Spanned<'a>>>,
    pos: usize,
    /// The original request text, kept only so a `BASE` directive can record the
    /// 1-based source position it was written at ([`Parser::directive_origin`]).
    /// The line table is built on that path alone, never on the happy path.
    src: &'a str,
    end: usize,
    prefixes: HashMap<String, String, FixedState>,
    /// The base IRIs in scope: the caller-supplied base (if any) at the bottom,
    /// rebound in place by every prologue `BASE` directive. Resolution itself is
    /// [`BaseScope`]'s — this parser owns no RFC-3986 arithmetic.
    base: BaseScope,
    /// The most recently parsed prologue `VERSION` declaration (last-wins across
    /// repeated declarations — see [`SparqlVersion`]).
    version: Option<SparqlVersion>,
    agg_counter: usize,
    anon_counter: usize,
    /// The label prefix [`Parser::fresh_anon`] mints under — see [`anon_label_prefix`].
    anon_prefix: String,
    group_counter: usize,
    /// The `EXISTS`/`NOT EXISTS` in-scope-set stack (SEP-0007 Part 3) — see
    /// [`Parser::exists_scope`] for what "in scope" means here and
    /// [`Parser::push_exists_scope_boundary`]/[`Parser::push_exists_scope_isolated`]
    /// for how frames are opened.
    exists_scope_stack: ExistsScopes,
    /// True while parsing a `SELECT`'s own projection list — its `(expr AS
    /// ?v)` targets and any aggregate arguments lifted out of them — i.e. the
    /// window BEFORE `WHERE` is even read, where [`Parser::exists_scope`] is
    /// necessarily still empty (see [`Parser::check_exists_body`]'s doc for
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
    /// Where the query form's `DatasetClause*` run sits in the request text, for
    /// [`SparqlParser::parse_query_split`] to report — `None` for a query that
    /// writes no `FROM`/`FROM NAMED` at all.
    ///
    /// Written by [`Parser::parse_dataset_clauses`], the one function that reads
    /// that run, so all four query forms report it the same way with no call site
    /// repeating the bookkeeping. Written only for a NON-EMPTY run, which is what
    /// makes "the last writer wins" a non-question: a successfully parsed query has
    /// at most one non-empty run in it, because the only other place
    /// [`Parser::parse_select`] is reached from is the sub-`SELECT` site, where a
    /// non-empty run is refused (`SubSelect` has no `DatasetClause`). A refused one
    /// may have been recorded a moment earlier, and is never observed: the error
    /// propagates to the public entry point and this `Parser` is not consulted again.
    dataset_at: Option<Range<usize>>,
    /// Where the WHOLE query's `DatasetClause*` run sits, empty or not, for
    /// [`SparqlParser::parse_query_dataset_slot`] to report.
    ///
    /// Written by [`Parser::parse_dataset_clauses`] on every call; the one caller
    /// that reads a run which is not the whole query's — the sub-`SELECT` site in
    /// [`Parser::parse_select`] — restores the value it found, so a sub-select read
    /// before the query's own clause (inside an `EXISTS` in the projection) or after
    /// it (inside the `WHERE`) never displaces it.
    dataset_slot: Option<Range<usize>>,
    /// One [`UpdateDatasetSlot`] per update operation parsed so far, pushed by
    /// [`Parser::parse_update`] for [`SparqlParser::parse_update_split`].
    update_slots: Vec<UpdateDatasetSlot>,
    /// The slot of the update operation being parsed, written by the operation's own
    /// production and taken by [`Parser::parse_update`] once it returns.
    op_slot: UpdateDatasetSlot,
    /// The basic graph pattern each author-written blank node label was first
    /// seen in, keyed by label — the state behind the rule that a label is
    /// scoped to ONE basic graph pattern (see [`Parser::scoped_blank_label`]).
    ///
    /// One map per query, so a label reused in a sub-`SELECT` or an `EXISTS`
    /// body is caught too; one map per UPDATE operation, whose `WHERE` clauses
    /// are separate patterns ([`Parser::parse_update`] clears it between them).
    blank_label_bgps: HashMap<String, usize, FixedState>,
    /// The next basic-graph-pattern ordinal a group's triples block is given.
    bgp_counter: usize,
    /// The basic graph pattern the triples block being parsed belongs to, or
    /// `None` outside a group's triples block — a template, a `VALUES` block or
    /// an expression, none of which is a basic graph pattern of the query.
    bgp_scope: Option<usize>,
    /// The stacks of the machine that reads groups, sub-`SELECT`s, solution modifiers
    /// and expressions (see [`machine`]), kept so later parses of this request reuse
    /// them.
    machine: Machine,
    /// The frame stack of the triples machine (see [`triples`]), reused likewise.
    triple_frames: Vec<TFrame>,
    /// The open levels of the property path being read, reused likewise.
    path_levels: Vec<PathLevel>,
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
            src: self.src,
            end: self.end,
            prefixes: self.prefixes.clone(),
            base: self.base.clone(),
            version: self.version.clone(),
            agg_counter: self.agg_counter,
            anon_counter: self.anon_counter,
            anon_prefix: self.anon_prefix.clone(),
            group_counter: self.group_counter,
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
            exists_scope_stack: ExistsScopes::default(),
            projection_scope_pending: false,
            in_aggregate_argument: false,
            projection_seen_targets: Vec::new(),
            pending_exists_scope_checks: Vec::new(),
            #[cfg(debug_assertions)]
            scope_consultations: self.scope_consultations,
            // Neither forked production can contain a query form, so neither can
            // contain a dataset clause: the fork records none, and the position the
            // outer parse recorded stays the outer parse's, on the outer parser.
            dataset_at: None,
            dataset_slot: None,
            update_slots: Vec::new(),
            op_slot: UpdateDatasetSlot::NoWhereClause,
            // A fork reads a template or quad-pattern block, which is not a basic
            // graph pattern of the query: nothing it reads is scoped to one, so it
            // starts with no scope and records nothing.
            blank_label_bgps: HashMap::with_hasher(FixedState::new()),
            bgp_counter: 0,
            bgp_scope: None,
            machine: Machine::default(),
            triple_frames: Vec::new(),
            path_levels: Vec::new(),
            options: self.options,
        }
    }

    fn set_counters(&mut self, counters: (usize, usize, usize)) {
        (self.agg_counter, self.anon_counter, self.group_counter) = counters;
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
        self.exists_scope_stack.top()
    }

    /// Open a fresh, EMPTY in-scope-set frame — nothing precedes it. Used at
    /// every query/update operation's own top-level `WHERE` clause and at a
    /// sub-`SELECT`'s (the one real scope boundary; see the module doc above).
    fn push_exists_scope_boundary(&mut self) {
        self.exists_scope_stack.push_boundary();
    }

    /// Open a fresh in-scope-set frame SEEDED with a copy of the frame beneath
    /// it — reads see everything the enclosing rows already bound, but nothing
    /// this frame goes on to introduce is written back once it is popped. Used
    /// at an `EXISTS`/`NOT EXISTS`/`MINUS` body (see the module doc above).
    fn push_exists_scope_isolated(&mut self) {
        self.exists_scope_stack.push_isolated();
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
        if self.exists_scope_stack.is_open() {
            let mut noted = VarScope::default();
            collect_vars(pattern, &mut noted);
            for v in noted.as_slice() {
                self.exists_scope_stack.note(v);
            }
        }
    }

    /// Record a single fresh binding (a `BIND` target) into the current
    /// in-scope-set frame.
    fn note_exists_scope_var(&mut self, variable: &Variable) {
        self.exists_scope_stack.note(variable);
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
        let group = self.parse_group()?;
        for v in group.scope.as_slice() {
            self.note_exists_scope_var(v);
        }
        self.pop_exists_scope_boundary();
        Ok(group.pattern)
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

    /// The query form alone, with the prologue already read: the half
    /// [`SparqlParser::parse_query_split`] needs to start after the offset it
    /// reports. Split out of `parse_query` rather than duplicated, so the two
    /// entries cannot parse a form differently.
    fn parse_query_form(&mut self) -> Result<Query> {
        let base_iri = self.base_named_node();
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
                let at = self.span();
                // The directive is NOT resolved as an ordinary IRIREF, and the
                // chaining below is NOT Turtle's rule imported by analogy — it is
                // what SPARQL's own normative reference requires. The chain is:
                //
                //   1. `BaseDecl ::= 'BASE' IRIREF` takes the general IRI
                //      *reference* production — the same one that spells the
                //      relative `<book1>` in the specification's own example —
                //      not an `absolute-IRI`. A relative operand is therefore
                //      well-formed, and the only open question is its meaning.
                //   2. SPARQL 1.2 Query §4.1.1.2 "Relative IRIs" (word-for-word
                //      SPARQL 1.1 §4.1.1.1): "The BASE keyword defines the Base
                //      IRI used to resolve relative IRIs per [RFC3986] section
                //      5.1.1, 'Base URI Embedded in Content'." SPARQL states no
                //      rule of its own; it delegates to RFC 3986.
                //   3. RFC 3986 §5.1 "Establishing a Base URI" — the section
                //      §5.1.1 is a subsection OF, so its requirements bind here:
                //      "A base URI must conform to the <absolute-URI> syntax rule
                //      (Section 4.3). If the base URI is obtained from a URI
                //      reference, then that reference must be converted to
                //      absolute form and stripped of any fragment component prior
                //      to its use as a base URI."
                //
                // A `BASE` operand is precisely "a base URI obtained from a URI
                // reference", so RFC 3986 requires it be *converted* to absolute
                // form — that conversion is §5.2 resolution against the base
                // already established — rather than refused; and the
                // `<absolute-URI>` requirement binds the RESULT of that
                // conversion. `BaseScope::rebind` is exactly that operation, so
                // `BASE <http://example.org/a/> BASE <b/>` yields
                // `http://example.org/a/`.
                //
                // The requirement on the result is what keeps a lone relative
                // directive an error: with no base established there is nothing
                // to convert against, so no `<absolute-URI>` can be produced and
                // `rebind` hard-fails with `iri-non-absolute-base` rather than
                // storing a base that would silently mis-resolve every reference
                // after it. The vendored W3C corpora contain no `BASE <relative>`
                // case in either direction (every `BASE` in
                // `crates/sparql-conformance/suite/w3c-sparql11` and
                // `w3c-sparql12` is absolute), so the suite neither licenses nor
                // forbids this; clause 3 above decides it.
                let directive = self.expect_raw_iriref()?;
                let origin = self.directive_origin(at);
                self.base
                    .rebind(&directive, origin)
                    .map_err(|e| iri_error(&directive, &e))?;
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
        let raw = self.expect_raw_iriref()?;
        self.resolve_iri(&raw)
    }

    /// Take the next token as an IRIREF *lexeme*, without resolving it.
    ///
    /// Only the `BASE` directive wants this: it resolves itself, against the base
    /// already in force rather than as an ordinary reference under it.
    fn expect_raw_iriref(&mut self) -> Result<String> {
        match self.bump() {
            Some(Token::Iri(s)) => Ok(s.into_owned()),
            other => Err(ParseError::syntax(
                format!("expected IRIREF, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// The provenance to record for a `BASE` directive whose IRIREF starts at
    /// byte offset `at`.
    ///
    /// The line table is built here, on the directive path, for the same reason
    /// [`ParseError::locate`] builds one on the error path: the happy path keeps
    /// a bare byte offset and pays nothing.
    fn directive_origin(&self, at: usize) -> BaseOrigin {
        let position = LineIndex::new(self.src).locate(self.src, at);
        BaseOrigin::Directive {
            line: position.line,
            column: position.column,
        }
    }

    /// The base currently in force as a [`NamedNode`], for the algebra's
    /// `base_iri` field. [`BaseIri`] already guarantees a valid absolute IRI, so
    /// there is nothing left here to validate or to fail on.
    fn base_named_node(&self) -> Option<NamedNode> {
        self.base
            .current()
            .map(|scoped| NamedNode::new_unchecked(scoped.iri().as_str()))
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

    /// Resolve a lexical IRIREF against the base in force, through the workspace's
    /// single resolution layer ([`BaseScope::resolve`]).
    ///
    /// A relative reference with NO base in scope is a typed [`ParseError::Iri`]
    /// carrying [`IriError`]'s shared diagnostic code — never a raw relative
    /// string handed onward, and never a base fabricated from somewhere else.
    fn resolve_iri(&self, s: &str) -> Result<String> {
        self.base
            .resolve(s)
            .map(|iri| iri.as_str().to_owned())
            .map_err(|e| iri_error(s, &e))
    }

    fn resolve_prefixed(&self, prefix: &str, local: &str) -> Result<NamedNode> {
        match self.prefixes.get(prefix) {
            Some(ns) => {
                // Exact-fit concatenation: one allocation per prefixed name where
                // `format!` grew a default buffer through the formatter.
                let mut iri = String::with_capacity(ns.len() + local.len());
                iri.push_str(ns);
                iri.push_str(local);
                NamedNode::new(iri)
            }
            None => Err(ParseError::syntax(
                format!("undeclared prefix {prefix:?}"),
                self.span(),
            )),
        }
    }

    // ── query forms ──────────────────────────────────────────────────────────

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
                    inner: Child::new(p),
                    expression: modifiers.order_by,
                };
            }
            if modifiers.offset.is_some() || modifiers.limit.is_some() {
                p = GraphPattern::Slice {
                    inner: Child::new(p),
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
                inner: Child::new(p),
                expression: modifiers.order_by,
            };
        }
        if modifiers.offset.is_some() || modifiers.limit.is_some() {
            p = GraphPattern::Slice {
                inner: Child::new(p),
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
    ///
    /// A non-empty run also records where it was written
    /// ([`Parser::dataset_at`]), for [`SparqlParser::parse_query_split`] to report.
    /// The recorded end is the start of the token *after* the run rather than the end
    /// of its last IRI, so the range carries the whitespace separating the clause from
    /// the `WHERE` that follows and a caller excising it is left with a text that
    /// still parses.
    fn parse_dataset_clauses(&mut self) -> Result<QueryDataset> {
        let at = self.span();
        let mut default = Vec::new();
        let mut named = Vec::new();
        while self.eat_kw("FROM") {
            if self.eat_kw("NAMED") {
                named.push(self.expect_iri_node()?);
            } else {
                default.push(self.expect_iri_node()?);
            }
        }
        let end = self.span();
        if !(default.is_empty() && named.is_empty()) {
            self.dataset_at = Some(at..end);
        }
        self.dataset_slot = Some(at..end);
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
        match self.parse_triples_block(TripleContext::Template)? {
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
        let base_iri = self.base_named_node();

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
        let mut prior_bnode_labels: HashSet<String, FixedState> =
            HashSet::with_hasher(FixedState::new());
        // Reused across iterations to avoid reallocating the set each loop.
        let mut this_op_labels: HashSet<String, FixedState> =
            HashSet::with_hasher(FixedState::new());
        loop {
            if self.pos >= self.tokens.len() {
                break;
            }
            // Each operation's `WHERE` is a pattern of its own, so the labels its
            // basic graph patterns claim are its own too.
            self.blank_label_bgps.clear();
            self.op_slot = UpdateDatasetSlot::NoWhereClause;
            let op = self.parse_update_operation()?;
            self.update_slots.push(std::mem::replace(
                &mut self.op_slot,
                UpdateDatasetSlot::NoWhereClause,
            ));
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
        let using = self.parse_using_clauses_at(None)?;
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
        let where_at = self.span();
        if self.eat_kw("WHERE") {
            // DELETE WHERE { QuadPattern } — the template IS the where pattern.
            // `fork_block` bounds the clone to this operation's braced block, so a
            // `;`-separated multi-operation UPDATE stays linear instead of the whole
            // request being O(n²) in the number of `DELETE WHERE` operations.
            let (delete, counters) = {
                let mut delete_parser = self.fork_block();
                // The fork holds exactly the braced block, so its last token is the
                // matching `}` and its end is the end of the pattern's text.
                let pattern_end = delete_parser
                    .tokens
                    .last()
                    .and_then(Option::as_ref)
                    .map_or(self.end, |closing| closing.end);
                self.op_slot = UpdateDatasetSlot::DeleteWhere {
                    where_at,
                    pattern_at: self.span()..pattern_end,
                };
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
        let using = self.parse_using_clauses_at(None)?;
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
        let with_start = self.span();
        self.expect_kw("WITH")?;
        let with = Some(self.expect_iri_node()?);
        let with_at = with_start..self.span();
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
        let using = self.parse_using_clauses_at(Some(with_at))?;
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
    ///
    /// Also records the operation's [`UpdateDatasetSlot::Modify`] slot: the run's range
    /// (empty, at the token after it, when there is none) beside the `WITH` range the
    /// caller read, if any.
    fn parse_using_clauses_at(
        &mut self,
        with_at: Option<Range<usize>>,
    ) -> Result<Vec<UsingClause>> {
        let at = self.span();
        let using = self.parse_using_clauses()?;
        self.op_slot = UpdateDatasetSlot::Modify {
            with_at,
            using_at: at..self.span(),
        };
        Ok(using)
    }

    /// The `UsingClause*` run itself.
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
        let mut sink = BlockSink::new(TripleContext::Template);
        let (subject, standalone_ok) = if self.at(&Token::LBracket) {
            (self.parse_graph_node(&mut sink)?, true)
        } else if self.at(&Token::LParen) {
            (self.parse_graph_node(&mut sink)?, false)
        } else if self.at(&Token::TripleOpen) {
            let node = self.parse_graph_node(&mut sink)?;
            let standalone = !matches!(node, TermPattern::Triple(_));
            (node, standalone)
        } else {
            (self.parse_term_pattern()?, false)
        };
        let standalone = standalone_ok
            && (self.at(&Token::Dot) || self.at(&Token::RBrace) || self.at(&Token::LBrace));
        if !standalone {
            self.parse_predicate_object_list(SubjectArgs::Term(subject), &mut sink)?;
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
    /// (when `reject_blank`). Descends into RDF 1.2 quoted triples over a work list,
    /// reporting the first violation in written order.
    fn check_data_triple(&self, t: &TriplePattern, reject_blank: bool) -> Result<()> {
        let mut terms: Vec<&TermPattern> = Vec::new();
        let mut triple = Some(t);
        loop {
            if let Some(t) = triple.take() {
                if let NamedNodePattern::Variable(_) = &t.predicate {
                    return Err(ParseError::syntax(
                        "variable predicate in INSERT/DELETE DATA is not allowed",
                        self.span(),
                    ));
                }
                terms.extend([&t.object, &t.subject]);
            }
            let Some(term) = terms.pop() else {
                return Ok(());
            };
            match term {
                TermPattern::NamedNode(_) | TermPattern::Literal(_) => {}
                TermPattern::Triple(tp) => triple = Some(tp),
                TermPattern::Variable(_) => {
                    return Err(ParseError::syntax(
                        "variable in INSERT/DELETE DATA is not allowed",
                        self.span(),
                    ));
                }
                TermPattern::BlankNode(_) => {
                    // INSERT DATA blanks are allowed (minted fresh per request).
                    if reject_blank {
                        return Err(ParseError::syntax(
                            "blank node in DELETE DATA is not allowed",
                            self.span(),
                        ));
                    }
                }
            }
        }
    }

    // ── triples blocks ───────────────────────────────────────────────────────

    /// Parse a run of triples (subject + predicate-object lists) into a BGP, any
    /// complex property-path `Path` nodes, and any property-function calls,
    /// assembled together by [`BlockSink::into_pattern`].
    fn parse_triples_block(&mut self, context: TripleContext) -> Result<GraphPattern> {
        let mut sink = BlockSink::new(context);
        loop {
            // The subject may be a blank-node property list `[ p o ; … ]` or an RDF
            // collection `( … )`, each of which emits its own triples and yields a
            // fresh node (the BNPL blank, or the collection's head).
            let (subject, standalone_capable) = if self.at(&Token::LBracket) {
                (SubjectArgs::Term(self.parse_graph_node(&mut sink)?), true)
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
                    (SubjectArgs::Term(self.parse_graph_node(&mut sink)?), false)
                }
            } else if self.at(&Token::TripleOpen) {
                // A reifying triple `<< s p o >>` emits its own reifier triples, so
                // it may stand alone (`<< s p o >> .`) with no predicate-object
                // list. A *triple term* `<<( s p o )>>` is a value: it may head a
                // subject's predicate-object list but must not stand alone.
                let node = self.parse_graph_node(&mut sink)?;
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
                self.parse_predicate_object_list(subject, &mut sink)?;
            }
            if !self.eat(&Token::Dot) {
                break;
            }
            // After a `.`, stop if the block ends (`}` or a keyword/brace).
            if self.at(&Token::RBrace) || self.block_boundary() {
                break;
            }
        }
        Ok(sink.into_pattern())
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
        self.options.is_property_fn(&iri).then_some(iri)
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
                if self.at(&Token::RBracket) {
                    return Err(self.empty_bracket_pair());
                }
                Err(ParseError::syntax(
                    "a populated blank-node property list is not allowed in a \
                     property-function argument list",
                    self.span(),
                ))
            }
            _ => self.parse_term_pattern(),
        }
    }

    /// The refusal every bracket-pair site below shares: a `[` whose matching `]`
    /// arrives with no predicate-object list between them.
    ///
    /// Two productions spell a bracket pair, and neither admits this:
    /// `BlankNodePropertyListPath ::= '[' PropertyListPathNotEmpty ']'` requires
    /// at least one predicate-object pair, and the anonymous blank node
    /// `ANON ::= '[' WS* ']'` is a **terminal**, so it admits only `WS` between
    /// its brackets — a comment is not `WS`, and a terminal has no interior a
    /// comment could sit in.
    ///
    /// The tokenizer cannot make this call. Its `ANON` scan is necessarily
    /// comment-blind, because `[ # c` … newline … `?p ?o ]` is a lawful *populated*
    /// property list and nothing at the lexical level distinguishes it from the
    /// empty case; teaching the scan to skip comments would only widen acceptance.
    /// So a real `[]` (or `[ ]`, or `[` tab/CR/LF `]`) arrives here as a single
    /// `Token::Anon` and never reaches a bracket-pair site at all, and an
    /// `LBracket` immediately followed by an `RBracket` can only have come from a
    /// comment between the brackets — which is precisely the input to refuse.
    fn empty_bracket_pair(&self) -> ParseError {
        ParseError::syntax(
            "`[` `]` with nothing between them is not a blank-node property list \
             (which requires at least one predicate-object pair) and not an \
             anonymous blank node (`ANON` admits only whitespace between its \
             brackets, and a comment is not whitespace)",
            self.span(),
        )
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
            object: TermPattern::Triple(Child::new(t.clone())),
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
                let at = self.span();
                let Some(Token::BlankNodeLabel(l)) = self.bump() else {
                    unreachable!()
                };
                Ok(TermPattern::BlankNode(self.scoped_blank_label(l, at)?))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                if self.at(&Token::RBracket) {
                    return Err(self.empty_bracket_pair());
                }
                Err(ParseError::syntax(
                    "a blank-node property list is not a reifier id; \
                     `VarOrReifierId` admits a variable, an IRI, a labelled blank \
                     node or the anonymous `[]`",
                    self.span(),
                ))
            }
            _ => Ok(TermPattern::BlankNode(self.fresh_anon())),
        }
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
            || self.peek_kw("UNFOLD")
    }

    // ── terms ────────────────────────────────────────────────────────────────

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
                    let at = self.span();
                    let Some(Token::LangTag(tag)) = self.bump() else {
                        unreachable!()
                    };
                    let (lang, dir) = split_lang_dir(tag, at)?;
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

    // ── solution modifiers ───────────────────────────────────────────────────

    /// True when the upcoming token can start a bare (non-parenthesized)
    /// `BuiltInCall` or `FunctionCall`: the bare alternative of a `Constraint`
    /// (`Constraint ::= BrackettedExpression | BuiltInCall | FunctionCall`) and,
    /// by the same productions, of a `GROUP BY` `GroupCondition`. Every bare call
    /// begins with a callee token (a builtin keyword, an IRI or a prefixed name);
    /// the modifier-list terminators (`HAVING`/`ORDER`/`LIMIT`/`OFFSET`/`VALUES`)
    /// and boolean literals are excluded so a `GROUP BY`, `HAVING` or `ORDER BY`
    /// list stops cleanly at the next clause.
    ///
    /// Used by `GROUP BY`'s condition loop, by `HAVING`'s `Constraint+` list (both
    /// to decide whether the first, mandatory constraint is bare, and whether a
    /// subsequent one begins) and by `ORDER BY`'s `OrderCondition ::= ... |
    /// (Constraint | Var)` alternative. The bracketed form (`Token::LParen`) is
    /// recognized separately at each call site, so this deliberately excludes a
    /// bare `Var` or literal (neither is a call).
    fn at_bare_constraint(&self) -> bool {
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => true,
            Some(Token::Word(w)) => !is_modifier_terminator_word(w),
            _ => false,
        }
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

    // ── aggregate clauses and fresh names ────────────────────────────────────

    /// Parse the `AGG(<iri>, …)` surface's optional trailing named
    /// scalar-value clauses: zero or more `; NAME=value` pairs, generalizing
    /// `GROUP_CONCAT`'s own `; SEPARATOR="…"` — SPARQL's existing precedent for
    /// a named scalar aggregate parameter (see
    /// [`crate::algebra::AggregateExpression::scalarvals`]'s docs) — to an arbitrary custom
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

    /// A blank node the query author labelled `_:label`, recorded against the
    /// basic graph pattern it is written in.
    ///
    /// SPARQL scopes a blank node label to the basic graph pattern it appears
    /// in, and forbids one label in two different basic graph patterns of the
    /// same query: within one it is a single non-distinguished variable, so
    /// two patterns sharing it would have to be one existential and two at once.
    /// A basic graph pattern is a run of triples blocks, optionally with
    /// `FILTER`s among them; any other element of the group ends it (the W3C
    /// syntax tests `syn-bad-34`…`38` and `syn-bad-{OPT,UNION,GRAPH}-breaks-BGP`
    /// state the rule). A property-function call or a property path written in
    /// the block is part of that basic graph pattern like any triple.
    ///
    /// Outside a group's triples block — a `CONSTRUCT` or UPDATE template —
    /// nothing is recorded: a template's labels mint fresh nodes per solution
    /// and are not the pattern's.
    fn scoped_blank_label(&mut self, label: &str, at: usize) -> Result<BlankNode> {
        if let Some(bgp) = self.bgp_scope {
            let first = *self.blank_label_bgps.entry(label.to_owned()).or_insert(bgp);
            if first != bgp {
                return Err(ParseError::syntax(
                    format!(
                        "blank node label _:{label} is used in two different basic graph \
                         patterns; a blank node label is scoped to the one basic graph \
                         pattern it appears in"
                    ),
                    at,
                ));
            }
        }
        Ok(BlankNode::new(label))
    }

    /// Mint a fresh, unique label for an anonymous blank node (`[]`). Each
    /// occurrence is a distinct existential; reusing one label (e.g. `""`) would
    /// wrongly fuse separate blank nodes into a single AST node.
    fn fresh_anon(&mut self) -> BlankNode {
        let b = BlankNode::new(format!("{}{}", self.anon_prefix, self.anon_counter));
        self.anon_counter += 1;
        b
    }
}

/// A parsed predicate: a simple verb (IRI/`a`/variable) yielding a triple, a
/// property path lowered into data triples when wholly linear, or retained as a
/// `GraphPattern::Path`, or a plain IRI under a
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
struct BlockSink {
    context: TripleContext,
    triples: Vec<TriplePattern>,
    paths: Vec<GraphPattern>,
    prop_fns: Vec<(usize, PropertyFunctionCall)>,
}

impl BlockSink {
    /// An empty block under its grammar production. Templates must retain every
    /// syntactic path so their caller can refuse it, including a linear one.
    fn new(context: TripleContext) -> Self {
        Self {
            context,
            triples: Vec::new(),
            paths: Vec::new(),
            prop_fns: Vec::new(),
        }
    }

    /// Record a property-function call at the current position in the block.
    fn push_property_function(&mut self, call: PropertyFunctionCall) {
        self.prop_fns.push((self.triples.len(), call));
    }

    /// Assemble the block: the data triples as a `Bgp`, each property-function
    /// call laterally joined onto everything written before it, then the
    /// property-path nodes joined on.
    fn into_pattern(mut self) -> GraphPattern {
        if self
            .paths
            .iter()
            .any(|path| !matches!(path, GraphPattern::Path { .. }))
        {
            self.promote_path_blanks();
        }
        let mut triples = self.triples.into_iter();
        let mut taken = 0usize;
        let mut g = GraphPattern::Bgp { patterns: vec![] };
        for (at, call) in self.prop_fns {
            let residual: Vec<TriplePattern> = triples.by_ref().take(at - taken).collect();
            taken = at;
            g = GraphPattern::Lateral {
                left: Child::new(join(g, GraphPattern::Bgp { patterns: residual })),
                right: Child::new(GraphPattern::PropertyFunction(call)),
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

    /// A lowered alternative splits one source triples block into multiple BGPs.
    /// Preserve that block's blank identities before those new UNION boundaries
    /// acquire their own scopes. A source UNION has separate BlockSinks, so its
    /// arm-local blanks are never joined by this translation.
    fn promote_path_blanks(&mut self) {
        let mut labels = std::collections::BTreeMap::new();
        let mut pending = Vec::new();
        let mut patterns: Vec<_> = self
            .paths
            .iter_mut()
            .filter(|path| !matches!(path, GraphPattern::Path { .. }))
            .collect();
        while let Some(pattern) = patterns.pop() {
            match pattern {
                GraphPattern::Bgp { patterns: triples } => {
                    for triple in triples {
                        pending.extend([&mut triple.subject, &mut triple.object]);
                    }
                }
                GraphPattern::Join { left, right } => patterns.extend([&mut **left, &mut **right]),
                GraphPattern::Union { arms } => patterns.extend(arms.iter_mut()),
                _ => unreachable!("a translated path contains only BGP, Join and Union"),
            }
        }
        Self::promote_terms(pending, &mut labels, true);
        let mut pending = Vec::new();
        for triple in &mut self.triples {
            pending.extend([&mut triple.subject, &mut triple.object]);
        }
        for (_, call) in &mut self.prop_fns {
            pending.extend(call.subject_args.iter_mut().chain(&mut call.object_args));
        }
        for path in &mut self.paths {
            if let GraphPattern::Path {
                subject, object, ..
            } = path
            {
                pending.extend([subject, object]);
            }
        }
        Self::promote_terms(pending, &mut labels, false);
    }

    /// Rename the selected source labels, descending into quoted endpoints too.
    fn promote_terms(
        mut pending: Vec<&mut TermPattern>,
        labels: &mut std::collections::BTreeMap<String, Variable>,
        discover: bool,
    ) {
        while let Some(term) = pending.pop() {
            match term {
                TermPattern::BlankNode(blank) => {
                    if discover && !labels.contains_key(blank.as_str()) {
                        labels
                            .entry(blank.as_str().to_owned())
                            .or_insert_with(|| Variable::hidden_blank(blank.as_str()));
                    }
                    if let Some(variable) = labels.get(blank.as_str()) {
                        *term = TermPattern::Variable(variable.clone());
                    }
                }
                TermPattern::Triple(triple) => {
                    let triple = &mut **triple;
                    pending.extend([&mut triple.subject, &mut triple.object]);
                }
                _ => {}
            }
        }
    }
}

/// A graph pattern lowers predicate-only paths; a template retains paths for its
/// syntax rejection.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TripleContext {
    Pattern,
    Template,
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
    if left.is_empty_bgp() {
        return right;
    }
    if right.is_empty_bgp() {
        return left;
    }
    match (left, right) {
        (GraphPattern::Bgp { mut patterns }, GraphPattern::Bgp { patterns: r }) => {
            patterns.extend(r);
            GraphPattern::Bgp { patterns }
        }
        (l, r) => GraphPattern::Join {
            left: Child::new(l),
            right: Child::new(r),
        },
    }
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
    let mut pending = vec![p];
    while let Some(p) = pending.pop() {
        match p {
            GraphPattern::PropertyFunction(_) => return true,
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                pending.extend([&**right, &**left]);
            }
            _ => {}
        }
    }
    false
}

/// If a property path is length-1 (a single predicate), return it as a triple
/// predicate; complex paths return `None` (they undergo whole-path translation).
fn simple_predicate(path: &PropertyPathExpression) -> Option<NamedNodePattern> {
    match path {
        PropertyPathExpression::NamedNode(n) => Some(NamedNodePattern::NamedNode(n.clone())),
        _ => None,
    }
}

/// Lift only the OPTIONAL group's own filters into its LeftJoin condition,
/// conjoined in written order (§18.3.2.7–9). Nested groups' filters remain in
/// the independent right operand, even after an empty-BGP join was simplified.
fn split_trailing_filters(
    mut pattern: GraphPattern,
    filter_count: usize,
) -> (GraphPattern, Option<Expression>) {
    let mut filters = Vec::with_capacity(filter_count);
    for _ in 0..filter_count {
        let GraphPattern::Filter { expr, inner } = pattern else {
            unreachable!("the group wraps each of its own filters around its pattern");
        };
        filters.push(expr);
        pattern = inner.into_inner();
    }
    (pattern, filters.into_iter().rev().reduce(Expression::and))
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
/// crate is wasm32-clean and deliberately depends on no hasher crate, and a
/// membership set that is never iterated for its OWN order — only `order`
/// ever is — needs no hash at all) removes both.
#[derive(Default)]
struct VarScope {
    order: Vec<Variable>,
    seen: std::collections::BTreeSet<Variable>,
}

impl VarScope {
    /// Record `v` as in scope; a no-op if it already is (first-appearance
    /// order is preserved, so a later re-mention never moves it).
    fn note(&mut self, v: &Variable) {
        if !v.is_hidden() && self.seen.insert(v.clone()) {
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

/// Every open `EXISTS` in-scope-set frame (see [`Parser::exists_scope`]), stored once.
///
/// Only the top frame is ever written, so the frames' variables form one trail: a
/// frame owns the trail's tail from its `mark` on, and sees the trail from its `start`
/// on. A boundary frame starts empty (`start` = `mark` = the trail's length); an
/// isolated frame sees everything the frame beneath it sees (`start` = that frame's
/// `start`) and owns only what it adds itself (`mark` = the trail's length). Popping a
/// frame truncates the trail to its `mark`. Seeding an isolated frame therefore copies
/// nothing, so `EXISTS`/`NOT EXISTS`/`MINUS` bodies nested `n` deep hold `O(n)`
/// variables in all rather than a copy of every enclosing frame per level.
#[derive(Default)]
struct ExistsScopes {
    /// The variables of every open frame, each frame's own after the frame beneath's.
    trail: Vec<Variable>,
    /// For every variable on the trail, its positions there, ascending.
    positions: std::collections::BTreeMap<Variable, Vec<usize>>,
    /// The open frames, innermost last.
    frames: Vec<ExistsFrame>,
}

/// One open frame of [`ExistsScopes`].
#[derive(Clone, Copy)]
struct ExistsFrame {
    /// Where on the trail the variables this frame sees begin.
    start: usize,
    /// Where on the trail the variables this frame added begin.
    mark: usize,
}

impl ExistsScopes {
    /// Whether any frame is open.
    fn is_open(&self) -> bool {
        !self.frames.is_empty()
    }

    /// The variables the top frame sees, in first-appearance order; empty when no frame
    /// is open.
    fn top(&self) -> &[Variable] {
        self.frames
            .last()
            .map_or(&[], |frame| &self.trail[frame.start..])
    }

    /// Open an empty frame.
    fn push_boundary(&mut self) {
        let len = self.trail.len();
        self.frames.push(ExistsFrame {
            start: len,
            mark: len,
        });
    }

    /// Open a frame that sees everything the current top frame sees; with no frame open,
    /// an empty one.
    fn push_isolated(&mut self) {
        let len = self.trail.len();
        let start = self.frames.last().map_or(len, |frame| frame.start);
        self.frames.push(ExistsFrame { start, mark: len });
    }

    /// Close the top frame, discarding what it added.
    fn pop(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };
        for variable in self.trail.drain(frame.mark..) {
            if let Some(at) = self.positions.get_mut(&variable) {
                at.pop();
                if at.is_empty() {
                    self.positions.remove(&variable);
                }
            }
        }
    }

    /// Record `variable` in the top frame; a no-op when the top frame already sees it,
    /// or when no frame is open.
    fn note(&mut self, variable: &Variable) {
        let Some(frame) = self.frames.last() else {
            return;
        };
        let seen = self
            .positions
            .get(variable)
            .and_then(|at| at.last())
            .is_some_and(|&at| at >= frame.start);
        if !seen {
            self.positions
                .entry(variable.clone())
                .or_default()
                .push(self.trail.len());
            self.trail.push(variable.clone());
        }
    }
}

/// Collect the in-scope variables of a pattern in first-appearance order
/// (used for `SELECT *` projection). Non-distinguished translation identities
/// are omitted; projection and grouping boundaries expose only their outputs.
/// Serializers and callers share this scope census rather than infer visibility
/// from every variable mentioned by an expression or a nested pattern.
#[must_use]
pub fn visible_variables(p: &GraphPattern) -> Vec<Variable> {
    let mut scope = VarScope::default();
    collect_vars(p, &mut scope);
    scope.into_vec()
}

/// One entry of [`collect_vars`]'s work list.
enum VarStep<'a> {
    /// A pattern whose in-scope variables are still to be noted.
    Pattern(&'a GraphPattern),
    /// A term whose variables are still to be noted.
    Term(&'a TermPattern),
    /// A variable to note.
    Note(&'a Variable),
}

/// Queue a triple pattern's variables: subject, predicate, object.
fn push_triple_vars<'a>(tp: &'a TriplePattern, pending: &mut Vec<VarStep<'a>>) {
    pending.push(VarStep::Term(&tp.object));
    if let NamedNodePattern::Variable(v) = &tp.predicate {
        pending.push(VarStep::Note(v));
    }
    pending.push(VarStep::Term(&tp.subject));
}

/// Note the variables `p` makes visible in its enclosing group, in the order a
/// left-to-right reading of it introduces them.
fn collect_vars(p: &GraphPattern, out: &mut VarScope) {
    let mut pending = vec![VarStep::Pattern(p)];
    note_vars(&mut pending, out);
}

/// Drain `pending`, noting each variable as it is reached. Every entry pushes what
/// follows it in reverse, so the work list pops them in written order.
fn note_vars(pending: &mut Vec<VarStep<'_>>, out: &mut VarScope) {
    while let Some(step) = pending.pop() {
        let p = match step {
            VarStep::Note(v) => {
                out.note(v);
                continue;
            }
            VarStep::Term(t) => {
                match t {
                    TermPattern::Variable(v) => out.note(v),
                    TermPattern::Triple(tp) => push_triple_vars(tp, pending),
                    _ => {}
                }
                continue;
            }
            VarStep::Pattern(p) => p,
        };
        match p {
            GraphPattern::Bgp { patterns } => {
                for tp in patterns.iter().rev() {
                    push_triple_vars(tp, pending);
                }
            }
            GraphPattern::Path {
                subject, object, ..
            } => {
                pending.extend([VarStep::Term(object), VarStep::Term(subject)]);
            }
            // Every argument variable of a property function — on either side — is
            // in scope in the enclosing group: the arguments are the call's inputs
            // AND its bindings.
            GraphPattern::PropertyFunction(call) => {
                pending.extend(
                    call.subject_args
                        .iter()
                        .chain(&call.object_args)
                        .rev()
                        .map(VarStep::Term),
                );
            }
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::LeftJoin { left, right, .. } => {
                pending.extend([VarStep::Pattern(right), VarStep::Pattern(left)]);
            }
            GraphPattern::Union { arms } => {
                pending.extend(arms.iter().rev().map(VarStep::Pattern));
            }
            // SPARQL §18.2.1: variables occurring only in the right operand of
            // MINUS are not in scope in the enclosing group graph pattern, so we
            // descend into `left` only.
            GraphPattern::Minus { left, .. } => pending.push(VarStep::Pattern(left)),
            GraphPattern::Filter { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => pending.push(VarStep::Pattern(inner)),
            GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
                pending.push(VarStep::Pattern(inner));
                if let NamedNodePattern::Variable(v) = name {
                    pending.push(VarStep::Note(v));
                }
            }
            GraphPattern::Extend {
                inner, variable, ..
            } => {
                pending.extend([VarStep::Note(variable), VarStep::Pattern(inner)]);
            }
            // `UNFOLD`'s one or two targets are ordinary in-scope bindings of the
            // enclosing group, exactly like `BIND`'s single one: `SELECT *`
            // projects them and a later `FILTER` in the same group sees them.
            GraphPattern::Unfold {
                inner,
                element,
                companion,
                ..
            } => {
                if let Some(companion) = companion {
                    pending.push(VarStep::Note(companion));
                }
                pending.extend([VarStep::Note(element), VarStep::Pattern(inner)]);
            }
            GraphPattern::Values { variables, .. } | GraphPattern::Project { variables, .. } => {
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
}

/// An `EXISTS`/`NOT EXISTS` scope check deferred out of
/// [`Parser::check_exists_body`] because it was reached while
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
    /// EXISTS` keyword — mirrors [`Parser::check_exists_body`]'s own `at`.
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
/// the parser's scope-conflict walk (`find_scope_conflict`) visits is a
/// `BIND`/`(expr AS ?v)` target, a `VALUES` variable or an `UNFOLD` target —
/// the shapes it can report, matching the message forms each call site
/// produces.
///
/// Public because the evaluator's `EXISTS` row-collision check reports the
/// same introductions in the same words: one enum, so the parser's refusal
/// and the evaluator's name a construct identically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeIntro {
    /// `BIND(expr AS ?v)`, a sub-`SELECT`'s `(expr AS ?v)` projection target,
    /// a `GROUP BY (expr AS ?v)` condition, or a `GROUP BY` aggregate's output
    /// variable — all lower to an `Extend`/`Group` introduction and share one
    /// message form.
    Bind,
    /// A `VALUES` block's column variable.
    Values,
    /// An `UNFOLD(expr AS ?e, ?i)` target — either of the two. Its own label
    /// rather than [`Self::Bind`]'s, because the offending text a reader must
    /// go find says `UNFOLD`, not `BIND`.
    Unfold,
}

impl ScopeIntro {
    /// The construct as a diagnostic names it: `BIND target`, `VALUES
    /// variable` or `UNFOLD target`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bind => "BIND target",
            Self::Values => "VALUES variable",
            Self::Unfold => "UNFOLD target",
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
    // Nothing in an empty scope can be rebound.
    if scope.is_empty() {
        return None;
    }
    // The scopes in force: the outer one, and each narrowing a sub-SELECT's
    // projection made. A pending pattern names the scope it is checked against.
    let mut scopes: Vec<Vec<Variable>> = vec![scope.to_vec()];
    let mut pending: Vec<(&'a GraphPattern, usize)> = vec![(pattern, 0)];
    while let Some((pattern, at)) = pending.pop() {
        let scope = &scopes[at];
        match pattern {
            // Leaves: nothing is introduced.
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::PropertyFunction(_) => {}
            // Binary nodes are transparent to scope: both operands are checked at
            // the SAME scope level (see the scope-level argument above), left first.
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::LeftJoin { left, right, .. } => {
                pending.extend([(&**right, at), (&**left, at)]);
            }
            GraphPattern::Union { arms } => pending.extend(arms.iter().rev().map(|arm| (arm, at))),
            // `MINUS` is the one binary node that is NOT scope-transparent on its
            // right operand: §18.2.1 puts a MINUS-right-only variable out of
            // scope, and §18.5's evaluation only ever uses the right side for the
            // compatibility test — its bindings are discarded, never carried
            // forward — so a `BIND`/`VALUES`/aggregate introduction confined to a
            // MINUS right operand can never be observed as a rebinding, at ANY
            // depth, not only under a `SELECT *` sub-select (which was one route
            // to this same shape, not a separate rule). Only the left operand is
            // walked.
            GraphPattern::Minus { left, .. } => pending.push((left, at)),
            // Unary wrappers are transparent to scope; any expression operand is
            // never visited.
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Service { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => pending.push((inner, at)),
            // `BIND`, a sub-SELECT's `(expr AS ?v)`, and a `GROUP BY (expr AS ?v)`
            // condition all lower to `Extend` — a fresh binding at this scope
            // level.
            GraphPattern::Extend {
                inner, variable, ..
            } => {
                if scope.contains(variable) {
                    return Some((variable, ScopeIntro::Bind));
                }
                pending.push((inner, at));
            }
            // `UNFOLD` introduces one or two fresh bindings at this scope level,
            // exactly as `BIND` introduces one — so a `LATERAL`/`EXISTS` right-hand
            // side that re-introduces an outer variable through `UNFOLD` is the
            // same conflict, reported in declaration order (element, then
            // companion). Its expression operand is never visited, for the reason
            // stated for `Extend`'s above.
            GraphPattern::Unfold {
                inner,
                element,
                companion,
                ..
            } => {
                for variable in std::iter::once(element).chain(companion.as_ref()) {
                    if scope.contains(variable) {
                        return Some((variable, ScopeIntro::Unfold));
                    }
                }
                pending.push((inner, at));
            }
            // `VALUES`: the first declared column that collides, in declaration
            // order.
            GraphPattern::Values { variables, .. } => {
                if let Some(v) = variables.iter().find(|v| scope.contains(v)) {
                    return Some((v, ScopeIntro::Values));
                }
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
                if !narrowed.is_empty() {
                    scopes.push(narrowed);
                    pending.push((inner, scopes.len() - 1));
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
                if let Some((v, _)) = aggregates.iter().find(|(v, _)| scope.contains(v)) {
                    return Some((v, ScopeIntro::Bind));
                }
                if let Some(conflict) = find_group_extend_conflict(inner, variables, scope) {
                    return Some(conflict);
                }
            }
        }
    }
    None
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
/// The first conflict reported is the earliest-DECLARED `GROUP BY (expr AS ?v)`
/// condition (the innermost `Extend`, closest to the ungrouped pattern),
/// preserving the walker's left-to-right determinism contract.
fn find_group_extend_conflict<'a>(
    mut inner: &'a GraphPattern,
    variables: &[Variable],
    lhs_scope: &[Variable],
) -> Option<(&'a Variable, ScopeIntro)> {
    let mut chain: Vec<&'a Variable> = Vec::new();
    while let GraphPattern::Extend {
        inner: next,
        variable,
        ..
    } = inner
    {
        if !variables.contains(variable) {
            break;
        }
        chain.push(variable);
        inner = next;
    }
    chain
        .into_iter()
        .rev()
        .find(|variable| lhs_scope.contains(variable))
        .map(|variable| (variable, ScopeIntro::Bind))
}

/// Collect the labels of every blank node in a run of quad patterns, descending
/// into RDF-1.2 quoted triples. Used to enforce the §19.6 rule that a blank node
/// label may not be shared across two operations of one update request.
fn collect_quad_bnode_labels(quads: &[QuadPattern], out: &mut HashSet<String, FixedState>) {
    for q in quads {
        collect_triple_bnode_labels(&q.triple, out);
    }
}

fn collect_triple_bnode_labels(t: &TriplePattern, out: &mut HashSet<String, FixedState>) {
    let mut pending = vec![&t.object, &t.subject];
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::BlankNode(b) => {
                out.insert(b.as_str().to_owned());
            }
            TermPattern::Triple(tp) => pending.extend([&tp.object, &tp.subject]),
            _ => {}
        }
    }
}

/// Hard-fail if any subject/object position of a triple pattern (descending into
/// RDF 1.2 quoted triples) is a blank node. Blank nodes are disallowed in DELETE
/// templates and `DELETE WHERE` (SPARQL 1.1 Update §3.1.3 / §3.1.3.2).
fn reject_blank_in_triple_pattern(t: &TriplePattern, at: usize) -> Result<()> {
    let mut pending = vec![&t.object, &t.subject];
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::BlankNode(_) => {
                return Err(ParseError::syntax(
                    "blank node in a DELETE template is not allowed",
                    at,
                ));
            }
            TermPattern::Triple(tp) => pending.extend([&tp.object, &tp.subject]),
            _ => {}
        }
    }
    Ok(())
}

/// Render an [`IriError`] as a typed [`ParseError::Iri`].
///
/// The reason leads with [`IriError::diagnostic_code`] so that "a relative IRI
/// reference with no base in scope" reads as the SAME condition here as in every
/// other codec, rather than as an eighth independently-worded spelling of it.
fn iri_error(lexical: &str, error: &IriError) -> ParseError {
    ParseError::Iri {
        lexical: lexical.to_owned(),
        reason: format!("{}: {error}", error.diagnostic_code()),
    }
}

/// The clause-terminator / boolean-literal words that end a bare `GROUP BY`
/// GroupCondition or `HAVING`/`ORDER BY` Constraint list — the word set
/// [`Parser::at_bare_constraint`] excludes.
const MODIFIER_TERMINATOR_WORDS: [&str; 8] = [
    "HAVING", "ORDER", "LIMIT", "OFFSET", "VALUES", "BINDINGS", "TRUE", "FALSE",
];

/// The refusal for a second `LIMIT`/`OFFSET` in one solution-modifier list
/// (`LimitOffsetClauses ::= LimitClause OffsetClause? | OffsetClause
/// LimitClause?`). `kw` names the clause that repeated, and `at` is the byte
/// offset of that repeat's keyword.
fn repeated_bound_clause(kw: &str, at: usize) -> ParseError {
    ParseError::syntax(
        format!(
            "repeated {kw} clause: LimitOffsetClauses allows at most one LIMIT \
             and at most one OFFSET, in either order"
        ),
        at,
    )
}

/// Case-insensitive membership of `w` in [`MODIFIER_TERMINATOR_WORDS`].
///
/// `eq_ignore_ascii_case` against each all-ASCII keyword is the same predicate
/// as matching `w.to_ascii_uppercase()`, minus the per-token `String` upper-case
/// copy the modifier loops paid on every peek.
fn is_modifier_terminator_word(w: &str) -> bool {
    MODIFIER_TERMINATOR_WORDS
        .iter()
        .any(|kw| w.eq_ignore_ascii_case(kw))
}

/// Split a `LANG_DIR` token into the language tag and its optional RDF 1.2
/// base direction (`en--ltr` → (`en`, `Ltr`)), refusing anything the
/// production does not admit.
///
/// The terminal is `LANG_DIR ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)* ('--'
/// [a-zA-Z]+)?` (Turtle 1.2 rule [42]; SPARQL 1.2 shares it), narrowed by
/// prose: SPARQL 1.2 §2.3.1 — "The base direction is restricted to either
/// `ltr` or `rtl`. Unlike a language tag, it is always lower case." — and
/// Turtle 1.2 §7.2 — "If present, the initial text direction MUST be either
/// `ltr` or `rtl`." The W3C rdf-tests pin both halves as negative syntax:
/// `@en--unk` ("undefined base direction") and `@en--LTR` ("upper case LTR").
///
/// The lexer hands over the raw `[a-zA-Z0-9-]+` run after `@`, so this is the
/// one site in the parser that enforces the production — but only the
/// **direction** half is decided here. An unknown or wrongly-cased suffix is a
/// syntax error here, never silently dropped to a plain `@en` and never folded
/// into the language tag as `en--foo`; the split is on the FIRST `--`, the
/// production admitting exactly one, so `en--x--ltr` is refused as a bad
/// direction (`x--ltr`) rather than read as language `en--x`.
///
/// The **language** half is not this function's to decide. It goes to
/// [`purrdf_iri::langtag`] under [`LANGTAG_PROFILE`], which is the same
/// acceptance language every RDF codec in the workspace applies, so that a tag
/// a query may write is a tag a document may hold.
fn split_lang_dir(tag: &str, at: usize) -> Result<(String, Option<BaseDirection>)> {
    let (lang, dir) = match tag.split_once("--") {
        Some((lang, dir)) => match BaseDirection::from_str_token(dir) {
            Some(direction) => (lang, Some(direction)),
            None => {
                return Err(ParseError::syntax(
                    format!(
                        "invalid base direction `--{dir}` in `@{tag}`: \
                         must be exactly `ltr` or `rtl` (lower case)"
                    ),
                    at,
                ));
            }
        },
        None => (tag, None),
    };
    if let Err(error) = langtag::parse_with(lang, LANGTAG_PROFILE) {
        return Err(ParseError::syntax(
            format!(
                "invalid language tag `@{tag}`: {error} [{code}]",
                code = error.diagnostic_code()
            ),
            at,
        ));
    }
    Ok((lang.to_owned(), dir))
}

/// The acceptance language every language tag in a SPARQL query is held to.
///
/// [`langtag::Profile::ConcreteSyntaxLangtagBounded`] — the same profile every
/// native RDF codec in this workspace names, and the same one the projection
/// term builder names. That is not tidiness, it is the round-trip obligation: a
/// literal written in a `VALUES` clause or a `BIND` must be expressible in the
/// RDF files the query is run against, and a query result is serialized by
/// those very codecs. When this site held its own transcription of the
/// `LANGTAG` terminal the workspace shipped three acceptance languages at once
/// — `"a"@cantbethislong` bound from SPARQL and was refused from N-Triples,
/// which is the same defect as a codec that cannot read back what a codec
/// wrote.
///
/// The profile is the terminal `[a-zA-Z]+ ('-' [a-zA-Z0-9]+)*` that SPARQL 1.2
/// rule [145] spells, plus the RFC 5646 §2.1 eight-character subtag ceiling
/// outside private use. Both halves are load-bearing: the terminal takes
/// `@en-fr-jura` and `@fr-be-fbcl`, which §2.1 has no reading for and W3C
/// corpora carry, while the ceiling refuses `@cantbethislong`, which the
/// N-Triples negative-syntax corpus requires to be refused.
const LANGTAG_PROFILE: langtag::Profile = langtag::Profile::ConcreteSyntaxLangtagBounded;

/// The language half of `LANG_DIR`, decided by [`purrdf_iri::langtag`] under
/// [`LANGTAG_PROFILE`] rather than by a private transcription of the terminal.
///
/// Kept as a predicate because [`crate::validate`] asks the same question of an
/// algebra assembled programmatically, where there is no token to point at and
/// so nothing to do with the typed error; [`split_lang_dir`] calls
/// [`langtag::parse_with`] directly so that the parse diagnostic can name the
/// production that refused.
pub(crate) fn is_langtag(lang: &str) -> bool {
    langtag::is_well_formed_with(lang, LANGTAG_PROFILE)
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
        // A KEYWORD alternative of production `[127+] Aggregate`, not the
        // `AGG(<iri>, …)` extension surface: `FOLD` names no IRI, so it belongs
        // in this dispatch table beside the SPARQL 1.1 built-ins.
        "FOLD" => AggregateFunction::Fold,
        _ => return None,
    })
}

/// Resolve a SPARQL built-in function NAME to the canonical keyword the grammar
/// spells it with, case-insensitively.
///
/// This is the name-to-function seam a host uses when a *name* (rather than
/// query text) identifies a SPARQL function — SHACL 1.2 Node Expressions §5's
/// `sparql:<NAME>` call form is the motivating caller. It answers from the very
/// table `builtin_function` resolves the grammar with, so a name is callable
/// here exactly when the parser would accept the keyword, and the two can never
/// drift apart into "the resolver claims a function the parser then rejects".
///
/// Returns `None` for a name that is not a keyword-callable built-in — including
/// the SPARQL forms that are operators (`+`, `&&`), grammar productions
/// (`BOUND`, `IF`, `COALESCE`, `sameTerm`, `EXISTS`) or aggregates rather than
/// function calls. Those are not function *calls* in the grammar, so there is no
/// keyword for this seam to return, and a caller that wants them must spell them
/// itself.
#[must_use]
pub fn builtin_function_keyword(name: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    let function = builtin_function(&upper)?;
    // Round-trip through the serializer's table so the returned spelling is the ONE
    // canonical keyword, not the caller's casing. `builtin_function` never yields
    // `Function::Purrdf`/`Function::Custom`, the only two variants without one.
    crate::serialize::function_keyword(&function)
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
    use crate::algebra::{AggregateExpression, ArithmeticOperator};
    use crate::algebra::{PurrdfCall, PurrdfFn};
    use crate::tree::{Chain, NonEmpty};

    const GM: &str =
        "PREFIX purrdf: <https://x/>\nPREFIX rdf: <http://r/>\nPREFIX rdfs: <http://s/>\n";

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("parse")
    }

    // ── `'[' ']'` with nothing between the brackets is not a production ────────────
    //
    // `BlankNodePropertyListPath ::= '[' PropertyListPathNotEmpty ']'` requires at
    // least one predicate-object pair, and the anonymous blank node
    // `ANON ::= '[' WS* ']'` is a TERMINAL, so no comment may sit inside it. Every
    // bracket-pair arm in this parser accepted the empty pair anyway — and so did
    // the Turtle/TriG reader — while asserting in its own rustdoc that "an empty
    // `[]` (SPARQL ANON) is legal". It is legal, but it is a DIFFERENT production:
    // `[]`, `[ ]` and `[` tab/CR/LF `]` all lex to a single `Token::Anon` and never
    // reach these arms. The only input that does reach them is `[ #` comment
    // newline `]`, which no conforming processor accepts.
    //
    // The refusal cannot live in the lexer. Its `ANON` scan is comment-blind by
    // construction, because `[ #` comment newline `?p ?o ]` is a lawful POPULATED
    // property list and nothing lexical separates the two cases; teaching the scan
    // about `#` would WIDEN acceptance, not narrow it. Only the parser, which has
    // the following tokens, can tell them apart.

    fn try_parse(q: &str) -> Result<Query> {
        SparqlParser::new().parse_query(q)
    }

    /// A parser configured with one property function, so the property-function
    /// argument-list arm is reachable from a vector.
    fn try_parse_with_prop_fn(q: &str) -> Result<Query> {
        let options = ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: Vec::new(),
            property_fn_iris: vec!["https://example.org/pf/solve".to_owned()],
        };
        SparqlParser::new().parse_query_with(q, &options)
    }

    /// The refusal, at every bracket-pair arm the parser has: the plain
    /// subject/object list, the triple-term/reifying-triple component, the
    /// reifier id, and the property-function argument list.
    #[test]
    fn an_empty_bracket_pair_is_refused_at_every_arm() {
        for q in [
            "SELECT * WHERE { [ # c\n ] }",
            "SELECT * WHERE { ?s ?p [ # c\n ] }",
            "SELECT * WHERE { ( [ # c\n ] ) <http://example.org/p> ?o }",
            "SELECT * WHERE { << ?s ?p [ # c\n ] >> ?q ?r }",
            "SELECT * WHERE { ?s ?p ?o ~ [ # c\n ] }",
            "SELECT * WHERE { ?s <https://example.org/pf/solve> ( [ # c\n ] ) }",
        ] {
            let err = try_parse_with_prop_fn(q)
                .expect_err("`[` `]` with only a comment between them is not a production");
            assert!(
                matches!(&err, ParseError::Syntax { reason, .. }
                    if reason.contains("with nothing between them")),
                "{q:?} must be refused by the empty-bracket-pair arm, got {err:?}"
            );
        }
    }

    /// Every neighbour of that refusal still parses. This is the half that makes
    /// the refusal a claim rather than a guess: the SAME comment in the SAME
    /// position is lawful the moment the list is populated, and the anonymous
    /// blank node — a different production — is untouched in all of its spellings.
    #[test]
    fn the_neighbours_of_the_empty_bracket_pair_still_parse() {
        for q in [
            "SELECT * WHERE { [ ?p ?o ] }",
            "SELECT * WHERE { [ # c\n ?p ?o ] }",
            "SELECT * WHERE { [ # c\n ?p ?o # d\n ] }",
            "SELECT * WHERE { ?s ?p [] }",
            "SELECT * WHERE { ?s ?p [ ] }",
            "SELECT * WHERE { ?s ?p [\t\r\n] }",
            "SELECT * WHERE { ?s ?p ( ) }",
            "SELECT * WHERE { ?s ?p ( # c\n ) }",
            "SELECT * WHERE { << ?s ?p [] >> ?q ?r }",
            "SELECT * WHERE { ?s ?p ?o ~ [] }",
        ] {
            try_parse(q).unwrap_or_else(|e| panic!("must still parse {q:?}: {e}"));
        }
        try_parse_with_prop_fn("SELECT * WHERE { ?s <https://example.org/pf/solve> ( [] ) }")
            .expect("an anonymous blank node is a lawful property-function argument");
    }

    /// `NIL ::= '(' WS* ')'` is the sibling terminal and still folds to `rdf:nil`
    /// — the bracket-pair refusal must not have leaked into the parenthesis pair.
    #[test]
    fn the_empty_collection_is_still_rdf_nil() {
        let query = parse("SELECT * WHERE { ?s <http://example.org/p> ( ) }");
        let rendered = format!("{query:?}");
        assert!(
            rendered.contains("22-rdf-syntax-ns#nil"),
            "`( )` must still be rdf:nil: {rendered}"
        );
    }

    /// The two lexer sites had to move together, and this is the vector that says
    /// why. With the `ANON` scan tightened but `skip_trivia` still liberal,
    /// `{ [<NBSP>] }` FLIPS from refused to ACCEPTED: the NO-BREAK SPACE stops
    /// closing the `ANON`, an `LBracket` is emitted instead, the liberal trivia
    /// skip then eats the NO-BREAK SPACE, the `]` arrives, and the empty-pair arm
    /// mints a blank node. Tightening both sites is what keeps it refused.
    /// The flip is nastier than a widened language, because `[ ]` in that exact
    /// position is NOT accepted: a bare `ANON` is not a triple, so `{ [ ] }` is a
    /// syntax error. A half-done change would therefore have made the NO-BREAK
    /// SPACE spelling the only one of the two that parsed.
    #[test]
    fn a_non_ws_space_between_the_brackets_is_refused() {
        assert!(
            try_parse("SELECT * WHERE { [\u{a0}] }").is_err(),
            "U+00A0 is not `WS`, so `[<NBSP>]` is neither an ANON nor a property list"
        );
        assert!(
            try_parse("SELECT * WHERE { [ ] }").is_err(),
            "and its ASCII spelling is a bare ANON, which is not a triple either"
        );
        // The neighbours that ARE lawful, in the same two positions, still parse:
        // the ANON as an object, and the populated property list as a subject.
        try_parse("SELECT * WHERE { ?s ?p [ ] }").expect("`[ ]` is an ANON in object position");
        try_parse("SELECT * WHERE { [ ?p ?o ] }")
            .expect("a populated property list may stand alone");
    }

    // -- LANG_DIR: the base direction is exactly `ltr` / `rtl` -----------------------

    /// A well-formed query whose only variable part is the language tag, so a
    /// refusal can come from nothing but the tag. (The vendored W3C
    /// `lang-basedir/langdir-literal-invalid.rq` spells its projection `AS v`,
    /// which is itself a syntax error — so that negative test cannot tell a
    /// refused direction from a refused projection. This shape can.)
    fn parse_lang_literal(tag: &str) -> Result<Query> {
        SparqlParser::new().parse_query(&format!("SELECT (\"x\"@{tag} AS ?v) WHERE {{}}"))
    }

    /// Every valid neighbour of the refusals below still parses, with the
    /// direction the grammar assigns it. `EN--ltr` is valid: only the DIRECTION
    /// is case-restricted, the language tag is not.
    #[test]
    fn lang_dir_accepts_every_grammar_admitted_shape() {
        for (tag, lang, dir) in [
            ("en", "en", None),
            ("en-US", "en-US", None),
            ("zh-Hant-TW", "zh-Hant-TW", None),
            ("x-klingon", "x-klingon", None),
            ("en-a1", "en-a1", None),
            ("en--ltr", "en", Some(BaseDirection::Ltr)),
            ("ar--rtl", "ar", Some(BaseDirection::Rtl)),
            ("en-US--rtl", "en-US", Some(BaseDirection::Rtl)),
            ("EN--ltr", "EN", Some(BaseDirection::Ltr)),
        ] {
            assert_eq!(
                split_lang_dir(tag, 0).expect(tag),
                (lang.to_owned(), dir),
                "@{tag}"
            );
            parse_lang_literal(tag).unwrap_or_else(|e| panic!("@{tag} must parse: {e}"));
        }
    }

    /// An unknown or wrongly-cased base direction is a SYNTAX ERROR, never a
    /// silently dropped suffix: before this pin, `"x"@en--foo` parsed as a plain
    /// `"x"@en` and `"x"@en--LTR` as `"x"@en--ltr`. The two vendored W3C
    /// rdf-tests spellings are first (`@en--unk`, `@en--LTR`), then the shapes
    /// the raw `[a-zA-Z0-9-]+` lexer run lets through that the production does
    /// not: an empty direction, a doubled one, a stray hyphen, an empty subtag,
    /// and a digit-led primary subtag.
    #[test]
    fn lang_dir_refuses_what_the_production_does_not_admit() {
        for tag in [
            "en--unk",
            "en--LTR",
            "en--foo",
            "en--Rtl",
            "en--",
            "en--ltr--ltr",
            "en--x--ltr",
            "en---ltr",
            "en-",
            "en--ltr-x",
            "-en",
            "en--ltr-",
            "1en",
            "en-.us",
        ] {
            let err = split_lang_dir(tag, 0).expect_err(tag);
            assert!(
                err.to_string().contains("invalid"),
                "@{tag}: diagnostic names the refusal, got {err}"
            );
            assert!(parse_lang_literal(tag).is_err(), "@{tag} must not parse");
        }
        let err = parse_lang_literal("en--foo").expect_err("unknown direction");
        assert!(
            err.to_string()
                .contains("invalid base direction `--foo` in `@en--foo`"),
            "the diagnostic names the suffix and the tag: {err}"
        );
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
            GraphPattern::Project { inner, .. } => inner.into_inner(),
            other => other,
        }
    }

    /// The `FILTER` expression of a parsed `SELECT * WHERE { FILTER(…) }`.
    fn filter_expr(q: &str) -> Expression {
        let GraphPattern::Filter { expr, .. } = unproject(select_pattern(q)) else {
            panic!("expected Filter");
        };
        expr
    }

    /// An expression operator chain is ONE n-ary node however many operators it has:
    /// a hundred thousand `||`, `&&`, `+`/`-` or `*`/`/` parse, with every operand
    /// present, in source order, at a height of one level above the operands. A chain
    /// is flat text, so charging a level per operator refused a generated
    /// `?x = 1 || ?x = 2 || …` past 511 alternatives although it nests nothing.
    #[test]
    fn expression_operator_chains_of_any_length_are_one_node() {
        const OPS: usize = 100_000;
        let or = filter_expr(&format!(
            "SELECT * WHERE {{ FILTER({}?y) }}",
            "?x || ".repeat(OPS)
        ));
        let Expression::Or(operands) = &or else {
            panic!("expected Or, got a different node");
        };
        assert_eq!(operands.len(), OPS + 1);
        assert_eq!(operands[OPS], Expression::Variable(Variable::new("y")));
        assert!(
            operands[..OPS]
                .iter()
                .all(|e| *e == Expression::Variable(Variable::new("x")))
        );

        let and = filter_expr(&format!(
            "SELECT * WHERE {{ FILTER({}?y) }}",
            "?x && ".repeat(OPS)
        ));
        assert!(matches!(&and, Expression::And(operands) if operands.len() == OPS + 1));

        // Additive and multiplicative operators alternate by precedence: `?x + 1 * 2`
        // is `?x + (1 * 2)`, so each `* 2` is its own two-operand chain, a step
        // operand of the one additive chain.
        let mixed = filter_expr(&format!(
            "SELECT * WHERE {{ FILTER(?x{} > 0) }}",
            " + 1 * 2 - ?z / 3".repeat(OPS / 2)
        ));
        let Expression::Greater(chain, _) = &mixed else {
            panic!("expected Greater");
        };
        let Expression::Arithmetic(first, steps) = &**chain else {
            panic!("expected Arithmetic");
        };
        assert_eq!(**first, Expression::Variable(Variable::new("x")));
        assert_eq!(steps.len(), OPS);
        for (i, (op, operand)) in steps.iter().enumerate() {
            let (expected_op, expected_inner) = if i % 2 == 0 {
                (ArithmeticOperator::Add, ArithmeticOperator::Multiply)
            } else {
                (ArithmeticOperator::Subtract, ArithmeticOperator::Divide)
            };
            assert_eq!(*op, expected_op, "step {i}");
            assert!(
                matches!(operand, Expression::Arithmetic(_, inner)
                    if matches!(inner.as_slice(), [(o, _)] if *o == expected_inner)),
                "step {i}"
            );
        }
    }

    /// A left spine is one chain whatever its brackets say: `(a + b) * c` is the chain
    /// `a`, `+ b`, `* c`, because the multiplication applies to the value of
    /// everything before it — exactly the left fold the binary tree denotes. A bracket
    /// on the right is a real level: `a - (b - c)` keeps its own chain as an operand.
    #[test]
    fn a_bracketed_left_operand_extends_the_chain_and_a_right_one_nests() {
        let x = || Expression::Variable(Variable::new("x"));
        let y = || Expression::Variable(Variable::new("y"));
        let z = || Expression::Variable(Variable::new("z"));
        assert_eq!(
            filter_expr("SELECT * WHERE { FILTER((?x + ?y) * ?z > 0) }"),
            Expression::Greater(
                Child::new(Expression::Arithmetic(
                    Child::new(x()),
                    NonEmpty::try_from(vec![
                        (ArithmeticOperator::Add, y()),
                        (ArithmeticOperator::Multiply, z())
                    ])
                    .expect("a nonempty of enough nodes"),
                )),
                Child::new(Expression::Literal(Literal::new_typed(
                    "0",
                    NamedNode::new_unchecked(XSD_INTEGER)
                ))),
            )
        );
        assert_eq!(
            filter_expr("SELECT * WHERE { FILTER(?x - (?y - ?z)) }"),
            Expression::Arithmetic(
                Child::new(x()),
                NonEmpty::try_from(vec![(
                    ArithmeticOperator::Subtract,
                    Expression::Arithmetic(
                        Child::new(y()),
                        NonEmpty::try_from(vec![(ArithmeticOperator::Subtract, z())])
                            .expect("one or more steps")
                    )
                )])
                .expect("a nonempty of enough nodes"),
            )
        );
        assert_eq!(
            filter_expr("SELECT * WHERE { FILTER((?x || ?y) || ?z) }"),
            Expression::Or(Chain::try_from(vec![x(), y(), z()]).expect("two or more nodes"))
        );
        assert_eq!(
            filter_expr("SELECT * WHERE { FILTER(?x || (?y || ?z)) }"),
            Expression::Or(
                Chain::try_from(vec![
                    x(),
                    Expression::Or(Chain::try_from(vec![y(), z()]).expect("two or more nodes"))
                ])
                .expect("two or more nodes")
            )
        );
    }

    /// A property-path expression chain (`p1 / p2 / …`, `p1 | p2 | …`) is ONE n-ary
    /// node however many operators it has: a hundred thousand `/` or `|` parse under
    /// `+`, which retains the expression, with every element present in source order.
    #[test]
    fn property_path_chains_of_any_length_are_one_node() {
        const OPS: usize = 100_000;
        let step = |i: usize| format!("<http://example.org/p{i}>");
        let element = |i: usize| {
            PropertyPathExpression::NamedNode(NamedNode::new_unchecked(format!(
                "http://example.org/p{i}"
            )))
        };
        for (op, is_sequence) in [('/', true), ('|', false)] {
            let text = (0..=OPS)
                .map(step)
                .collect::<Vec<_>>()
                .join(&op.to_string());
            let PropertyPathExpression::OneOrMore(path) =
                path_of(&format!("SELECT * WHERE {{ ?s ({text})+ ?o }}"))
            else {
                panic!("the quantifier retains its path expression");
            };
            let ((PropertyPathExpression::Sequence(elements), true)
            | (PropertyPathExpression::Alternative(elements), false)) = (&*path, is_sequence)
            else {
                panic!("expected one {op} chain");
            };
            assert_eq!(elements.len(), OPS + 1, "{op}");
            assert!(
                elements.iter().enumerate().all(|(i, e)| *e == element(i)),
                "{op}: every element in source order"
            );
        }
    }

    /// `/` binds tighter than `|`, so a mixed chain is an alternative of sequences; a
    /// bracketed LEFT element of the same operator extends the chain (composition and
    /// bag union are associative, so it is the same relation), and a bracketed later
    /// one stays one element, as written.
    #[test]
    fn path_chains_keep_precedence_and_bracketing() {
        use PropertyPathExpression::{Alternative as Alt, Sequence as Seq};
        let p = |name: &str| {
            PropertyPathExpression::NamedNode(NamedNode::new_unchecked(format!(
                "http://example.org/{name}"
            )))
        };
        let q = |path: &str| {
            let PropertyPathExpression::OneOrMore(path) = path_of(&format!(
                "PREFIX ex: <http://example.org/> SELECT * WHERE {{ ?s ({path})+ ?o }}"
            )) else {
                panic!("the quantifier retains its path expression");
            };
            path.into_inner()
        };
        assert_eq!(
            q("ex:a/ex:b|ex:c/ex:d|ex:e"),
            Alt(Chain::try_from(vec![
                Seq(Chain::try_from(vec![p("a"), p("b")]).expect("two or more nodes")),
                Seq(Chain::try_from(vec![p("c"), p("d")]).expect("two or more nodes")),
                p("e")
            ])
            .expect("a chain of enough nodes"))
        );
        assert_eq!(
            q("(ex:a/ex:b)|ex:c/ex:d"),
            Alt(Chain::try_from(vec![
                Seq(Chain::try_from(vec![p("a"), p("b")]).expect("two or more nodes")),
                Seq(Chain::try_from(vec![p("c"), p("d")]).expect("two or more nodes"))
            ])
            .expect("two or more nodes"))
        );
        assert_eq!(
            q("(ex:a/ex:b)/ex:c"),
            Seq(Chain::try_from(vec![p("a"), p("b"), p("c")]).expect("two or more nodes"))
        );
        assert_eq!(
            q("ex:a/(ex:b/ex:c)"),
            Seq(Chain::try_from(vec![
                p("a"),
                Seq(Chain::try_from(vec![p("b"), p("c")]).expect("two or more nodes"))
            ])
            .expect("two or more nodes"))
        );
        assert_eq!(
            q("(ex:a|ex:b)|ex:c"),
            Alt(Chain::try_from(vec![p("a"), p("b"), p("c")]).expect("two or more nodes"))
        );
        assert_eq!(
            q("ex:a|(ex:b|ex:c)"),
            Alt(Chain::try_from(vec![
                p("a"),
                Alt(Chain::try_from(vec![p("b"), p("c")]).expect("two or more nodes"))
            ])
            .expect("two or more nodes"))
        );
        assert_eq!(
            q("(ex:a|ex:b)/ex:c"),
            Seq(Chain::try_from(vec![
                Alt(Chain::try_from(vec![p("a"), p("b")]).expect("two or more nodes")),
                p("c")
            ])
            .expect("two or more nodes"))
        );
    }

    /// A run of `UNION` arms is ONE node, one level above its arms, so it is charged
    /// as its tallest arm, not as the sum of them: twenty thousand arms parse into one
    /// `Union` with every arm in source order.
    #[test]
    fn a_union_arm_run_of_any_length_is_one_node() {
        const ARMS: usize = 20_000;
        let body = (0..ARMS)
            .map(|i| format!("{{ ?s <https://example.org/p> {i} }}"))
            .collect::<Vec<_>>()
            .join(" UNION ");
        let GraphPattern::Union { arms } =
            unproject(select_pattern(&format!("SELECT * WHERE {{ {body} }}")))
        else {
            panic!("expected one Union");
        };
        assert_eq!(arms.len(), ARMS);
        for (i, arm) in arms.iter().enumerate() {
            let GraphPattern::Bgp { patterns } = arm else {
                panic!("arm {i} is a BGP");
            };
            assert_eq!(
                patterns[0].object,
                TermPattern::Literal(Literal::new_typed(
                    i.to_string(),
                    NamedNode::new_unchecked(XSD_INTEGER)
                )),
                "arm {i} in source order"
            );
        }
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
    fn optional_lifts_the_conjunction_of_its_own_filters() {
        let q = format!(
            "{GM}SELECT * {{ ?x purrdf:p ?v . OPTIONAL {{ ?y purrdf:q ?w . FILTER(?v=2) FILTER(?w=3) }} }}"
        );
        let GraphPattern::LeftJoin {
            right, expression, ..
        } = unproject(select_pattern(&q))
        else {
            panic!("expected LeftJoin");
        };
        let Some(Expression::And(filters)) = expression else {
            panic!("both group-owned filters must be the LeftJoin condition");
        };
        assert_eq!(filters.len(), 2);
        assert!(matches!(*right, GraphPattern::Bgp { .. }));
    }

    #[test]
    fn optional_keeps_a_nested_groups_filter_in_the_right_operand() {
        let q = format!(
            "{GM}SELECT * {{ ?x purrdf:p ?v . OPTIONAL {{ {{ ?y purrdf:q ?w . FILTER(?v=2) }} }} }}"
        );
        let GraphPattern::LeftJoin {
            right, expression, ..
        } = unproject(select_pattern(&q))
        else {
            panic!("expected LeftJoin");
        };
        assert!(expression.is_none());
        assert!(matches!(*right, GraphPattern::Filter { .. }));
    }

    #[test]
    fn optional_lifts_only_its_own_conjunction_around_a_nested_filter() {
        let q = format!(
            "{GM}SELECT * {{ ?x purrdf:p ?v . OPTIONAL {{ {{ ?y purrdf:q ?w . FILTER(?w=3) }} FILTER(?v=2) FILTER(?w<4) }} }}"
        );
        let GraphPattern::LeftJoin {
            right, expression, ..
        } = unproject(select_pattern(&q))
        else {
            panic!("expected LeftJoin");
        };
        let Some(Expression::And(filters)) = expression else {
            panic!("only the two enclosing filters belong to the LeftJoin condition");
        };
        assert_eq!(filters.len(), 2);
        assert!(matches!(*right, GraphPattern::Filter { .. }));
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
        } = inner.into_inner()
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
        let inner = inner.into_inner();
        let GraphPattern::Extend {
            inner: group_inner,
            variable,
            ..
        } = inner
        else {
            panic!("expected Extend under OrderBy, got {inner:?}");
        };
        assert_eq!(variable, Variable::new("c"));
        let GraphPattern::Group { aggregates, .. } = group_inner.into_inner() else {
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
        let inner = inner.into_inner();
        let GraphPattern::Filter { expr, inner: group } = inner else {
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
        let inner = inner.into_inner();
        let GraphPattern::Filter {
            expr: outer_expr,
            inner: mid,
        } = inner
        else {
            panic!("expected outer Filter (2nd HAVING condition), got {inner:?}");
        };
        assert!(
            matches!(outer_expr, Expression::Exists(_)),
            "expected the 2nd condition (EXISTS) outermost, got {outer_expr:?}"
        );
        let mid = mid.into_inner();
        let GraphPattern::Filter {
            expr: inner_expr,
            inner: group,
        } = mid
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

    /// `SolutionModifier ::= GroupClause? HavingClause? OrderClause?
    /// LimitOffsetClauses?` and `ValuesClause ::= ( 'VALUES' DataBlock )?` — no
    /// clause in the list may repeat, so none of these is a production. Only
    /// the bound clauses were read by a loop that could overwrite an earlier
    /// value; each clause here is parsed by a single conditional, so a repeat
    /// falls out of the modifier list and is refused (by the end-of-query
    /// guard, or — for `GROUP BY`, whose bare-condition arm consumes the
    /// repeated keyword as a callee — as an unsupported function name). Pinned
    /// so the overwrite shape cannot be reintroduced here either.
    #[test]
    fn no_other_solution_modifier_may_repeat() {
        for q in [
            "SELECT ?a WHERE { ?a ?p ?o } GROUP BY ?a GROUP BY ?p",
            "SELECT ?a WHERE { ?a ?p ?o } GROUP BY ?a ORDER BY ?a GROUP BY ?p",
            "SELECT (COUNT(?o) AS ?n) WHERE { ?a ?p ?o } GROUP BY ?a \
             HAVING (?n > 1) HAVING (?n > 2)",
            "SELECT ?a WHERE { ?a ?p ?o } ORDER BY ?a ORDER BY ?p",
            "SELECT ?a WHERE { ?a ?p ?o } VALUES ?a { 1 } VALUES ?a { 2 }",
        ] {
            let err = try_parse(q).expect_err("a repeated solution modifier is not a production");
            assert!(
                matches!(
                    &err,
                    ParseError::Syntax { reason, .. } if reason.starts_with("unexpected trailing token")
                ) || matches!(&err, ParseError::Unsupported(_)),
                "`{q}` must be refused, got {err:?}"
            );
        }
    }

    /// The `Slice` (or its absence) for every `LimitOffsetClauses` spelling the
    /// grammar admits — `LimitClause OffsetClause? | OffsetClause LimitClause?`,
    /// so BOTH clause orders, either clause alone, and neither. The mirror of
    /// [`a_repeated_bound_clause_is_refused`]: refusing the second clause must
    /// not narrow the set the grammar permits, and `LIMIT 0` in particular must
    /// stay a real zero-row bound rather than being read as "no bound".
    #[test]
    fn every_bound_clause_spelling_the_grammar_admits_still_parses() {
        fn slice_of(q: &str) -> Option<(usize, Option<usize>)> {
            match select_pattern(q) {
                GraphPattern::Slice { start, length, .. } => Some((start, length)),
                other => {
                    assert!(
                        matches!(other, GraphPattern::Project { .. }),
                        "an unbounded query keeps its Project at the top, got {other:?}"
                    );
                    None
                }
            }
        }
        let base = format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }}");
        assert_eq!(
            slice_of(&base),
            None,
            "no bound clause must build no Slice at all"
        );
        for (tail, start, length) in [
            ("LIMIT 5", 0, Some(5)),
            ("OFFSET 2", 2, None),
            ("LIMIT 5 OFFSET 2", 2, Some(5)),
            ("OFFSET 2 LIMIT 5", 2, Some(5)),
            ("LIMIT 0", 0, Some(0)),
            ("OFFSET 0", 0, None),
            ("LIMIT 0 OFFSET 0", 0, Some(0)),
            ("OFFSET 0 LIMIT 0", 0, Some(0)),
            // A bound far past any plausible row count is still just a bound —
            // the value is carried, never clamped. Held below 2^32 so the
            // assertion means the same thing on a 32-bit/wasm32 target.
            (
                "LIMIT 4000000000 OFFSET 4000000000",
                4_000_000_000,
                Some(4_000_000_000),
            ),
        ] {
            let q = format!("{base} {tail}");
            assert_eq!(
                slice_of(&q),
                Some((start, length)),
                "`{tail}` must reach Slice {{ start: {start}, length: {length:?} }}"
            );
        }
    }

    /// `LimitOffsetClauses` admits at most ONE `LIMIT` and at most one
    /// `OFFSET`. A repeat used to be swallowed by the bound-clause loop, which
    /// overwrote the earlier value — so `LIMIT 2 LIMIT 13` returned thirteen
    /// rows and the caller's own bound of two vanished. It is refused, naming
    /// the clause that repeated and pointing at that repeat's keyword, at every
    /// query form and on a sub-select.
    #[test]
    fn a_repeated_bound_clause_is_refused() {
        fn expected(clause: &str) -> String {
            format!(
                "repeated {clause} clause: LimitOffsetClauses allows at most one LIMIT \
                 and at most one OFFSET, in either order"
            )
        }
        let base = format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }}");
        for (tail, clause) in [
            ("LIMIT 2 LIMIT 13", "LIMIT"),
            ("LIMIT 13 LIMIT 2", "LIMIT"),
            ("OFFSET 1 OFFSET 4", "OFFSET"),
            // The interleaved spellings: the repeat is separated from the first
            // clause by the other one, which the loop read as a fresh pair.
            ("LIMIT 2 OFFSET 1 LIMIT 13", "LIMIT"),
            ("OFFSET 1 LIMIT 2 OFFSET 4", "OFFSET"),
            ("ORDER BY ?a LIMIT 2 LIMIT 13", "LIMIT"),
        ] {
            let q = format!("{base} {tail}");
            let err = try_parse(&q).expect_err("a repeated bound clause is not a production");
            let ParseError::Syntax { reason, at } = &err else {
                panic!("`{tail}` must be a Syntax refusal, got {err:?}");
            };
            assert_eq!(*reason, expected(clause), "for `{tail}`");
            assert_eq!(
                *at,
                q.rfind(clause)
                    .expect("the repeated keyword is in the query"),
                "`{tail}` must report the byte offset of the REPEATED {clause}"
            );
        }
        // The same bound-clause parse backs every query form and the sub-select,
        // so the refusal is not SELECT-only.
        for q in [
            format!("{GM}CONSTRUCT {{ ?a a purrdf:U }} WHERE {{ ?a a purrdf:T }} LIMIT 2 LIMIT 13"),
            format!("{GM}DESCRIBE ?a WHERE {{ ?a a purrdf:T }} OFFSET 1 OFFSET 4"),
            format!(
                "{GM}SELECT ?a WHERE {{ {{ SELECT ?a WHERE {{ ?a a purrdf:T }} LIMIT 2 LIMIT 13 }} }}"
            ),
        ] {
            let err = try_parse(&q).expect_err("a repeated bound clause is not a production");
            let ParseError::Syntax { reason, .. } = &err else {
                panic!("`{q}` must be a Syntax refusal, got {err:?}");
            };
            assert!(
                reason.starts_with("repeated "),
                "`{q}` must reach the repeated-bound-clause refusal, got {reason:?}"
            );
        }
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

    /// **A dataset clause inside a sub-`SELECT` is refused, at the clause's own
    /// offset.**
    ///
    /// `SubSelect ::= SelectClause WhereClause SolutionModifier ValuesClause` has no
    /// `DatasetClause`, so a `FROM` there is not a production. This parser read one
    /// anyway — one function serves both positions — and the sub-select site then
    /// unwrapped the `Query` with `..`, which dropped the clause. That is the worst
    /// available outcome: the text parsed, the query answered, and the graphs the
    /// caller named decided nothing. A wrapping layer that moved a top-level `FROM`
    /// inside a sub-select was therefore silently voiding it.
    ///
    /// The offset is asserted, not just the refusal, because a diagnostic pointing at
    /// the enclosing `}` would send its reader to the wrong clause in a query with two
    /// sub-selects in it.
    #[test]
    fn a_sub_select_dataset_clause_is_refused_and_a_top_level_one_is_not() {
        const REASON: &str = "a sub-SELECT carries no dataset clause: FROM and FROM NAMED are \
                              written on a whole query, which is the scope they apply to";

        for inner in [
            "FROM <http://example.org/g>",
            "FROM NAMED <http://example.org/n>",
            "FROM <http://example.org/g> FROM NAMED <http://example.org/n>",
        ] {
            let q = format!(
                "{GM}SELECT ?a WHERE {{ {{ SELECT ?a {inner} WHERE {{ ?a a purrdf:T }} }} }}"
            );
            let err = try_parse(&q).expect_err("a sub-SELECT dataset clause is not a production");
            let ParseError::Syntax { reason, at } = &err else {
                panic!("`{inner}` must be a Syntax refusal, got {err:?}");
            };
            assert_eq!(*reason, REASON, "for `{inner}`");
            assert_eq!(
                *at,
                q.find(inner).expect("the clause is in the query"),
                "`{inner}` must be reported at its own first FROM"
            );
        }

        // An `EXISTS` body is reached through the same sub-select site, so the refusal
        // is not a property of one enclosing shape.
        let nested = format!(
            "{GM}SELECT ?a WHERE {{ ?a a purrdf:T \
             FILTER EXISTS {{ {{ SELECT ?a FROM <http://example.org/g> WHERE {{ ?a a purrdf:U }} }} }} }}"
        );
        let err = try_parse(&nested).expect_err("an EXISTS body is a group graph pattern too");
        let ParseError::Syntax { reason, at } = &err else {
            panic!("expected a Syntax refusal, got {err:?}");
        };
        assert_eq!(*reason, REASON);
        assert_eq!(
            *at,
            nested.find("FROM").expect("the clause is in the query"),
            "reported at the clause, wherever the sub-select was reached from"
        );

        // THE NEIGHBOURS THAT MUST STILL PARSE. Every one of these is valid SPARQL and
        // none of them is what the refusal is about: a sub-select with no dataset
        // clause, a whole query with one, and a whole query with one that also contains
        // a sub-select — the shape a wrapping layer emits, and the shape a refusal
        // reaching one token too far would have closed.
        for q in [
            format!("{GM}SELECT ?a WHERE {{ {{ SELECT ?a WHERE {{ ?a a purrdf:T }} }} }}"),
            format!("{GM}SELECT ?a FROM <http://example.org/g> WHERE {{ ?a a purrdf:T }}"),
            format!(
                "{GM}SELECT ?a FROM <http://example.org/g> FROM NAMED <http://example.org/n> \
                 WHERE {{ {{ SELECT ?a WHERE {{ ?a a purrdf:T }} }} }}"
            ),
            format!(
                "{GM}SELECT ?a FROM NAMED <http://example.org/n> \
                 WHERE {{ GRAPH ?g {{ {{ SELECT ?a WHERE {{ ?a a purrdf:T }} }} }} }}"
            ),
        ] {
            try_parse(&q).unwrap_or_else(|err| panic!("`{q}` is valid SPARQL, got {err:?}"));
        }
    }

    /// **The dataset clause's byte range is reported beside the prologue's offset, and
    /// excising it leaves a query that still parses.**
    ///
    /// This is the position a layer wrapping a supplied query in a sub-`SELECT` has to
    /// have: the clause cannot stay in the body, because the body becomes a sub-select
    /// and that production refuses it, and it cannot be re-rendered from the algebra
    /// without rewriting spellings the caller depends on. So it is moved as text, which
    /// needs a range.
    ///
    /// The range deliberately ends at the token *after* the run rather than at the last
    /// IRI, so it carries its own trailing whitespace and the two remaining halves
    /// rejoin without a token collision.
    #[test]
    fn the_dataset_clause_range_is_reported_and_excising_it_leaves_a_query() {
        let options = ParserOptions::default();
        let parser = SparqlParser::new();

        let text = "PREFIX ex: <http://example.org/ns#>\nSELECT ?s FROM <http://example.org/g> \
                    FROM NAMED <http://example.org/n>\nWHERE { ?s ex:p ?o }";
        let split = parser
            .parse_query_split(text, &options)
            .expect("a query with a dataset clause parses");
        let at = split.dataset_at.clone().expect("the clause is reported");
        assert_eq!(
            &text[at.clone()],
            "FROM <http://example.org/g> FROM NAMED <http://example.org/n>\n",
            "the whole run, from the first FROM to the token after the last clause"
        );
        assert!(
            at.start >= split.body_at,
            "a dataset clause follows the query form's head, so it is inside the body"
        );

        // The excision the wrapping layer performs, executed: the clause comes out, the
        // rest is untouched, and what is left is still a query.
        let excised = format!("{}{}", &text[..at.start], &text[at.end..]);
        assert_eq!(
            excised, "PREFIX ex: <http://example.org/ns#>\nSELECT ?s WHERE { ?s ex:p ?o }",
            "nothing but the dataset clause moves"
        );
        parser
            .parse_query_split(&excised, &options)
            .expect("the body without its dataset clause is still a query");

        // All four query forms carry the clause, and all four report it: the position is
        // a fact about the text rather than about the one form a caller happens to wrap.
        for form in [
            "SELECT ?s FROM <http://example.org/g> WHERE { ?s ?p ?o }",
            "CONSTRUCT { ?s ?p ?o } FROM <http://example.org/g> WHERE { ?s ?p ?o }",
            "ASK FROM <http://example.org/g> WHERE { ?s ?p ?o }",
            "DESCRIBE ?s FROM <http://example.org/g> WHERE { ?s ?p ?o }",
        ] {
            let reported = parser
                .parse_query_split(form, &options)
                .unwrap_or_else(|err| panic!("`{form}` parses, got {err:?}"))
                .dataset_at
                .unwrap_or_else(|| panic!("`{form}` writes a dataset clause"));
            assert_eq!(
                &form[reported], "FROM <http://example.org/g> ",
                "for `{form}`"
            );
        }
    }

    /// **A query's dataset slot is its clause when it writes one, and the empty range a
    /// clause would occupy when it does not — never a sub-`SELECT`'s position.**
    ///
    /// Inserting a clause at the empty slot is executed, and the algebra read back from
    /// the spliced text is asserted to carry it, for every query form (both `CONSTRUCT`
    /// forms, and a `WHERE`-less group).
    #[test]
    fn the_dataset_slot_is_reported_for_every_query_with_or_without_a_clause() {
        let options = ParserOptions::default();
        let parser = SparqlParser::new();

        let written = "SELECT ?s FROM <http://example.org/g> WHERE { ?s ?p ?o }";
        let slot = parser
            .parse_query_dataset_slot(written, &options)
            .expect("parses");
        assert_eq!(&written[slot.dataset_at], "FROM <http://example.org/g> ");

        for (form, before) in [
            ("SELECT ?s WHERE { ?s ?p ?o }", "WHERE"),
            ("SELECT ?s{ ?s ?p ?o }", "{"),
            ("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }", "WHERE"),
            ("CONSTRUCT WHERE { ?s ?p ?o }", "WHERE"),
            ("ASK { ?s ?p ?o }", "{"),
            ("DESCRIBE ?s WHERE { ?s ?p ?o }", "WHERE"),
            // A sub-select inside the WHERE is read after the query's own slot, and one
            // inside a projected EXISTS is read before it; neither displaces it.
            (
                "SELECT ?s WHERE { { SELECT ?s WHERE { ?s ?p ?o } } }",
                "WHERE { {",
            ),
            (
                "SELECT (EXISTS { { SELECT ?s WHERE { ?s ?p ?o } } } AS ?e) WHERE { ?s ?p ?o }",
                "WHERE { ?s ?p ?o }",
            ),
        ] {
            let slot = parser
                .parse_query_dataset_slot(form, &options)
                .unwrap_or_else(|err| panic!("`{form}` parses, got {err:?}"));
            assert!(
                slot.dataset_at.is_empty(),
                "no clause is written in `{form}`"
            );
            assert!(
                form[slot.dataset_at.start..].starts_with(before),
                "`{form}`: the slot sits before `{before}`, got {:?}",
                &form[slot.dataset_at.start..]
            );
            let spliced = format!(
                "{} FROM <http://example.org/g> {}",
                &form[..slot.dataset_at.start],
                &form[slot.dataset_at.start..]
            );
            let query = parser
                .parse_query_with(&spliced, &options)
                .unwrap_or_else(|err| panic!("`{spliced}` parses, got {err:?}"));
            let dataset = match query {
                Query::Select { dataset, .. }
                | Query::Construct { dataset, .. }
                | Query::Ask { dataset, .. }
                | Query::Describe { dataset, .. } => dataset,
            };
            assert_eq!(
                dataset.default.len(),
                1,
                "`{spliced}` carries the inserted FROM"
            );
        }
    }

    /// **Each update operation reports where its dataset clause is or would go, by the
    /// production it was read under.**
    #[test]
    fn the_update_split_reports_each_operations_dataset_slot() {
        let options = ParserOptions::default();
        let parser = SparqlParser::new();
        let text = "PREFIX ex: <http://example.org/>\n\
                    INSERT DATA { ex:a ex:p ex:b } ;\n\
                    INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o } ;\n\
                    DELETE { ?s ex:q ?o } INSERT { ?s ex:r ?o } USING ex:g USING NAMED ex:n WHERE { ?s ex:q ?o } ;\n\
                    WITH ex:g DELETE { ?s ex:r ?o } WHERE { ?s ex:r ?o } ;\n\
                    DELETE WHERE { ?s ex:p ?o } ;\n\
                    CLEAR ALL";
        let split = parser
            .parse_update_split(text, &options)
            .expect("the request parses");
        assert_eq!(split.operations.len(), split.update.operations.len());
        assert_eq!(split.operations.len(), 6);
        assert_eq!(split.operations[0], UpdateDatasetSlot::NoWhereClause);
        let UpdateDatasetSlot::Modify { with_at, using_at } = &split.operations[1] else {
            panic!("INSERT … WHERE is a modify: {:?}", split.operations[1]);
        };
        assert_eq!(*with_at, None);
        assert!(using_at.is_empty());
        assert!(text[using_at.start..].starts_with("WHERE { ?s ex:p ?o } ;"));
        let UpdateDatasetSlot::Modify { with_at, using_at } = &split.operations[2] else {
            panic!(
                "DELETE … INSERT … WHERE is a modify: {:?}",
                split.operations[2]
            );
        };
        assert_eq!(*with_at, None);
        assert_eq!(&text[using_at.clone()], "USING ex:g USING NAMED ex:n ");
        let UpdateDatasetSlot::Modify { with_at, using_at } = &split.operations[3] else {
            panic!("WITH … is a modify: {:?}", split.operations[3]);
        };
        assert_eq!(&text[with_at.clone().expect("WITH")], "WITH ex:g ");
        assert!(using_at.is_empty());
        let UpdateDatasetSlot::DeleteWhere {
            where_at,
            pattern_at,
        } = &split.operations[4]
        else {
            panic!(
                "DELETE WHERE is its own shorthand: {:?}",
                split.operations[4]
            );
        };
        assert!(text[*where_at..].starts_with("WHERE { ?s ex:p ?o }"));
        assert_eq!(&text[pattern_at.clone()], "{ ?s ex:p ?o }");
        assert_eq!(split.operations[5], UpdateDatasetSlot::NoWhereClause);

        // The long form the shorthand is defined as, written at the reported positions,
        // parses to the same operation with a USING clause added.
        let one = "DELETE WHERE { ?s <http://example.org/p> ?o }";
        let split = parser.parse_update_split(one, &options).expect("parses");
        let UpdateDatasetSlot::DeleteWhere {
            where_at,
            pattern_at,
        } = split.operations[0].clone()
        else {
            panic!("DELETE WHERE: {:?}", split.operations[0]);
        };
        let long = format!(
            "{}{}\nUSING <http://example.org/g> {}",
            &one[..where_at],
            &one[pattern_at],
            &one[where_at..]
        );
        let update = parser
            .parse_update_with(&long, &options)
            .expect("long form parses");
        let GraphUpdateOperation::DeleteInsert { delete, using, .. } = &update.operations[0] else {
            panic!("a modify: {:?}", update.operations[0]);
        };
        let GraphUpdateOperation::DeleteInsert {
            delete: short_delete,
            ..
        } = &split.update.operations[0]
        else {
            panic!("a modify: {:?}", split.update.operations[0]);
        };
        assert_eq!(
            delete, short_delete,
            "the template is the shorthand's pattern"
        );
        assert_eq!(using.len(), 1);
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
        let inner = inner.into_inner();
        let GraphPattern::Group {
            variables,
            aggregates,
            ..
        } = inner
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        assert_eq!(agg.scalarvals, [] as [_; 0]);
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
    /// [`crate::algebra::AggregateExpression::scalarvals`] with the upper-cased name and the
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        let inner = inner.into_inner();
        let GraphPattern::Join { right, .. } = inner else {
            panic!("expected the trailing VALUES joined in, got {inner:?}");
        };
        let right = right.into_inner();
        let GraphPattern::Values { bindings, .. } = right else {
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
            panic!("expected Group under Extend");
        };
        assert_eq!(aggregates.len(), 1);
        assert_eq!(aggregates[0].1.args, [] as [_; 0]);
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
        let GraphPattern::Group { aggregates, .. } = inner.into_inner() else {
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

    // ── a blank node label belongs to one basic graph pattern ────────────────
    //
    // Every refusal below is paired with the neighbouring query that must still
    // parse: the same label kept inside ONE basic graph pattern.

    /// The syntax error a query reusing `_:a` across basic graph patterns gets.
    fn blank_scope_err(q: &str) -> ParseError {
        let err = try_parse(q).expect_err("a label shared by two basic graph patterns");
        assert!(
            matches!(&err, ParseError::Syntax { reason, .. }
                if reason.contains("_:a") && reason.contains("two different basic graph patterns")),
            "expected the blank-label scope error for {q:?}, got {err:?}"
        );
        err
    }

    #[test]
    fn a_blank_label_reused_across_basic_graph_patterns_is_a_syntax_error() {
        // The W3C `syntax-sparql4` cases, verbatim in shape.
        for q in [
            // syn-bad-34: into a nested group.
            "SELECT * WHERE { _:a ?p ?v . { _:a ?q 1 } }",
            // syn-bad-35 / -37: out of a nested group.
            "SELECT * WHERE { { _:a ?p ?v . } _:a ?q 1 }",
            // syn-bad-36: across UNION arms.
            "SELECT * WHERE { { _:a ?p ?v . } UNION { _:a ?q 1 } }",
            // syn-bad-38: into an OPTIONAL.
            "SELECT * WHERE { _:a ?p ?v . OPTIONAL { _:a ?q 1 } }",
            // syn-bad-OPT-breaks-BGP / -UNION- / -GRAPH-: the element between
            // two triples blocks of ONE group ends the first basic graph pattern.
            "SELECT * WHERE { _:a ?p ?v . OPTIONAL { ?s ?p ?v } _:a ?q 1 }",
            "SELECT * WHERE { _:a ?p ?v1 { ?s <http://example.org/p1> ?o } UNION { ?s <http://example.org/p2> ?o } _:a ?p ?v2 }",
            "SELECT * WHERE { _:a ?p ?v . GRAPH ?g { ?s ?p ?v } _:a ?q 1 }",
            // The other elements that end one, by the same rule.
            "SELECT * WHERE { _:a ?p ?v . BIND(1 AS ?x) _:a ?q ?y }",
            "SELECT * WHERE { _:a ?p ?v . VALUES ?x { 1 } _:a ?q ?y }",
            "SELECT * WHERE { _:a ?p ?v . MINUS { ?s ?p ?v } _:a ?q ?y }",
            "SELECT * WHERE { _:a ?p ?v . MINUS { _:a ?q ?y } }",
            "SELECT * WHERE { _:a ?p ?v . FILTER EXISTS { _:a ?q ?y } }",
            "SELECT * WHERE { _:a ?p ?v . { SELECT ?y WHERE { _:a ?q ?y } } }",
            // A reifier id is a label like any other.
            "SELECT * WHERE { ?s ?p ?o ~ _:a . OPTIONAL { _:a ?q ?y } }",
        ] {
            blank_scope_err(q);
        }
    }

    #[test]
    fn a_blank_label_kept_inside_one_basic_graph_pattern_still_parses() {
        for q in [
            "SELECT * WHERE { _:a ?p ?v . _:a ?q 1 }",
            "SELECT * WHERE { _:a ?p ?v ; ?q _:a }",
            // A FILTER does not end a basic graph pattern.
            "SELECT * WHERE { _:a ?p ?v . FILTER(true) _:a ?q 1 }",
            "SELECT * WHERE { _:a ?p ?v FILTER(true) . FILTER(false) _:a ?q 1 }",
            // A property path is part of the block it is written in.
            "SELECT * WHERE { ?s ?p _:a . _:a <http://example.org/q>+ ?o }",
            // Inside a quoted triple and a blank-node property list.
            "SELECT * WHERE { _:a ?p <<( _:a ?q ?o )>> . [ ?r _:a ] }",
            // One nested group reusing ITS OWN label, and a sibling with another.
            "SELECT * WHERE { { _:a ?p ?v . _:a ?q 1 } OPTIONAL { _:b ?p ?v . _:b ?q 1 } }",
            // A CONSTRUCT template is not a basic graph pattern of the query.
            "CONSTRUCT { _:a <http://example.org/p> ?v } WHERE { _:a ?p ?v . _:a ?q 1 }",
        ] {
            try_parse(q).unwrap_or_else(|e| panic!("{q:?} is one basic graph pattern: {e:?}"));
        }
        // A property-function call is part of its block's basic graph pattern.
        try_parse_with_prop_fn(
            "SELECT * WHERE { ?s <http://example.org/p> _:a . ( ?s ) <https://example.org/pf/solve> ( \"x\" _:a ) }",
        )
        .expect("a label shared by a call and a triple of one block");
        let err = try_parse_with_prop_fn(
            "SELECT * WHERE { ?s <http://example.org/p> _:a . OPTIONAL { ( ?s ) <https://example.org/pf/solve> ( _:a ) } }",
        )
        .expect_err("a call in an OPTIONAL is another basic graph pattern");
        assert!(matches!(err, ParseError::Syntax { .. }), "{err:?}");
    }

    #[test]
    fn an_anonymous_blank_never_takes_a_label_the_author_wrote() {
        // `[]` mints a label; the author may write any label at all, including the
        // one the parser would otherwise mint first. The two must stay two nodes.
        let q = try_parse(
            "SELECT * WHERE { [] <http://example.org/p> ?o . \
             _:__purrdf_anon_0 <http://example.org/q> ?x }",
        )
        .expect("both are legal blank nodes");
        let Query::Select { pattern, .. } = q else {
            panic!("a SELECT");
        };
        let GraphPattern::Project { inner, .. } = pattern else {
            panic!("a projection");
        };
        let inner = inner.into_inner();
        let GraphPattern::Bgp { patterns } = inner else {
            panic!("one basic graph pattern, got {inner:?}");
        };
        let (TermPattern::BlankNode(anon), TermPattern::BlankNode(written)) =
            (&patterns[0].subject, &patterns[1].subject)
        else {
            panic!("two blank subjects, got {patterns:?}");
        };
        assert_eq!(written.as_str(), "__purrdf_anon_0");
        assert_ne!(anon, written, "the minted label avoided the written one");
    }

    #[test]
    fn each_update_operation_scopes_its_own_where_labels() {
        // Two operations' WHERE clauses are separate patterns.
        parse_update(
            "DELETE { ?s purrdf:p ?o } WHERE { ?s purrdf:p ?o . _:a purrdf:q ?s } ; \
             DELETE { ?s purrdf:p ?o } WHERE { ?s purrdf:p ?o . _:a purrdf:q ?s }",
        );
        // Within one operation the query rule holds.
        let err = update_err(
            "DELETE { ?s purrdf:p ?o } WHERE { ?s purrdf:p _:a . OPTIONAL { _:a purrdf:q ?o } }",
        );
        assert!(matches!(err, ParseError::Syntax { .. }), "{err:?}");
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
        // the `TermPattern::Triple` descent in `collect_triple_bnode_labels`.
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
        assert_eq!(insert.as_slice(), []);
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
        assert_eq!(using.as_slice(), []);
    }

    #[test]
    fn update_insert_only_modify() {
        let u = parse_update("INSERT { ?s purrdf:q purrdf:o } WHERE { ?s a purrdf:T }");
        let GraphUpdateOperation::DeleteInsert { delete, insert, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(delete.as_slice(), []);
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

    /// The prologue offset is the start of the query form, so the two halves are
    /// the caller's own bytes and the algebra is the one `parse_query_with` returns.
    ///
    /// Both neighbours are asserted: a text with directives splits after the last
    /// one, and a text without them splits at zero — which is what makes the offset
    /// a position rather than a guess about where `SELECT` tends to be written.
    #[test]
    fn a_prologue_splits_at_the_query_form_and_an_absent_one_splits_at_zero() {
        let options = ParserOptions::default();
        let parser = SparqlParser::new();
        let prefixed = "BASE <http://example.org/>\nPREFIX ex: <http://example.org/ns#>\n# and a comment\nSELECT ?s WHERE { ?s ex:p ?o }";
        let split = parser
            .parse_query_split(prefixed, &options)
            .expect("a prefixed query parses");
        assert_eq!(
            &prefixed[split.body_at..],
            "SELECT ?s WHERE { ?s ex:p ?o }",
            "the offset is the start of the query form, comments and all"
        );
        assert_eq!(
            split.query,
            parser
                .parse_query_with(prefixed, &options)
                .expect("the same text parses through the plain entry"),
            "and the algebra beside the offset is the ordinary one"
        );
        assert_eq!(
            split.dataset_at, None,
            "this text writes no dataset clause, and an absence is reported as one"
        );

        let bare = "SELECT ?s WHERE { ?s <http://example.org/ns#p> ?o }";
        let at = parser
            .parse_query_split(bare, &options)
            .expect("a query with no prologue parses")
            .body_at;
        assert_eq!(at, 0, "no prologue is a split at the front of the text");

        assert!(
            parser
                .parse_query_split("PREFIX ex: <urn:x#>", &options)
                .is_err(),
            "a prologue with no query form is still not a query"
        );
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
        assert_eq!(u.operations, [] as [_; 0]);
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

    /// The subject IRI the parser resolved the first `INSERT DATA` quad to —
    /// the shortest route from "a prologue plus one IRIREF" to "the IRI that
    /// reached the algebra".
    fn inserted_subject(request: &str) -> String {
        let update = SparqlParser::new()
            .parse_update(request)
            .unwrap_or_else(|e| panic!("{request:?} must parse: {e}"));
        let GraphUpdateOperation::InsertData { data } = &update.operations[0] else {
            panic!("expected InsertData");
        };
        match &data[0].triple.subject {
            TermPattern::NamedNode(n) => n.as_str().to_owned(),
            other => panic!("expected a named-node subject, got {other:?}"),
        }
    }

    /// A relative `BASE` resolves against the base already in force, so a chain
    /// of directives composes left to right instead of the later one being taken
    /// verbatim.
    ///
    /// This is NOT Turtle §6.1's permission carried across by analogy. SPARQL
    /// states no relative-`BASE` rule of its own; it delegates, and the clause it
    /// delegates to settles the question:
    ///
    /// > **SPARQL 1.2 Query §4.1.1.2, "Relative IRIs"** (word-for-word SPARQL 1.1
    /// > §4.1.1.1): "The `BASE` keyword defines the Base IRI used to resolve
    /// > relative IRIs per \[RFC3986\] section 5.1.1, 'Base URI Embedded in
    /// > Content'."
    ///
    /// > **RFC 3986 §5.1, "Establishing a Base URI"** — the section §5.1.1 is a
    /// > subsection of, so its requirements govern the delegated case: "A base
    /// > URI must conform to the `<absolute-URI>` syntax rule (Section 4.3). If
    /// > the base URI is obtained from a URI reference, then that reference must
    /// > be converted to absolute form and stripped of any fragment component
    /// > prior to its use as a base URI."
    ///
    /// `BaseDecl ::= 'BASE' IRIREF` takes the general IRI *reference* production,
    /// so a `BASE` operand is exactly "a base URI obtained from a URI reference":
    /// RFC 3986 requires it be **converted** to absolute form — §5.2 resolution
    /// against the base already established — not refused, and applies the
    /// `<absolute-URI>` requirement to the result of that conversion. Chaining is
    /// therefore mandated, not merely permitted.
    ///
    /// Corpus evidence, for completeness: the vendored W3C suites carry no
    /// `BASE <relative>` case at all — every `BASE` declaration under
    /// `crates/sparql-conformance/suite/w3c-sparql11` and `w3c-sparql12` is
    /// absolute — so no positive or negative syntax test bears on this and the
    /// clauses above are the whole authority. See
    /// `relative_base_directive_with_no_base_in_scope_is_an_error` for the other
    /// half of the same clause.
    #[test]
    fn base_directive_chains_against_the_base_in_force() {
        assert_eq!(
            inserted_subject(
                "BASE <http://example.org/a/> BASE <b/> \
                 INSERT DATA { <s> <http://example.org/p> <http://example.org/o> }"
            ),
            "http://example.org/a/b/s"
        );
    }

    /// The other half of RFC 3986 §5.1: "A base URI must conform to the
    /// `<absolute-URI>` syntax rule (Section 4.3). If the base URI is obtained
    /// from a URI reference, then that reference must be converted to absolute
    /// form … prior to its use as a base URI." With no base established there is
    /// nothing to convert against, so no `<absolute-URI>` can be produced and the
    /// directive is refused outright rather than stored as a base that would
    /// silently mis-resolve every reference after it. That the requirement binds
    /// the *converted* result — not the operand as written — is exactly why
    /// `base_directive_chains_against_the_base_in_force` is the correct reading
    /// of the same clause, and why this case is the only one that errors.
    #[test]
    fn relative_base_directive_with_no_base_in_scope_is_an_error() {
        let err = SparqlParser::new()
            .parse_update(
                "BASE <b/> INSERT DATA { <s> <http://example.org/p> <http://example.org/o> }",
            )
            .expect_err("a relative BASE with no base in scope has nothing to resolve against");
        assert!(matches!(err, ParseError::Iri { .. }), "got {err:?}");
        assert!(
            err.to_string().contains("iri-non-absolute-base"),
            "the shared diagnostic code must reach the SPARQL message: {err}"
        );
    }

    /// The RFC-3986 §5.2 same-document corners, driven through a SPARQL `BASE`:
    /// `<>` keeps the base's query and drops its fragment, `<#x>` replaces only
    /// the fragment, and `<?y=2>` replaces the query and drops the fragment.
    #[test]
    fn same_document_references_follow_rfc_3986_under_a_sparql_base() {
        for (reference, expected) in [
            ("<>", "http://example.org/d?q=1"),
            ("<#x>", "http://example.org/d?q=1#x"),
            ("<?y=2>", "http://example.org/d?y=2"),
        ] {
            let request = format!(
                "BASE <http://example.org/d?q=1#frag> \
                 INSERT DATA {{ {reference} <http://example.org/p> <http://example.org/o> }}"
            );
            assert_eq!(inserted_subject(&request), expected, "for {reference}");
        }
    }

    /// A network-path reference (RFC-3986 §4.2) replaces the authority but
    /// inherits the base's scheme.
    #[test]
    fn network_path_reference_inherits_the_base_scheme() {
        assert_eq!(
            inserted_subject(
                "BASE <http://example.org/a/b> \
                 INSERT DATA { <//example.org/x> <http://example.org/p> <http://example.org/o> }"
            ),
            "http://example.org/x"
        );
    }

    /// PurRDF never fabricates a base, so a relative reference with none in
    /// scope is a hard, typed failure carrying the shared diagnostic code.
    #[test]
    fn relative_iri_with_no_base_is_a_parse_error() {
        let err = SparqlParser::new()
            .parse_query("SELECT ?o WHERE { <cats> <http://example.org/p> ?o }")
            .expect_err("no base is in scope");
        assert!(matches!(err, ParseError::Iri { .. }), "got {err:?}");
        assert!(
            err.to_string().contains("iri-relative-no-base"),
            "the shared diagnostic code must reach the SPARQL message: {err}"
        );
    }

    /// A caller-supplied base that is not absolute cannot resolve anything, and
    /// `with_base_iri` returns `Self` — so the failure surfaces at the parse
    /// call rather than being silently dropped.
    #[test]
    fn a_non_absolute_caller_base_is_reported_at_parse_time() {
        let err = SparqlParser::new()
            .with_base_iri("not/absolute/")
            .parse_query("SELECT ?o WHERE { <cats> <http://example.org/p> ?o }")
            .expect_err("a scheme-less caller base is unusable");
        assert!(matches!(err, ParseError::Iri { .. }), "got {err:?}");
        assert!(err.to_string().contains("iri-non-absolute-base"), "{err}");
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
            GraphPattern::Extend { inner, .. } => inner.into_inner(),
            other => other,
        };
        match group {
            GraphPattern::Group {
                inner, variables, ..
            } => {
                assert_eq!(variables, vec![Variable::new("z")]);
                match inner.into_inner() {
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
            GraphPattern::Extend { inner, .. } => inner.into_inner(),
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

    /// A `GROUP BY (expr AS ?v)` target already in scope is refused exactly as a
    /// `BIND` or `SELECT`-list target is: in the `WHERE` clause, behind a nested
    /// group, in a sub-`SELECT`'s own `WHERE`, and as an earlier condition's target.
    /// The neighbours — the same conditions over a fresh target, the W3C
    /// `grouping/group04` and `aggregates/agg-group-builtin` shapes, a plain key over
    /// the in-scope variable, and a target that a sub-`SELECT` below hides — parse,
    /// and lower to the `Extend` beneath the `Group`.
    #[test]
    fn group_by_target_already_in_scope_is_rejected() {
        for query in [
            "SELECT ?c WHERE { ?c purrdf:p ?o } GROUP BY (purrdf:x AS ?c)",
            "SELECT ?c WHERE { ?c purrdf:p ?o } GROUP BY (?o AS ?c)",
            "SELECT ?c WHERE { { ?c purrdf:p ?o } } GROUP BY (COALESCE(?nope, 1) AS ?c)",
            "SELECT ?k WHERE { ?c purrdf:p ?o } GROUP BY (?o AS ?k) (?c AS ?k)",
            "SELECT ?c WHERE { { SELECT ?c WHERE { ?c purrdf:p ?o } GROUP BY (purrdf:x AS ?c) } }",
        ] {
            let q = format!("{GM}{query}");
            let err = SparqlParser::new()
                .parse_query(&q)
                .expect_err("a GROUP BY target over an in-scope variable must fail");
            assert!(
                matches!(&err, ParseError::Syntax { reason, .. }
                    if reason.contains("GROUP BY target ?") && reason.contains("already in scope")),
                "expected the GROUP BY scope refusal for {query:?}, got {err:?}"
            );
        }
        for query in [
            "SELECT ?k WHERE { ?c purrdf:p ?o } GROUP BY (purrdf:x AS ?k)",
            "SELECT ?X (SAMPLE(?v) AS ?S) { ?s purrdf:p ?v OPTIONAL { ?s purrdf:q ?w } } \
             GROUP BY (COALESCE(?w, 1) AS ?X)",
            "SELECT ?d (COUNT(*) AS ?n) WHERE { ?s ?p ?o } GROUP BY (DATATYPE(?o) AS ?d)",
            "SELECT ?c WHERE { ?c purrdf:p ?o } GROUP BY ?c",
            "SELECT ?k ?j WHERE { ?c purrdf:p ?o } GROUP BY (?o AS ?k) (?c AS ?j)",
            "SELECT ?h WHERE { { SELECT ?o WHERE { ?h purrdf:p ?o } } } GROUP BY (?o AS ?h)",
        ] {
            let q = format!("{GM}{query}");
            let parsed = SparqlParser::new()
                .parse_query(&q)
                .unwrap_or_else(|err| panic!("a fresh GROUP BY target parses: {query:?}: {err}"));
            let Query::Select { pattern, .. } = parsed else {
                panic!("a SELECT: {query:?}");
            };
            assert!(
                format!("{pattern:?}").contains("Group {"),
                "the condition lowers beneath a Group: {query:?}"
            );
        }
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
            p.parse_prologue().expect("prologue");
            p.parse_query_form().expect("parse");
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
        let left = left.into_inner();
        let GraphPattern::Bgp { patterns: lp } = left else {
            panic!("expected the left to be the preceding BGP");
        };
        assert_eq!(lp.len(), 1);
        assert_eq!(lp[0].subject, TermPattern::Variable(Variable::new("s")));
        assert_eq!(lp[0].object, TermPattern::Variable(Variable::new("o")));
        let right = right.into_inner();
        let GraphPattern::Bgp { patterns: rp } = right else {
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
        let right = right.into_inner();
        let GraphPattern::Bgp { patterns } = right else {
            panic!("expected the outermost right to be `?c purrdf:r ?d`");
        };
        assert_eq!(
            patterns[0].subject,
            TermPattern::Variable(Variable::new("c"))
        );
        let left = left.into_inner();
        let GraphPattern::Lateral {
            left: inner_left,
            right: inner_right,
        } = left
        else {
            panic!("expected the outermost left to itself be a Lateral");
        };
        assert!(matches!(*inner_left, GraphPattern::Bgp { .. }));
        let GraphPattern::Bgp { patterns: irp } = inner_right.into_inner() else {
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
    fn exists_scope_frames_share_the_enclosing_variables_and_discard_their_own() {
        let var = |name: &str| Variable::new(name);
        let mut scopes = ExistsScopes::default();
        scopes.push_boundary();
        scopes.note(&var("a"));
        scopes.push_isolated();
        assert_eq!(
            scopes.top(),
            [var("a")],
            "an isolated frame sees the one beneath"
        );
        scopes.note(&var("a"));
        scopes.note(&var("b"));
        assert_eq!(
            scopes.top(),
            [var("a"), var("b")],
            "a seen variable is not noted twice"
        );
        scopes.push_boundary();
        assert_eq!(scopes.top(), [], "a boundary frame sees nothing beneath it");
        scopes.note(&var("a"));
        assert_eq!(
            scopes.top(),
            [var("a")],
            "a boundary frame notes what it alone sees"
        );
        scopes.pop();
        scopes.pop();
        assert_eq!(
            scopes.top(),
            [var("a")],
            "an isolated frame's own variables are discarded"
        );
        scopes.note(&var("b"));
        assert_eq!(
            scopes.top(),
            [var("a"), var("b")],
            "a discarded variable is noted again"
        );
        scopes.pop();
        assert_eq!(scopes.top(), []);
        assert_eq!(scopes.trail, []);
        assert!(
            scopes.positions.is_empty(),
            "every position left the trail with its variable"
        );
    }

    #[test]
    fn deeply_nested_not_exists_parses_and_still_checks_its_scope() {
        let nested = |depth: usize, innermost: &str| {
            let mut body = innermost.to_owned();
            for k in (1..=depth).rev() {
                body = format!(
                    "FILTER NOT EXISTS {{ ?x{k} <https://example.org/next> ?x{} {body} }}",
                    k + 1
                );
            }
            format!("SELECT ?x0 WHERE {{ ?x0 <https://example.org/next> ?x1 {body} }}")
        };
        SparqlParser::new()
            .parse_query(&nested(20_000, ""))
            .expect("20 000 nested FILTER NOT EXISTS parse");
        // At the bottom of 2 000 levels, rebinding the outermost row's variable is refused,
        // and binding a fresh one is its valid neighbour.
        let err = SparqlParser::new()
            .parse_query(&nested(2_000, "BIND(1 AS ?x0)"))
            .expect_err("a BIND of a variable every enclosing row binds must fail");
        assert!(
            err.to_string()
                .contains("BIND target ?x0 inside EXISTS is already in scope"),
            "unexpected message: {err}"
        );
        SparqlParser::new()
            .parse_query(&nested(2_000, "BIND(1 AS ?fresh)"))
            .expect("a BIND of a fresh variable parses");
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
        assert_eq!(
            ParserOptions::default().extension_fn_namespaces,
            [] as [String; 0]
        );
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
            matches!(
                bound_expr(sub),
                Expression::Arithmetic(_, steps)
                    if matches!(steps.as_slice(), [(ArithmeticOperator::Subtract, _)])
            ),
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
        assert!(
            ParserOptions::default().extension_fn_namespaces.is_empty(),
            "the premise of this test is that NO extension namespace is configured, \
             so CDT dispatch cannot be riding the extension seam; got {:?}",
            ParserOptions::default().extension_fn_namespaces
        );
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
        let Expression::Literal(literal) = left.into_inner() else {
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
                | GraphPattern::Minus { left, right } => {
                    walk(left, out);
                    walk(right, out);
                }
                GraphPattern::Union { arms } => {
                    for arm in arms {
                        walk(arm, out);
                    }
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
        assert_eq!(
            ParserOptions::default().property_fn_namespaces,
            [] as [String; 0]
        );
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
        assert_eq!(
            pf_calls(&pf_pattern_off(q)),
            [] as [&PropertyFunctionCall; 0]
        );
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
                left: Child::new(GraphPattern::Bgp { patterns: vec![] }),
                right: Child::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
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
        assert_eq!(call.subject_args, [] as [_; 0]);
        assert_eq!(call.object_args, [] as [_; 0]);
    }

    #[test]
    fn bare_rdf_nil_is_a_one_element_argument_vector() {
        // … whereas an explicitly-spelled rdf:nil IRI is a one-element vector
        // holding that IRI — the two spellings are deliberately distinct.
        let p = pf_pattern(&format!("SELECT * WHERE {{ <{RDF_NIL}> pf:solve ?o }}"));
        let call = pf_only_call(&p);
        assert_eq!(call.subject_args, vec![pf_iri(RDF_NIL)]);
        let p2 = pf_pattern("SELECT * WHERE { () pf:solve ?o }");
        assert_eq!(pf_only_call(&p2).subject_args, [] as [_; 0]);
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
        assert_eq!(pf_calls(&p), [] as [&PropertyFunctionCall; 0]);
        assert_eq!(pf_triples(&p).len(), 1);
    }

    #[test]
    fn property_path_predicate_is_never_a_property_function() {
        for (q, retained) in [
            ("SELECT * WHERE { ?s pf:related+ ?o }", true),
            ("SELECT * WHERE { ?s pf:related/ex:p ?o }", false),
            ("SELECT * WHERE { ?s ^pf:related ?o }", false),
            ("SELECT * WHERE { ?s !(pf:related) ?o }", true),
        ] {
            let p = pf_pattern(q);
            assert!(pf_calls(&p).is_empty(), "`{q}` must stay data matching");
            assert_eq!(
                matches!(p, GraphPattern::Path { .. }),
                retained,
                "`{q}` → {p:?}"
            );
            assert_eq!(p, pf_pattern_off(q), "recognition does not change a path");
        }
        // …so a collection subject in front of one is still an ordinary
        // collection, cons cells and all — identical to the seam-off parse.
        let q = "SELECT * WHERE { ( ?a ?b ) pf:related+ ?o }";
        assert_eq!(pf_pattern(q), pf_pattern_off(q));
        assert_eq!(pf_calls(&pf_pattern(q)), [] as [&PropertyFunctionCall; 0]);
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
        assert_eq!(pf_calls(&p), [] as [&PropertyFunctionCall; 0]);
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

    // ── `LANGTAG` is decided by the workspace's owner, not by this parser ─────────

    /// A query in a `VALUES` clause built from the tag under test, which is the
    /// shortest path from a `LANGTAG` token to a bound term.
    fn query_with_tag(tag: &str) -> String {
        format!("SELECT ?v WHERE {{ VALUES ?v {{ \"a\"@{tag} }} }}")
    }

    /// The refusal side: a tag this parser used to bind and every RDF codec in
    /// the workspace refuses.
    ///
    /// `@cantbethislong` is a bare fourteen-character subtag. The N-Triples
    /// negative-syntax corpus requires it to be refused, and it was — from
    /// N-Triples. From SPARQL it bound, because this parser carried its own
    /// transcription of the `LANGTAG` terminal with no length bound in it. One
    /// binary, two acceptance languages, and a `VALUES` row that could never be
    /// written to a file.
    ///
    /// The diagnostic is asserted as well as the refusal: the reason now names
    /// the production that bit (`langtag-subtag-length-over-eight`) rather than
    /// restating the terminal, which is what makes the two surfaces' refusals
    /// recognisably the same refusal.
    #[test]
    fn a_subtag_over_the_length_ceiling_is_refused_in_a_query() {
        let error = try_parse(&query_with_tag("cantbethislong"))
            .expect_err("a bare fourteen-character subtag is over the §2.1 ceiling");
        assert!(
            matches!(error, ParseError::Syntax { .. }),
            "the error type and its byte offset are unchanged: {error:?}"
        );
        let message = error.to_string();
        for expected in [
            "langtag-subtag-length-over-eight",
            "language-tag subtag longer than 8 characters",
        ] {
            assert!(
                message.contains(expected),
                "the typed reason must reach the user, missing {expected:?}: {message}"
            );
        }
    }

    /// The acceptance side, which is the half that catches an over-refusal.
    ///
    /// The first row is the neighbour of the refusal above — one character
    /// inside the ceiling — and the second is the same over-long subtag moved
    /// behind the `x` private-use marker, where the ceiling does not apply. The
    /// rest are the tags the workspace's own fixtures, the approved W3C corpora
    /// and downstream projects publish: `en-fr-jura` and `fr-be-fbcl` have no
    /// RFC 5646 reading at all and must still bind, and the `x-…-<language
    /// name>` families run past the eight-character private-use cap by design.
    #[test]
    fn every_tag_the_workspace_writes_still_binds_in_a_query() {
        for tag in [
            "abcdefgh",
            "en-x-cantbethislong",
            "en",
            "en-US",
            "zh-Hans-CN",
            "i-enochian",
            "de-CH-x-phonebk",
            "en-fr-jura",
            "fr-be-fbcl",
            "x-purrdf-english",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "x-gmeow-norwegiannynorsk",
        ] {
            assert!(
                try_parse(&query_with_tag(tag)).is_ok(),
                "`@{tag}` is written by this workspace and must still parse"
            );
        }
    }

    /// The terminal's own refusals, which the swap must not have loosened, and
    /// which now arrive under the owner's stable codes instead of this parser's
    /// private prose — so a consumer can tell `@9-9` from `@en-` without
    /// matching on a sentence.
    #[test]
    fn the_langtag_terminals_own_refusals_still_bite_in_a_query() {
        for (tag, code) in [
            ("9-9", "langtag-terminal-primary-not-alpha"),
            ("123-456", "langtag-terminal-primary-not-alpha"),
            ("en-", "langtag-subtag-length-zero"),
        ] {
            let error = try_parse(&query_with_tag(tag))
                .expect_err("the `LANGTAG` terminal does not admit this")
                .to_string();
            assert!(
                error.contains(code),
                "`@{tag}` must refuse under {code}, got {error}"
            );
        }
    }
}
