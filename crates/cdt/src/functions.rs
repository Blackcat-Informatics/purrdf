// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 **function library**: the closed registry [`CdtFn`] and the pure
//! value-space operation behind each of its members.
//!
//! # Where the semantics come from
//!
//! SEP-0009 ships a conformance corpus, and this module is written against it rather
//! than against a reading of the prose. The corpus lives in `vectors/sparql-cdt`,
//! under `list-functions/` and `map-functions/`, and every rule below names the test
//! that pins it. Where the corpus is silent the rustdoc says so in as many words,
//! and the choice made in its absence is argued rather than assumed.
//!
//! # These IRIs are not minted
//!
//! Every function IRI here is a fixed, third-party, spec-defined string under
//! [`CDT_NS`](crate::CDT_NS), exactly as [`CDT_LIST`](crate::CDT_LIST) and the `xsd:`
//! IRIs are. The namespace is deliberately **not** configurable: a CDT function
//! library reachable at some other IRI is a different, non-conformant language, not a
//! deployment of this one.
//!
//! Two of the fifteen — `cdt:List` and `cdt:Map` — share their local name with the
//! two *datatype* IRIs. That collision is the spec's, and it is harmless: a datatype
//! IRI is resolved by [`CdtDatatype::from_iri`](crate::CdtDatatype::from_iri) in
//! datatype position, and [`CdtFn::from_iri`] is consulted only in **call** position.
//!
//! # One IRI, one variant — overloading is on the argument, not on the name
//!
//! `cdt:get` and `cdt:size` each apply to both composite datatypes with different
//! argument shapes: `cdt:get(list, index)` and `cdt:get(map, key)`;
//! `cdt:size(list)` and `cdt:size(map)`. They are therefore **one** [`CdtFn`]
//! variant each, dispatched on the runtime datatype of the first argument by [`get`]
//! and [`size`]. Modelling them as two variants would make a parser choose between
//! them before it can know which one it is looking at.
//!
//! # Three outcomes, never two
//!
//! Every fallible operation returns [`CdtOutcome`], which separates a value from a
//! SPARQL *expression error* from a *bound* failure. Collapsing the last two would
//! turn a refusal to allocate a gigabyte into an ordinary unbound variable, and a
//! query would then silently return fewer rows instead of failing. See
//! [`CdtOutcome`] for what a consumer must do with each.
//!
//! # Minting is bounded, and bounded *before* it allocates
//!
//! Six of the fifteen functions mint a composite that never passed through the
//! lexical scanner, so the scanner's bounds never saw it: `cdt:List`, `cdt:Map`,
//! `cdt:concat`, `cdt:merge`, `cdt:put` and `cdt:subseq`. Each of those computes the
//! element count, the nesting depth and the exact canonical byte length of the
//! result **from borrowed inputs**, checks all three, and only then clones anything.
//! `cdt:put(?m, ?k, ?m)` doubles a map's element count on every application, so
//! without that a query of twenty-one lines could ask for a value no host can hold;
//! with it, the twenty-first application is a [`CdtOutcome::Bound`] that allocated
//! nothing.
//!
//! Every walk in this module is iterative over an explicit heap worklist, matching
//! the rest of the crate: a stack overflow in Rust is an `abort` that no caller can
//! catch, and these inputs are attacker-shaped.

use alloc::vec::Vec;

use purrdf_xsd::XsdValue;

use crate::error::{CdtError, CdtTypeError};
use crate::limits::{check_extent, list_extent, map_extent};
use crate::literal::LiteralValue;
use crate::ops::total_key_cmp;
use crate::term::{CdtEntry, CdtKey, CdtTerm};
use crate::value::{CdtContents, CdtValue};

// ── The registry ────────────────────────────────────────────────────────────────

/// `cdt:List` in call position — the list constructor.
const FN_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
/// `cdt:Map` in call position — the map constructor.
const FN_MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";
/// `cdt:concat`.
const FN_CONCAT: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/concat";
/// `cdt:contains`.
const FN_CONTAINS: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/contains";
/// `cdt:containsKey`.
const FN_CONTAINS_KEY: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/containsKey";
/// `cdt:get`.
const FN_GET: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/get";
/// `cdt:head`.
const FN_HEAD: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/head";
/// `cdt:keys`.
const FN_KEYS: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/keys";
/// `cdt:merge`.
const FN_MERGE: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/merge";
/// `cdt:put`.
const FN_PUT: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/put";
/// `cdt:remove`.
const FN_REMOVE: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/remove";
/// `cdt:reverse`.
const FN_REVERSE: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/reverse";
/// `cdt:size`.
const FN_SIZE: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/size";
/// `cdt:subseq`.
const FN_SUBSEQ: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/subseq";
/// `cdt:tail`.
const FN_TAIL: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/tail";

/// How many arguments a [`CdtFn`] admits.
///
/// A parser needs this *before* it evaluates anything: SPARQL has no overloading on
/// argument count, so a call with the wrong number of arguments is a **static**
/// error in the query, not a runtime expression error, and must be rejected at parse
/// time rather than silently evaluated to unbound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CdtArity {
    /// Exactly this many arguments.
    Fixed(usize),
    /// At least `min` and at most `max` arguments, inclusive.
    Range {
        /// The smallest admissible argument count.
        min: usize,
        /// The largest admissible argument count.
        max: usize,
    },
    /// This many arguments or more, with no upper bound.
    AtLeast(usize),
    /// An even number of arguments — alternating keys and values. Zero is admitted
    /// (it builds the empty map).
    Pairs,
}

impl CdtArity {
    /// Whether a call with `argc` arguments has an admissible shape.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtArity, CdtFn};
    ///
    /// // `cdt:subseq(list, start)` and `cdt:subseq(list, start, length)` both parse.
    /// assert!(CdtFn::Subseq.arity().admits(2));
    /// assert!(CdtFn::Subseq.arity().admits(3));
    /// assert!(!CdtFn::Subseq.arity().admits(4));
    /// // `cdt:Map` takes key/value pairs, so an odd count is a static error.
    /// assert!(CdtFn::MapConstructor.arity().admits(4));
    /// assert!(!CdtFn::MapConstructor.arity().admits(3));
    /// assert!(CdtArity::Fixed(1).admits(1));
    /// ```
    #[must_use]
    pub const fn admits(self, argc: usize) -> bool {
        match self {
            Self::Fixed(n) => argc == n,
            Self::Range { min, max } => min <= argc && argc <= max,
            Self::AtLeast(min) => argc >= min,
            Self::Pairs => argc.is_multiple_of(2),
        }
    }
}

/// The closed set of SEP-0009 functions.
///
/// Closed on purpose, exactly as [`CdtDatatype`](crate::CdtDatatype) is: SEP-0009
/// defines these fifteen and does not grow at runtime, so there is no function
/// registry to configure and no way for a caller to shadow a spec function with its
/// own. An IRI outside the set is simply not a CDT function — [`CdtFn::from_iri`]
/// answers `None` and the consumer's ordinary "unknown function" path takes over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CdtFn {
    /// `cdt:List(…)` — build a list from the argument terms.
    ListConstructor,
    /// `cdt:concat(…)` — the concatenation of the argument lists.
    Concat,
    /// `cdt:contains(list, term)` — does the list hold a term equal to this one?
    Contains,
    /// `cdt:get(list, index)` / `cdt:get(map, key)` — one element, by position or by
    /// key.
    Get,
    /// `cdt:head(list)` — the first element.
    Head,
    /// `cdt:tail(list)` — everything but the first element.
    Tail,
    /// `cdt:reverse(list)` — the elements in the opposite order.
    Reverse,
    /// `cdt:size(list)` / `cdt:size(map)` — how many elements or entries.
    Size,
    /// `cdt:subseq(list, start[, length])` — a contiguous run of elements.
    Subseq,
    /// `cdt:Map(…)` — build a map from alternating key and value arguments.
    MapConstructor,
    /// `cdt:containsKey(map, key)` — is this key one of the map's keys?
    ContainsKey,
    /// `cdt:keys(map)` — the map's keys, as a list.
    Keys,
    /// `cdt:merge(…)` — the union of the argument maps.
    Merge,
    /// `cdt:put(map, key[, value])` — the map with one entry set.
    Put,
    /// `cdt:remove(map, key)` — the map without one entry.
    Remove,
}

