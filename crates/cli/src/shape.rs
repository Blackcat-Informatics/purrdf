// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `shape` subcommand: `Source → aggregate figures → one JSON document`.
//!
//! The verb reports how a dataset is built and nothing about what it says, so the owner of a
//! private graph can hand the figures on without handing the graph on. Nothing that names or
//! spells content is ever formed, let alone printed: the document is assembled from integers
//! and from fixed ASCII keys, and no term, literal text, language tag, blank-node label,
//! datatype IRI, path or parser message reaches stdout or stderr.
//!
//! # What is measured
//!
//! * counts: quads, distinct IRIs, distinct literals, blank nodes, named graphs, triple
//!   terms, reifiers and annotations, and the maximum nesting depth of a triple term;
//! * like blank siblings: for each blank node with blank children, the size of its largest
//!   group of children that share a first-degree description, as a histogram and a maximum;
//! * literal lengths in bytes of the lexical form, as a histogram, with the number of
//!   literals per named XSD datatype and one count for every other datatype;
//! * language tags: the number of language-tagged literals and of distinct tags;
//! * the line weight of the canonical labelling, or the typed reason it is absent.
//!
//! # Bucket boundaries and the small-count threshold
//!
//! A histogram bin covers a fixed range: bin `0` holds the value `0`, and bin `i` holds
//! `2^(i-1)` through `2^i - 1`. Boundaries never depend on the data, so two outputs compare
//! bin for bin. Only non-empty bins are printed, each with its bounds.
//!
//! A count of one to four is printed as the string `"<5"`; zero and counts of five or more
//! print exactly. Five is the threshold ([`SMALL_COUNT_THRESHOLD`]): a figure that holds for
//! at most four items describes a handful of particular items (the one literal of an unusual
//! datatype, the one language tag in use) and so can single content out, while a figure
//! shared by five or more cannot be traced to one of them. A zero names no item. The two maxima
//! (the largest like group and the triple-term depth) print exactly unless they would pin a hidden
//! count, in which case one under five prints as `"<5"` or the maximum is left out.
//!
//! # Suppression
//!
//! Printed counts are tied to one another by sums and by structural bounds of an RDF 1.2
//! dataset, and a reader who knows the ties can combine them. [`Figures::plan`] puts every count
//! and every tie into one system, and [`suppress`] treats each printed form as an interval,
//! propagates the ties, and withholds further counts until no hidden count is pinned. A hidden
//! `"<5"` count is pinned when its upper bound falls below four or fewer than three values remain
//! possible for it; a withheld or lower-bounded count when fewer than two remain. That is the
//! guarantee: a small count stays a bucket, never an exact value, for a reader who knows the
//! ties below. A count that no other form can leave unpinned is left out as a typed omission;
//! once every non-zero count is hidden or left out no tie can pin one, since a left-out count
//! is confined to nothing and a zero is a constant, so the search always ends with the
//! document printed.
//!
//! The ties come from the data model and from what [`measure`] counts. A statement count is the
//! number of quads. A literal is the object of a quad, an annotation or a triple term, and
//! nowhere else, so the distinct literals are at most those three counts together; the
//! datatype classes and the tagged literals are disjoint classes of literals, and a tag is used
//! by a tagged literal. A blank node occupies a subject or object position of a quad, an
//! annotation or a triple term, a reifier, or a graph name (one per named graph), so the blank
//! nodes are at most twice the quads, annotations and triple terms plus the reifiers and the
//! named graphs. An IRI occupies a subject, predicate or object position of those, a reifier, or
//! a graph name, so the IRIs are at most three per quad, annotation and triple term plus one
//! per reifier and per named graph. A parent is a blank node with a quad of its own. Each parent
//! of a histogram bin has at least the bin's minimum of like blank children and a quad to each,
//! so the quads are at least the sum of minimum times count over the bins; the largest like
//! group lies within the highest bin that has a parent, and needs that many blank nodes besides
//! its parent. A triple term nests as deep as its depth. The distinct literals, and the
//! parents, are the sums of their breakdowns. Every printed figure is a count of the dataset, a
//! maximum of it, or a bucket of one of them, and each relation among them that follows from the
//! data model is one of these.
//!
//! # Language-tag case
//!
//! The data model lowercases a language tag when the term is interned, so only a reader that
//! sees the tag as written can report its case form. A GTS read does, through the envelope's
//! `language_tag_forms`. The text readers record no such census, which the document states as
//! the omission `text-reader-records-no-census`, and a pack holds only the lower-cased tag,
//! which it states as `pack-holds-only-folded-tag`. The case forms are chosen after every other
//! figure, each held at its printed form, with the equation that the forms sum to the tagged
//! literals added; only a form cell may be withheld, so no other figure depends on them. When a
//! hidden count would still be pinned, they are omitted as `small-counts-would-be-exposed`.
//! Equality across syntaxes holds for every figure except `case_forms`.
//!
//! # Memory
//!
//! The verb streams nothing: every figure needs the whole dataset. A source is read whole into
//! the dataset model before any figure is taken (a line-oriented
//! text source is parsed as a stream, but its dataset is resident all the same). On top of
//! the dataset the verb holds one small record per distinct term it reaches, one record per
//! blank-node incidence and the canonical labelling's lines when that figure is requested;
//! `--no-labelling` drops the last, and `--labelling-limit` bounds its search.

