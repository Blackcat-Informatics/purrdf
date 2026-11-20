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
//! # Language-tag case
//!
//! The data model lowercases a language tag when the term is interned, so the case form of
//! an authored tag does not survive reading, and a figure about it would differ with the
//! reader rather than with the data. The document states that as a typed omission.
//!
//! # Memory
//!
//! A source is read whole into the dataset model before any figure is taken (a line-oriented
//! text source is parsed as a stream, but its dataset is resident all the same). On top of
//! the dataset the verb holds one small record per distinct term it reaches, one record per
//! blank-node incidence and the canonical labelling's lines when that figure is requested;
//! `--no-labelling` drops the last, and `--labelling-limit` bounds its search.

use std::convert::Infallible;

use purrdf::xsd::XsdDatatype;
use purrdf_core::canonical_labelling::{Limit, Refusal, Scope, ViewError, label_view};
use purrdf_core::{DatasetView, FastMap, FastSet, TermRef};

use crate::cli::CliRdfFormat;
use crate::error::CliError;
use crate::source::{InputView, ViewOp};
use crate::{format, source};

/// The version of the document layout. A figure added, removed or renamed raises it.
const SCHEMA_VERSION: u64 = 1;

/// A count below this prints as `"<5"` rather than exactly; see the module documentation.
pub(crate) const SMALL_COUNT_THRESHOLD: u64 = 5;

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
/// # Errors
///
/// Every failure to resolve the format or to read the source is reported by a fixed sentence
/// that quotes nothing from the input, its name, or the parser's message.
pub(crate) fn run(options: &ShapeOptions<'_>) -> Result<(), CliError> {
    let source_format = format::resolve(options.from, options.input).map_err(|_| {
        CliError::Usage(
            "shape: the input format cannot be inferred; pass --from (no detail is printed, \
             because it could quote the input)"
                .to_owned(),
        )
    })?;
    let token = source_format.token();
    let labelling = if options.no_labelling {
        LabellingRequest::Disabled
    } else {
        LabellingRequest::Limit(options.labelling_limit.map_or(Limit::ENGINE, Limit::new))
    };
    let mut canonical = crate::canonical::Request::new(None, None)?;
    let document = source::run_over_input(
        options.input,
        source_format,
        options.base,
        None,
        ShapeOp { labelling },
        &mut canonical,
    )
    .map_err(|_| {
        CliError::Runtime(format!(
            "shape: the input could not be read as {token}; no detail is printed, because it \
             could quote the input"
        ))
    })?;
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
    type Output = String;

    fn run<D: InputView>(
        self,
        view: &D,
        _canonical: &mut crate::canonical::Request,
    ) -> Result<String, CliError> {
        Ok(measure(view, self.labelling).render())
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

    fn render(&self, out: &mut String) {
        out.push('[');
        let mut first = true;
        for (bin, &count) in self
            .bins
            .iter()
            .enumerate()
            .filter(|(_, count)| **count != 0)
        {
            if !first {
                out.push(',');
            }
            first = false;
            let (min, max) = bin_bounds(bin);
            put(
                out,
                format_args!(
                    "{{\"min\":{min},\"max\":{max},\"count\":{}}}",
                    count_text(count)
                ),
            );
        }
        out.push(']');
    }
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

/// A count as JSON: exact, or the string `"<5"` when it is one to four.
fn count_text(count: u64) -> String {
    if (1..SMALL_COUNT_THRESHOLD).contains(&count) {
        format!("\"<{SMALL_COUNT_THRESHOLD}\"")
    } else {
        count.to_string()
    }
}

/// The canonical labelling figure.
enum Labelling {
    Disabled,
    Refused { name: &'static str, code: u8 },
    Unavailable,
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
fn measure<D: InputView>(view: &D, labelling: LabellingRequest) -> Figures {
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
    Figures {
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
        labelling: label(view, labelling),
    }
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
                TermRef::Iri(iri) => XsdDatatype::from_iri(iri).map_or(Datatype::Other, |named| {
                    Datatype::Named(xsd_local(named.iri()))
                }),
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

/// The local name of an XSD datatype IRI, which is a W3C term and so safe to print.
fn xsd_local(iri: &'static str) -> &'static str {
    iri.rsplit_once('#').map_or(iri, |(_, local)| local)
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

/// The line weight of the canonical labelling, or the typed reason there is none.
fn label<D: DatasetView>(view: &D, request: LabellingRequest) -> Labelling {
    let LabellingRequest::Limit(limit) = request else {
        return Labelling::Disabled;
    };
    match label_view(view, &[], Scope::Dataset, limit, None) {
        Ok(labelling) => {
            let lines = &labelling.labelled().lines;
            Labelling::Labelled {
                lines: lines.len() as u64,
                bytes: lines.iter().map(|line| line.len() as u64).sum(),
            }
        }
        Err(ViewError::Refused(refusal)) => Labelling::Refused {
            name: match refusal {
                Refusal::Input { .. } => "input",
                Refusal::Graph { .. } => "graph",
                Refusal::Width { .. } => "width",
                Refusal::Limit { .. } => "limit",
            },
            code: refusal.code(),
        },
        Err(_) => Labelling::Unavailable,
    }
}

impl Figures {
    /// The one JSON document, newline terminated.
    fn render(&self) -> String {
        let mut out = String::new();
        put(
            &mut out,
            format_args!(
                "{{\"schema_version\":{SCHEMA_VERSION},\"small_count_threshold\":{SMALL_COUNT_THRESHOLD},\"counts\":{{\"quads\":{},\"distinct_iris\":{},\"distinct_literals\":{},\"blank_nodes\":{},\"named_graphs\":{},\"triple_terms\":{},\"reifiers\":{},\"annotations\":{}}},\"max_triple_term_depth\":{},",
                count_text(self.quads),
                count_text(self.iris),
                count_text(self.literals),
                count_text(self.blank_nodes),
                count_text(self.named_graphs),
                count_text(self.triple_terms),
                count_text(self.reifiers),
                count_text(self.annotations),
                self.max_triple_depth,
            ),
        );
        put(
            &mut out,
            format_args!(
                "\"blank_siblings\":{{\"parents\":{},\"max_largest_like_group\":{},\"largest_like_group_histogram\":",
                count_text(self.sibling_parents),
                self.sibling_largest.largest,
            ),
        );
        self.sibling_largest.render(&mut out);
        out.push_str("},\"literal_length_bytes_histogram\":");
        self.literal_lengths.render(&mut out);
        out.push_str(",\"datatypes\":{\"named_xsd\":{");
        for (index, (name, count)) in self.named_datatypes.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            put(&mut out, format_args!("\"{name}\":{}", count_text(*count)));
        }
        put(
            &mut out,
            format_args!(
                "}},\"other\":{}}},\"language_tags\":{{\"tagged_literals\":{},\"distinct_tags\":{},\"case_forms\":{{\"status\":\"omitted\",\"reason\":\"language-tag-case-not-retained\"}}}},\"canonical_labelling\":",
                count_text(self.other_datatypes),
                count_text(self.tagged_literals),
                count_text(self.distinct_tags),
            ),
        );
        match &self.labelling {
            Labelling::Disabled => {
                out.push_str("{\"status\":\"omitted\",\"reason\":\"disabled\"}");
            }
            Labelling::Unavailable => {
                out.push_str("{\"status\":\"omitted\",\"reason\":\"unavailable\"}");
            }
            Labelling::Refused { name, code } => put(
                &mut out,
                format_args!("{{\"status\":\"refused\",\"refusal\":\"{name}\",\"code\":{code}}}"),
            ),
            Labelling::Labelled { lines, bytes } => {
                if *lines < SMALL_COUNT_THRESHOLD {
                    put(
                        &mut out,
                        format_args!(
                            "{{\"status\":\"omitted\",\"reason\":\"below-threshold\",\"lines\":{}}}",
                            count_text(*lines)
                        ),
                    );
                } else {
                    let mean_thousandths = u128::from(*bytes) * 1000 / u128::from(*lines);
                    put(
                        &mut out,
                        format_args!(
                            "{{\"status\":\"labelled\",\"lines\":{lines},\"line_bytes_total\":{bytes},\"line_bytes_mean_thousandths\":{mean_thousandths}}}"
                        ),
                    );
                }
            }
        }
        out.push_str("}\n");
        out
    }
}