/// Every [`CdtFn`], in declaration order.
///
/// Exists so a consumer can *enumerate* the library — to register the functions, to
/// document them, or to assert in its own tests that it handles all of them — rather
/// than transcribing a list that then drifts.
pub const CDT_FUNCTIONS: [CdtFn; 15] = [
    CdtFn::ListConstructor,
    CdtFn::Concat,
    CdtFn::Contains,
    CdtFn::Get,
    CdtFn::Head,
    CdtFn::Tail,
    CdtFn::Reverse,
    CdtFn::Size,
    CdtFn::Subseq,
    CdtFn::MapConstructor,
    CdtFn::ContainsKey,
    CdtFn::Keys,
    CdtFn::Merge,
    CdtFn::Put,
    CdtFn::Remove,
];

impl CdtFn {
    /// Resolve a function IRI, or `None` when the IRI names no CDT function.
    ///
    /// Consult this in **call position only**. `cdt:List` and `cdt:Map` are also the
    /// two datatype IRIs; in datatype position
    /// [`CdtDatatype::from_iri`](crate::CdtDatatype::from_iri) is the right resolver
    /// and this one would be a category error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::CdtFn;
    ///
    /// assert_eq!(
    ///     CdtFn::from_iri("http://w3id.org/awslabs/neptune/SPARQL-CDTs/subseq"),
    ///     Some(CdtFn::Subseq)
    /// );
    /// // The constructor shares its IRI with the datatype; in call position it is
    /// // the constructor.
    /// assert_eq!(CdtFn::from_iri(purrdf_cdt::CDT_LIST), Some(CdtFn::ListConstructor));
    /// assert_eq!(CdtFn::from_iri("http://example.org/get"), None);
    /// ```
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        match iri {
            FN_LIST => Some(Self::ListConstructor),
            FN_MAP => Some(Self::MapConstructor),
            FN_CONCAT => Some(Self::Concat),
            FN_CONTAINS => Some(Self::Contains),
            FN_CONTAINS_KEY => Some(Self::ContainsKey),
            FN_GET => Some(Self::Get),
            FN_HEAD => Some(Self::Head),
            FN_KEYS => Some(Self::Keys),
            FN_MERGE => Some(Self::Merge),
            FN_PUT => Some(Self::Put),
            FN_REMOVE => Some(Self::Remove),
            FN_REVERSE => Some(Self::Reverse),
            FN_SIZE => Some(Self::Size),
            FN_SUBSEQ => Some(Self::Subseq),
            FN_TAIL => Some(Self::Tail),
            _ => None,
        }
    }

    /// This function's IRI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CDT_NS, CDT_FUNCTIONS, CdtFn};
    ///
    /// assert!(CdtFn::Keys.iri().starts_with(CDT_NS));
    /// // `iri` and `from_iri` are inverse on every member of the library.
    /// for function in CDT_FUNCTIONS {
    ///     assert_eq!(CdtFn::from_iri(function.iri()), Some(function));
    /// }
    /// ```
    #[must_use]
    pub const fn iri(self) -> &'static str {
        match self {
            Self::ListConstructor => FN_LIST,
            Self::MapConstructor => FN_MAP,
            Self::Concat => FN_CONCAT,
            Self::Contains => FN_CONTAINS,
            Self::ContainsKey => FN_CONTAINS_KEY,
            Self::Get => FN_GET,
            Self::Head => FN_HEAD,
            Self::Keys => FN_KEYS,
            Self::Merge => FN_MERGE,
            Self::Put => FN_PUT,
            Self::Remove => FN_REMOVE,
            Self::Reverse => FN_REVERSE,
            Self::Size => FN_SIZE,
            Self::Subseq => FN_SUBSEQ,
            Self::Tail => FN_TAIL,
        }
    }

    /// The local name this function is written with under the `cdt:` prefix.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::CdtFn;
    ///
    /// assert_eq!(CdtFn::ContainsKey.local_name(), "containsKey");
    /// // The two constructors are spelled with the datatypes' own local names.
    /// assert_eq!(CdtFn::ListConstructor.local_name(), "List");
    /// ```
    #[must_use]
    pub const fn local_name(self) -> &'static str {
        match self {
            Self::ListConstructor => "List",
            Self::MapConstructor => "Map",
            Self::Concat => "concat",
            Self::Contains => "contains",
            Self::ContainsKey => "containsKey",
            Self::Get => "get",
            Self::Head => "head",
            Self::Keys => "keys",
            Self::Merge => "merge",
            Self::Put => "put",
            Self::Remove => "remove",
            Self::Reverse => "reverse",
            Self::Size => "size",
            Self::Subseq => "subseq",
            Self::Tail => "tail",
        }
    }

    /// How many arguments this function admits.
    ///
    /// Corpus citations for the shapes that are not obvious:
    ///
    /// * `cdt:concat` is variadic **from zero**: `list-functions/concat-08.rq` calls
    ///   it with none and expects `[]`, `concat-09.rq` with one and `concat-10.rq`
    ///   with three.
    /// * `cdt:List` is variadic from zero too — `list-functions/list-constructor-01.rq`.
    /// * `cdt:subseq` takes two or three — `list-functions/subseq-03.rq` omits the
    ///   length, `subseq-02.rq` supplies it.
    /// * `cdt:put` takes two or three — `map-functions/put-03.rq` omits the value.
    /// * `cdt:Map` takes key/value **pairs**, so its count is even —
    ///   `map-functions/map-constructor-01.rq` (none) and `map-constructor-02.rq`
    ///   (two pairs).
    /// * `cdt:merge` is given exactly two arguments everywhere in the corpus
    ///   (`map-functions/merge-01.rq` … `merge-08.rq`); nothing there pins a third.
    ///   It is modelled as variadic from two because [`map_merge`] resolves conflicts
    ///   by taking the **first** map that carries a key, which is associative, so the
    ///   n-argument reading agrees with the corpus's two-argument one on every input
    ///   the corpus contains.
    #[must_use]
    pub const fn arity(self) -> CdtArity {
        match self {
            Self::ListConstructor | Self::Concat => CdtArity::AtLeast(0),
            Self::MapConstructor => CdtArity::Pairs,
            Self::Head | Self::Tail | Self::Reverse | Self::Size | Self::Keys => CdtArity::Fixed(1),
            Self::Contains | Self::ContainsKey | Self::Get | Self::Remove => CdtArity::Fixed(2),
            Self::Subseq | Self::Put => CdtArity::Range { min: 2, max: 3 },
            Self::Merge => CdtArity::AtLeast(2),
        }
    }
}

// ── The three outcomes ──────────────────────────────────────────────────────────

/// What evaluating a SEP-0009 function produced.
///
/// Three states, and a consumer must keep them apart:
///
/// * [`CdtOutcome::Value`] — the function has an answer.
/// * [`CdtOutcome::Error`] — a SPARQL **expression error**. The enclosing expression
///   becomes an error: inside `BIND` the variable stays unbound, inside `FILTER` the
///   solution is dropped. This is the outcome the corpus writes as
///   `FILTER(!BOUND(?x))`, and it is emphatically **not** "false".
/// * [`CdtOutcome::Bound`] — one of [`crate::MAX_NESTING_DEPTH`],
///   [`crate::MAX_ELEMENTS`] or [`crate::MAX_LEXICAL_BYTES`] would have been exceeded by the value the function was
///   asked to mint. This is a **hard failure of the query**, not an expression
///   error: degrading it to an unbound variable would let a hostile query silently
///   change a result set instead of being refused, so a consumer must propagate it
///   as a query failure.
///
/// The type is deliberately not `Result<Option<T>, _>` and offers no `ok()`: the two
/// failures answer different questions and there is no correct way to merge them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdtOutcome<T> {
    /// The function produced this value.
    Value(T),
    /// The function raised a SPARQL expression error.
    Error(CdtTypeError),
    /// The function refused to mint a value that would exceed a resource bound.
    Bound(CdtError),
}