use std::convert::Infallible;

use purrdf::xsd::XsdDatatype;
use purrdf_core::canonical_labelling::{Limit, Refusal, Scope, ViewError, label_view};
use purrdf_core::{DatasetView, FastMap, FastSet, LanguageTagForms, TermRef};
use purrdf_rdf::SourceFormat;

use crate::cli::CliRdfFormat;
use crate::error::CliError;
use crate::shape_suppress::{Count, Linear, SMALL_COUNT_THRESHOLD, Sum, confined, suppress};
use crate::source::{InputView, ViewOp};
use crate::{format, source};

/// The version of the document layout. A figure added, removed or renamed raises it.
const SCHEMA_VERSION: u64 = 1;

/// The resolved `shape` flags.
pub(crate) struct ShapeOptions<'a> {
    /// `--from`: the input-format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the base IRI a relative reference in the input resolves against.
    pub(crate) base: Option<&'a str>,
    /// `--no-labelling`: omit the canonical labelling figure.
    pub(crate) no_labelling: bool,
    /// `--labelling-limit`: the limited units the labelling may spend; the engine limit when absent.
    pub(crate) labelling_limit: Option<u64>,
    /// The input path, or `-` for stdin.
    pub(crate) input: &'a str,
}

/// Run the `shape` subcommand.
///
/// A pack is verified before it is read, which canonicalizes it, so a pack input takes the
/// caller's `--canonical-search` and `--canonicalization-receipts` exactly as the other verbs
/// do. Any other input performs no canonicalization and the flags are refused for it.
///
/// # Errors
///
/// Every failure is reported by a fixed sentence that quotes nothing from the input, its
/// name, or the parser's message. The sentence names the class of the failure: a format that
/// cannot be inferred, an unreadable input, a pack read without canonical control, a pack
/// under a transport wrapper, a source that is not well formed, a labelling failure.
pub(crate) fn run(
    options: &ShapeOptions<'_>,
    canonical: &mut crate::canonical::Request,
) -> Result<(), CliError> {
    let source_format = format::resolve(options.from, options.input).map_err(|_| {
        CliError::Usage(
            "shape: the input format cannot be inferred; pass --from (no detail is printed, \
             because it could quote the input)"
                .to_owned(),
        )
    })?;
    let token = source_format.token();
    let is_pack = matches!(source_format, SourceFormat::Pack);
    if options.base.is_some() && matches!(source_format, SourceFormat::Gts | SourceFormat::Pack) {
        return Err(CliError::Usage(
            "shape: --base has no effect on a GTS or pack input, whose terms are already \
             resolved; remove it"
                .to_owned(),
        ));
    }
    if canonical.is_configured() && !is_pack {
        return Err(CliError::Usage(
            "shape: only a pack input canonicalizes; remove --canonical-search and \
             --canonicalization-receipts"
                .to_owned(),
        ));
    }
    if options.input != "-" && std::fs::File::open(options.input).is_err() {
        return Err(CliError::Runtime(
            "shape: the input cannot be opened (no detail is printed, because it could quote \
             the input's name)"
                .to_owned(),
        ));
    }
    if is_pack {
        // Name the missing control before the read, so it is not reported as a bad pack.
        canonical.control().map_err(|_| {
            CliError::Usage(
                "shape: a pack input is verified by canonicalizing it; supply --canonical-search \
                 unlimited or a canonical decimal u64 and --canonicalization-receipts DEST"
                    .to_owned(),
            )
        })?;
    }
    let labelling = if options.no_labelling {
        LabellingRequest::Disabled
    } else {
        LabellingRequest::Limit(options.labelling_limit.map_or(Limit::ENGINE, Limit::new))
    };
    let read_failure = || {
        CliError::Runtime(format!(
            "shape: the input could not be read as {token}; no detail is printed, because it \
             could quote the input"
        ))
    };
    let measured = if matches!(source_format, SourceFormat::Gts) {
        let (dataset, _ledger, forms) =
            source::load_gts_with_forms(options.input, source::TransportPolicy::Detect)
                .map_err(|_| read_failure())?;
        measure(&*dataset, labelling, CaseForms::Reported(forms))
    } else {
        source::run_over_input(
            options.input,
            source_format,
            options.base,
            None,
            ShapeOp {
                labelling,
                forms: CaseForms::Omitted(if is_pack {
                    PACK_OMISSION
                } else {
                    TEXT_OMISSION
                }),
            },
            canonical,
        )
        .map_err(|error| match error {
            CliError::Usage(_) if is_pack => CliError::Usage(
                "shape: the pack cannot be read through a transport wrapper; decompress it first"
                    .to_owned(),
            ),
            _ => read_failure(),
        })?
    }?;
    let document = measured.render()?;
    canonical.before_output()?;
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    out.write_all(document.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_| CliError::Runtime("shape: the figures could not be written".to_owned()))
}

