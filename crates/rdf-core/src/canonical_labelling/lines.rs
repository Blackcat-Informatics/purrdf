// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed lines, their admission and their one byte encoding.
//!
//! A caller hands the labelling a set of [`Line`]s over [`Term`]s. Admission
//! interns every constant leaf (an IRI or a literal) by its encoding and every
//! blank node by its source identifier, and stores each line as a sequence of
//! [`Tok`]ens in walk order: the structural tags, the annotation count, the
//! interned constants and the blank leaves. Every encoding the definition
//! orders (a masked line, an entry, a canonical line) is a function of those
//! tokens and of what each blank leaf is replaced by, so nothing is ever spelt
//! as text and nothing is parsed back.

use crate::FastMap;

/// The tag of an IRI leaf.
pub(crate) const TAG_IRI: u8 = 0x01;
/// The tag of a literal leaf.
pub(crate) const TAG_LITERAL: u8 = 0x02;
/// The tag of a triple term.
pub(crate) const TAG_TRIPLE: u8 = 0x03;
/// The tag of a free blank leaf.
pub(crate) const TAG_FREE: u8 = 0x04;
/// The tag of a fixed blank leaf, followed by its eight-byte position.
pub(crate) const TAG_FIXED: u8 = 0x05;
/// The tag of an absent option.
pub(crate) const TAG_ABSENT: u8 = 0x20;
/// The tag of a present option, followed by its value.
pub(crate) const TAG_PRESENT: u8 = 0x21;
/// The tag of the left-to-right base direction.
pub(crate) const TAG_LTR: u8 = 0x30;
/// The tag of the right-to-left base direction.
pub(crate) const TAG_RTL: u8 = 0x31;

/// The base direction of a directional literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    /// Left to right.
    Ltr,
    /// Right to left.
    Rtl,
}

/// A term of a line: one of the four variants of the definition. Absence is
/// always `None`, never an empty byte string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Term {
    /// An IRI, by its bytes.
    Iri(Box<[u8]>),
    /// A literal with its datatype and its optional language and direction.
    Literal {
        /// The lexical form.
        lexical: Box<[u8]>,
        /// The datatype IRI's bytes.
        datatype: Box<[u8]>,
        /// The language tag, compared as bytes.
        language: Option<Box<[u8]>>,
        /// The base direction.
        direction: Option<Direction>,
    },
    /// A blank node, by its opaque source identifier.
    Blank(Box<[u8]>),
    /// A triple term of three terms, nesting allowed.
    Triple(Box<[Self; 3]>),
}

impl Term {
    /// An IRI term.
    #[must_use]
    pub fn iri(bytes: &[u8]) -> Self {
        Self::Iri(bytes.into())
    }

    /// A blank node term.
    #[must_use]
    pub fn blank(source: &[u8]) -> Self {
        Self::Blank(source.into())
    }

    /// A literal with a datatype and no language or direction.
    #[must_use]
    pub fn typed(lexical: &[u8], datatype: &[u8]) -> Self {
        Self::Literal {
            lexical: lexical.into(),
            datatype: datatype.into(),
            language: None,
            direction: None,
        }
    }

    /// A triple term.
    #[must_use]
    pub fn triple(s: Self, p: Self, o: Self) -> Self {
        Self::Triple(Box::new([s, p, o]))
    }

    /// The code of the term's variant in the `graph` refusal.
    pub(crate) const fn variant(&self) -> GraphTerm {
        match self {
            Self::Iri(_) => GraphTerm::Iri,
            Self::Literal { .. } => GraphTerm::Literal,
            Self::Blank(_) => GraphTerm::Blank,
            Self::Triple(_) => GraphTerm::Triple,
        }
    }
}

