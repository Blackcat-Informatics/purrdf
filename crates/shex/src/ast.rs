// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ShExJ-aligned ShEx 2.1 abstract syntax tree.
//!
//! The shapes mirror the ShExJ JSON-LD object model (ShEx 2.1 spec, Appendix A
//! / the `shex.jsonld` context vendored at `vectors/shexTest/context.jsonld`):
//! a [`Schema`] holds `startActs`, `start`, `imports` and labeled shape
//! declarations; a [`ShapeExpr`] is the boolean algebra over
//! [`NodeConstraint`]s, [`Shape`]s, externals and references; a [`TripleExpr`]
//! is `EachOf`/`OneOf`/`TripleConstraint`/reference with cardinalities.
//!
//! Conventions shared with the ShExJ wire format ([`crate::shexj`]):
//!
//! * Labels ([`ShapeLabel`]) are plain strings: an IRI, or a blank-node label
//!   carrying its `_:` prefix (exactly the ShExJ encoding).
//! * `max` cardinality `-1` means unbounded (`*` / `{m,}` in ShExC).
//! * `min`/`max` are `None` when no explicit cardinality was written.
//! * Empty `Vec`s stand for "absent" list-valued properties.
//!
//! Semantic actions and annotations are carried faithfully but are inert in
//! phase 1 (no extension dispatch, no validator).

/// A shape-expression or triple-expression label: an absolute (or, when the
/// schema was parsed without a base, relative) IRI, or a blank-node label
/// spelled with its `_:` prefix — the ShExJ string encoding.
pub type ShapeLabel = String;

/// A parsed ShEx schema (ShExJ `Schema` object).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schema {
    /// `IMPORT` directives, in document order (ShExJ `imports`).
    pub imports: Vec<String>,
    /// Schema-level semantic actions (ShExJ `startActs`).
    pub start_acts: Vec<SemAct>,
    /// The `start =` shape expression, when declared (ShExJ `start`).
    pub start: Option<Box<ShapeExpr>>,
    /// Labeled shape-expression declarations (ShExJ `shapes`).
    pub shapes: Vec<ShapeDecl>,
}

/// A labeled top-level shape expression (a ShExJ `shapes` entry, whose `id`
/// is inlined on the shape-expression object).
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeDecl {
    /// The declaration's label.
    pub id: ShapeLabel,
    /// The declared expression (`EXTERNAL` becomes [`ShapeExpr::External`]).
    pub expr: ShapeExpr,
}

/// The shape-expression algebra (ShExJ `shapeExpr` union).
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeExpr {
    /// Conjunction (`AND`; ShExJ `ShapeAnd`). Flattened: `a AND b AND c` is a
    /// single node with three children.
    And(Vec<Self>),
    /// Disjunction (`OR`; ShExJ `ShapeOr`), likewise flattened.
    Or(Vec<Self>),
    /// Negation (`NOT`; ShExJ `ShapeNot`).
    Not(Box<Self>),
    /// A node constraint (kind/datatype/facets/value set).
    Node(NodeConstraint),
    /// A triple-expression shape (`{ ... }` with `CLOSED`/`EXTRA`).
    Shape(Shape),
    /// An externally-defined shape (`EXTERNAL`; ShExJ `ShapeExternal`).
    External,
    /// A reference to a labeled shape expression (`@label`; a bare string in
    /// ShExJ).
    Ref(ShapeLabel),
}

/// RDF node kinds a [`NodeConstraint`] may demand (ShExJ `nodeKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// `IRI`
    Iri,
    /// `BNODE`
    BNode,
    /// `NONLITERAL`
    NonLiteral,
    /// `LITERAL`
    Literal,
}

impl NodeKind {
    /// The ShExJ string encoding (`"iri"` / `"bnode"` / `"nonliteral"` /
    /// `"literal"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Iri => "iri",
            Self::BNode => "bnode",
            Self::NonLiteral => "nonliteral",
            Self::Literal => "literal",
        }
    }
}