/// The reason a text reader reports no case forms: it does not feed a case-form census.
const TEXT_OMISSION: &str = "text-reader-records-no-census";

/// The reason a pack reports no case forms: it holds only the lower-cased tag.
const PACK_OMISSION: &str = "pack-holds-only-folded-tag";

/// The reason the case forms are not printed when printing them would expose a small count.
const SUPPRESSION_OMISSION: &str = "small-counts-would-be-exposed";

/// The written case forms of the language tags, or why a reader has none.
#[derive(Clone, Copy)]
enum CaseForms {
    Reported(LanguageTagForms),
    Omitted(&'static str),
}

/// Whether and how the canonical labelling figure is taken.
#[derive(Clone, Copy)]
enum LabellingRequest {
    /// Not taken.
    Disabled,
    /// Taken within this limit.
    Limit(Limit),
}

/// The measurement as the generic operation [`source::run_over_input`] monomorphizes per view.
struct ShapeOp {
    labelling: LabellingRequest,
    forms: CaseForms,
}

impl ViewOp for ShapeOp {
    /// The inner result is the measurement's own: a source failure is the outer one.
    type Output = Result<Figures, CliError>;

    fn run<D: InputView>(
        self,
        view: &D,
        _canonical: &mut crate::canonical::Request,
    ) -> Result<Self::Output, CliError> {
        Ok(measure(view, self.labelling, self.forms))
    }
}

/// The class of a term, which is all the verb keeps of it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Iri,
    Blank,
    Literal,
    Triple,
}

/// The classes of the datatype of a literal that is not language-tagged.
#[derive(Clone, Copy)]
enum Datatype {
    Named(&'static str),
    Other,
}

/// A histogram over the fixed power-of-two bins.
#[derive(Clone)]
struct Pow2 {
    bins: [u64; 65],
    largest: u64,
}

impl Pow2 {
    const fn new() -> Self {
        Self {
            bins: [0; 65],
            largest: 0,
        }
    }

    fn add(&mut self, value: u64) {
        let bin = if value == 0 {
            0
        } else {
            value.bit_width() as usize
        };
        self.bins[bin] += 1;
        self.largest = self.largest.max(value);
    }

    /// The non-empty bins, as `(min, max, count)` in ascending order.
    fn cells(&self) -> Vec<(u64, u64, u64)> {
        self.bins
            .iter()
            .enumerate()
            .filter(|(_, count)| **count != 0)
            .map(|(bin, &count)| {
                let (min, max) = bin_bounds(bin);
                (min, max, count)
            })
            .collect()
    }
}

/// The histogram cells as a JSON array.
fn render_cells(cells: &[(u64, u64, Count)], out: &mut String) {
    out.push('[');
    for (index, (min, max, count)) in cells.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        put(
            out,
            format_args!("{{\"min\":{min},\"max\":{max},\"count\":{}}}", count.text()),
        );
    }
    out.push(']');
}

/// The inclusive bounds of histogram bin `bin`.
const fn bin_bounds(bin: usize) -> (u64, u64) {
    match bin {
        0 => (0, 0),
        64 => (1 << 63, u64::MAX),
        _ => (1 << (bin - 1), (1 << bin) - 1),
    }
}

/// Append formatted text to `out`.
fn put(out: &mut String, text: std::fmt::Arguments<'_>) {
    // INVARIANT: writing to a `String` never fails; the only error `write_fmt` can carry is
    // one the formatted value itself raises, and every value here is an integer or a literal.
    if std::fmt::Write::write_fmt(out, text).is_err() {
        unreachable!("a String accepts every formatted write");
    }
}

/// The canonical labelling figure.
enum Labelling {
    Disabled,
    Refused { name: &'static str, code: u8 },
    Labelled { lines: u64, bytes: u64 },
}

/// Every figure of a dataset.
struct Figures {
    quads: u64,
    iris: u64,
    literals: u64,
    blank_nodes: u64,
    named_graphs: u64,
    triple_terms: u64,
    reifiers: u64,
    annotations: u64,
    max_triple_depth: u64,
    sibling_parents: u64,
    sibling_largest: Pow2,
    literal_lengths: Pow2,
    named_datatypes: Vec<(&'static str, u64)>,
    other_datatypes: u64,
    tagged_literals: u64,
    distinct_tags: u64,
    /// The written case forms of the language tags, where the reader saw them written.
    forms: CaseForms,
    labelling: Labelling,
}

/// One edge at a blank node, as it reads from that node.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Edge<Id> {
    incoming: bool,
    predicate: Id,
    other: Other<Id>,
    graph: Option<Id>,
}

/// The far end of an [`Edge`], with every blank node but the parent made anonymous.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Other<Id> {
    Parent,
    Blank,
    Term(Id),
}

