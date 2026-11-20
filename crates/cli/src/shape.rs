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
//! shared by five or more cannot be traced to one of them. A zero names no item. Maxima and
//! nesting depths are single extremes of a distribution rather than counts of items and print
//! exactly.
//!
//! # Subtraction
//!
//! A total that is the sum of the cells of a partition would give a suppressed cell by
//! subtraction, and two partitions of one total bound each other. [`suppress`] treats each
//! printed form as an interval, propagates the sum equations to find what a reader learns, and
//! withholds further cells until every hidden cell is confined to exactly its own printed form.
//!
//! # Language-tag case
//!
//! The data model lowercases a language tag when the term is interned, so only a reader that
//! sees the tag as written can report its case form. A GTS read does, through the envelope's
//! `language_tag_forms`. The text and pack readers do not hand on the distinct-literal count,
//! which needs the dataset builder, and the document states that as a typed omission. Equality
//! across syntaxes holds for every figure except this one.
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
use crate::shape_suppress::{Count, SMALL_COUNT_THRESHOLD, Sum, suppress};
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
        measure(&*dataset, labelling, Some(forms))
    } else {
        source::run_over_input(
            options.input,
            source_format,
            options.base,
            None,
            ShapeOp { labelling },
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
    let document = measured.render();
    canonical.before_output()?;
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    out.write_all(document.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_| CliError::Runtime("shape: the figures could not be written".to_owned()))
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
}

impl ViewOp for ShapeOp {
    /// The inner result is the measurement's own: a source failure is the outer one.
    type Output = Result<Figures, CliError>;

    fn run<D: InputView>(
        self,
        view: &D,
        _canonical: &mut crate::canonical::Request,
    ) -> Result<Self::Output, CliError> {
        Ok(measure(view, self.labelling, None))
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
    forms: Option<LanguageTagForms>,
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
    forms: Option<LanguageTagForms>,
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

impl Figures {
    /// The one JSON document, newline terminated.
    fn render(&self) -> String {
        let length_raw = self.literal_lengths.cells();
        let sibling_cells: Vec<(u64, u64, Count)> = self
            .sibling_largest
            .cells()
            .into_iter()
            .map(|(min, max, count)| (min, max, Count::of(count)))
            .collect();
        let sibling_counts: Vec<Count> = sibling_cells.iter().map(|cell| cell.2).collect();

        // The distinct literals are partitioned by length and by datatype (the
        // language-tagged literals being one datatype cell, themselves partitioned by case
        // form where the reader saw the tag as written). Every printed figure is chosen
        // together, so no hidden cell is pinned by the others.
        let mut vars: Vec<(Count, u64)> = vec![(Count::of(self.literals), self.literals)];
        let length_at = vars.len();
        vars.extend(length_raw.iter().map(|cell| (Count::of(cell.2), cell.2)));
        let datatype_at = vars.len();
        vars.extend(
            self.named_datatypes
                .iter()
                .map(|(_, count)| (Count::of(*count), *count)),
        );
        let other_at = vars.len();
        vars.push((Count::of(self.other_datatypes), self.other_datatypes));
        let tagged_at = vars.len();
        vars.push((Count::of(self.tagged_literals), self.tagged_literals));
        let form_at = vars.len();
        if let Some(forms) = self.forms {
            vars.extend([forms.lower, forms.upper, forms.mixed].map(|n| (Count::of(n), n)));
        }
        let mut sums = vec![
            Sum {
                total: 0,
                cells: (length_at..datatype_at).collect(),
            },
            Sum {
                total: 0,
                cells: (datatype_at..form_at).collect(),
            },
        ];
        if self.forms.is_some() {
            sums.push(Sum {
                total: tagged_at,
                cells: (form_at..form_at + 3).collect(),
            });
        }
        suppress(&mut vars, &sums);
        let length_cells: Vec<(u64, u64, Count)> = length_raw
            .iter()
            .zip(&vars[length_at..])
            .map(|(cell, var)| (cell.0, cell.1, var.0))
            .collect();
        let literals = vars[0].0;
        let named: Vec<(&str, Count)> = self
            .named_datatypes
            .iter()
            .map(|(name, _)| *name)
            .zip(vars[datatype_at..other_at].iter().map(|var| var.0))
            .collect();
        let other = vars[other_at].0;
        let tagged = vars[tagged_at].0;
        let case_cells: Option<[Count; 3]> = self
            .forms
            .map(|_| [vars[form_at].0, vars[form_at + 1].0, vars[form_at + 2].0]);
        let parents = Count::total(self.sibling_parents, &sibling_counts);
        let quads = Count::of(self.quads);
        let reifiers = Count::of(self.reifiers);
        let annotations = Count::of(self.annotations);
        let named_graphs = Count::of(self.named_graphs);

        let mut out = String::new();
        put(
            &mut out,
            format_args!(
                "{{\"schema_version\":{SCHEMA_VERSION},\"small_count_threshold\":{SMALL_COUNT_THRESHOLD},\"counts\":{{\"quads\":{},\"distinct_iris\":{},\"distinct_literals\":{},\"blank_nodes\":{},\"named_graphs\":{},\"triple_terms\":{},\"reifiers\":{},\"annotations\":{}}},\"max_triple_term_depth\":{},",
                quads.text(),
                Count::of(self.iris).text(),
                literals.text(),
                Count::of(self.blank_nodes).text(),
                named_graphs.text(),
                Count::of(self.triple_terms).text(),
                reifiers.text(),
                annotations.text(),
                self.max_triple_depth,
            ),
        );
        put(
            &mut out,
            format_args!(
                "\"blank_siblings\":{{\"parents\":{},\"max_largest_like_group\":{},\"largest_like_group_histogram\":",
                parents.text(),
                self.sibling_largest.largest,
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
        put(
            &mut out,
            format_args!(
                "}},\"other\":{}}},\"language_tags\":{{\"tagged_literals\":{},\"distinct_tags\":{},\"case_forms\":",
                other.text(),
                tagged.text(),
                Count::of(self.distinct_tags).text(),
            ),
        );
        match case_cells {
            Some([lower, upper, mixed]) => put(
                &mut out,
                format_args!(
                    "{{\"status\":\"reported\",\"lower\":{},\"upper\":{},\"mixed\":{}}}",
                    lower.text(),
                    upper.text(),
                    mixed.text()
                ),
            ),
            None => {
                out.push_str("{\"status\":\"omitted\",\"reason\":\"needs-dataset-builder\"}");
            }
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
        out
    }
}
