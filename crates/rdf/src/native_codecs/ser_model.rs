// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party in-memory serialization model + RDF text serializers.
//!
//! [`SerGraph`] is the first-party term/quad/reifier/annotation shape the frozen
//! [`RdfDataset`](crate::RdfDataset) IR is lowered into before egress. The Turtle /
//! TriG / N-Triples / N-Quads serializers walk exactly this shape, emitting literal
//! lexical forms VERBATIM — they never canonicalize a literal's value-space nor narrow
//! its datatype (the whole point of the native codec: byte-for-byte lexical fidelity).

use std::borrow::Cow;

use crate::RdfDiagnostic;

/// The kind of a serialization term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SerTermKind {
    Iri,
    Bnode,
    Literal,
    Triple,
}

/// A single RDF term in the serialization model, carried by integer id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SerTerm {
    /// Term kind.
    pub kind: SerTermKind,
    /// IRI string, literal lexical form, or blank-node label (scope-local).
    pub value: Option<String>,
    /// Term-id of the literal's datatype IRI, when explicit.
    pub datatype: Option<usize>,
    /// Literal language tag (BCP 47).
    pub lang: Option<String>,
    /// RDF 1.2 literal base direction (`"ltr"` or `"rtl"`).
    pub direction: Option<String>,
    /// Term-id of the reifier of a quoted triple (`kind == Triple`).
    pub reifier: Option<usize>,
}

/// A quad of term-ids; the graph slot is `None` for the default graph.
pub(crate) type SerQuad = (usize, usize, usize, Option<usize>);
/// A subject/predicate/object triple of term-ids.
pub(crate) type SerTriple3 = (usize, usize, usize);
/// A reifier row: `(reifier, (s, p, o), graph?)`.
pub(crate) type SerReifierRow = (usize, SerTriple3, Option<usize>);
/// An annotation row: `(reifier, predicate, value, graph?)`.
pub(crate) type SerAnnotationRow = (usize, usize, usize, Option<usize>);

/// The serialization graph: terms plus the base quads and the RDF 1.2 statement layer
/// (reifier bindings + annotations). Each row carries an `Option<usize>` graph slot
/// (`None` = default graph).
#[derive(Debug, Default)]
pub(crate) struct SerGraph {
    pub terms: Vec<SerTerm>,
    pub quads: Vec<SerQuad>,
    pub reifiers: Vec<SerReifierRow>,
    pub annotations: Vec<SerAnnotationRow>,
}

impl SerGraph {
    /// Look up a reifier binding: the `(s, p, o)` of the FIRST `reifiers` row whose id
    /// equals `rid`.
    pub(crate) fn reifier(&self, rid: usize) -> Option<SerTriple3> {
        self.reifiers
            .iter()
            .find(|(r, _, _)| *r == rid)
            .map(|(_, spo, _)| *spo)
    }

    /// Reorder the base quads and the RDF 1.2 statement layer into a **canonical,
    /// backend-independent** order, keyed on each row's rendered term text.
    ///
    /// The term-table indices a [`SerGraph`] carries are assigned in the interning
    /// order of whichever [`DatasetView`](crate::DatasetView) fed the builder, so two
    /// backends holding the SAME dataset (e.g. the production `RdfDataset` and a
    /// `PackView` over its pack bytes) index their terms differently and thus iterate
    /// quads in different orders. Sorting on the *rendered value* — a pure function of
    /// the term, identical across backends — makes the emitted document byte-identical
    /// regardless of backend and removes the interning-order dependence from the
    /// serializer — serializers are byte-deterministic.
    ///
    /// The lookup in [`Self::reifier`] is by id, so permuting `reifiers` never changes
    /// which binding a quoted-triple term resolves to; the self-reifier sentinel rows
    /// (skipped on output) are permuted harmlessly among the real reifier rows.
    pub(crate) fn sort_canonical(&mut self) {
        // The rows are sorted THROUGH a comparator that renders on demand, rather than
        // by precomputing a key per row. The keys were `Vec<String>` — four or five
        // allocations for every quad, reifier and annotation in the document, every one
        // of them alive for the whole sort — and they were compared element by element
        // and then thrown away. Two reusable buffers give the identical ordering: the
        // comparison is still term-by-term on rendered text, and a `None` graph still
        // renders as the empty string, which is what places it before any named graph.
        //
        // The trade is deliberate. This renders O(n log n) times instead of n, and holds
        // O(1) scratch instead of O(n) keys. It is the right way round for a serializer,
        // whose peak is what decides whether a large export completes at all — and the
        // rendering it repeats is a walk over a term table already in memory.
        //
        // Each vector is taken out before sorting so the comparator can borrow `self`
        // immutably while the rows it orders are a local.
        let mut left = String::new();
        let mut right = String::new();

        let mut quads = std::mem::take(&mut self.quads);
        quads.sort_by(|&(s1, p1, o1, g1), &(s2, p2, o2, g2)| {
            cmp_terms(self, &[s1, p1, o1], &[s2, p2, o2], &mut left, &mut right)
                .then_with(|| cmp_graph(self, g1, g2, &mut left, &mut right))
        });
        self.quads = quads;

        let mut reifiers = std::mem::take(&mut self.reifiers);
        reifiers.sort_by(|&(r1, (s1, p1, o1), g1), &(r2, (s2, p2, o2), g2)| {
            cmp_terms(
                self,
                &[r1, s1, p1, o1],
                &[r2, s2, p2, o2],
                &mut left,
                &mut right,
            )
            .then_with(|| cmp_graph(self, g1, g2, &mut left, &mut right))
        });
        self.reifiers = reifiers;

        let mut annotations = std::mem::take(&mut self.annotations);
        annotations.sort_by(|&(r1, p1, o1, g1), &(r2, p2, o2, g2)| {
            cmp_terms(self, &[r1, p1, o1], &[r2, p2, o2], &mut left, &mut right)
                .then_with(|| cmp_graph(self, g1, g2, &mut left, &mut right))
        });
        self.annotations = annotations;
    }
}