fn infallible<T>(result: Result<T, Infallible>) -> T {
    result.unwrap_or_else(|never| match never {})
}

/// Take every figure of `view`.
fn measure<D: InputView>(
    view: &D,
    labelling: LabellingRequest,
    forms: CaseForms,
) -> Result<Figures, CliError> {
    let mut census = Census::default();
    let mut quads = 0_u64;
    for quad in view.quads() {
        quads += 1;
        census.reach(view, quad.s);
        census.reach(view, quad.p);
        census.reach(view, quad.o);
        if let Some(graph) = quad.g {
            census.reach(view, graph);
        }
    }
    let mut reifiers = FastSet::default();
    for quad in view.reifier_quads() {
        reifiers.insert(quad.s);
        census.reach(view, quad.s);
        census.reach(view, quad.o);
        if let Some(graph) = quad.g {
            census.reach(view, graph);
        }
    }
    let mut annotations = 0_u64;
    for quad in view.annotation_quads() {
        annotations += 1;
        census.reach(view, quad.s);
        census.reach(view, quad.p);
        census.reach(view, quad.o);
        if let Some(graph) = quad.g {
            census.reach(view, graph);
        }
    }
    let mut named_graphs = 0_u64;
    for graph in view.named_graphs() {
        named_graphs += 1;
        census.reach(view, graph);
    }

    let (sibling_parents, sibling_largest) = like_siblings(view, &census);
    let max_triple_depth = census.max_triple_depth();
    let count_class =
        |class: Class| census.classes.values().filter(|&&c| c == class).count() as u64;
    let mut named: Vec<(&'static str, u64)> = census.named_datatypes.into_iter().collect();
    named.sort_unstable();
    Ok(Figures {
        quads,
        iris: count_class(Class::Iri),
        literals: count_class(Class::Literal),
        blank_nodes: count_class(Class::Blank),
        named_graphs,
        triple_terms: count_class(Class::Triple),
        reifiers: reifiers.len() as u64,
        annotations,
        max_triple_depth,
        sibling_parents,
        sibling_largest,
        literal_lengths: census.literal_lengths,
        named_datatypes: named,
        other_datatypes: census.other_datatypes,
        tagged_literals: census.tagged_literals,
        distinct_tags: census.tags.len() as u64,
        forms,
        labelling: label(view, labelling)?,
    })
}

/// The terms reached so far and what was read off each.
struct Census<Id> {
    classes: FastMap<Id, Class>,
    triples: FastMap<Id, [Id; 3]>,
    datatypes: FastMap<Id, Datatype>,
    literal_lengths: Pow2,
    named_datatypes: FastMap<&'static str, u64>,
    other_datatypes: u64,
    tagged_literals: u64,
    tags: FastSet<Box<str>>,
}

impl<Id> Default for Census<Id> {
    fn default() -> Self {
        Self {
            classes: FastMap::default(),
            triples: FastMap::default(),
            datatypes: FastMap::default(),
            literal_lengths: Pow2::new(),
            named_datatypes: FastMap::default(),
            other_datatypes: 0,
            tagged_literals: 0,
            tags: FastSet::default(),
        }
    }
}

impl<Id: purrdf_core::ViewTermId> Census<Id> {
    /// Visit `root` and, when it is a triple term, everything inside it, once each.
    fn reach<D: DatasetView<Id = Id, ReadError = Infallible>>(&mut self, view: &D, root: Id) {
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if self.classes.contains_key(&id) {
                continue;
            }
            let class = infallible(view.with_term(id, |term| match term {
                TermRef::Iri(_) => (Class::Iri, None),
                TermRef::Blank { .. } => (Class::Blank, None),
                TermRef::Literal {
                    lexical,
                    datatype,
                    language,
                    ..
                } => {
                    self.literal_lengths.add(lexical.len() as u64);
                    match language {
                        Some(tag) => {
                            self.tagged_literals += 1;
                            if !self.tags.contains(tag) {
                                self.tags.insert(tag.into());
                            }
                            (Class::Literal, None)
                        }
                        None => (Class::Literal, Some(datatype)),
                    }
                }
                TermRef::Triple { s, p, o } => {
                    self.triples.insert(id, [s, p, o]);
                    pending.extend([s, p, o]);
                    (Class::Triple, None)
                }
            }));
            self.classes.insert(id, class.0);
            if let Some(datatype) = class.1 {
                match self.datatype(view, datatype) {
                    Datatype::Named(name) => *self.named_datatypes.entry(name).or_insert(0) += 1,
                    Datatype::Other => self.other_datatypes += 1,
                }
            }
        }
    }

    fn datatype<D: DatasetView<Id = Id, ReadError = Infallible>>(
        &mut self,
        view: &D,
        id: Id,
    ) -> Datatype {
        *self.datatypes.entry(id).or_insert_with(|| {
            infallible(view.with_term(id, |term| match term {
                TermRef::Iri(iri) => named_xsd(iri).map_or(Datatype::Other, Datatype::Named),
                _ => Datatype::Other,
            }))
        })
    }

    /// The deepest triple term: a triple term holding none has depth one.
    fn max_triple_depth(&self) -> u64 {
        let mut depth: FastMap<Id, u64> = FastMap::default();
        let mut deepest = 0;
        for &root in self.triples.keys() {
            let mut stack = vec![root];
            while let Some(&top) = stack.last() {
                if depth.contains_key(&top) {
                    stack.pop();
                    continue;
                }
                let parts = self.triples[&top];
                let waiting: Vec<Id> = parts
                    .into_iter()
                    .filter(|part| self.triples.contains_key(part) && !depth.contains_key(part))
                    .collect();
                if waiting.is_empty() {
                    let inner = parts
                        .into_iter()
                        .filter_map(|part| depth.get(&part).copied())
                        .max()
                        .unwrap_or(0);
                    depth.insert(top, inner + 1);
                    deepest = deepest.max(inner + 1);
                    stack.pop();
                } else {
                    stack.extend(waiting);
                }
            }
        }
        deepest
    }
}