/// A line: one of the five kinds. A graph slot of `None` is the default graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Line {
    /// One statement.
    Quad {
        /// Subject.
        subject: Term,
        /// Predicate; admits only an IRI.
        predicate: Term,
        /// Object.
        object: Term,
        /// The graph that holds the statement.
        graph: Option<Term>,
    },
    /// A reifier standing for a triple term.
    ReifierBinding {
        /// The reifier.
        reifier: Term,
        /// The triple term; admits only a triple.
        triple: Term,
        /// The graph that holds the binding.
        graph: Option<Term>,
    },
    /// One property of a reifier, with the multiplicity a format carries it.
    Annotation {
        /// The multiplicity, at least one.
        count: u64,
        /// The reifier.
        reifier: Term,
        /// Predicate; admits only an IRI.
        predicate: Term,
        /// Object.
        object: Term,
        /// The graph that holds the annotation.
        graph: Option<Term>,
    },
    /// The named graph of this name is in the dataset and is empty.
    EmptyGraph(Term),
    /// A caller's record: an opaque payload, compared whole and never read,
    /// and an ordered list of terms whose blank leaves are occurrences like
    /// any other. It denotes nothing in RDF: no graph slot, no part in W5.
    Record {
        /// The payload; may be empty.
        payload: Box<[u8]>,
        /// The terms, in order; may be empty.
        terms: Vec<Term>,
    },
}

/// The kind tags of the definition's closed list of lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// `quad`.
    Quad = 0x40,
    /// `reifier-binding`.
    ReifierBinding = 0x41,
    /// `annotation`.
    Annotation = 0x42,
    /// `empty-graph`.
    EmptyGraph = 0x43,
    /// `record`.
    Record = 0x44,
}

/// The variant of a term held by an `empty-graph` line and carried by another
/// line, with the blank node's source identifier as the diagnostic field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphTerm {
    /// An IRI (code 0).
    Iri,
    /// A literal (code 1).
    Literal,
    /// A blank node (code 2) and its source identifier.
    Blank,
    /// A triple term (code 3).
    Triple,
}

impl GraphTerm {
    /// The variant's code.
    #[must_use]
    pub const fn code(&self) -> u8 {
        match self {
            Self::Iri => 0,
            Self::Literal => 1,
            Self::Blank => 2,
            Self::Triple => 3,
        }
    }
}

/// Why an input is not well formed (`input` refusal reasons, by code).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InputReason {
    /// A slot holds a variant it does not admit (code 0).
    Slot = 0,
    /// An annotation count is zero (code 1).
    Count = 1,
    /// Two lines agree in every field other than the count (code 2).
    Repeated = 2,
    /// A kind tag outside the closed list (code 3). Typed lines cannot carry
    /// one; the variant completes the closed list for interfaces that receive
    /// a kind as a number.
    Kind = 3,
}

/// The quantity whose value would not fit in 64 bits (`width` refusal).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Quantity {
    /// Blank nodes (code 0).
    Nodes = 0,
    /// Lines (code 1).
    Lines = 1,
    /// Incidences (code 2).
    Incidences = 2,
    /// Occurrences (code 3).
    Occurrences = 3,
    /// The length of a byte string or an encoding (code 4).
    Length = 4,
    /// A count (code 5).
    Count = 5,
    /// A counter of units the engine forms (code 6); the allowance saturates
    /// instead and is never a quantity.
    Units = 6,
}

/// One element of a stored line, in walk order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Tok {
    /// A structural tag byte: the kind, a triple term, an option.
    Tag(u8),
    /// The annotation count.
    Count(u64),
    /// The number of terms of a record. Not the annotation count: W3, which
    /// ignores the count, compares it like any field.
    Len(u64),
    /// An interned constant leaf, by index into the constant table.
    Const(usize),
    /// A blank leaf, by node index.
    Blank(usize),
}

/// What a blank leaf becomes in an encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// `free`.
    Free,
    /// `fixed(q)`.
    Fixed(u64),
}