impl<T> CdtOutcome<T> {
    /// The value, if this is [`CdtOutcome::Value`].
    ///
    /// Borrowing rather than consuming, so that reaching for it cannot be mistaken
    /// for a way to *discard* the two failure states.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Error(_) | Self::Bound(_) => None,
        }
    }

    /// Whether this is [`CdtOutcome::Value`].
    #[must_use]
    pub const fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Whether this is a SPARQL expression error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Whether this is a resource-bound refusal.
    #[must_use]
    pub const fn is_bound(&self) -> bool {
        matches!(self, Self::Bound(_))
    }

    /// Apply `f` to the value, leaving either failure exactly as it is.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::CdtOutcome;
    ///
    /// let outcome: CdtOutcome<usize> = CdtOutcome::Value(2);
    /// assert_eq!(outcome.map(|n| n * 3), CdtOutcome::Value(6));
    /// ```
    #[must_use]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> CdtOutcome<U> {
        match self {
            Self::Value(value) => CdtOutcome::Value(f(value)),
            Self::Error(error) => CdtOutcome::Error(error),
            Self::Bound(error) => CdtOutcome::Bound(error),
        }
    }
}

/// A SPARQL expression error with a fixed explanation.
fn raise<T>(reason: &'static str) -> CdtOutcome<T> {
    CdtOutcome::Error(CdtTypeError::undefined(reason))
}

// ── Argument coercion ───────────────────────────────────────────────────────────

/// The integer an index or length argument denotes, or `None` when it denotes none.
///
/// `cdt:get` on a list and both of `cdt:subseq`'s numeric arguments require an
/// integer, and the corpus is explicit that nothing else will do:
/// `list-functions/get-error-05.rq` passes `"invalid"` and `get-error-06.rq` passes
/// `2.0` — an `xsd:decimal`, whose value *is* an integer — and both must be errors.
/// `list-functions/subseq-error-02.rq` says the same for `cdt:subseq`.
///
/// **A choice the corpus does not pin.** The corpus exercises `xsd:integer` on the
/// accepting side and `xsd:string` / `xsd:decimal` on the rejecting side, and never
/// mentions the derived integer datatypes. This function accepts the whole
/// integer family — `xsd:long`, `xsd:int`, `xsd:nonNegativeInteger` and the rest —
/// because they are in the integer value space and SPARQL treats them as integers
/// everywhere else. That admits strictly more than the corpus tests without
/// contradicting any of it: `2.0` is still refused, because `xsd:decimal` is a
/// different value space, which is exactly the distinction `get-error-06.rq` draws.
///
/// A lexical form outside the range this workspace's integer parser models — an
/// index with forty digits, say — is `None`, which is also the right answer: such an
/// index is out of range for every list that can exist.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, integer_argument};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
///
/// let one = CdtTerm::Literal(CdtLiteral::typed("1", XSD_INTEGER));
/// assert_eq!(integer_argument(&one), Some(1));
/// // An `xsd:decimal` is not an integer, however it is spelled.
/// let two_point_oh = CdtTerm::Literal(CdtLiteral::typed("2.0", XSD_DECIMAL));
/// assert_eq!(integer_argument(&two_point_oh), None);
/// assert_eq!(integer_argument(&CdtTerm::Null), None);
/// ```
#[must_use]
pub fn integer_argument(term: &CdtTerm) -> Option<i128> {
    let CdtTerm::Literal(literal) = term else {
        return None;
    };
    if literal.language.is_some() {
        return None;
    }
    // Routed through the crate's single lexical-to-value choke point rather than
    // through `purrdf_xsd::parse_by_iri`, so the tri-state is never collapsed by
    // accident here either. `cdt:get`'s contract makes all three non-integer outcomes
    // one answer — `get-error-05.rq` (a string) and `get-error-06.rq` (an
    // `xsd:decimal`) both require the call to be unbound — so this function returns
    // `None` for an unmodelled datatype and for an ill-typed one alike. The
    // distinction is preserved where it is observable, which is the comparison
    // relations in `crate::ops`, not here.
    match crate::literal::parse_literal(&literal.lexical, &literal.datatype) {
        LiteralValue::Xsd(XsdValue::Integer { value, .. }) => Some(value),
        LiteralValue::Xsd(_)
        | LiteralValue::Cdt(_)
        | LiteralValue::IllTyped { .. }
        | LiteralValue::Opaque => None,
    }
}

// ── cdt:List and cdt:Map — the two constructors ─────────────────────────────────

/// `cdt:List(…)` — build a list from the argument terms, in argument order.
///
/// # Nulls are how an argument's failure is carried, not a failure of the call
///
/// An argument that is unbound, or whose own evaluation raised, becomes the SEP-0009
/// `null` element: `list-functions/list-constructor-null-01.rq` binds
/// `cdt:List(?unbound)` and requires the result to spell `[null]`, and
/// `list-constructor-null-02.rq` requires the same of `cdt:List(1/0)`, whose
/// argument is a division by zero. The constructor therefore never fails because of
/// an argument — the consumer maps each failed argument to [`CdtTerm::Null`] and
/// passes it in — which is the opposite of the ordinary SPARQL rule and is why it
/// has to be said out loud. `list-constructor-12.rq` pins the mixed case.
///
/// The only failure is [`CdtOutcome::Bound`].
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtOutcome, CdtTerm, CdtValue, list_constructor, parse_list};
///
/// // `cdt:List(?unbound)` is `[null]`, not an error.
/// let outcome = list_constructor(vec![CdtTerm::Null]);
/// assert_eq!(outcome.value().map(CdtValue::canonical_lexical).as_deref(), Some("[null]"));
///
/// let empty = list_constructor(Vec::new());
/// assert_eq!(empty, CdtOutcome::Value(parse_list("[]")?));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_constructor(items: Vec<CdtTerm>) -> CdtOutcome<CdtValue> {
    if let Err(error) = check_extent(&list_extent(items.iter())) {
        return CdtOutcome::Bound(error);
    }
    CdtOutcome::Value(CdtValue::from_checked_items(items))
}

/// `cdt:Map(…)` — build a map from alternating key and value arguments.
///
/// # Keys and values fail differently, and the corpus is explicit about both
///
/// * A **key** argument that cannot be a map key makes the whole pair vanish, with no
///   error: `map-functions/map-constructor-08.rq` passes an unbound key in the middle
///   of three pairs and expects `{1:2, 5:6}`, and `map-constructor-09.rq` does the
///   same with `BNODE()`, whose comment says in as many words that a blank node is
///   not a valid map key. Pass such a pair in with a key term that
///   [`CdtKey::from_term`] rejects — `null` for an unbound argument, or the blank
///   node itself — and it is dropped here.
/// * A **value** argument that failed becomes `null` and the entry stays:
///   `map-constructor-10.rq` requires `cdt:Map(1,2, 3,?unbound)` to have size 2, to
///   contain key 3, and for `cdt:get` on that key to be unbound.
/// * A key given **twice**, the last one wins: `map-constructor-03.rq` requires
///   `cdt:Map(1,2,1,4)` to be `{1:4}`, and `map-constructor-05.rq` repeats the point
///   with three pairs. Key identity is the *term*, so `1` and `"01"^^xsd:integer`
///   are two keys, not one — `map-constructor-04.rq`.
///
/// The only failure is [`CdtOutcome::Bound`].
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, CdtValue, map_constructor, parse_map};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// // The last binding of a repeated key wins.
/// let outcome = map_constructor(&[(int("1"), int("2")), (int("1"), int("4"))]);
/// assert_eq!(outcome.value(), Some(&parse_map("{1:4}")?));
///
/// // A blank node cannot be a key, so the pair is dropped rather than raising.
/// let dropped = map_constructor(&[(CdtTerm::Blank("b0".into()), int("4"))]);
/// assert_eq!(dropped.value(), Some(&parse_map("{}")?));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn map_constructor(pairs: &[(CdtTerm, CdtTerm)]) -> CdtOutcome<CdtValue> {
    // Index only the pairs whose key term can be a key at all, then sort by key with
    // the LATEST authoring position first, so `dedup_by` — which keeps the first of
    // each run — keeps the last binding.
    let mut keyed: Vec<(CdtKey, usize)> = pairs
        .iter()
        .enumerate()
        .filter_map(|(index, (key, _))| CdtKey::from_term(key).map(|key| (key, index)))
        .collect();
    keyed.sort_by(|a, b| total_key_cmp(&a.0, &b.0).then_with(|| b.1.cmp(&a.1)));
    keyed.dedup_by(|a, b| a.0 == b.0);

    let extent = map_extent(keyed.iter().map(|(key, index)| (key, &pairs[*index].1)));
    if let Err(error) = check_extent(&extent) {
        return CdtOutcome::Bound(error);
    }
    let entries = keyed
        .into_iter()
        .map(|(key, index)| CdtEntry {
            key,
            value: pairs[index].1.clone(),
        })
        .collect();
    CdtOutcome::Value(CdtValue::from_checked_entries(entries))
}