/// The local name of a named XSD datatype, or none for any other IRI.
///
/// The names are a closed list of W3C terms, so a printed name never carries data: the
/// value-space datatypes `purrdf_xsd` models, and the four further XSD datatypes RDF 1.1
/// Concepts lists as RDF-compatible (`language`, `Name`, `NCName`, `NMTOKEN`), which its
/// value space does not model.
fn named_xsd(iri: &str) -> Option<&'static str> {
    use purrdf::xsd::datatype::{XSD_LANGUAGE, XSD_NAME, XSD_NCNAME, XSD_NMTOKEN, XSD_NS};
    let known = XsdDatatype::from_iri(iri)
        .map(XsdDatatype::iri)
        .or_else(|| {
            [XSD_LANGUAGE, XSD_NAME, XSD_NCNAME, XSD_NMTOKEN]
                .into_iter()
                .find(|candidate| *candidate == iri)
        })?;
    known.strip_prefix(XSD_NS)
}

/// For each blank node with blank children, the size of its largest group of children that
/// share a first-degree description: the statements of the child read from the child, with
/// the parent and every other blank node left anonymous. Statements are the quads only.
fn like_siblings<D: DatasetView<ReadError = Infallible>>(
    view: &D,
    census: &Census<D::Id>,
) -> (u64, Pow2) {
    let blank = |id: &D::Id| census.classes.get(id) == Some(&Class::Blank);
    let mut children: FastMap<D::Id, Vec<D::Id>> = FastMap::default();
    let mut edges: FastMap<D::Id, Vec<Edge<D::Id>>> = FastMap::default();
    for quad in view.quads() {
        let (subject_blank, object_blank) = (blank(&quad.s), blank(&quad.o));
        if subject_blank && object_blank && quad.s != quad.o {
            children.entry(quad.s).or_default().push(quad.o);
        }
        if subject_blank {
            edges.entry(quad.s).or_default().push(Edge {
                incoming: false,
                predicate: quad.p,
                other: Other::Term(quad.o),
                graph: quad.g,
            });
        }
        if object_blank {
            edges.entry(quad.o).or_default().push(Edge {
                incoming: true,
                predicate: quad.p,
                other: Other::Term(quad.s),
                graph: quad.g,
            });
        }
    }
    let mut parents = 0_u64;
    let mut histogram = Pow2::new();
    for (parent, mut kids) in children {
        kids.sort_unstable();
        kids.dedup();
        let mut groups: FastMap<Vec<Edge<D::Id>>, u64> = FastMap::default();
        for kid in kids {
            let mut description: Vec<Edge<D::Id>> = edges.get(&kid).map_or_else(Vec::new, |list| {
                list.iter()
                    .map(|edge| {
                        let other = match edge.other {
                            Other::Term(id) if id == parent => Other::Parent,
                            Other::Term(id) if blank(&id) => Other::Blank,
                            keep => keep,
                        };
                        Edge {
                            other,
                            ..edge.clone()
                        }
                    })
                    .collect()
            });
            description.sort_unstable();
            description.dedup();
            *groups.entry(description).or_insert(0) += 1;
        }
        parents += 1;
        histogram.add(groups.values().copied().max().unwrap_or(0));
    }
    (parents, histogram)
}