/// Crockford Base32 alphabet (the ULID rendering alphabet).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// A rendered ULID is 26 Crockford Base32 digits.
const ULID_LEN: usize = 26;

/// A deterministic blank-node label with the given `prefix`, byte-identical to the
/// prior purrdf-gts `deterministic_label(prefix, counter)`: `prefix` plus the 26-digit
/// Crockford Base32 rendering of a zero-timestamp ULID built from `counter`.
///
/// With a zero timestamp the rendered ULID value equals `counter` for any
/// `counter < 2^80`, so this renders the 128-bit big-endian value `counter as u128`
/// as 26 Crockford Base32 digits, digit `i` being `(value >> (125 - i*5)) & 0x1f`.
pub(crate) fn deterministic_blank_label_with_prefix(prefix: &str, counter: usize) -> String {
    let value = counter as u128;
    let mut buffer = [0u8; ULID_LEN];
    for (index, byte) in buffer.iter_mut().enumerate() {
        let shift = 125 - index * 5;
        let digit = ((value >> shift) & 0x1f) as usize;
        *byte = CROCKFORD[digit];
    }
    // The buffer is ASCII (every byte comes from the Crockford alphabet), so the
    // UTF-8 conversion never fails.
    let rendered = std::str::from_utf8(&buffer).expect("Crockford digits are ASCII");
    format!("{prefix}{rendered}")
}

/// A deterministic blank-node label, byte-identical to the prior purrdf-gts
/// `deterministic_label("gts_", counter)`. See
/// [`deterministic_blank_label_with_prefix`].
pub(crate) fn deterministic_blank_label(counter: usize) -> String {
    deterministic_blank_label_with_prefix("gts_", counter)
}

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Return whether `direction` is a valid RDF 1.2 base direction token.
fn is_literal_direction(direction: &str) -> bool {
    matches!(direction, "ltr" | "rtl")
}

/// Bytes that pass through an `IRIREF` body untouched: printable ASCII
/// (`0x21..=0x7E`) minus the nine grammar-forbidden delimiters
/// (`<`, `>`, `"`, `{`, `}`, `|`, `^`, `` ` ``, `\`). Space (`0x20`), every control
/// (C0/DEL), and any byte `>= 0x80` (which may lead a C1 control) are `false`, so they
/// fall through to per-char classification.
const IRI_CLEAN: [bool; 256] = {
    let mut t = [false; 256];
    let mut i = 0x21usize;
    while i <= 0x7E {
        t[i] = !matches!(
            i as u8,
            b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'{' | b'|' | b'}'
        );
        i += 1;
    }
    t
};

/// Uppercase hex-nibble lookup table for `push_uchar_00`.
const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Bytes that pass through a literal lexical form untouched: printable ASCII
/// (`0x20..=0x7E`) minus `"` and `\`. C0/DEL controls, the two ASCII escapables, and
/// any byte `>= 0x80` (which may lead a C1 control that must ride as `\uXXXX`) are
/// `false`, so they fall through to per-char classification.
const LITERAL_CLEAN: [bool; 256] = {
    let mut t = [false; 256];
    let mut i = 0x20usize;
    while i <= 0x7E {
        t[i] = i != b'"' as usize && i != b'\\' as usize;
        i += 1;
    }
    t
};