/// An admitted, well-formed set of typed lines.
#[derive(Clone, Debug, Default)]
pub struct LineSet {
    /// The encodings of the distinct constant leaves.
    pub(crate) consts: Vec<Box<[u8]>>,
    /// The source identifiers of the blank nodes, by node index.
    pub(crate) nodes: Vec<Box<[u8]>>,
    /// The lines as tokens.
    pub(crate) lines: Vec<Box<[Tok]>>,
}

impl LineSet {
    /// The number of lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the set holds no line.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The number of blank nodes.
    #[must_use]
    pub fn blank_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// The source identifier of node `node`.
    #[must_use]
    pub fn source(&self, node: usize) -> &[u8] {
        &self.nodes[node]
    }

    /// The blank occurrences of line `line` in walk order.
    pub(crate) fn occurrences(&self, line: usize) -> impl Iterator<Item = usize> + '_ {
        self.lines[line].iter().filter_map(|t| match t {
            Tok::Blank(b) => Some(*b),
            _ => None,
        })
    }

    /// The encoding of line `line` with every blank leaf replaced by `leaf`.
    pub(crate) fn encode(&self, line: usize, leaf: impl FnMut(usize) -> Leaf, out: &mut Vec<u8>) {
        encode_tokens(&self.consts, &self.lines[line], leaf, out);
    }

    /// The length of the encoding of line `line` with every leaf free.
    pub(crate) fn free_len(&self, line: usize) -> u64 {
        self.lines[line]
            .iter()
            .map(|t| match t {
                Tok::Tag(_) => 1,
                Tok::Count(_) | Tok::Len(_) => 8,
                Tok::Const(c) => self.consts[*c].len() as u64,
                Tok::Blank(_) => 1,
            })
            .sum()
    }
}

/// Append the encoding of `tokens` to `out`.
pub(crate) fn encode_tokens(
    consts: &[Box<[u8]>],
    tokens: &[Tok],
    mut leaf: impl FnMut(usize) -> Leaf,
    out: &mut Vec<u8>,
) {
    for token in tokens {
        match *token {
            Tok::Tag(tag) => out.push(tag),
            Tok::Count(n) | Tok::Len(n) => out.extend_from_slice(&n.to_be_bytes()),
            Tok::Const(c) => out.extend_from_slice(&consts[c]),
            Tok::Blank(b) => match leaf(b) {
                Leaf::Free => out.push(TAG_FREE),
                Leaf::Fixed(q) => {
                    out.push(TAG_FIXED);
                    out.extend_from_slice(&q.to_be_bytes());
                }
            },
        }
    }
}

/// `bytes(b)` of the line encoding: the length of `b` as eight BIG-endian
/// bytes, then `b`. The definition fixes the order (its 3.7: every integer is
/// `u64(n)`, big-endian, so integers inside an encoding compare numerically
/// bytewise); the workspace frame of `purrdf_hash::frame` is little-endian
/// and would change every published encoding, identity and digest.
pub(crate) fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}

/// The encoding of an IRI leaf.
pub(crate) fn iri_encoding(value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + value.len());
    out.push(TAG_IRI);
    push_bytes(&mut out, value);
    out
}

/// The encoding of a literal leaf.
pub(crate) fn literal_encoding(
    lexical: &[u8],
    datatype: &[u8],
    language: Option<&[u8]>,
    direction: Option<Direction>,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(TAG_LITERAL);
    push_bytes(&mut out, lexical);
    push_bytes(&mut out, datatype);
    match language {
        None => out.push(TAG_ABSENT),
        Some(language) => {
            out.push(TAG_PRESENT);
            push_bytes(&mut out, language);
        }
    }
    match direction {
        None => out.push(TAG_ABSENT),
        Some(direction) => {
            out.push(TAG_PRESENT);
            out.push(match direction {
                Direction::Ltr => TAG_LTR,
                Direction::Rtl => TAG_RTL,
            });
        }
    }
    out
}