/// The line weight of the canonical labelling, a typed refusal of the definition, or a
/// failure that is reported as one.
fn label<D: DatasetView<ReadError = Infallible>>(
    view: &D,
    request: LabellingRequest,
) -> Result<Labelling, CliError> {
    let LabellingRequest::Limit(limit) = request else {
        return Ok(Labelling::Disabled);
    };
    let failed = |what: &str| {
        CliError::Runtime(format!(
            "shape: the canonical labelling failed: {what}; no detail is printed, because it \
             could quote the input"
        ))
    };
    match label_view(view, &[], Scope::Dataset, limit, None) {
        Ok(labelling) => {
            let lines = &labelling.labelled().lines;
            Ok(Labelling::Labelled {
                lines: lines.len() as u64,
                bytes: lines.iter().map(|line| line.len() as u64).sum(),
            })
        }
        Err(ViewError::Refused(refusal)) => Ok(Labelling::Refused {
            name: match refusal {
                Refusal::Input { .. } => "input",
                Refusal::Graph { .. } => "graph",
                Refusal::Width { .. } => "width",
                Refusal::Limit { .. } => "limit",
            },
            code: refusal.code(),
        }),
        Err(ViewError::ForeignId) => Err(failed("a row names a term the view does not hold")),
        Err(ViewError::Storage(_)) => Err(failed("its working storage was refused")),
        Err(ViewError::Stopped) => Err(failed("it was stopped")),
        Err(ViewError::Read(never)) => match never {},
        Err(_) => Err(failed("an unclassified failure")),
    }
}

/// The printed counts of one document and the ties between them.
struct Plan {
    vars: Vec<(Count, u64)>,
    sums: Vec<Sum>,
    ties: Vec<Linear>,
    at: At,
}

/// Where each count sits in a [`Plan`].
struct At {
    quads: usize,
    iris: usize,
    literals: usize,
    blanks: usize,
    graphs: usize,
    triples: usize,
    reifiers: usize,
    annotations: usize,
    depth: usize,
    parents: usize,
    group: usize,
    tags: usize,
    lengths: usize,
    datatypes: usize,
    other: usize,
    tagged: usize,
    siblings: usize,
}