// ── The list functions ──────────────────────────────────────────────────────────

/// `cdt:size(list)` — how many elements the list has.
///
/// Total: every list has a size, and a `null` element counts like any other —
/// `list-functions/size-07.rq` requires `cdt:size("[null]")` to be 1 and
/// `size-12.rq` requires `cdt:size("[null, 2]")` to be 2. Nesting does not flatten:
/// `size-11.rq` requires `cdt:size("[[1], 2]")` to be 2.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtValue, list_size, parse_list};
///
/// let items = parse_list("[null, 2]")?.into_list().expect("a cdt:List");
/// assert_eq!(list_size(&items), 2);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn list_size(items: &[CdtTerm]) -> usize {
    items.len()
}

/// `cdt:get(list, index)` — the element at a **1-based** position.
///
/// # The index is 1-based
///
/// `list-functions/get-01.rq` requires `cdt:get("[1]", 1)` to be `1`, and
/// `get-error-03.rq` requires index `0` to be an error. `list-functions/subseq-01.rq`
/// says the same for `cdt:subseq`: `cdt:subseq("[1..10]", 1, 1)` is `[1]`.
///
/// # Every way of missing is a SPARQL error, not a type error the caller can catch
///
/// The corpus writes them all as `FILTER(!BOUND(?element))`, so they are one outcome
/// from a query's point of view, and this function returns
/// [`CdtOutcome::Error`] for each:
///
/// * an index past the end (`get-error-02.rq`), `0` (`get-error-03.rq`) or negative
///   (`get-error-04.rq`) — an out-of-range `cdt:get` is an **error**, never a `null`
///   and never an empty binding that later becomes something else;
/// * an index that is not an integer: `"invalid"` (`get-error-05.rq`) or `2.0`
///   (`get-error-06.rq`);
/// * a position that holds `null`. `get-null-01.rq` requires `cdt:get("[null]", 1)`
///   to be unbound. This is the heart of SEP-0009's null: it is a *position in the
///   value* that carries no term, so asking for the term raises, exactly as reading
///   an unbound variable does. `get-null-02.rq` shows the same unbound outcome for
///   the empty list, which is why the two cases are indistinguishable to a query.
///
/// A non-composite first argument is also an error (`get-error-01.rq`), but that is
/// the consumer's dispatch: this function is reached only with a list in hand. See
/// [`get`] for the dispatching form.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, CdtValue, list_get, parse_list};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// let items = parse_list("[1, null, 3]")?.into_list().expect("a cdt:List");
/// assert_eq!(list_get(&items, &int("1")).value(), Some(&int("1")));
/// // Position 2 holds a null, so there is no term to return.
/// assert!(list_get(&items, &int("2")).is_error());
/// // …and so is position 0, and position 4.
/// assert!(list_get(&items, &int("0")).is_error());
/// assert!(list_get(&items, &int("4")).is_error());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_get(items: &[CdtTerm], index: &CdtTerm) -> CdtOutcome<CdtTerm> {
    let Some(index) = integer_argument(index) else {
        return raise("cdt:get on a cdt:List needs an xsd:integer index");
    };
    if index < 1 {
        return raise("cdt:get on a cdt:List needs an index of at least 1");
    }
    let Ok(offset) = usize::try_from(index - 1) else {
        return raise("cdt:get on a cdt:List was given an index beyond any list");
    };
    let Some(item) = items.get(offset) else {
        return raise("cdt:get on a cdt:List was given an index past the end");
    };
    if item.is_null() {
        return raise("cdt:get on a cdt:List addressed a null element");
    }
    CdtOutcome::Value(item.clone())
}

/// `cdt:head(list)` — the first element.
///
/// Exactly `cdt:get(list, 1)`, including both of its ways of having no answer:
/// the empty list is an error (`list-functions/head-01.rq`,
/// `head-null-02.rq`) and so is a leading `null` (`head-07.rq`, `head-null-01.rq`,
/// and `head-10.rq` where a term does follow the null). A blank node **is** an
/// answer, and the same blank node each time — `head-11.rq` and `head-12.rq`.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtValue, list_head, parse_list};
///
/// let items = parse_list("[null, 2]")?.into_list().expect("a cdt:List");
/// assert!(list_head(&items).is_error());
/// let items = parse_list("[]")?.into_list().expect("a cdt:List");
/// assert!(list_head(&items).is_error());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_head(items: &[CdtTerm]) -> CdtOutcome<CdtTerm> {
    let Some(item) = items.first() else {
        return raise("cdt:head on an empty cdt:List");
    };
    if item.is_null() {
        return raise("cdt:head on a cdt:List whose first element is null");
    }
    CdtOutcome::Value(item.clone())
}

/// `cdt:tail(list)` — every element but the first, as a list.
///
/// The empty list is an error — `list-functions/tail-01.rq`. A leading `null` is
/// **not**: `tail-07.rq` requires `cdt:tail("[null]")` to be `[]` and `tail-10.rq`
/// requires `cdt:tail("[null, 2]")` to be `[2]`. That is the asymmetry worth
/// noticing — `cdt:head` raises on a leading null because it must produce that
/// element, while `cdt:tail` discards it and never looks at it.
///
/// A `null` anywhere after the first position is carried through untouched.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtOutcome, CdtValue, list_tail, parse_list};
///
/// let items = parse_list("[null, 2]")?.into_list().expect("a cdt:List");
/// assert_eq!(list_tail(&items), CdtOutcome::Value(parse_list("[2]")?));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_tail(items: &[CdtTerm]) -> CdtOutcome<CdtValue> {
    let Some((_, rest)) = items.split_first() else {
        return raise("cdt:tail on an empty cdt:List");
    };
    CdtOutcome::Value(CdtValue::from_checked_items(rest.to_vec()))
}

/// `cdt:reverse(list)` — the same elements in the opposite order.
///
/// Total: there is no list it cannot answer for, and it never raises
/// (`list-functions/reverse-01.rq` … `reverse-10.rq`). Nulls keep their identity as
/// positions and simply move — `reverse-07.rq` and `reverse-10.rq` compare the result
/// against a `cdt:List(…)` built from unbound arguments with `SAMETERM`, so the null
/// must survive the round trip *and* be spelled the same way the constructor spells
/// it.
///
/// No bound can be exceeded: the result holds exactly the input's elements, at
/// exactly the input's depth, and its canonical form is a permutation of the input's
/// element spellings, so it is the same length.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtValue, list_reverse, parse_list};
///
/// let items = parse_list("[null, 2]")?.into_list().expect("a cdt:List");
/// assert_eq!(list_reverse(&items), parse_list("[2, null]")?);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn list_reverse(items: &[CdtTerm]) -> CdtValue {
    let mut reversed = items.to_vec();
    reversed.reverse();
    CdtValue::from_checked_items(reversed)
}