/// Interns constants and blank nodes while lines are admitted.
#[derive(Default)]
pub(crate) struct Interner {
    pub(crate) set: LineSet,
    const_index: FastMap<Box<[u8]>, usize>,
    node_index: FastMap<Box<[u8]>, usize>,
}

impl Interner {
    /// The index of a constant leaf of encoding `encoding`.
    pub(crate) fn constant(&mut self, encoding: &[u8]) -> usize {
        if let Some(&c) = self.const_index.get(encoding) {
            return c;
        }
        let c = self.set.consts.len();
        self.set.consts.push(encoding.into());
        self.const_index.insert(encoding.into(), c);
        c
    }

    /// The index of the blank node of source identifier `source`.
    pub(crate) fn node(&mut self, source: &[u8]) -> usize {
        if let Some(&b) = self.node_index.get(source) {
            return b;
        }
        let b = self.set.nodes.len();
        self.set.nodes.push(source.into());
        self.node_index.insert(source.into(), b);
        b
    }

    /// The tokens of `term` in walk order (`s`, `p`, `o` inside a triple
    /// term), over a heap work list so that nesting depth costs no stack.
    fn term(&mut self, term: &Term, out: &mut Vec<Tok>) {
        let mut pending = vec![term];
        while let Some(term) = pending.pop() {
            match term {
                Term::Blank(source) => out.push(Tok::Blank(self.node(source))),
                Term::Triple(parts) => {
                    out.push(Tok::Tag(TAG_TRIPLE));
                    pending.extend(parts.iter().rev());
                }
                Term::Iri(value) => {
                    let c = self.constant(&iri_encoding(value));
                    out.push(Tok::Const(c));
                }
                Term::Literal {
                    lexical,
                    datatype,
                    language,
                    direction,
                } => {
                    let encoding =
                        literal_encoding(lexical, datatype, language.as_deref(), *direction);
                    out.push(Tok::Const(self.constant(&encoding)));
                }
            }
        }
    }

    fn graph(&mut self, graph: Option<&Term>, out: &mut Vec<Tok>) {
        match graph {
            None => out.push(Tok::Tag(TAG_ABSENT)),
            Some(term) => {
                out.push(Tok::Tag(TAG_PRESENT));
                self.term(term, out);
            }
        }
    }

    /// The tokens of `line`.
    pub(crate) fn line(&mut self, line: &Line) -> Vec<Tok> {
        let mut out = Vec::new();
        match line {
            Line::Quad {
                subject,
                predicate,
                object,
                graph,
            } => {
                out.push(Tok::Tag(Kind::Quad as u8));
                self.term(subject, &mut out);
                self.term(predicate, &mut out);
                self.term(object, &mut out);
                self.graph(graph.as_ref(), &mut out);
            }
            Line::ReifierBinding {
                reifier,
                triple,
                graph,
            } => {
                out.push(Tok::Tag(Kind::ReifierBinding as u8));
                self.term(reifier, &mut out);
                self.term(triple, &mut out);
                self.graph(graph.as_ref(), &mut out);
            }
            Line::Annotation {
                count,
                reifier,
                predicate,
                object,
                graph,
            } => {
                out.push(Tok::Tag(Kind::Annotation as u8));
                out.push(Tok::Count(*count));
                self.term(reifier, &mut out);
                self.term(predicate, &mut out);
                self.term(object, &mut out);
                self.graph(graph.as_ref(), &mut out);
            }
            Line::EmptyGraph(term) => {
                out.push(Tok::Tag(Kind::EmptyGraph as u8));
                self.term(term, &mut out);
            }
            Line::Record { payload, terms } => {
                out.push(Tok::Tag(Kind::Record as u8));
                // The payload is one constant whose encoding is
                // `bytes(payload)`; only the tokens of a record's payload
                // slot ever hold it, so it is never compared with a leaf.
                let mut encoding = Vec::with_capacity(8 + payload.len());
                push_bytes(&mut encoding, payload);
                out.push(Tok::Const(self.constant(&encoding)));
                out.push(Tok::Len(terms.len() as u64));
                for term in terms {
                    self.term(term, &mut out);
                }
            }
        }
        out
    }
}