/// A node constraint (ShExJ `NodeConstraint`): node kind, datatype, XML-Schema
/// facets and/or a value set. All properties are optional and combinable per
/// the grammar (the ShExC parser enforces the litNodeConstraint /
/// nonLitNodeConstraint split at parse time).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeConstraint {
    /// `IRI` / `BNODE` / `NONLITERAL` / `LITERAL`.
    pub node_kind: Option<NodeKind>,
    /// Datatype IRI.
    pub datatype: Option<String>,
    /// `LENGTH n` string facet.
    pub length: Option<u64>,
    /// `MINLENGTH n` string facet.
    pub minlength: Option<u64>,
    /// `MAXLENGTH n` string facet.
    pub maxlength: Option<u64>,
    /// `/pattern/` string facet (XPath regex source, `\/` unescaped and UCHARs
    /// decoded exactly as the ShExJ wire format expects).
    pub pattern: Option<String>,
    /// Regex flags attached to `pattern` (`smix`).
    pub flags: Option<String>,
    /// `MININCLUSIVE n` numeric facet.
    pub mininclusive: Option<NumericLiteral>,
    /// `MINEXCLUSIVE n` numeric facet.
    pub minexclusive: Option<NumericLiteral>,
    /// `MAXINCLUSIVE n` numeric facet.
    pub maxinclusive: Option<NumericLiteral>,
    /// `MAXEXCLUSIVE n` numeric facet.
    pub maxexclusive: Option<NumericLiteral>,
    /// `TOTALDIGITS n` numeric facet.
    pub totaldigits: Option<u64>,
    /// `FRACTIONDIGITS n` numeric facet.
    pub fractiondigits: Option<u64>,
    /// The `[ ... ]` value set, when present (may be present and empty).
    pub values: Option<Vec<ValueSetValue>>,
}

/// A numeric facet value (ShExJ encodes these as bare JSON numbers).
///
/// JSON cannot distinguish `xsd:decimal` from `xsd:double`, so any value with
/// a fractional part is [`NumericLiteral::Fractional`]; integral values
/// (including ShExC lexical forms like `5.0` or `5E0`) normalize to
/// [`NumericLiteral::Integer`], matching the reference ShExJ conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumericLiteral {
    /// An integral value.
    Integer(i64),
    /// A non-integral (decimal/double) value.
    Fractional(f64),
}

/// One member of a value set (ShExJ `valueSetValue` union).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueSetValue {
    /// A single IRI (a bare string in ShExJ).
    Iri(String),
    /// A single literal (ShExJ `ObjectLiteral`).
    Literal(ObjectLiteral),
    /// `iri~` — every IRI starting with `stem`.
    IriStem {
        /// The IRI prefix.
        stem: String,
    },
    /// `iri~ - ex1 - ex2~` or `. - ex1 …` — a stem (or wildcard) minus
    /// exclusions.
    IriStemRange {
        /// The IRI prefix, or the `.` wildcard.
        stem: StemValue,
        /// Excluded IRIs / IRI stems.
        exclusions: Vec<IriExclusion>,
    },
    /// `"lit"~` — every literal whose lexical form starts with `stem`.
    LiteralStem {
        /// The lexical-form prefix.
        stem: String,
    },
    /// `"lit"~ - "ex" …` or `. - "ex" …`.
    LiteralStemRange {
        /// The lexical-form prefix, or the `.` wildcard.
        stem: StemValue,
        /// Excluded literals / literal stems.
        exclusions: Vec<LiteralExclusion>,
    },
    /// `@tag` — literals with exactly this language tag.
    Language {
        /// The language tag (case preserved from the source).
        language_tag: String,
    },
    /// `@tag~` (or `@~` for the empty stem) — language-tag prefix match.
    LanguageStem {
        /// The language-tag prefix (may be empty).
        stem: String,
    },
    /// `@tag~ - @ex …` or `. - @ex …`.
    LanguageStemRange {
        /// The language-tag prefix, or the `.` wildcard.
        stem: StemValue,
        /// Excluded language tags / stems.
        exclusions: Vec<LanguageExclusion>,
    },
}

/// A stem that is either a concrete prefix string or the `.` wildcard
/// (ShExJ `{"type": "Wildcard"}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StemValue {
    /// A concrete prefix.
    Str(String),
    /// The `.` wildcard.
    Wildcard,
}

/// An exclusion inside an [`ValueSetValue::IriStemRange`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IriExclusion {
    /// `- <iri>` — exclude one IRI.
    Iri(String),
    /// `- <iri>~` — exclude a whole IRI stem.
    Stem(String),
}