/// `cdt:subseq(list, start[, length])` — a contiguous run of elements.
///
/// # The arguments are a 1-based start and a LENGTH, not two positions
///
/// `list-functions/subseq-02.rq` settles it: `cdt:subseq("[1,…,10]", 2, 3)` is
/// `[2, 3, 4]`. Read as an end position that would be `[2, 3]`; read as a length it
/// is three elements starting at the second, which is what the corpus expects.
/// `subseq-01.rq` — `cdt:subseq("[1,…,10]", 1, 1) = [1]` — pins the 1-based start.
/// Omitting the length runs to the end of the list: `subseq-03.rq` requires
/// `cdt:subseq("[1,…,10]", 7)` to be `[7,8,9,10]`.
///
/// # The range must lie inside the list, and one position past its end is inside
///
/// A start of `size + 1` with nothing left to take is **legal** and yields `[]` —
/// `subseq-04.rq` (`cdt:subseq("[]", 1, 0)`), `subseq-05.rq` (`cdt:subseq("[]", 1)`),
/// `subseq-06.rq` and `subseq-07.rq` (start 4 on a three-element list). One position
/// further is an error: `subseq-08.rq` and `subseq-09.rq` (start 5 on a
/// three-element list). So is a start below 1: `subseq-12.rq` (`0`) and
/// `subseq-13.rq` (`-2`).
///
/// A length that would reach past the end is an error, not a truncation:
/// `subseq-10.rq` (start 4, length 1, on a three-element list) and `subseq-11.rq`
/// (start 10, length 2, on a ten-element list) both expect unbound.
///
/// A non-integer argument is an error — `subseq-error-02.rq`. **The corpus does not
/// exercise a negative length**; it is refused here, because the only alternatives
/// are to invent a truncation rule the spec does not state or to silently return
/// `[]`, and `.goals` forbids both.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtOutcome, CdtTerm, CdtValue, list_subseq, parse_list};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// let items = parse_list("[1,2,3,4,5]")?.into_list().expect("a cdt:List");
/// // A start and a LENGTH.
/// assert_eq!(
///     list_subseq(&items, &int("2"), Some(&int("3"))),
///     CdtOutcome::Value(parse_list("[2,3,4]")?)
/// );
/// // One past the end is the empty subsequence, not an error.
/// assert_eq!(
///     list_subseq(&items, &int("6"), Some(&int("0"))),
///     CdtOutcome::Value(parse_list("[]")?)
/// );
/// // Two past the end is an error.
/// assert!(list_subseq(&items, &int("7"), None).is_error());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_subseq(
    items: &[CdtTerm],
    start: &CdtTerm,
    length: Option<&CdtTerm>,
) -> CdtOutcome<CdtValue> {
    let Some(start) = integer_argument(start) else {
        return raise("cdt:subseq needs an xsd:integer start position");
    };
    if start < 1 {
        return raise("cdt:subseq needs a start position of at least 1");
    }
    let Ok(from) = usize::try_from(start - 1) else {
        return raise("cdt:subseq was given a start position beyond any list");
    };
    if from > items.len() {
        return raise("cdt:subseq was given a start position past the end of the list");
    }
    let to = match length {
        None => items.len(),
        Some(length) => {
            let Some(length) = integer_argument(length) else {
                return raise("cdt:subseq needs an xsd:integer length");
            };
            let Ok(length) = usize::try_from(length) else {
                return raise("cdt:subseq needs a length of at least 0");
            };
            let Some(to) = from.checked_add(length) else {
                return raise("cdt:subseq was given a length beyond any list");
            };
            if to > items.len() {
                return raise("cdt:subseq was given a range that ends past the end of the list");
            }
            to
        }
    };
    let taken = &items[from..to];
    if let Err(error) = check_extent(&list_extent(taken.iter())) {
        return CdtOutcome::Bound(error);
    }
    CdtOutcome::Value(CdtValue::from_checked_items(taken.to_vec()))
}

/// `cdt:concat(…)` — the concatenation of the argument lists, in argument order.
///
/// Variadic from zero: `list-functions/concat-08.rq` requires `cdt:concat()` to be
/// `[]` and `concat-09.rq` requires the one-argument call to be its argument.
/// `concat-10.rq` uses three arguments and pins that a list may appear twice.
///
/// Nulls are copied as positions, not resolved: `concat-null-01.rq` concatenates
/// `[null]` with itself and requires a result of size 2 whose every `cdt:get` is
/// unbound.
///
/// A non-list argument is a SPARQL error (`concat-error-01.rq`, and
/// `concat-error-02.rq` for the single-argument case); that is [`concat()`]'s dispatch,
/// since this function is reached with lists in hand.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtOutcome, CdtValue, list_concat, parse_list};
///
/// let a = parse_list("[1]")?.into_list().expect("a cdt:List");
/// let b = parse_list("[2,3]")?.into_list().expect("a cdt:List");
/// assert_eq!(
///     list_concat(&[&a, &b, &a]),
///     CdtOutcome::Value(parse_list("[1,2,3,1]")?)
/// );
/// assert_eq!(list_concat(&[]), CdtOutcome::Value(parse_list("[]")?));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_concat(lists: &[&[CdtTerm]]) -> CdtOutcome<CdtValue> {
    let extent = list_extent(lists.iter().copied().flatten());
    if let Err(error) = check_extent(&extent) {
        return CdtOutcome::Bound(error);
    }
    let items: Vec<CdtTerm> = lists.iter().copied().flatten().cloned().collect();
    CdtOutcome::Value(CdtValue::from_checked_items(items))
}

/// `cdt:contains(list, term)` — does the list hold an element **equal by value** to
/// this term?
///
/// # Equality here is SPARQL `=`, not term identity
///
/// `list-functions/contains-03.rq` is the load-bearing test: a list holding `1` must
/// answer `true` for `1`, `"+1"^^xsd:integer`, `"01"^^xsd:integer`, `1.0` and `1e0`
/// alike, and a list holding `'b'@en` must answer `true` for `'b'@en` and `false` for
/// `'b'`. Nested composites are compared by value too, and a `cdt:List` written out
/// as a literal with an explicit datatype is the same value as one written with the
/// datatype's shorthand — `contains-07.rq` … `contains-10.rq`. Blank nodes are the
/// exception the corpus insists on: they compare by identity, so a freshly minted
/// `BNODE()` is **not** in a list holding `_:b` (`contains-05.rq`, which also
/// requires the answer to be bound `false` rather than an error), while the very term
/// `cdt:head` just returned from that list **is** (`contains-06.rq`).
///
/// # A null element never matches and never poisons the search
///
/// `contains-null-01.rq` searches `[1, null, 2]` for `1.0` and for `2.0` and requires
/// `true` both times: the `null` in the middle is simply an element that is not equal
/// to what is being looked for, so it neither matches nor raises.
///
/// # A definite hit dominates an undecidable comparison
///
/// If some element cannot be compared with the sought term at all — two literals in
/// datatypes with no shared value space — that pair alone is a type error. This
/// function still keeps looking, and answers `true` if any *other* element matches;
/// only when nothing matched does the withheld error surface. That is SPARQL's own
/// rule for an existential over comparisons, and it is the reason a single opaque
/// element in a list cannot stop `cdt:contains` from finding what is there.
///
/// # A blank node in the list is a definite miss, not an undecidable comparison
///
/// `contains-05.rq` searches `"[_:b,null,'_:b']"^^cdt:List` for a freshly minted
/// `BNODE()` and requires the answer to be **bound** and `false`, while
/// `contains-06.rq` requires the very term `cdt:head` just returned from that list to
/// be found. Membership therefore compares blank nodes by identity, which is *not*
/// what SEP-0009's `=` does — `list-equals-06.rq` requires `[_:b1] = [_:b2]` to be
/// unbound. The two relations are asking different questions, and
/// `crate::ops::membership_equal` is where the difference lives.
///
/// **The corpus does not exercise searching for `null` itself** — a SPARQL argument
/// is never the null element, since an unbound argument raises before the call. If a
/// consumer does pass [`CdtTerm::Null`], the answer follows [`crate::term_equal`],
/// under which nulls are mutually indistinguishable, so the search finds a null.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtOutcome, CdtTerm, CdtValue, list_contains, parse_list};
///
/// const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
///
/// let items = parse_list("[1, null, 2]")?.into_list().expect("a cdt:List");
/// let one_point_oh = CdtTerm::Literal(CdtLiteral::typed("1.0", XSD_DECIMAL));
/// assert_eq!(list_contains(&items, &one_point_oh), CdtOutcome::Value(true));
/// let three = CdtTerm::Literal(CdtLiteral::typed("3.0", XSD_DECIMAL));
/// assert_eq!(list_contains(&items, &three), CdtOutcome::Value(false));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn list_contains(items: &[CdtTerm], term: &CdtTerm) -> CdtOutcome<bool> {
    let mut withheld: Option<CdtTypeError> = None;
    for item in items {
        match crate::ops::membership_equal(item, term) {
            Ok(true) => return CdtOutcome::Value(true),
            Ok(false) => {}
            Err(error) => {
                if withheld.is_none() {
                    withheld = Some(error);
                }
            }
        }
    }
    match withheld {
        Some(error) => CdtOutcome::Error(error),
        None => CdtOutcome::Value(false),
    }
}