/// Scan-first escape: copy maximal runs of `clean` bytes wholesale (one `push_str`),
/// routing only each boundary char through `escape_one` (the per-char escape logic).
///
/// This is byte-identical to a per-char loop whose clean arm is `out.push(c)`: the
/// clean run — the vast majority of every production IRI / literal — is batched
/// instead of pushed a char at a time, and every non-clean char takes the exact same
/// `escape_one` decision it would have taken per-char. `clean` marks only single-byte
/// ASCII as clean, so the first non-clean byte is always a UTF-8 char boundary.
///
/// When every byte of `s` is `clean` (the stated common case: every production IRI,
/// numeric/plain literals), this borrows `s` directly rather than allocating and
/// copying — a single linear scan replaces the wasted `String::with_capacity` + copy.
#[inline]
fn escape_scan<'a>(
    s: &'a str,
    clean: &[bool; 256],
    escape_one: impl Fn(&mut String, char),
) -> Cow<'a, str> {
    let bytes = s.as_bytes();
    if bytes.iter().all(|&b| clean[b as usize]) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut run_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if clean[bytes[i] as usize] {
            i += 1;
            continue;
        }
        if run_start < i {
            out.push_str(&s[run_start..i]);
        }
        let c = s[i..]
            .chars()
            .next()
            .expect("first non-clean byte is a char boundary");
        escape_one(&mut out, c);
        i += c.len_utf8();
        run_start = i;
    }
    if run_start < bytes.len() {
        out.push_str(&s[run_start..]);
    }
    Cow::Owned(out)
}

/// Push the `\u00XX` UCHAR escape for a code point known to be `<= 0xFF`
/// (every escapable byte here: C0/DEL/C1 controls, space, and the IRIREF
/// grammar delimiters). Byte-identical to `write!(out, "\\u{:04X}", v)` for
/// `v <= 0xFF`, without the `fmt` machinery.
#[inline]
fn push_uchar_00(out: &mut String, v: u32) {
    debug_assert!(v <= 0xFF);
    out.push_str("\\u00");
    out.push(HEX_UPPER[((v >> 4) & 0xF) as usize] as char);
    out.push(HEX_UPPER[(v & 0xF) as usize] as char);
}

/// Escape an IRI body for an N-Triples / Turtle / TriG `<…>` `IRIREF`. The W3C grammar
/// forbids `<`, `>`, `"`, `{`, `}`, `|`, `^`, `` ` ``, `\`, the space character, and every
/// control code point (C0 `0x00-0x1F`, DEL `0x7F`, and the C1 block `0x80-0x9F`) appearing
/// raw; each rides as a `\uXXXX` `UCHAR` (the text parser decodes them back). A clean ASCII
/// IRI (every production IRI) passes through byte-for-byte unchanged.
pub(crate) fn escape_iri(iri: &str) -> Cow<'_, str> {
    escape_scan(iri, &IRI_CLEAN, |out, ch| match ch {
        '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
            push_uchar_00(out, ch as u32);
        }
        c if c.is_control() || c == ' ' => {
            push_uchar_00(out, c as u32);
        }
        c => out.push(c),
    })
}

/// Escape a literal lexical form for N-Triples. Escapes `\` and `"`, emits the readable ECHAR
/// forms for `\n`/`\r`/`\t`, and rides EVERY other control character (C0, DEL, and the C1 block
/// `0x80-0x9F`) as `\uXXXX`. This deliberately escapes MORE than the W3C-pinned canonical form
/// (`purrdf_core::ir::canon::write_literal_escaped`, which keeps C1 raw): this serializer's
/// output is embedded verbatim inside an XML text node by the CL-dialect carrier, and an XML
/// parser normalizes/replaces raw C1 code points on read — so the payload only survives an XML
/// round-trip if the full control range rides as ASCII `\uXXXX`. The canonical form answers to
/// RDFC-1.0 byte-conformance; this one answers to XML transport.
pub(crate) fn escape_literal(lex: &str) -> Cow<'_, str> {
    escape_scan(lex, &LITERAL_CLEAN, |out, ch| match ch {
        '\\' => out.push_str("\\\\"),
        '"' => out.push_str("\\\""),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if c.is_control() => {
            push_uchar_00(out, c as u32);
        }
        c => out.push(c),
    })
}