/// An exclusion inside a [`ValueSetValue::LiteralStemRange`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiteralExclusion {
    /// `- "lit"` — exclude one literal (by lexical form).
    Literal(String),
    /// `- "lit"~` — exclude a lexical-form stem.
    Stem(String),
}

/// An exclusion inside a [`ValueSetValue::LanguageStemRange`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageExclusion {
    /// `- @tag` — exclude one language tag.
    Language(String),
    /// `- @tag~` — exclude a language-tag stem.
    Stem(String),
}

/// A literal value in a value set or annotation object (ShExJ
/// `ObjectLiteral`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectLiteral {
    /// The lexical form.
    pub value: String,
    /// The language tag, for language-tagged strings.
    pub language: Option<String>,
    /// The datatype IRI (ShExJ key `type`), for datatyped literals.
    pub datatype: Option<String>,
}

/// An IRI or literal in annotation-object position (ShExJ `objectValue`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectValue {
    /// An IRI (a bare string in ShExJ).
    Iri(String),
    /// A literal.
    Literal(ObjectLiteral),
}

/// A `{ ... }` shape (ShExJ `Shape`): a triple expression plus the
/// closed-world modifiers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shape {
    /// `CLOSED`.
    pub closed: Option<bool>,
    /// `EXTRA p1 p2 …` predicate IRIs.
    pub extra: Vec<String>,
    /// The body; `None` for the empty shape `{ }`.
    pub expression: Option<TripleExpr>,
    /// Trailing semantic actions.
    pub sem_acts: Vec<SemAct>,
    /// Trailing annotations.
    pub annotations: Vec<Annotation>,
}

/// The triple-expression algebra (ShExJ `tripleExpr` union).
#[derive(Clone, Debug, PartialEq)]
pub enum TripleExpr {
    /// `e1; e2; …` conjunction (ShExJ `EachOf`).
    EachOf(TripleExprGroup),
    /// `e1 | e2 | …` alternation (ShExJ `OneOf`).
    OneOf(TripleExprGroup),
    /// A single predicate/value constraint.
    TripleConstraint(TripleConstraint),
    /// `&label` — inclusion of a labeled triple expression (a bare string in
    /// ShExJ).
    Ref(ShapeLabel),
}

/// The shared shell of `EachOf`/`OneOf` groups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TripleExprGroup {
    /// `$label` — the group's triple-expression label.
    pub id: Option<ShapeLabel>,
    /// The members, in document order.
    pub expressions: Vec<TripleExpr>,
    /// Minimum cardinality; `None` when no explicit cardinality was written.
    pub min: Option<i64>,
    /// Maximum cardinality; `-1` = unbounded.
    pub max: Option<i64>,
    /// Attached semantic actions.
    pub sem_acts: Vec<SemAct>,
    /// Attached annotations.
    pub annotations: Vec<Annotation>,
}

/// A triple constraint (ShExJ `TripleConstraint`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TripleConstraint {
    /// `$label` — the constraint's triple-expression label.
    pub id: Option<ShapeLabel>,
    /// `^` — match the inverse arc. `Some(true)` when written; the wire format
    /// omits the key otherwise.
    pub inverse: Option<bool>,
    /// The predicate IRI (`a` expands to `rdf:type`).
    pub predicate: String,
    /// The value expression; `None` for the `.` wildcard.
    pub value_expr: Option<Box<ShapeExpr>>,
    /// Minimum cardinality; `None` when no explicit cardinality was written.
    pub min: Option<i64>,
    /// Maximum cardinality; `-1` = unbounded.
    pub max: Option<i64>,
    /// Attached semantic actions.
    pub sem_acts: Vec<SemAct>,
    /// Attached annotations.
    pub annotations: Vec<Annotation>,
}

/// A semantic action `%name{ code %}` / `%name%` (ShExJ `SemAct`). Carried
/// verbatim and inert in phase 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemAct {
    /// The extension IRI.
    pub name: String,
    /// The code block with ShExC `\%` / `\\` / UCHAR escapes decoded; `None`
    /// for the no-code form `%name%`.
    pub code: Option<String>,
}

/// An annotation `// predicate object` (ShExJ `Annotation`). Carried verbatim
/// and inert in phase 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    /// The annotation predicate IRI.
    pub predicate: String,
    /// The annotation object (IRI or literal).
    pub object: ObjectValue,
}