// ── The map functions ───────────────────────────────────────────────────────────

/// `cdt:size(map)` — how many entries the map has.
///
/// Total, and an entry whose value is `null` counts like any other —
/// `map-functions/size-05.rq` requires `cdt:size("{1: 'one', 2: null}")` to be 2.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtValue, map_size, parse_map};
///
/// let entries = parse_map("{1: 'one', 2: null}")?.into_map().expect("a cdt:Map");
/// assert_eq!(map_size(&entries), 2);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn map_size(entries: &[CdtEntry]) -> usize {
    entries.len()
}

/// `cdt:get(map, key)` — the value stored under a key.
///
/// # Keys match by TERM, not by value — the one place SEP-0009 is not value-based
///
/// `map-functions/get-02.rq` holds `1` and `"02"^^xsd:integer` as separate keys of
/// one map and requires each to retrieve its own value; `containsKey-02.rq` requires
/// `"01"^^xsd:integer` **not** to find the entry under `1`. So the lookup compares
/// lexical form, datatype, language tag and base direction, which is exactly
/// [`CdtKey`]'s own equality and exactly why [`crate::CdtLiteral`] keeps literals
/// lexical-verbatim.
///
/// # Two ways to have no answer, and a query cannot tell them apart
///
/// Both are [`CdtOutcome::Error`]:
///
/// * the key is not in the map (`get-error-01.rq`);
/// * the key is in the map and its value is `null` (`get-null-01.rq`).
///
/// The second is what distinguishes `cdt:get` from `cdt:containsKey`: a map can hold
/// a key whose `cdt:get` raises, and `map-functions/put-02.rq` and
/// `merge-null-01.rq` both build one on purpose and then check `cdt:containsKey` is
/// `true` while `cdt:get` is unbound. A consumer that implemented `cdt:containsKey`
/// as "`cdt:get` is bound" would fail those tests.
///
/// A key argument that could never be a map key at all — a blank node, a nested
/// composite — is likewise an error, since no map holds it.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, CdtValue, map_get, parse_map};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// let entries = parse_map("{1: 'one', 2: null}")?.into_map().expect("a cdt:Map");
/// assert_eq!(
///     map_get(&entries, &int("1")).value(),
///     Some(&CdtTerm::Literal(CdtLiteral::plain("one")))
/// );
/// // Present, but holding a null.
/// assert!(map_get(&entries, &int("2")).is_error());
/// // A different term, so a different key.
/// assert!(map_get(&entries, &int("01")).is_error());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn map_get(entries: &[CdtEntry], key: &CdtTerm) -> CdtOutcome<CdtTerm> {
    let Some(key) = CdtKey::from_term(key) else {
        return raise("cdt:get on a cdt:Map was given a term that cannot be a map key");
    };
    let Ok(index) = entries.binary_search_by(|entry| total_key_cmp(&entry.key, &key)) else {
        return raise("cdt:get on a cdt:Map addressed a key the map does not hold");
    };
    if entries[index].value.is_null() {
        return raise("cdt:get on a cdt:Map addressed a key whose value is null");
    }
    CdtOutcome::Value(entries[index].value.clone())
}

/// `cdt:containsKey(map, key)` — is this term one of the map's keys?
///
/// Total: it has an answer for every map and every term, and the corpus insists on
/// it — `map-functions/containsKey-01.rq` requires `cdt:containsKey("{}", 1)` to be
/// *bound* and `false`, not an error. Key identity is the term, so
/// `containsKey-02.rq` requires `"01"^^xsd:integer` to answer `false` against a map
/// keyed by `1`, and `containsKey-03.rq` requires the values not to be searched.
///
/// It is **not** "`cdt:get` succeeds": a key whose value is `null` is present.
/// `map-functions/put-02.rq` and `merge-null-01.rq` both check exactly that pair of
/// answers.
///
/// A term that could never be a map key answers `false`. **The corpus does not test
/// that directly for `cdt:containsKey`**, but no map can hold such a key — production
/// `[7] MapKey` does not admit one — so `false` is the only answer consistent with
/// the type, and it agrees with `remove-01.rq`, where removing a blank-node key
/// leaves the map untouched.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, CdtValue, map_contains_key, parse_map};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// let entries = parse_map("{1: null}")?.into_map().expect("a cdt:Map");
/// // The key is there even though its value is a null.
/// assert!(map_contains_key(&entries, &int("1")));
/// assert!(!map_contains_key(&entries, &int("01")));
/// assert!(!map_contains_key(&entries, &CdtTerm::Blank("b0".into())));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn map_contains_key(entries: &[CdtEntry], key: &CdtTerm) -> bool {
    CdtKey::from_term(key).is_some_and(|key| {
        entries
            .binary_search_by(|entry| total_key_cmp(&entry.key, &key))
            .is_ok()
    })
}

/// `cdt:keys(map)` — the map's keys, as a `cdt:List`.
///
/// Total, and it never raises. `map-functions/keys-01.rq` requires `cdt:keys("{}")`
/// to be `[]` and `keys-02.rq` requires `cdt:keys("{1: 'one'}")` to be `[1]`.
///
/// # The order
///
/// **The corpus does not pin an order.** `keys-03.rq` is its only multi-key test and
/// it deliberately checks `cdt:size` and two `cdt:contains` rather than comparing
/// against a list — which is the corpus saying that a `cdt:Map` is unordered and its
/// keys therefore have no intrinsic sequence. Something must nonetheless be written,
/// so the keys come out in this crate's syntactic key order
/// ([`crate::total_key_cmp`]) — the same order a map's entries are held and rendered
/// in. That makes `cdt:keys` byte-deterministic across runs, processes and hosts,
/// which the workspace requires of everything it computes, and it makes the result a
/// pure function of the map's *value* rather than of how the map was authored.
///
/// No bound can be exceeded: the result has one leaf element per entry, so it is
/// smaller than the map in every one of the three dimensions.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{map_keys, parse_list, parse_map, CdtValue};
///
/// let entries = parse_map("{2: 'two', 1: 'one'}")?.into_map().expect("a cdt:Map");
/// assert_eq!(map_keys(&entries), parse_list("[1, 2]")?);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn map_keys(entries: &[CdtEntry]) -> CdtValue {
    CdtValue::from_checked_items(entries.iter().map(|entry| entry.key.to_term()).collect())
}