/// Render a term-id as an N-Triples token.
/// Compare two term-id sequences by their rendered text, position by position.
///
/// The sequences are the same length at every call site. `left` and `right` are scratch
/// reused across the whole sort, which is the point: this is the comparison a
/// `Vec<String>` key used to make after allocating one string per position per row.
fn cmp_terms(
    g: &SerGraph,
    a: &[usize],
    b: &[usize],
    left: &mut String,
    right: &mut String,
) -> std::cmp::Ordering {
    for (&x, &y) in a.iter().zip(b.iter()) {
        left.clear();
        right.clear();
        write_term(g, x, left);
        write_term(g, y, right);
        let ordering = left.as_str().cmp(right.as_str());
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    std::cmp::Ordering::Equal
}

/// Compare two optional graph slots by rendered text, `None` rendering as empty.
///
/// Empty is what `unwrap_or_default` produced for the absent graph in the previous key,
/// and it is load-bearing: the empty string sorts before every rendered term, so the
/// default graph's rows lead. Rendering `None` as anything else would reorder the
/// document.
fn cmp_graph(
    g: &SerGraph,
    a: Option<usize>,
    b: Option<usize>,
    left: &mut String,
    right: &mut String,
) -> std::cmp::Ordering {
    left.clear();
    right.clear();
    if let Some(x) = a {
        write_term(g, x, left);
    }
    if let Some(y) = b {
        write_term(g, y, right);
    }
    left.as_str().cmp(right.as_str())
}

/// Append one term's N-Triples surface to `out`.
///
/// Appends rather than returns. Building each term as its own `String` cost an
/// allocation per term — three or four per quad, every one of them alive until the
/// whole document had been assembled — and every byte was going to be copied into the
/// output anyway. A literal with a datatype paid twice over, since the datatype's IRI
/// was rendered into a `String` only to be `format!`ed into the literal's.
///
/// `escape_iri` and `escape_literal` already return a [`Cow`], so an unescaped value —
/// which is nearly all of them — is borrowed straight from the term table and reaches
/// `out` without an intermediate of any kind.
fn write_term(g: &SerGraph, tid: usize, out: &mut String) {
    use std::fmt::Write as _;

    let t = &g.terms[tid];
    match t.kind {
        SerTermKind::Iri => {
            out.push('<');
            out.push_str(&escape_iri(t.value.as_deref().unwrap_or("")));
            out.push('>');
        }
        SerTermKind::Bnode => match &t.value {
            Some(v) => {
                out.push_str("_:");
                out.push_str(v);
            }
            None => {
                let _ = write!(out, "_:b{tid}");
            }
        },
        SerTermKind::Literal => {
            out.push('"');
            out.push_str(&escape_literal(t.value.as_deref().unwrap_or("")));
            out.push('"');
            if let Some(lang) = &t.lang {
                out.push('@');
                out.push_str(lang);
                if let Some(direction) = t.direction.as_deref().filter(|d| is_literal_direction(d))
                {
                    out.push_str("--");
                    out.push_str(direction);
                }
            } else if let Some(dt) = t.datatype {
                out.push_str("^^");
                write_term(g, dt, out);
            }
            // else: plain literal == xsd:string, written bare
        }
        // quoted triple (RDF 1.2 triple term), resolved through its reifier
        SerTermKind::Triple => match t.reifier.and_then(|rf| g.reifier(rf)) {
            Some((s, p, o)) => {
                out.push_str("<<( ");
                write_term(g, s, out);
                out.push(' ');
                write_term(g, p, out);
                out.push(' ');
                write_term(g, o, out);
                out.push_str(" )>>");
            }
            // degraded but syntactically valid: an unbound reifier becomes a blank node
            None => {
                let _ = write!(out, "_:unbound_triple_{tid}");
            }
        },
    }
}

/// Append a [`SerGraph`]'s N-Quads text to `out`.
///
/// One line is built at a time into `out` itself. The previous shape collected a
/// `String` per line into a `Vec`, `join`ed it — copying the whole document — and then
/// `format!`ed the result to add a trailing newline, copying the whole document a
/// second time. Peak was therefore about twice the output on top of one live `String`
/// per quad; Turtle paid a third copy by wrapping this function's result.
pub(crate) fn write_nquads(g: &SerGraph, out: &mut String) {
    let mut any = false;

    for &(s, p, o, gname) in &g.quads {
        write_term(g, s, out);
        out.push(' ');
        write_term(g, p, out);
        out.push(' ');
        write_term(g, o, out);
        write_graph_terminator(g, gname, out);
        any = true;
    }

    for &(rid, (s, p, o), gname) in &g.reifiers {
        if g.terms
            .get(rid)
            .is_some_and(|term| term.kind == SerTermKind::Triple && term.reifier == Some(rid))
        {
            continue;
        }
        write_term(g, rid, out);
        out.push_str(" <");
        out.push_str(RDF_REIFIES);
        out.push_str("> <<( ");
        write_term(g, s, out);
        out.push(' ');
        write_term(g, p, out);
        out.push(' ');
        write_term(g, o, out);
        out.push_str(" )>>");
        write_graph_terminator(g, gname, out);
        any = true;
    }

    for &(r, p, v, gname) in &g.annotations {
        write_term(g, r, out);
        out.push(' ');
        write_term(g, p, out);
        out.push(' ');
        write_term(g, v, out);
        write_graph_terminator(g, gname, out);
        any = true;
    }

    let _ = any;
}

/// Close one N-Quads statement: the optional graph name, the `.`, and the line break.
fn write_graph_terminator(g: &SerGraph, gname: Option<usize>, out: &mut String) {
    if let Some(gv) = gname {
        out.push(' ');
        write_term(g, gv, out);
    }
    out.push_str(" .\n");
}

/// Assert that no row of `g` carries a named-graph slot — the single-graph syntaxes
/// (N-Triples, Turtle) cannot serialize named-graph quads. Mirrors the upstream
/// `ensure_default_graph_projection` rejection.
fn ensure_default_graph_projection(g: &SerGraph, format: &str) -> Result<(), RdfDiagnostic> {
    let named = g.quads.iter().any(|(_, _, _, gname)| gname.is_some())
        || g.reifiers.iter().any(|(_, _, gname)| gname.is_some())
        || g.annotations.iter().any(|(_, _, _, gname)| gname.is_some());
    if named {
        return Err(RdfDiagnostic::error(
            "native-codec-serialize",
            format!("{format} cannot serialize a named graph"),
        ));
    }
    Ok(())
}

/// Serialise a [`SerGraph`] to N-Triples text (default graph only).
pub(crate) fn write_ntriples(g: &SerGraph, out: &mut String) -> Result<(), RdfDiagnostic> {
    ensure_default_graph_projection(g, "N-Triples")?;
    write_nquads(g, out);
    Ok(())
}

/// Serialise a [`SerGraph`] to Turtle text (default graph only); the N-Quads body is
/// prefixed with the `rdf:`/`xsd:` `@prefix` header. IRIs in the body stay full
/// `<...>` — they are NOT abbreviated against the declared prefixes.
pub(crate) fn write_turtle(g: &SerGraph, out: &mut String) -> Result<(), RdfDiagnostic> {
    ensure_default_graph_projection(g, "Turtle")?;

    // The header is written first and RETRACTED if the body turns out to be empty,
    // rather than the body being built into its own `String` so its emptiness can be
    // tested before deciding. Building it separately meant the whole document was
    // copied a second time to place it after the header — on top of the two copies
    // `to_nquads` itself was making — so a Turtle export peaked at roughly three times
    // its own output.
    let start = out.len();
    out.push_str("@prefix rdf: <");
    out.push_str(RDF_NS);
    out.push_str("> .\n@prefix xsd: <");
    out.push_str(XSD_NS);
    out.push_str("> .\n\n");
    let header = out.len();

    write_nquads(g, out);
    if out.len() == header {
        // An empty graph emits nothing at all, header included — unchanged behaviour.
        // Truncating to where this call began (not `clear`) is what lets a caller write
        // more than one document into one buffer.
        out.truncate(start);
    }
    Ok(())
}

// ── TriG ──────────────────────────────────────────────────────────────────────────

/// Append one term's TriG surface to `out`.
///
/// TriG differs from N-Triples in exactly one place — `rdf:reifies` is written through
/// the declared prefix rather than as a full IRI — so this mirrors [`write_term`] and
/// appends for the same reason: a term built as its own `String` is an allocation whose
/// every byte was going to be copied into the output regardless.
fn write_trig_term(g: &SerGraph, tid: usize, out: &mut String) {
    use std::fmt::Write as _;

    let t = &g.terms[tid];
    match t.kind {
        SerTermKind::Iri if t.value.as_deref() == Some(RDF_REIFIES) => out.push_str("rdf:reifies"),
        SerTermKind::Iri => {
            out.push('<');
            out.push_str(&escape_iri(t.value.as_deref().unwrap_or("")));
            out.push('>');
        }
        SerTermKind::Bnode => match &t.value {
            Some(v) => {
                out.push_str("_:");
                out.push_str(v);
            }
            None => {
                let _ = write!(out, "_:b{tid}");
            }
        },
        SerTermKind::Literal => {
            out.push('"');
            out.push_str(&escape_literal(t.value.as_deref().unwrap_or("")));
            out.push('"');
            if let Some(lang) = &t.lang {
                out.push('@');
                out.push_str(lang);
                if let Some(direction) = t.direction.as_deref().filter(|d| is_literal_direction(d))
                {
                    out.push_str("--");
                    out.push_str(direction);
                }
            } else if let Some(dt) = t.datatype {
                out.push_str("^^");
                write_trig_term(g, dt, out);
            }
        }
        SerTermKind::Triple => match t.reifier.and_then(|rf| g.reifier(rf)) {
            Some((s, p, o)) => {
                out.push_str("<<( ");
                write_trig_term(g, s, out);
                out.push(' ');
                write_trig_term(g, p, out);
                out.push(' ');
                write_trig_term(g, o, out);
                out.push_str(" )>>");
            }
            None => write_term(g, tid, out),
        },
    }
}

/// Close the open `GRAPH { … }` block, if one is open.
fn close_graph(out: &mut String, open_graph: &mut Option<String>) {
    if open_graph.take().is_some() {
        out.push_str("}\n");
    }
}

/// Put `out` in the right block for `graph_name` and write the statement's indent.
///
/// The caller then appends the statement's own terms directly, rather than handing over
/// a finished `String`. `open_graph` still holds the RENDERED graph name because that
/// name is what decides whether the next statement continues this block or starts
/// another — but it is now rebuilt only when the graph CHANGES, not once per statement.
fn begin_statement(
    out: &mut String,
    open_graph: &mut Option<String>,
    graph: &SerGraph,
    graph_name: Option<usize>,
) {
    let Some(gid) = graph_name else {
        close_graph(out, open_graph);
        return;
    };
    let mut rendered = String::new();
    write_trig_term(graph, gid, &mut rendered);
    if open_graph.as_deref() != Some(rendered.as_str()) {
        close_graph(out, open_graph);
        out.push_str(&rendered);
        out.push_str(" {\n");
        *open_graph = Some(rendered);
    }
    out.push_str("  ");
}

/// Append a [`SerGraph`]'s TriG text to `out`.
///
/// Statements are written in place. The previous shape collected every line into a
/// `Vec<String>`, `join`ed it — copying the whole document — and then `format!`ed the
/// result to add a trailing newline, copying it again. Writing each line followed by
/// its own newline produces exactly those bytes: a join with `"\n"` plus one trailing
/// `"\n"` is the same sequence as one `"\n"` after each line.
pub(crate) fn write_trig(g: &SerGraph, out: &mut String) {
    if g.quads.is_empty() && g.reifiers.is_empty() && g.annotations.is_empty() {
        return;
    }

    out.push_str("@prefix rdf: <");
    out.push_str(RDF_NS);
    out.push_str("> .\n\n");
    let mut open_graph: Option<String> = None;

    for &(s, p, o, gname) in &g.quads {
        begin_statement(out, &mut open_graph, g, gname);
        write_trig_term(g, s, out);
        out.push(' ');
        write_trig_term(g, p, out);
        out.push(' ');
        write_trig_term(g, o, out);
        out.push_str(" .\n");
    }

    for &(rid, (s, p, o), gname) in &g.reifiers {
        // A triple TERM keys its own components under its own id (a self-reference, not
        // a reifier relationship); rendering it as `<<( … )>> rdf:reifies <<( … )>>`
        // would assert a triple term in subject position. Its components are already
        // carried inline wherever the term appears, so skip the entry.
        if g.terms
            .get(rid)
            .is_some_and(|t| t.kind == SerTermKind::Triple && t.reifier == Some(rid))
        {
            continue;
        }
        begin_statement(out, &mut open_graph, g, gname);
        write_trig_term(g, rid, out);
        out.push_str(" rdf:reifies <<( ");
        write_trig_term(g, s, out);
        out.push(' ');
        write_trig_term(g, p, out);
        out.push(' ');
        write_trig_term(g, o, out);
        out.push_str(" )>> .\n");
    }

    for &(r, p, v, gname) in &g.annotations {
        begin_statement(out, &mut open_graph, g, gname);
        write_trig_term(g, r, out);
        out.push(' ');
        write_trig_term(g, p, out);
        out.push(' ');
        write_trig_term(g, v, out);
        out.push_str(" .\n");
    }

    close_graph(out, &mut open_graph);
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fmt::Write as _;

    // Collect-into-a-`String` shims. Production has no such function any more: every
    // caller reaches the writers through `RdfCodec::serialize_into` and supplies its own
    // buffer, so a `to_*` in the crate proper would be dead code kept alive by tests.
    // These assert on a whole document, which is the one place materialising it is the
    // point rather than a cost.

    /// The lazy comparator orders rows exactly as the materialised keys did.
    ///
    /// `sort_canonical` used to build a `Vec<String>` key per row and sort on that. The
    /// keys are gone; the ORDER they produced is a published property, because it is
    /// what makes an exported document byte-identical across backends. So the old key
    /// construction is reproduced here and the two orderings are compared directly —
    /// asserting the replacement is equivalent, not merely that it is some valid order.
    ///
    /// The graph is built to exercise the part a round-trip vector is least likely to:
    /// rows identical in subject, predicate and object that differ ONLY in the graph
    /// slot, including the absent one. The old key rendered `None` through
    /// `unwrap_or_default` as the empty string, which sorts before every rendered term
    /// and therefore puts the default graph's rows first — behaviour the comparator has
    /// to reproduce deliberately rather than inherit.
    #[test]
    fn the_lazy_comparator_orders_rows_exactly_as_the_materialised_keys_did() {
        fn term(g: &mut SerGraph, iri: &str) -> usize {
            g.terms.push(SerTerm {
                kind: SerTermKind::Iri,
                value: Some(iri.to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });
            g.terms.len() - 1
        }

        let mut g = SerGraph::default();
        let s = term(&mut g, "https://example.org/s");
        let p = term(&mut g, "https://example.org/p");
        let o = term(&mut g, "https://example.org/o");
        let o2 = term(&mut g, "https://example.org/a");
        let ga = term(&mut g, "https://example.org/gz");
        let gb = term(&mut g, "https://example.org/ga");

        // Deliberately inserted out of order, with the default-graph rows in the middle
        // so neither the input order nor a stable sort can produce the answer by luck.
        g.quads = vec![
            (s, p, o, Some(ga)),
            (s, p, o2, None),
            (s, p, o, Some(gb)),
            (s, p, o, None),
            (s, p, o2, Some(ga)),
        ];

        // The key the previous implementation built, verbatim in shape.
        let key = |g: &SerGraph, &(a, b, c, d): &SerQuad| -> Vec<String> {
            vec![
                render_term(g, a),
                render_term(g, b),
                render_term(g, c),
                d.map(|x| render_term(g, x)).unwrap_or_default(),
            ]
        };
        let mut expected = g.quads.clone();
        expected.sort_by_key(|x| key(&g, x));

        g.sort_canonical();
        assert_eq!(
            g.quads, expected,
            "the comparator must reproduce the key-based order exactly; the emitted \
             byte order is what that order decides"
        );

        // And the property that empty-renders-`None` actually buys. The graph slot is
        // the LAST key component, so it separates rows only once subject, predicate and
        // object have tied — the default graph leads within each such group, not the
        // document. Stated as the invariant rather than as fixed positions, because
        // fixed positions would also pass for a comparator that ignored the graph.
        for pair in g.quads.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if (a.0, a.1, a.2) == (b.0, b.1, b.2) {
                assert!(
                    !(a.3.is_some() && b.3.is_none()),
                    "within one subject/predicate/object group an absent graph renders \
                     as the empty string and must precede every named one: {:?}",
                    g.quads
                );
            }
        }
    }

    /// One term's N-Triples surface as an owned `String`.
    ///
    /// Production has none: `sort_canonical` compares through reusable buffers and the
    /// serializers append, so nothing outside this module ever wants a term on its own.
    /// It survives here to reconstruct the key the sort used to build, which is what
    /// lets the ordering test compare against the old behaviour rather than against
    /// itself.
    fn render_term(g: &SerGraph, tid: usize) -> String {
        let mut out = String::new();
        write_term(g, tid, &mut out);
        out
    }

    fn to_ntriples(g: &SerGraph) -> Result<String, RdfDiagnostic> {
        let mut out = String::new();
        write_ntriples(g, &mut out)?;
        Ok(out)
    }

    fn to_turtle(g: &SerGraph) -> Result<String, RdfDiagnostic> {
        let mut out = String::new();
        write_turtle(g, &mut out)?;
        Ok(out)
    }

    #[test]
    fn deterministic_blank_label_matches_zero_timestamp_ulid() {
        // The raw blank-label shape is byte-identity critical: the W3C canonical
        // comparison relabels blanks and will NOT catch a label-shape regression, so
        // these exact strings are checked directly. Each is the 26-digit Crockford
        // Base32 rendering of the zero-timestamp ULID built from the counter.
        assert_eq!(
            deterministic_blank_label(0),
            "gts_00000000000000000000000000"
        );
        assert_eq!(
            deterministic_blank_label(1),
            "gts_00000000000000000000000001"
        );
        assert_eq!(
            deterministic_blank_label(31),
            "gts_0000000000000000000000000Z"
        );
        assert_eq!(
            deterministic_blank_label(32),
            "gts_00000000000000000000000010"
        );
        assert_eq!(
            deterministic_blank_label(1000),
            "gts_000000000000000000000000Z8"
        );
    }

    /// A single-quad graph `<s> <p> "<lit>"` over default-graph terms.
    fn lit_graph(lexical: &str, datatype_iri: &str) -> SerGraph {
        let mut g = SerGraph::default();
        // 0: s, 1: p, 2: datatype IRI, 3: literal
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some(datatype_iri.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Literal,
            value: Some(lexical.to_owned()),
            datatype: Some(2),
            lang: None,
            direction: None,
            reifier: None,
        });
        g.quads.push((0, 1, 3, None));
        g
    }

    #[test]
    fn decimal_lexical_preserved_verbatim_in_ntriples() {
        // The trailing zero of "0.90"^^xsd:decimal MUST survive verbatim: no
        // value-space canonicalization, no datatype narrowing.
        let g = lit_graph("0.90", "http://www.w3.org/2001/XMLSchema#decimal");
        let nt = to_ntriples(&g).expect("ntriples");
        assert!(
            nt.contains("\"0.90\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "raw N-Triples output must carry the verbatim lexical form, got: {nt}"
        );
    }

    #[test]
    fn turtle_begins_with_prefix_header() {
        let g = lit_graph("0.90", "http://www.w3.org/2001/XMLSchema#decimal");
        let ttl = to_turtle(&g).expect("turtle");
        let expected = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n";
        assert!(
            ttl.starts_with(expected),
            "Turtle must begin with the two @prefix lines, got: {ttl}"
        );
        // The IRI body stays full <...>, NOT abbreviated against the declared prefixes.
        assert!(
            ttl.contains("\"0.90\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "Turtle body keeps the verbatim lexical form + full datatype IRI"
        );
    }

    #[test]
    fn empty_turtle_is_empty_string() {
        let g = SerGraph::default();
        assert_eq!(to_turtle(&g).expect("turtle"), "");
    }

    #[test]
    fn ntriples_rejects_named_graph() {
        let mut g = lit_graph("x", "http://www.w3.org/2001/XMLSchema#string");
        // Re-point the literal as a plain literal and add a named-graph quad.
        g.quads.push((0, 1, 0, Some(0)));
        assert!(
            to_ntriples(&g).is_err(),
            "N-Triples must reject a named-graph quad"
        );
    }

    #[test]
    fn language_tag_with_direction_renders() {
        let mut g = SerGraph::default();
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Literal,
            value: Some("hi".to_owned()),
            datatype: None,
            lang: Some("en".to_owned()),
            direction: Some("ltr".to_owned()),
            reifier: None,
        });
        g.quads.push((0, 1, 2, None));
        let nt = to_ntriples(&g).expect("ntriples");
        assert!(nt.contains("\"hi\"@en--ltr"), "got: {nt}");
    }

    // ── serializer escape: byte-identity of the scan-first fast path ───────────────

    /// The pre-optimization per-char `escape_iri`, frozen verbatim as a test oracle:
    /// the scan-first implementation must match it byte-for-byte on every input.
    fn escape_iri_oracle(iri: &str) -> String {
        let mut out = String::with_capacity(iri.len());
        for ch in iri.chars() {
            match ch {
                '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                    let _ = write!(out, "\\u{:04X}", ch as u32);
                }
                c if c.is_control() || c == ' ' => {
                    let _ = write!(out, "\\u{:04X}", c as u32);
                }
                c => out.push(c),
            }
        }
        out
    }

    /// The pre-optimization per-char `escape_literal`, frozen verbatim as a test oracle.
    fn escape_literal_oracle(lex: &str) -> String {
        let mut out = String::with_capacity(lex.len());
        for ch in lex.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => {
                    let _ = write!(out, "\\u{:04X}", c as u32);
                }
                c => out.push(c),
            }
        }
        out
    }

    #[test]
    fn escape_iri_fixed_adversarial_goldens() {
        // Every IRIREF-forbidden delimiter rides as an uppercase 4-hex `\uXXXX`.
        assert_eq!(escape_iri("a<b"), "a\\u003Cb");
        assert_eq!(escape_iri("a>b"), "a\\u003Eb");
        assert_eq!(escape_iri("a\"b"), "a\\u0022b");
        assert_eq!(escape_iri("a{b"), "a\\u007Bb");
        assert_eq!(escape_iri("a}b"), "a\\u007Db");
        assert_eq!(escape_iri("a|b"), "a\\u007Cb");
        assert_eq!(escape_iri("a^b"), "a\\u005Eb");
        assert_eq!(escape_iri("a`b"), "a\\u0060b");
        assert_eq!(escape_iri("a\\b"), "a\\u005Cb");
        assert_eq!(escape_iri("a b"), "a\\u0020b"); // space
        assert_eq!(escape_iri("a\u{01}b"), "a\\u0001b"); // C0
        assert_eq!(escape_iri("a\u{7F}b"), "a\\u007Fb"); // DEL
        assert_eq!(escape_iri("a\u{85}b"), "a\\u0085b"); // C1 (NEL)
        assert_eq!(escape_iri("a\u{E9}b"), "a\u{E9}b"); // clean non-ASCII: verbatim
        assert_eq!(
            // A clean ASCII IRI passes byte-for-byte.
            escape_iri("http://example.org/path"),
            "http://example.org/path"
        );
    }

    #[test]
    fn escape_literal_fixed_adversarial_goldens() {
        assert_eq!(escape_literal("a\"b"), "a\\\"b");
        assert_eq!(escape_literal("a\\b"), "a\\\\b");
        assert_eq!(escape_literal("a\nb"), "a\\nb");
        assert_eq!(escape_literal("a\rb"), "a\\rb");
        assert_eq!(escape_literal("a\tb"), "a\\tb");
        assert_eq!(escape_literal("a\u{01}b"), "a\\u0001b"); // C0
        assert_eq!(escape_literal("a\u{7F}b"), "a\\u007Fb"); // DEL
        assert_eq!(escape_literal("a\u{85}b"), "a\\u0085b"); // C1
        assert_eq!(escape_literal("a\u{E9}b"), "a\u{E9}b"); // clean unicode
        assert_eq!(escape_literal("clean text 123"), "clean text 123");
        assert_eq!(escape_literal("x\"y\\z\n"), "x\\\"y\\\\z\\n"); // mixed
    }

    proptest! {
        /// The scan-first `escape_iri` equals the frozen per-char oracle on every
        /// arbitrary string (controls, C1, multi-byte unicode, and clean runs).
        #[test]
        fn escape_iri_matches_oracle(s in any::<String>()) {
            let got = escape_iri(&s);
            prop_assert_eq!(got.as_ref(), escape_iri_oracle(&s));
        }

        /// The scan-first `escape_literal` equals the frozen per-char oracle on every
        /// arbitrary string.
        #[test]
        fn escape_literal_matches_oracle(s in any::<String>()) {
            let got = escape_literal(&s);
            prop_assert_eq!(got.as_ref(), escape_literal_oracle(&s));
        }
    }
}