impl Figures {
    /// Every printed count and every tie between them, for the groups that are kept.
    ///
    /// The ties are facts of an RDF 1.2 dataset: the distinct literals are partitioned by length
    /// and by datatype, the parents by the size of their largest like group, and each
    /// literal is the object of a statement, an annotation or a triple term; a parent is a
    /// blank node with a statement of its own, and a group of like children needs that many
    /// blank nodes besides its parent; a triple term nests as deep as it has terms; an
    /// identifier occupies at most the positions of the statements that name it.
    fn plan(&self) -> Plan {
        let mut vars: Vec<(Count, u64)> = Vec::new();
        let push = |vars: &mut Vec<(Count, u64)>, printed: Count, truth: u64| {
            vars.push((printed, truth));
            vars.len() - 1
        };
        let counted = |n: u64| Count::of(n);
        let quads = push(&mut vars, counted(self.quads), self.quads);
        let iris = push(&mut vars, counted(self.iris), self.iris);
        let literals = push(&mut vars, counted(self.literals), self.literals);
        let blanks = push(&mut vars, counted(self.blank_nodes), self.blank_nodes);
        let graphs = push(&mut vars, counted(self.named_graphs), self.named_graphs);
        let triples = push(&mut vars, counted(self.triple_terms), self.triple_terms);
        let reifiers = push(&mut vars, counted(self.reifiers), self.reifiers);
        let annotations = push(&mut vars, counted(self.annotations), self.annotations);
        let depth = push(
            &mut vars,
            Count::Exact(self.max_triple_depth),
            self.max_triple_depth,
        );
        let parents = push(
            &mut vars,
            counted(self.sibling_parents),
            self.sibling_parents,
        );
        let group = push(
            &mut vars,
            Count::Exact(self.sibling_largest.largest),
            self.sibling_largest.largest,
        );
        let tags = push(&mut vars, counted(self.distinct_tags), self.distinct_tags);
        let lengths = vars.len();
        for cell in self.literal_lengths.cells() {
            push(&mut vars, counted(cell.2), cell.2);
        }
        let datatypes = vars.len();
        for (_, count) in &self.named_datatypes {
            push(&mut vars, counted(*count), *count);
        }
        let other = push(
            &mut vars,
            counted(self.other_datatypes),
            self.other_datatypes,
        );
        let tagged = push(
            &mut vars,
            counted(self.tagged_literals),
            self.tagged_literals,
        );
        let siblings = vars.len();
        let sibling_cells = self.sibling_largest.cells();
        for cell in &sibling_cells {
            push(&mut vars, counted(cell.2), cell.2);
        }
        let end = vars.len();

        let sums = vec![
            Sum {
                total: literals,
                cells: (lengths..datatypes).collect(),
            },
            Sum {
                total: literals,
                cells: (datatypes..siblings).collect(),
            },
            Sum {
                total: parents,
                cells: (siblings..end).collect(),
            },
        ];
        let one = |v: usize| (1, v);
        let mut ties = vec![
            // every literal is the object of a statement, an annotation or a triple term
            Linear::at_most(
                &[one(literals)],
                0,
                &[one(quads), one(annotations), one(triples)],
            ),
            // the datatype classes and the tagged literals are disjoint classes of literals,
            // and a tag is used by a tagged literal
            Linear::at_most(&[one(other), one(tagged)], 0, &[one(literals)]),
            Linear::at_most(&[one(tags)], 0, &[one(tagged)]),
            // a parent is a blank node with a statement of its own
            Linear::at_most(&[one(parents)], 0, &[one(blanks)]),
            Linear::at_most(&[one(parents)], 0, &[one(quads)]),
            // a triple term is nested as deep as its depth
            Linear::at_most(&[one(depth)], 0, &[one(triples)]),
            // blank nodes and identifiers occupy at most the positions that name them
            Linear::at_most(
                &[one(blanks)],
                0,
                &[
                    (2, quads),
                    (2, annotations),
                    (2, triples),
                    one(reifiers),
                    one(graphs),
                ],
            ),
            Linear::at_most(
                &[one(iris)],
                0,
                &[
                    (3, quads),
                    (3, annotations),
                    (3, triples),
                    one(reifiers),
                    one(graphs),
                ],
            ),
        ];
        // a bin or a datatype that is printed has a member, whatever its count prints as
        for index in (lengths..datatypes)
            .chain(datatypes..other)
            .chain(siblings..end)
        {
            ties.push(Linear::at_most(&[], 1, &[one(index)]));
        }
        if !sibling_cells.is_empty() {
            // each parent of a bin has at least the bin's minimum of like children and a
            // statement to each
            let weighted: Vec<(i128, usize)> = sibling_cells
                .iter()
                .enumerate()
                .map(|(index, cell)| (i128::from(cell.0), siblings + index))
                .collect();
            ties.push(Linear::at_most(&weighted, 0, &[one(quads)]));
            // the largest group lies within the highest bin that has a parent, and needs that
            // many blank nodes besides its parent
            if let Some(highest) = sibling_cells.last() {
                ties.push(Linear::at_most(&[], i128::from(highest.0), &[one(group)]));
                ties.push(Linear::at_most(&[one(group)], -i128::from(highest.1), &[]));
            }
            ties.push(Linear::at_most(&[one(group)], 1, &[one(blanks)]));
            ties.push(Linear::at_most(&[one(group)], 0, &[one(quads)]));
        }
        Plan {
            vars,
            sums,
            ties,
            at: At {
                quads,
                iris,
                literals,
                blanks,
                graphs,
                triples,
                reifiers,
                annotations,
                depth,
                parents,
                group,
                tags,
                lengths,
                datatypes,
                other,
                tagged,
                siblings,
            },
        }
    }