/// `cdt:put(map, key[, value])` — the map with one entry set to a value.
///
/// # An absent or failed value argument is a null entry, not a failure
///
/// `map-functions/put-03.rq` calls `cdt:put(?map, 1)` with no value at all and
/// requires a map of size 1 that contains key 1 and whose `cdt:get` on that key is
/// unbound; `put-02.rq` requires the same of an unbound third argument. Both are
/// spelled here by passing [`CdtTerm::Null`] as `value`. `put-07.rq` and `put-08.rq`
/// repeat the point on a non-empty map, and `put-06.rq` runs it the other way — a
/// `null` already stored is replaced by a real term.
///
/// # A key argument that cannot be a key raises
///
/// `put-error-03.rq` requires `cdt:put(?map, BNODE(), "one")` to be unbound, and
/// `put-error-04.rq` requires the same of an unbound key. This is the opposite of
/// [`map_constructor`], which silently drops such a pair, and of [`map_remove`],
/// which returns the map unchanged — three functions, three different treatments of
/// the same bad key, each pinned by its own test.
///
/// # An existing key is overwritten, and position is not observable
///
/// `put-05.rq` requires `cdt:put("{1:'one', 2:'two'}", 1, "alsoOne")` to equal
/// `"{1:'alsoOne', 2:'two'}"`, and `put-04.rq` requires putting a key's own value
/// back to leave the map equal to itself. Every `cdt:put` test compares with `=`,
/// never `SAMETERM`, because a `cdt:Map` is unordered and an entry has no position to
/// preserve or move. This crate holds entries in [`crate::total_key_cmp`] order
/// regardless, so the question does not arise: the result's arrangement is a function
/// of its keys and nothing else, and re-putting a key lands it exactly where it
/// already was. Key identity is the term — `put-12.rq` adds `"01"^^xsd:integer`
/// *beside* `1` rather than replacing it, and `put-14.rq` does the same with
/// `'hello'` beside `'hello'@en`.
///
/// # Bounds
///
/// `cdt:put(?m, ?k, ?m)` roughly doubles a map's element count each time it is
/// applied, so this is the function that most needs a bound. The result's element
/// count, depth and canonical byte length are computed from the borrowed map and the
/// borrowed value first; only if all three fit is anything cloned.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtOutcome, CdtTerm, CdtValue, map_put, parse_map};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// let entries = parse_map("{1:'one', 2:'two'}")?.into_map().expect("a cdt:Map");
/// let value = CdtTerm::Literal(CdtLiteral::plain("alsoOne"));
/// assert_eq!(
///     map_put(&entries, &int("1"), &value),
///     CdtOutcome::Value(parse_map("{1:'alsoOne', 2:'two'}")?)
/// );
/// // A blank node cannot be a key, and `cdt:put` refuses rather than ignoring it.
/// assert!(map_put(&entries, &CdtTerm::Blank("b0".into()), &value).is_error());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn map_put(entries: &[CdtEntry], key: &CdtTerm, value: &CdtTerm) -> CdtOutcome<CdtValue> {
    let Some(key) = CdtKey::from_term(key) else {
        return raise("cdt:put was given a term that cannot be a map key");
    };
    let position = entries.binary_search_by(|entry| total_key_cmp(&entry.key, &key));
    let mut pairs: Vec<(&CdtKey, &CdtTerm)> = Vec::with_capacity(entries.len() + 1);
    match position {
        Ok(replaced) => {
            for (index, entry) in entries.iter().enumerate() {
                let stored = if index == replaced {
                    value
                } else {
                    &entry.value
                };
                pairs.push((&entry.key, stored));
            }
        }
        Err(insert) => {
            for entry in &entries[..insert] {
                pairs.push((&entry.key, &entry.value));
            }
            pairs.push((&key, value));
            for entry in &entries[insert..] {
                pairs.push((&entry.key, &entry.value));
            }
        }
    }
    if let Err(error) = check_extent(&map_extent(pairs.iter().copied())) {
        return CdtOutcome::Bound(error);
    }
    let built = pairs
        .into_iter()
        .map(|(key, value)| CdtEntry {
            key: key.clone(),
            value: value.clone(),
        })
        .collect();
    CdtOutcome::Value(CdtValue::from_checked_entries(built))
}

/// What `cdt:remove` did.
///
/// [`MapRemoval::Unchanged`] is not an evasion — it is an *observable* answer that
/// the corpus tests with `SAMETERM`. `map-functions/remove-01.rq` removes a
/// `BNODE()` key from a map literal and requires the result to be **the same RDF
/// term** as the input, not merely an equal map. A consumer must therefore return
/// its original literal verbatim, with its original lexical form, rather than
/// re-rendering an equal value into the canonical form — those are different terms,
/// and `SAMETERM` can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapRemoval {
    /// The key was not in the map, so the input map *is* the answer.
    Unchanged,
    /// The key was in the map; this is the map without it.
    Removed(CdtValue),
}

/// `cdt:remove(map, key)` — the map without the entry under a key.
///
/// Total: it never raises, for any map and any term.
///
/// # A key that is absent, or that could never be a key, leaves the map alone
///
/// `map-functions/remove-02.rq` removes from the empty map, `remove-06.rq` and
/// `remove-07.rq` use a key that differs only in lexical form, `remove-09.rq` and
/// `remove-10.rq` differ only in the language tag, and `remove-01.rq` passes a
/// `BNODE()` — every one of them is bound, and the last requires `SAMETERM` with the
/// input. See [`MapRemoval`] for why that distinction has to reach the caller.
///
/// Key identity is again the term, so `remove-05.rq` removes `"02"^^xsd:integer`
/// while leaving `1` and `remove-11.rq` removes an IRI key.
///
/// No bound can be exceeded: the result is a sub-sequence of the input's entries.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, CdtValue, MapRemoval, map_remove, parse_map};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// let entries = parse_map("{1:'one', 2:'two'}")?.into_map().expect("a cdt:Map");
/// assert_eq!(
///     map_remove(&entries, &int("1")),
///     MapRemoval::Removed(parse_map("{2:'two'}")?)
/// );
/// // Not a key of this map, and not a key at all: both leave it alone.
/// assert_eq!(map_remove(&entries, &int("01")), MapRemoval::Unchanged);
/// assert_eq!(map_remove(&entries, &CdtTerm::Blank("b0".into())), MapRemoval::Unchanged);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn map_remove(entries: &[CdtEntry], key: &CdtTerm) -> MapRemoval {
    let Some(key) = CdtKey::from_term(key) else {
        return MapRemoval::Unchanged;
    };
    let Ok(index) = entries.binary_search_by(|entry| total_key_cmp(&entry.key, &key)) else {
        return MapRemoval::Unchanged;
    };
    let mut kept = Vec::with_capacity(entries.len() - 1);
    kept.extend_from_slice(&entries[..index]);
    kept.extend_from_slice(&entries[index + 1..]);
    MapRemoval::Removed(CdtValue::from_checked_entries(kept))
}

/// `cdt:merge(…)` — the union of the argument maps.
///
/// # On a duplicate key the FIRST map wins
///
/// `map-functions/merge-05.rq` merges `{1: 'one', 2: 'two'}` with
/// `{1: 'another one', 3: 'three'}` and requires `1` to map to `'one'` — the left
/// argument's value. `merge-08.rq` repeats it with an IRI key. The rule survives
/// nulls in both directions and this is where it would be easiest to get backwards:
/// `merge-null-03.rq` merges `{1: null}` with `{1: 'one'}` and requires the result's
/// entry for `1` to be **unbound**, so a stored `null` is a real value that wins like
/// any other and is not treated as an absence to be filled in; `merge-null-04.rq`
/// runs it the other way and keeps `'one'`.
///
/// Keys not in conflict are simply unioned (`merge-04.rq`), and key identity is
/// again the term, so `1` and `"01"^^xsd:integer` both survive a merge as separate
/// entries (`merge-06.rq`), as do `'hello'` and `'hello'@en` (`merge-08.rq`).
///
/// First-wins makes the operation associative, which is what lets the variadic
/// reading of [`CdtFn::Merge`]'s arity agree with the corpus's two-argument tests.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtOutcome, CdtValue, map_merge, parse_map};
///
/// let a = parse_map("{1: 'one', 2: 'two'}")?.into_map().expect("a cdt:Map");
/// let b = parse_map("{1: 'another one', 3: 'three'}")?.into_map().expect("a cdt:Map");
/// assert_eq!(
///     map_merge(&[&a, &b]),
///     CdtOutcome::Value(parse_map("{1: 'one', 2: 'two', 3: 'three'}")?)
/// );
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn map_merge(maps: &[&[CdtEntry]]) -> CdtOutcome<CdtValue> {
    // Flatten in argument order, so a lower position means an earlier map. Sorting by
    // key and then by that position puts the winning entry of each key first, and
    // `dedup_by` keeps the first of every run.
    let mut selected: Vec<(usize, &CdtEntry)> = Vec::new();
    for entries in maps {
        for entry in *entries {
            selected.push((selected.len(), entry));
        }
    }
    selected.sort_by(|a, b| total_key_cmp(&a.1.key, &b.1.key).then_with(|| a.0.cmp(&b.0)));
    selected.dedup_by(|a, b| a.1.key == b.1.key);

    let extent = map_extent(selected.iter().map(|(_, entry)| (&entry.key, &entry.value)));
    if let Err(error) = check_extent(&extent) {
        return CdtOutcome::Bound(error);
    }
    let entries = selected
        .into_iter()
        .map(|(_, entry)| entry.clone())
        .collect();
    CdtOutcome::Value(CdtValue::from_checked_entries(entries))
}