/// The refusal reasons of the input conditions W1 to W5, each the smallest
/// that applies, and the admitted set when none applies.
pub(crate) struct Admission {
    pub(crate) set: LineSet,
    pub(crate) input: Option<InputReason>,
    pub(crate) graph: Option<(GraphTerm, Option<Box<[u8]>>)>,
}

/// Admit `lines`: intern them and check W1 to W5.
pub(crate) fn admit<'a>(lines: impl IntoIterator<Item = &'a Line>) -> Admission {
    let mut interner = Interner::default();
    let mut input: Option<InputReason> = None;
    let mut note = |reason: InputReason| {
        input = Some(input.map_or(reason, |r| r.min(reason)));
    };
    // Lines by their tokens without the count (W3).
    let mut seen: FastMap<Vec<Tok>, ()> = FastMap::default();
    // Graph terms carried by statement lines and held by empty-graph lines.
    let mut carried: Vec<(Vec<Tok>, GraphTerm)> = Vec::new();
    let mut empties: Vec<(Vec<Tok>, GraphTerm)> = Vec::new();
    for line in lines {
        let iri = |t: &Term| matches!(t, Term::Iri(_));
        let slots_ok = match line {
            Line::Quad { predicate, .. } | Line::Annotation { predicate, .. } => iri(predicate),
            Line::ReifierBinding { triple, .. } => matches!(triple, Term::Triple(_)),
            // Every term of a record's list admits every variant.
            Line::EmptyGraph(_) | Line::Record { .. } => true,
        };
        if !slots_ok {
            note(InputReason::Slot);
        }
        if let Line::Annotation { count: 0, .. } = line {
            note(InputReason::Count);
        }
        let tokens = interner.line(line);
        let uncounted: Vec<Tok> = tokens
            .iter()
            .filter(|t| !matches!(t, Tok::Count(_)))
            .copied()
            .collect();
        if seen.insert(uncounted, ()).is_some() {
            note(InputReason::Repeated);
        }
        let graph_tokens = |term: &Term, interner: &mut Interner| {
            let mut out = Vec::new();
            interner.term(term, &mut out);
            (out, term.variant())
        };
        match line {
            Line::Quad { graph: Some(g), .. }
            | Line::ReifierBinding { graph: Some(g), .. }
            | Line::Annotation { graph: Some(g), .. } => {
                carried.push(graph_tokens(g, &mut interner));
            }
            Line::EmptyGraph(g) => empties.push(graph_tokens(g, &mut interner)),
            // No graph slot, or the default graph; a record denotes nothing
            // and takes no part in W5.
            Line::Quad { graph: None, .. }
            | Line::ReifierBinding { graph: None, .. }
            | Line::Annotation { graph: None, .. }
            | Line::Record { .. } => {}
        }
        interner.set.lines.push(tokens.into_boxed_slice());
    }
    // W5: the smallest term code, and for a blank the smallest source.
    let carried: FastMap<Vec<Tok>, ()> = carried.into_iter().map(|(t, _)| (t, ())).collect();
    let mut graph: Option<(GraphTerm, Option<Box<[u8]>>)> = None;
    for (tokens, variant) in empties {
        if !carried.contains_key(&tokens) {
            continue;
        }
        let source = match (&variant, tokens.as_slice()) {
            (GraphTerm::Blank, [Tok::Blank(b)]) => Some(interner.set.nodes[*b].clone()),
            _ => None,
        };
        let better = graph
            .as_ref()
            .is_none_or(|(v, s)| (variant.code(), source.as_deref()) < (v.code(), s.as_deref()));
        if better {
            graph = Some((variant, source));
        }
    }
    Admission {
        set: interner.set,
        input,
        graph,
    }
}