    /// The one JSON document, newline terminated.
    fn render(&self) -> Result<String, CliError> {
        // Choose every printed count together. A count that cannot be hidden by any other
        // form is left out, so the search ends with nothing pinned.
        let mut plan = self.plan();
        if !suppress(&mut plan.vars, &plan.sums, &plan.ties, &[], None) {
            return Err(CliError::Runtime(
                "shape: a small count could not be suppressed; no figures are printed".to_owned(),
            ));
        }
        // A maximum that the printed histogram bounds already confine to one value gives
        // nothing away, so it prints as that value.
        let bounds = confined(&plan.vars, &plan.sums, &plan.ties);
        for index in [plan.at.depth, plan.at.group] {
            if let (Some(low), Some(high)) = bounds[index]
                && low == high
            {
                plan.vars[index].0 = Count::Exact(low);
            }
        }
        let at = &plan.at;
        let vars = &plan.vars;
        let printed = |index: usize| vars[index].0;
        let length_cells: Vec<(u64, u64, Count)> = self
            .literal_lengths
            .cells()
            .iter()
            .zip(&vars[at.lengths..])
            .map(|(cell, var)| (cell.0, cell.1, var.0))
            .collect();
        let sibling_cells: Vec<(u64, u64, Count)> = self
            .sibling_largest
            .cells()
            .iter()
            .zip(&vars[at.siblings..])
            .map(|(cell, var)| (cell.0, cell.1, var.0))
            .collect();
        let named: Vec<(&str, Count)> = self
            .named_datatypes
            .iter()
            .map(|(name, _)| *name)
            .zip(vars[at.datatypes..at.other].iter().map(|var| var.0))
            .collect();
        // The case forms partition the tagged literals. They are chosen against every figure
        // already printed, each held at its printed form, and only a form cell may be
        // withheld, so no other figure depends on them. When a hidden count would still be
        // pinned, the forms are not printed.
        let case_cells: Result<[Count; 3], &str> = match self.forms {
            CaseForms::Omitted(reason) => Err(reason),
            CaseForms::Reported(forms) => {
                let mut joint = plan.vars.clone();
                let form_at = joint.len();
                joint.extend([forms.lower, forms.upper, forms.mixed].map(|n| (Count::of(n), n)));
                let mut joint_sums = plan.sums.clone();
                joint_sums.push(Sum {
                    total: at.tagged,
                    cells: (form_at..form_at + 3).collect(),
                });
                let fixed: Vec<usize> = (0..form_at).collect();
                let only_forms: Vec<usize> = (form_at..form_at + 3).collect();
                if suppress(
                    &mut joint,
                    &joint_sums,
                    &plan.ties,
                    &fixed,
                    Some(&only_forms),
                ) {
                    Ok([joint[form_at].0, joint[form_at + 1].0, joint[form_at + 2].0])
                } else {
                    Err(SUPPRESSION_OMISSION)
                }
            }
        };
        let (quads, reifiers) = (printed(at.quads), printed(at.reifiers));
        let (annotations, named_graphs) = (printed(at.annotations), printed(at.graphs));
        let literals = printed(at.literals);
        let parents = printed(at.parents);
        let other = printed(at.other);
        let tagged = printed(at.tagged);

        let mut out = String::new();
        put(
            &mut out,
            format_args!(
                "{{\"schema_version\":{SCHEMA_VERSION},\"small_count_threshold\":{SMALL_COUNT_THRESHOLD},\"counts\":{{\"quads\":{},\"distinct_iris\":{},\"distinct_literals\":{},\"blank_nodes\":{},\"named_graphs\":{},\"triple_terms\":{},\"reifiers\":{},\"annotations\":{}}},\"max_triple_term_depth\":{},",
                quads.text(),
                printed(at.iris).text(),
                literals.text(),
                printed(at.blanks).text(),
                named_graphs.text(),
                printed(at.triples).text(),
                reifiers.text(),
                annotations.text(),
                printed(at.depth).text(),
            ),
        );
        put(
            &mut out,
            format_args!(
                "\"blank_siblings\":{{\"parents\":{},\"max_largest_like_group\":{},\"largest_like_group_histogram\":",
                parents.text(),
                printed(at.group).text(),
            ),
        );
        render_cells(&sibling_cells, &mut out);
        out.push_str("},\"literal_length_bytes_histogram\":");
        render_cells(&length_cells, &mut out);
        out.push_str(",\"datatypes\":{\"named_xsd\":{");
        for (index, (name, count)) in named.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            put(&mut out, format_args!("\"{name}\":{}", count.text()));
        }
        out.push('}');
        put(
            &mut out,
            format_args!(
                ",\"other\":{}}},\"language_tags\":{{\"tagged_literals\":{},\"distinct_tags\":{},\"case_forms\":",
                other.text(),
                tagged.text(),
                printed(at.tags).text(),
            ),
        );
        match case_cells {
            Ok([lower, upper, mixed]) => put(
                &mut out,
                format_args!(
                    "{{\"status\":\"reported\",\"lower\":{},\"upper\":{},\"mixed\":{}}}",
                    lower.text(),
                    upper.text(),
                    mixed.text()
                ),
            ),
            Err(reason) => put(
                &mut out,
                format_args!("{{\"status\":\"omitted\",\"reason\":\"{reason}\"}}"),
            ),
        }
        out.push_str("},\"canonical_labelling\":");
        // The line count is the sum of these statements, so it prints exactly only when each
        // of them does.
        let lines_derivable = [quads, reifiers, annotations, named_graphs]
            .iter()
            .all(|count| count.is_exact());
        match &self.labelling {
            Labelling::Disabled => {
                out.push_str("{\"status\":\"omitted\",\"reason\":\"disabled\"}");
            }
            Labelling::Refused { name, code } => put(
                &mut out,
                format_args!("{{\"status\":\"refused\",\"refusal\":\"{name}\",\"code\":{code}}}"),
            ),
            Labelling::Labelled { lines, .. } if *lines < SMALL_COUNT_THRESHOLD => {
                out.push_str("{\"status\":\"omitted\",\"reason\":\"below-threshold\"}");
            }
            Labelling::Labelled { .. } if !lines_derivable => {
                out.push_str("{\"status\":\"omitted\",\"reason\":\"derived-count-suppressed\"}");
            }
            Labelling::Labelled { lines, bytes } => {
                let mean_thousandths = u128::from(*bytes) * 1000 / u128::from(*lines);
                put(
                    &mut out,
                    format_args!(
                        "{{\"status\":\"labelled\",\"lines\":{lines},\"line_bytes_total\":{bytes},\"line_bytes_mean_thousandths\":{mean_thousandths}}}"
                    ),
                );
            }
        }
        out.push_str("}\n");
        Ok(out)
    }
}