// ── Dispatch on the runtime composite datatype ──────────────────────────────────

/// A SPARQL type error for a function applied to the wrong composite datatype.
fn wrong_datatype<T>(wanted: &'static str) -> CdtOutcome<T> {
    CdtOutcome::Error(CdtTypeError::undefined(wanted))
}

/// `cdt:size(…)` — dispatching on the argument's composite datatype.
///
/// This is one of the two overloaded names in the library, and it is the one whose
/// type-error contract is most often got wrong. Applied to a `cdt:List` it is the
/// element count and applied to a `cdt:Map` it is the entry count — **neither is an
/// error**, which is why there is one `cdt:size` IRI and not two. Applied to a
/// literal that is not a composite at all it *is* an error:
/// `list-functions/size-error-01.rq` binds the plain string `"[1,2]"` with no
/// datatype and requires `cdt:size` on it to be unbound. That case never reaches this
/// function, because it is decided when the consumer tries and fails to obtain a
/// [`CdtValue`] from the argument; keeping it out here is what makes `size` total.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{parse_list, parse_map, size};
///
/// assert_eq!(size(&parse_list("[1, 2]")?), 2);
/// assert_eq!(size(&parse_map("{1: 'one'}")?), 1);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn size(value: &CdtValue) -> usize {
    match value.contents() {
        CdtContents::List(items) => list_size(items),
        CdtContents::Map(entries) => map_size(entries),
    }
}

/// `cdt:get(…)` — dispatching on the argument's composite datatype.
///
/// A list takes a 1-based integer index ([`list_get`]) and a map takes a key
/// ([`map_get`]). One IRI, two argument shapes, chosen by what the first argument
/// turned out to be at run time.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, get, parse_list, parse_map};
///
/// const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// let int = |lexical: &str| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER));
///
/// // On a list the second argument is a position…
/// assert_eq!(get(&parse_list("[7, 8]")?, &int("2")).value(), Some(&int("8")));
/// // …and on a map it is a key.
/// assert_eq!(get(&parse_map("{2: 8}")?, &int("2")).value(), Some(&int("8")));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn get(value: &CdtValue, argument: &CdtTerm) -> CdtOutcome<CdtTerm> {
    match value.contents() {
        CdtContents::List(items) => list_get(items, argument),
        CdtContents::Map(entries) => map_get(entries, argument),
    }
}

/// `cdt:contains(list, term)` — raising for a `cdt:Map`.
///
/// **The corpus places `cdt:contains` under `list-functions` only** and gives maps
/// their own `cdt:containsKey`; there is no map test for it and the spec's map
/// section lists `cdt:containsKey`, not `cdt:contains`. Applying it to a map is
/// therefore a type error here rather than a guess at whether it would mean "contains
/// this value" or "contains this key" — inventing an answer where the spec has none
/// is what `.goals` forbids.
pub fn contains(value: &CdtValue, term: &CdtTerm) -> CdtOutcome<bool> {
    match value.contents() {
        CdtContents::List(items) => list_contains(items, term),
        CdtContents::Map(_) => {
            wrong_datatype("cdt:contains applies to a cdt:List; a cdt:Map has cdt:containsKey")
        }
    }
}

/// `cdt:head(list)` — raising for a `cdt:Map`.
pub fn head(value: &CdtValue) -> CdtOutcome<CdtTerm> {
    match value.contents() {
        CdtContents::List(items) => list_head(items),
        CdtContents::Map(_) => wrong_datatype("cdt:head applies to a cdt:List"),
    }
}

/// `cdt:tail(list)` — raising for a `cdt:Map`.
pub fn tail(value: &CdtValue) -> CdtOutcome<CdtValue> {
    match value.contents() {
        CdtContents::List(items) => list_tail(items),
        CdtContents::Map(_) => wrong_datatype("cdt:tail applies to a cdt:List"),
    }
}

/// `cdt:reverse(list)` — raising for a `cdt:Map`.
pub fn reverse(value: &CdtValue) -> CdtOutcome<CdtValue> {
    match value.contents() {
        CdtContents::List(items) => CdtOutcome::Value(list_reverse(items)),
        CdtContents::Map(_) => wrong_datatype("cdt:reverse applies to a cdt:List"),
    }
}

/// `cdt:subseq(list, start[, length])` — raising for a `cdt:Map`.
pub fn subseq(value: &CdtValue, start: &CdtTerm, length: Option<&CdtTerm>) -> CdtOutcome<CdtValue> {
    match value.contents() {
        CdtContents::List(items) => list_subseq(items, start, length),
        CdtContents::Map(_) => wrong_datatype("cdt:subseq applies to a cdt:List"),
    }
}

/// `cdt:concat(…)` — raising when any argument is a `cdt:Map`.
///
/// `list-functions/concat-error-01.rq` requires a single non-list argument among two
/// to make the whole call unbound, and `concat-error-02.rq` requires the same when it
/// is the only argument.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{concat, parse_list, parse_map};
///
/// let list = parse_list("[1]")?;
/// assert_eq!(concat(&[list.clone(), list.clone()]).value(), Some(&parse_list("[1,1]")?));
/// // One map among the arguments poisons the whole call.
/// assert!(concat(&[list, parse_map("{}")?]).is_error());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn concat(values: &[CdtValue]) -> CdtOutcome<CdtValue> {
    let mut lists: Vec<&[CdtTerm]> = Vec::with_capacity(values.len());
    for value in values {
        match value.contents() {
            CdtContents::List(items) => lists.push(items),
            CdtContents::Map(_) => {
                return wrong_datatype("cdt:concat applies to cdt:List arguments only");
            }
        }
    }
    list_concat(&lists)
}

/// `cdt:containsKey(map, key)` — raising for a `cdt:List`.
pub fn contains_key(value: &CdtValue, key: &CdtTerm) -> CdtOutcome<bool> {
    match value.contents() {
        CdtContents::Map(entries) => CdtOutcome::Value(map_contains_key(entries, key)),
        CdtContents::List(_) => {
            wrong_datatype("cdt:containsKey applies to a cdt:Map; a cdt:List has cdt:contains")
        }
    }
}

/// `cdt:keys(map)` — raising for a `cdt:List`.
pub fn keys(value: &CdtValue) -> CdtOutcome<CdtValue> {
    match value.contents() {
        CdtContents::Map(entries) => CdtOutcome::Value(map_keys(entries)),
        CdtContents::List(_) => wrong_datatype("cdt:keys applies to a cdt:Map"),
    }
}

/// `cdt:merge(…)` — raising when any argument is a `cdt:List`.
pub fn merge(values: &[CdtValue]) -> CdtOutcome<CdtValue> {
    let mut maps: Vec<&[CdtEntry]> = Vec::with_capacity(values.len());
    for value in values {
        match value.contents() {
            CdtContents::Map(entries) => maps.push(entries),
            CdtContents::List(_) => {
                return wrong_datatype("cdt:merge applies to cdt:Map arguments only");
            }
        }
    }
    map_merge(&maps)
}

/// `cdt:put(map, key[, value])` — raising for a `cdt:List`.
pub fn put(value: &CdtValue, key: &CdtTerm, item: &CdtTerm) -> CdtOutcome<CdtValue> {
    match value.contents() {
        CdtContents::Map(entries) => map_put(entries, key, item),
        CdtContents::List(_) => wrong_datatype("cdt:put applies to a cdt:Map"),
    }
}

/// `cdt:remove(map, key)` — raising for a `cdt:List`.
///
/// See [`MapRemoval`] for why the "nothing was removed" case has to be distinguished
/// from "here is an equal map".
pub fn remove(value: &CdtValue, key: &CdtTerm) -> CdtOutcome<MapRemoval> {
    match value.contents() {
        CdtContents::Map(entries) => CdtOutcome::Value(map_remove(entries, key)),
        CdtContents::List(_) => wrong_datatype("cdt:remove applies to a cdt:Map"),
    }
}
