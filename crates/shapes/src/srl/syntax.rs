// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SPARQL 1.2 RL concrete syntax: a recursive-descent parser for the grammar of
//! SPARQL 1.2 RL §7 ("SPARQL-RL Grammar", W3C Working Draft 19 September 2026,
//! <https://www.w3.org/TR/2026/WD-sparql12-rl-20260919/>), producing the rule-set IR.
//!
//! "A SPARQL-RL Document is an RDF string encoded in UTF-8 [RFC3629] and starting with
//! the RuleSet production and conforming to the additional constraints defined in 7.6
//! Grammar." Every production of §7.6 is implemented here, one function per production
//! family, and each function's documentation quotes the productions it reads.
//!
//! # What is shared with SPARQL
//!
//! The terminals (§7.6 [119]–[153]) are SPARQL 1.2's terminals, so the document is
//! tokenized by `purrdf-sparql-algebra`'s lexer; SRL's three multi-character terminal
//! strings the SPARQL lexer splits — `':='`, `'<<('` and `')>>'` — are recognized as
//! two ADJACENT tokens, which is exactly the terminal string. The expressions (§7.6
//! [103]–[118]) are built as `purrdf-sparql-algebra` [`Expression`]s, the algebra SPARQL
//! evaluation already runs, over SRL's own productions: SRL's `BuiltInCall` is a strict
//! subset of SPARQL's ("Some functions are not included in the SRL syntax. There is no
//! COALESCE, nor BOUND. There are no hash functions. There is no RAND"), its
//! `ExprTripleTerm` admits a literal subject SPARQL's does not, and it has no `EXISTS`
//! ("The syntax of NOT limits the inner body to triple patterns and filters, and does not
//! allow nested patterns"), so parsing through SPARQL's expression production would
//! accept what SRL refuses and refuse what SRL accepts. The triple productions differ
//! the same way — SRL's property paths are "only … paths that can be expanded into
//! triple patterns", its `DATA` blocks admit no variable ("Variables are not allowed in a
//! DATA block"), and its rule-body blank nodes are one variable across the whole body,
//! negation elements included — so they are SRL's own too.
//!
//! # Grammar notes (§7.6)
//!
//! "1. The entry point of the grammar is RuleSet. 2. Keywords are case-insensitive
//! except for 'a' which is case-sensitive. 3. Escape sequences UCHAR and ECHAR are case
//! sensitive. 4. Variables are not allowed in a DATA block. 5. When tokenizing the input
//! and choosing grammar rules, the longest match is chosen. 6. The SPARQL-RL grammar is
//! LL(1) and LALR(1) when the rules with uppercased names are used as terminals."
//!
//! # Abbreviations
//!
//! Collections, blank-node property lists, reified triples, reifiers, annotation blocks
//! and property paths are expanded into triple patterns (templates, data triples) here,
//! as Turtle and SPARQL expand them: a collection into its `rdf:first`/`rdf:rest` chain
//! ending in `rdf:nil`, a reified triple `<< s p o ~ r >>` into `r rdf:reifies <<( s p o
//! )>>` (the reifier `r` fresh when omitted), an annotation block into triples about the
//! reifier the block follows (a fresh one when none precedes it), a path sequence
//! `s p1/p2 o` into `s p1 _:x . _:x p2 o` and an inverse `^p` into the swapped triple. A
//! fresh node is a blank node with a label no author label in the document starts with.
//! In a rule body a blank node "behave[s] like variables"; in a rule head it is fresh per
//! solution; in a data block it is a blank node of the data.

use std::collections::HashMap;

use ::purrdf::RdfTextDirection;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, LineIndex, langtag};
use purrdf_sparql_algebra::lexer::{Spanned, Token, tokenize};
use purrdf_sparql_algebra::{BaseDirection, Expression, Function, Variable};

use super::ir::{Element, ElementRule, PatternTerm, TriplePattern};
use crate::model::{rdf, xsd};
use crate::term::{Literal, NamedNode, Term, Triple};

/// A syntax error: what the grammar refused, and the byte offset it refused at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntaxError {
    /// What was refused.
    pub(crate) message: String,
    /// The byte offset in the document.
    pub(crate) offset: usize,
}

/// A parse result.
type Parse<T> = Result<T, SyntaxError>;

/// One parsed `Rule`.
#[derive(Debug, Clone)]
pub(crate) struct ParsedRule {
    /// `'RULE' iri?`: the rule's IRI, when written.
    pub(crate) iri: Option<NamedNode>,
    /// The rule.
    pub(crate) rule: ElementRule,
    /// The byte offset of the `RULE` keyword.
    pub(crate) offset: usize,
}

/// A parsed document.
#[derive(Debug, Clone, Default)]
pub(crate) struct Parsed {
    /// The rules, in document order.
    pub(crate) rules: Vec<ParsedRule>,
    /// The triples of every data block, in document order.
    pub(crate) data: Vec<[Term; 3]>,
    /// The `IMPORTS` IRIs, in document order.
    pub(crate) imports: Vec<NamedNode>,
    /// The `VERSION` labels, in document order.
    pub(crate) versions: Vec<String>,
}

/// Parse a SPARQL 1.2 RL document. `base` is the base IRI from outside the document
/// (§7.4: the encapsulating entity's or the retrieval URI's), if any.
pub(crate) fn parse(text: &str, base: Option<&str>) -> Parse<Parsed> {
    let tokens = tokenize(text).map_err(|e| SyntaxError {
        message: e.to_string(),
        offset: lex_offset(&e),
    })?;
    let base = match base {
        None => BaseScope::empty(),
        Some(iri) => BaseScope::rooted(
            BaseIri::parse(iri).map_err(|e| SyntaxError {
                message: format!("the base IRI <{iri}> is unusable: {e}"),
                offset: 0,
            })?,
            BaseOrigin::Caller,
        ),
    };
    let mut anon_prefix = String::from("srl-anon-");
    while tokens.iter().any(
        |t| matches!(&t.token, Token::BlankNodeLabel(label) if label.starts_with(anon_prefix.as_str())),
    ) {
        anon_prefix.insert(0, 'x');
    }
    let mut parser = SrlParser {
        src: text,
        tokens,
        pos: 0,
        base,
        prefixes: HashMap::new(),
        anon_prefix,
        anon: 0,
    };
    parser.rule_set()
}

/// The byte offset a lexer error names.
fn lex_offset(error: &purrdf_sparql_algebra::ParseError) -> usize {
    match error {
        purrdf_sparql_algebra::ParseError::Lex { at, .. }
        | purrdf_sparql_algebra::ParseError::Syntax { at, .. } => *at,
        _ => 0,
    }
}

/// Where a triple production is: its variables, verbs and blank nodes differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A `DATA` block: no variables, `VerbData ::= iri | 'a'`.
    Data,
    /// A rule head: `Verb ::= VarOrIri | 'a'`.
    Head,
    /// A rule body: `( VerbPath | Var )`.
    Body,
}

/// A verb: a term (an IRI or a variable) or a property path.
#[derive(Debug, Clone)]
enum Verb {
    /// An IRI or a variable.
    Term(PatternTerm),
    /// A path of more than one step, or an inverse.
    Path(SrlPath),
}

/// `Path ::= PathSequence`, `PathSequence ::= PathEltOrInverse ( '/' PathEltOrInverse )*`,
/// `PathEltOrInverse ::= PathElt | '^' PathElt`, `PathElt ::= ( iri | 'a' | '(' Path ')' )`.
#[derive(Debug, Clone)]
enum SrlPath {
    /// One IRI step.
    Iri(NamedNode),
    /// `^` step.
    Inverse(Box<Self>),
    /// A sequence of two or more steps.
    Sequence(Vec<Self>),
}

/// The parser state.
struct SrlParser<'t> {
    /// The document.
    src: &'t str,
    /// Its tokens.
    tokens: Vec<Spanned<'t>>,
    /// The cursor.
    pos: usize,
    /// The base IRIs in scope.
    base: BaseScope,
    /// The declared prefixes.
    prefixes: HashMap<String, String>,
    /// The label prefix of fresh blank nodes.
    anon_prefix: String,
    /// Fresh blank nodes minted so far.
    anon: usize,
}

/// The shared keywords.
const KEYWORDS_AFTER_BODY: [&str; 3] = ["FILTER", "NOT", "SET"];

impl<'t> SrlParser<'t> {
    // ── cursor ────────────────────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token<'t>> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn peek_at(&self, ahead: usize) -> Option<&Token<'t>> {
        self.tokens.get(self.pos + ahead).map(|t| &t.token)
    }

    /// The byte offset of the current token (the end of the document at its end).
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.src.len(), |t| t.start)
    }

    /// Whether the tokens at `self.pos + ahead` and the one after it touch.
    fn adjacent(&self, ahead: usize) -> bool {
        match (
            self.tokens.get(self.pos + ahead),
            self.tokens.get(self.pos + ahead + 1),
        ) {
            (Some(a), Some(b)) => a.end == b.start,
            _ => false,
        }
    }

    fn at(&self, token: &Token<'_>) -> bool {
        self.peek() == Some(token)
    }

    fn eat(&mut self, token: &Token<'_>) -> bool {
        if self.at(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn error<T>(&self, message: impl Into<String>) -> Parse<T> {
        Err(SyntaxError {
            message: message.into(),
            offset: self.offset(),
        })
    }

    fn describe(&self) -> String {
        self.tokens.get(self.pos).map_or_else(
            || "the end of the document".to_owned(),
            |t| format!("`{}`", &self.src[t.start..t.end]),
        )
    }

    fn expect(&mut self, token: &Token<'_>, spelled: &str) -> Parse<()> {
        if self.eat(token) {
            Ok(())
        } else {
            self.error(format!("expected `{spelled}`, found {}", self.describe()))
        }
    }

    fn at_kw(&self, keyword: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(keyword))
    }

    fn eat_kw(&mut self, keyword: &str) -> bool {
        if self.at_kw(keyword) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_kw(&mut self, keyword: &str) -> Parse<()> {
        if self.eat_kw(keyword) {
            Ok(())
        } else {
            self.error(format!("expected `{keyword}`, found {}", self.describe()))
        }
    }

    /// `'a'`, case-sensitively (grammar note 2).
    fn at_a(&self) -> bool {
        matches!(self.peek(), Some(Token::Word("a")))
    }

    /// `NIL ::= '(' WS* ')'`.
    fn at_nil(&self) -> bool {
        self.at(&Token::LParen) && matches!(self.peek_at(1), Some(Token::RParen))
    }

    /// `'<<('`: the two tokens `<<` `(`, touching.
    fn at_triple_term_open(&self) -> bool {
        self.at(&Token::TripleOpen)
            && matches!(self.peek_at(1), Some(Token::LParen))
            && self.adjacent(0)
    }

    /// `'<<'` opening a reified triple (not the `'<<('` of a triple term).
    fn at_reified_open(&self) -> bool {
        self.at(&Token::TripleOpen) && !self.at_triple_term_open()
    }

    fn expect_triple_term_open(&mut self) -> Parse<()> {
        if self.at_triple_term_open() {
            self.pos += 2;
            Ok(())
        } else {
            self.error(format!("expected `<<(`, found {}", self.describe()))
        }
    }

    /// `')>>'`: the two tokens `)` `>>`, touching.
    fn expect_triple_term_close(&mut self) -> Parse<()> {
        if self.at(&Token::RParen)
            && matches!(self.peek_at(1), Some(Token::TripleClose))
            && self.adjacent(0)
        {
            self.pos += 2;
            Ok(())
        } else {
            self.error(format!("expected `)>>`, found {}", self.describe()))
        }
    }

    /// `':='`: the lexer reads it as the empty prefixed name `:` touching `=`.
    fn expect_assign_op(&mut self) -> Parse<()> {
        let colon = matches!(self.peek(), Some(Token::PrefixedName("", local)) if local.is_empty())
            && self.tokens[self.pos].end - self.tokens[self.pos].start == 1;
        if colon && matches!(self.peek_at(1), Some(Token::Eq)) && self.adjacent(0) {
            self.pos += 2;
            Ok(())
        } else {
            self.error(format!("expected `:=`, found {}", self.describe()))
        }
    }

    fn fresh(&mut self) -> PatternTerm {
        self.anon += 1;
        PatternTerm::BlankNode(format!("{}{}", self.anon_prefix, self.anon))
    }

    // ── [1]–[12] rule set, prologue, rule, data ────────────────────────────────────

    /// `[1] RuleSet ::= RuleOrDataBlock`,
    /// `[2] RuleOrDataBlock ::= Prologue ( RuleOrData+ ( Prologue1 RuleOrData? )* )?`,
    /// `[3] RuleOrData ::= Rule | Data`, `[4] Prologue ::= Prologue1*`.
    ///
    /// Production [2] is read as written: once a declaration follows a rule or data
    /// block, at most one rule or data block follows it before the next declaration.
    fn rule_set(&mut self) -> Parse<Parsed> {
        let mut out = Parsed::default();
        while self.prologue1(&mut out)? {}
        if self.at_rule_or_data() {
            while self.at_rule_or_data() {
                self.rule_or_data(&mut out)?;
            }
            while self.prologue1(&mut out)? {
                if self.at_rule_or_data() {
                    self.rule_or_data(&mut out)?;
                }
            }
        }
        if self.pos < self.tokens.len() {
            if self.at_rule_or_data() && !out.rules.is_empty() {
                return self.error(
                    "a second rule or data block after a declaration that follows a rule or \
                     data block; SPARQL 1.2 RL grammar rule [2] `RuleOrDataBlock ::= Prologue \
                     ( RuleOrData+ ( Prologue1 RuleOrData? )* )?` admits at most one there",
                );
            }
            return self.error(format!(
                "expected RULE, DATA, BASE, PREFIX, VERSION or IMPORTS, found {}",
                self.describe()
            ));
        }
        Ok(out)
    }

    fn at_rule_or_data(&self) -> bool {
        self.at_kw("RULE") || self.at_kw("DATA")
    }

    fn rule_or_data(&mut self, out: &mut Parsed) -> Parse<()> {
        if self.at_kw("RULE") {
            let rule = self.rule()?;
            out.rules.push(rule);
            Ok(())
        } else {
            self.data(out)
        }
    }

    /// `[5] Prologue1 ::= BaseDecl | PrefixDecl | VersionDecl | ImportsDecl`,
    /// `[6] BaseDecl ::= 'BASE' IRIREF`, `[7] PrefixDecl ::= 'PREFIX' PNAME_NS IRIREF`,
    /// `[8] VersionDecl ::= 'VERSION' VersionSpecifier`,
    /// `[9] VersionSpecifier ::= STRING_LITERAL1 | STRING_LITERAL2`,
    /// `[10] ImportsDecl ::= 'IMPORTS' iri`. Returns whether one was read.
    ///
    /// §7.4: "Each BASE directive sets a new In-Scope Base URI, relative to the previous
    /// one." §7.1: "Multiple VERSION directives may appear in a SPARQL-RL Document"; the
    /// only version label the specification defines is `"1.2"`, and a label is recorded
    /// as written.
    fn prologue1(&mut self, out: &mut Parsed) -> Parse<bool> {
        let at = self.offset();
        if self.eat_kw("BASE") {
            let Some(Token::Iri(directive)) = self.peek().cloned() else {
                return self.error(format!(
                    "expected an IRIREF after BASE, found {}",
                    self.describe()
                ));
            };
            let position = LineIndex::new(self.src).locate(self.src, at);
            self.base
                .rebind(
                    &directive,
                    BaseOrigin::Directive {
                        line: position.line,
                        column: position.column,
                    },
                )
                .map_err(|e| SyntaxError {
                    message: format!("BASE <{directive}>: {e}"),
                    offset: self.offset(),
                })?;
            self.pos += 1;
            Ok(true)
        } else if self.eat_kw("PREFIX") {
            let prefix = match self.peek() {
                Some(Token::PrefixedName(prefix, local)) if local.is_empty() => {
                    (*prefix).to_owned()
                }
                _ => {
                    return self.error(format!(
                        "expected a PNAME_NS (`prefix:`) after PREFIX, found {}",
                        self.describe()
                    ));
                }
            };
            self.pos += 1;
            let Some(Token::Iri(iri)) = self.peek().cloned() else {
                return self.error(format!(
                    "expected an IRIREF after PREFIX {prefix}:, found {}",
                    self.describe()
                ));
            };
            let resolved = self.resolve(&iri)?;
            self.pos += 1;
            self.prefixes.insert(prefix, resolved);
            Ok(true)
        } else if self.eat_kw("VERSION") {
            match self.peek() {
                Some(Token::StringLit(label)) => {
                    out.versions.push(label.to_string());
                    self.pos += 1;
                    Ok(true)
                }
                _ => self.error(format!(
                    "expected a STRING_LITERAL1 or STRING_LITERAL2 version label after VERSION, \
                     found {}",
                    self.describe()
                )),
            }
        } else if self.eat_kw("IMPORTS") {
            let iri = self.iri()?;
            out.imports.push(iri);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// `[11] Rule ::= 'RULE' iri? HeadTemplate 'WHERE' 'DATA'? BodyPattern`.
    fn rule(&mut self) -> Parse<ParsedRule> {
        let offset = self.offset();
        self.expect_kw("RULE")?;
        let iri = if matches!(self.peek(), Some(Token::Iri(_) | Token::PrefixedName(..))) {
            Some(self.iri()?)
        } else {
            None
        };
        let head = self.head_template()?;
        self.expect_kw("WHERE")?;
        let data = self.eat_kw("DATA");
        let body = self.body_pattern()?;
        Ok(ParsedRule {
            iri,
            rule: ElementRule { head, body, data },
            offset,
        })
    }

    /// `[12] Data ::= 'DATA' '{' DataTriplesBlock? '}'`,
    /// `[25] DataTriplesBlock ::= TriplesSameSubjectData ( '.' DataTriplesBlock? )?`.
    fn data(&mut self, out: &mut Parsed) -> Parse<()> {
        self.expect_kw("DATA")?;
        self.expect(&Token::LBrace, "{")?;
        let mut triples = Vec::new();
        if !self.at(&Token::RBrace) {
            self.triples_block(Place::Data, &mut triples)?;
        }
        self.expect(&Token::RBrace, "}")?;
        for triple in triples {
            out.data.push([
                ground(triple.subject),
                ground(triple.predicate),
                ground(triple.object),
            ]);
        }
        Ok(())
    }

    /// `[13] HeadTemplate ::= '{' HeadTemplateBlock? '}'`,
    /// `[47] HeadTemplateBlock ::= TriplesBlockTemplate`,
    /// `[48] TriplesBlockTemplate ::= TriplesSameSubjectTemplate ( '.' TriplesBlockTemplate? )?`.
    fn head_template(&mut self) -> Parse<Vec<TriplePattern>> {
        self.expect(&Token::LBrace, "{")?;
        let mut head = Vec::new();
        if !self.at(&Token::RBrace) {
            self.triples_block(Place::Head, &mut head)?;
        }
        self.expect(&Token::RBrace, "}")?;
        Ok(head)
    }

    /// `[14] BodyPattern ::= '{' BodyTriplesBlock? ( BodyNotTriples '.'? BodyTriplesBlock? )* '}'`,
    /// `[15] BodyNotTriples ::= Filter | Negation | Assignment`,
    /// `[61] BodyTriplesBlock ::= TriplesBlockPattern`.
    fn body_pattern(&mut self) -> Parse<Vec<Element>> {
        self.expect(&Token::LBrace, "{")?;
        let body = self.body_sequence(false)?;
        self.expect(&Token::RBrace, "}")?;
        Ok(body)
    }

    /// The inside of a `BodyPattern` (`negation` false) or of a `BodyBasic` (`negation`
    /// true): `[22] BodyBasic ::= BodyTriplesBlock? ( BodyBasicNotTriples '.'?
    /// BodyTriplesBlock? )*`, `[23] BodyBasicNotTriples ::= Filter`.
    fn body_sequence(&mut self, negation: bool) -> Parse<Vec<Element>> {
        let mut elements = Vec::new();
        let at_not_triples = |p: &Self| {
            if negation {
                p.at_kw("FILTER")
            } else {
                KEYWORDS_AFTER_BODY.iter().any(|k| p.at_kw(k))
            }
        };
        if !at_not_triples(self) && !self.at(&Token::RBrace) {
            let mut triples = Vec::new();
            self.triples_block(Place::Body, &mut triples)?;
            elements.extend(triples.into_iter().map(Element::Pattern));
        }
        while at_not_triples(self) {
            elements.push(self.body_not_triples()?);
            self.eat(&Token::Dot);
            if !at_not_triples(self) && !self.at(&Token::RBrace) {
                let mut triples = Vec::new();
                self.triples_block(Place::Body, &mut triples)?;
                elements.extend(triples.into_iter().map(Element::Pattern));
            }
        }
        Ok(elements)
    }

    /// One `BodyNotTriples`: `[16] Filter ::= 'FILTER' Constraint`,
    /// `[21] Negation ::= 'NOT' 'DATA'? '{' BodyBasic '}'`,
    /// `[24] Assignment ::= 'SET' '(' Var ':=' Expression ')'`.
    fn body_not_triples(&mut self) -> Parse<Element> {
        if self.eat_kw("FILTER") {
            return Ok(Element::Filter(self.constraint()?));
        }
        if self.eat_kw("NOT") {
            let data = self.eat_kw("DATA");
            self.expect(&Token::LBrace, "{")?;
            let elements = self.body_sequence(true)?;
            self.expect(&Token::RBrace, "}")?;
            return Ok(Element::Negation { elements, data });
        }
        self.expect_kw("SET")?;
        self.expect(&Token::LParen, "(")?;
        let Some(Token::Variable(name)) = self.peek().cloned() else {
            return self.error(format!(
                "expected a variable after `SET (`, found {}",
                self.describe()
            ));
        };
        self.pos += 1;
        self.expect_assign_op()?;
        let expression = self.expression()?;
        self.expect(&Token::RParen, ")")?;
        Ok(Element::Assign {
            variable: name.to_owned(),
            expression,
        })
    }

    // ── triples ─────────────────────────────────────────────────────────────────────

    /// `TriplesBlock ::= TriplesSameSubject ( '.' TriplesBlock? )?` in each of its three
    /// spellings ([25], [48], [62]).
    fn triples_block(&mut self, ctx: Place, out: &mut Vec<TriplePattern>) -> Parse<()> {
        loop {
            self.triples_same_subject(ctx, out)?;
            if !self.eat(&Token::Dot) || !self.can_start_triples(ctx) {
                return Ok(());
            }
        }
    }

    /// Whether the current token can start a `TriplesSameSubject`.
    fn can_start_triples(&self, ctx: Place) -> bool {
        match self.peek() {
            Some(Token::Variable(_)) => ctx != Place::Data,
            Some(
                Token::Iri(_)
                | Token::PrefixedName(..)
                | Token::BlankNodeLabel(_)
                | Token::Anon
                | Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_)
                | Token::Plus
                | Token::Minus
                | Token::LParen
                | Token::LBracket
                | Token::TripleOpen,
            ) => true,
            Some(Token::Word(w)) => *w == "true" || *w == "false",
            _ => false,
        }
    }

    /// `[26] TriplesSameSubjectData ::= RDFTermData PropertyListNotEmptyData |
    /// TriplesNodeData PropertyListData | ReifiedTripleBlockData`,
    /// `[49] TriplesSameSubjectTemplate ::= VarOrRDFTerm PropertyListNotEmptyTemplate |
    /// TriplesNodeTemplate PropertyListTemplate | ReifiedTripleBlockTemplate`,
    /// `[64] TriplesSameSubjectPattern ::= VarOrRDFTerm PropertyListNotEmptyPattern |
    /// TriplesNodePattern PropertyListPattern | ReifiedTripleBlockPattern`, with
    /// `[40]`/`[60]`/`[63] ReifiedTripleBlock ::= ReifiedTriple PropertyList`.
    fn triples_same_subject(&mut self, ctx: Place, out: &mut Vec<TriplePattern>) -> Parse<()> {
        if self.at_reified_open()
            || self.at(&Token::LBracket)
            || (self.at(&Token::LParen) && !self.at_nil())
        {
            let subject = if self.at_reified_open() {
                self.reified_triple(ctx, out)?
            } else {
                self.triples_node(ctx, out)?
            };
            if self.can_start_verb(ctx) {
                self.property_list_not_empty(ctx, &subject, out)?;
            }
            return Ok(());
        }
        let subject = self.var_or_term(ctx)?;
        self.property_list_not_empty(ctx, &subject, out)
    }

    /// Whether the current token can start a verb in `ctx`.
    fn can_start_verb(&self, ctx: Place) -> bool {
        if self.at_a() {
            return true;
        }
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(..)) => true,
            Some(Token::Variable(_)) => ctx != Place::Data,
            Some(Token::Caret | Token::LParen) => ctx == Place::Body,
            _ => false,
        }
    }

    /// `[28] PropertyListNotEmptyData ::= VerbData ObjectListData ( ';' ( VerbData
    /// ObjectListData )? )*`, `[51]` (templates, `Verb`), `[66]
    /// PropertyListNotEmptyPattern ::= ( VerbPath | Var ) ObjectListPattern ( ';' ( (
    /// VerbPath | Var ) ObjectListPattern )? )*`.
    fn property_list_not_empty(
        &mut self,
        ctx: Place,
        subject: &PatternTerm,
        out: &mut Vec<TriplePattern>,
    ) -> Parse<()> {
        loop {
            let verb = self.verb(ctx)?;
            self.object_list(ctx, subject, &verb, out)?;
            if !self.at(&Token::Semicolon) {
                return Ok(());
            }
            while self.eat(&Token::Semicolon) {}
            if !self.can_start_verb(ctx) {
                return Ok(());
            }
        }
    }

    /// `[29] VerbData ::= iri | 'a'`, `[83] Verb ::= VarOrIri | 'a'`, `[84] VerbPath ::=
    /// Path` (the body's `VerbPath | Var`).
    fn verb(&mut self, ctx: Place) -> Parse<Verb> {
        if ctx == Place::Body && !matches!(self.peek(), Some(Token::Variable(_))) {
            let path = self.path()?;
            if matches!(
                self.peek(),
                Some(Token::Star | Token::Question | Token::Pipe)
            ) {
                return self.error(format!(
                    "{} after a property path: SPARQL 1.2 RL paths are sequences and inverses \
                     only (\"Property path syntax only covers paths that can be expanded into \
                     triple patterns. It does not allow the arbitrary length operators * and \
                     +\")",
                    self.describe()
                ));
            }
            return Ok(match path {
                SrlPath::Iri(iri) => Verb::Term(PatternTerm::Term(Term::NamedNode(iri))),
                path => Verb::Path(path),
            });
        }
        Ok(Verb::Term(self.simple_verb(ctx)?))
    }

    /// A verb without paths: `Verb ::= VarOrIri | 'a'` (`VerbData ::= iri | 'a'` in a
    /// data block).
    fn simple_verb(&mut self, ctx: Place) -> Parse<PatternTerm> {
        if self.at_a() {
            self.pos += 1;
            return Ok(iri_term(rdf::TYPE));
        }
        match self.peek() {
            Some(Token::Variable(name)) if ctx != Place::Data => {
                let name = (*name).to_owned();
                self.pos += 1;
                Ok(PatternTerm::Variable(name))
            }
            Some(Token::Variable(_)) => self.error("variables are not allowed in a DATA block"),
            Some(Token::Iri(_) | Token::PrefixedName(..)) => {
                Ok(PatternTerm::Term(Term::NamedNode(self.iri()?)))
            }
            _ => self.error(format!(
                "expected a predicate (an IRI{} or `a`), found {}",
                if ctx == Place::Data {
                    ""
                } else {
                    ", a variable"
                },
                self.describe()
            )),
        }
    }

    /// `[85] Path ::= PathSequence`,
    /// `[86] PathSequence ::= PathEltOrInverse ( '/' PathEltOrInverse )*`.
    fn path(&mut self) -> Parse<SrlPath> {
        let mut steps = vec![self.path_elt_or_inverse()?];
        while self.eat(&Token::Slash) {
            steps.push(self.path_elt_or_inverse()?);
        }
        Ok(if steps.len() == 1 {
            steps.pop().expect("one step")
        } else {
            SrlPath::Sequence(steps)
        })
    }

    /// `[87] PathEltOrInverse ::= PathElt | '^' PathElt`,
    /// `[88] PathElt ::= ( iri | 'a' | '(' Path ')' )`.
    fn path_elt_or_inverse(&mut self) -> Parse<SrlPath> {
        let inverse = self.eat(&Token::Caret);
        let elt = if self.at_a() {
            self.pos += 1;
            SrlPath::Iri(NamedNode::from(rdf::TYPE))
        } else if self.eat(&Token::LParen) {
            let path = self.path()?;
            self.expect(&Token::RParen, ")")?;
            path
        } else if matches!(self.peek(), Some(Token::Iri(_) | Token::PrefixedName(..))) {
            SrlPath::Iri(self.iri()?)
        } else {
            return self.error(format!(
                "expected a property path step (an IRI, `a`, `^` or `(`), found {}; SPARQL 1.2 \
                 RL paths are sequences and inverses only (\"It does not allow the arbitrary \
                 length operators * and +\")",
                self.describe()
            ));
        };
        Ok(if inverse {
            SrlPath::Inverse(Box::new(elt))
        } else {
            elt
        })
    }

    /// `[30] ObjectListData ::= ObjectData ( ',' ObjectData )*` (and [52], [67]).
    fn object_list(
        &mut self,
        ctx: Place,
        subject: &PatternTerm,
        verb: &Verb,
        out: &mut Vec<TriplePattern>,
    ) -> Parse<()> {
        self.object(ctx, subject, verb, out)?;
        while self.eat(&Token::Comma) {
            self.object(ctx, subject, verb, out)?;
        }
        Ok(())
    }

    /// `[31] ObjectData ::= GraphNodeData AnnotationData`,
    /// `[53] ObjectTemplate ::= GraphNodeTemplate AnnotationTemplate`,
    /// `[68] ObjectPattern ::= GraphNodePattern AnnotationPattern`.
    fn object(
        &mut self,
        ctx: Place,
        subject: &PatternTerm,
        verb: &Verb,
        out: &mut Vec<TriplePattern>,
    ) -> Parse<()> {
        let object = self.graph_node(ctx, out)?;
        let triple = match verb {
            Verb::Term(predicate) => {
                let triple = TriplePattern::new(subject.clone(), predicate.clone(), object);
                out.push(triple.clone());
                Some(triple)
            }
            Verb::Path(path) => {
                let before = out.len();
                self.expand_path(subject.clone(), path, object, out);
                (out.len() == before + 1).then(|| out[before].clone())
            }
        };
        self.annotation(ctx, triple.as_ref(), out)
    }

    /// Expand `subject path object` into triple patterns.
    fn expand_path(
        &mut self,
        subject: PatternTerm,
        path: &SrlPath,
        object: PatternTerm,
        out: &mut Vec<TriplePattern>,
    ) {
        match path {
            SrlPath::Iri(iri) => out.push(TriplePattern::new(
                subject,
                PatternTerm::Term(Term::NamedNode(iri.clone())),
                object,
            )),
            SrlPath::Inverse(inner) => self.expand_path(object, inner, subject, out),
            SrlPath::Sequence(steps) => {
                let mut from = subject;
                for (index, step) in steps.iter().enumerate() {
                    let to = if index + 1 == steps.len() {
                        object.clone()
                    } else {
                        self.fresh()
                    };
                    self.expand_path(from, step, to.clone(), out);
                    from = to;
                }
            }
        }
    }

    /// `[36] AnnotationData ::= ( ReifierData | AnnotationBlockData )*`,
    /// `[37] AnnotationBlockData ::= '{|' PropertyListNotEmptyData '|}'`,
    /// `[38] ReifierData ::= '~' ReifierIdData?`, `[39] ReifierIdData ::= iri | BlankNode`;
    /// `[58]`/`[59]`/`[72]`/`[73]` and `[75] Reifier ::= '~' ReifierId?`,
    /// `[76] ReifierId ::= Var | iri | BlankNode` in templates and patterns.
    ///
    /// A reifier `r` of the triple `s p o` is the triple `r rdf:reifies <<( s p o )>>`,
    /// `r` fresh when not written; an annotation block holds triples about the reifier
    /// written just before it, or about a fresh reifier when none is.
    fn annotation(
        &mut self,
        ctx: Place,
        triple: Option<&TriplePattern>,
        out: &mut Vec<TriplePattern>,
    ) -> Parse<()> {
        let mut reifier: Option<PatternTerm> = None;
        loop {
            if !self.at(&Token::Tilde) && !self.at(&Token::AnnotationOpen) {
                return Ok(());
            }
            let Some(triple) = triple else {
                return self.error(
                    "a reifier or annotation block names ONE triple, and a property path of \
                     more than one step is not one triple",
                );
            };
            if self.eat(&Token::Tilde) {
                let id = if self.can_start_reifier_id(ctx) {
                    self.reifier_id(ctx)?
                } else {
                    self.fresh()
                };
                out.push(reifies(id.clone(), triple));
                reifier = Some(id);
            } else {
                self.pos += 1;
                let id = match reifier.take() {
                    Some(id) => id,
                    None => {
                        let id = self.fresh();
                        out.push(reifies(id.clone(), triple));
                        id
                    }
                };
                self.property_list_not_empty(ctx, &id, out)?;
                self.expect(&Token::AnnotationClose, "|}")?;
            }
        }
    }

    fn can_start_reifier_id(&self, ctx: Place) -> bool {
        match self.peek() {
            Some(Token::Variable(_)) => ctx != Place::Data,
            Some(
                Token::Iri(_) | Token::PrefixedName(..) | Token::BlankNodeLabel(_) | Token::Anon,
            ) => true,
            _ => false,
        }
    }

    /// `ReifierIdData ::= iri | BlankNode`, `ReifierId ::= Var | iri | BlankNode`.
    fn reifier_id(&mut self, ctx: Place) -> Parse<PatternTerm> {
        match self.peek() {
            Some(Token::Variable(_)) if ctx == Place::Data => {
                self.error("variables are not allowed in a DATA block")
            }
            Some(Token::Variable(name)) => {
                let name = (*name).to_owned();
                self.pos += 1;
                Ok(PatternTerm::Variable(name))
            }
            Some(Token::Iri(_) | Token::PrefixedName(..)) => {
                Ok(PatternTerm::Term(Term::NamedNode(self.iri()?)))
            }
            _ => self.blank_node(),
        }
    }

    /// `[32] GraphNodeData ::= RDFTermData | TriplesNodeData | ReifiedTripleData`,
    /// `[54] GraphNodeTemplate ::= VarOrRDFTerm | TriplesNodeTemplate | ReifiedTriple`,
    /// `[74] GraphNodePattern ::= VarOrRDFTerm | TriplesNodePattern | ReifiedTriple`.
    fn graph_node(&mut self, ctx: Place, out: &mut Vec<TriplePattern>) -> Parse<PatternTerm> {
        if self.at_reified_open() {
            self.reified_triple(ctx, out)
        } else if self.at(&Token::LBracket) || (self.at(&Token::LParen) && !self.at_nil()) {
            self.triples_node(ctx, out)
        } else {
            self.var_or_term(ctx)
        }
    }

    /// `[33] TriplesNodeData ::= CollectionData | BlankNodePropertyListData`,
    /// `[34] BlankNodePropertyListData ::= '[' PropertyListNotEmptyData ']'`,
    /// `[35] CollectionData ::= '(' GraphNodeData+ ')'` (and [55]–[57], [69]–[71]).
    fn triples_node(&mut self, ctx: Place, out: &mut Vec<TriplePattern>) -> Parse<PatternTerm> {
        if self.eat(&Token::LBracket) {
            let node = self.fresh();
            self.property_list_not_empty(ctx, &node, out)?;
            self.expect(&Token::RBracket, "]")?;
            return Ok(node);
        }
        self.expect(&Token::LParen, "(")?;
        let mut items = Vec::new();
        while !self.at(&Token::RParen) {
            if self.pos >= self.tokens.len() {
                return self.error("unterminated collection");
            }
            items.push(self.graph_node(ctx, out)?);
        }
        self.pos += 1;
        let cells: Vec<PatternTerm> = items.iter().map(|_| self.fresh()).collect();
        for (index, (cell, item)) in cells.iter().zip(items).enumerate() {
            out.push(TriplePattern::new(cell.clone(), iri_term(rdf::FIRST), item));
            let rest = cells
                .get(index + 1)
                .cloned()
                .unwrap_or_else(|| iri_term(rdf::NIL));
            out.push(TriplePattern::new(cell.clone(), iri_term(rdf::REST), rest));
        }
        Ok(cells
            .into_iter()
            .next()
            .expect("a collection holds at least one item"))
    }

    /// `ReifiedTriple ::= '<<' ReifiedTripleSubject Verb ReifiedTripleObject Reifier? '>>'`
    /// ([41], [77]), `ReifiedTripleSubject ::= Var | iri | RDFLiteral | NumericLiteral |
    /// BooleanLiteral | BlankNode | ReifiedTriple | TripleTerm` ([42], [43], [78], [79]).
    /// It denotes its reifier, and states `reifier rdf:reifies <<( s p o )>>`.
    fn reified_triple(&mut self, ctx: Place, out: &mut Vec<TriplePattern>) -> Parse<PatternTerm> {
        self.expect(&Token::TripleOpen, "<<")?;
        let subject = self.reified_component(ctx, out)?;
        let predicate = self.simple_verb(ctx)?;
        let object = self.reified_component(ctx, out)?;
        let id = if self.eat(&Token::Tilde) {
            if self.can_start_reifier_id(ctx) {
                self.reifier_id(ctx)?
            } else {
                self.fresh()
            }
        } else {
            self.fresh()
        };
        self.expect(&Token::TripleClose, ">>")?;
        out.push(reifies(
            id.clone(),
            &TriplePattern::new(subject, predicate, object),
        ));
        Ok(id)
    }

    fn reified_component(
        &mut self,
        ctx: Place,
        out: &mut Vec<TriplePattern>,
    ) -> Parse<PatternTerm> {
        if self.at_reified_open() {
            return self.reified_triple(ctx, out);
        }
        if self.at_nil() || self.at(&Token::LParen) || self.at(&Token::LBracket) {
            return self.error(format!(
                "a reified triple's subject and object are terms, found {}",
                self.describe()
            ));
        }
        self.var_or_term(ctx)
    }

    /// `TripleTerm ::= '<<(' TripleTermSubject Verb TripleTermObject ')>>'` ([44], [80]),
    /// `TripleTermSubject ::= Var | iri | RDFLiteral | NumericLiteral | BooleanLiteral |
    /// BlankNode | TripleTerm` ([45], [46], [81], [82]).
    fn triple_term(&mut self, ctx: Place) -> Parse<PatternTerm> {
        self.expect_triple_term_open()?;
        let subject = self.triple_term_component(ctx)?;
        let predicate = self.simple_verb(ctx)?;
        let object = self.triple_term_component(ctx)?;
        self.expect_triple_term_close()?;
        Ok(make_triple(subject, predicate, object))
    }

    fn triple_term_component(&mut self, ctx: Place) -> Parse<PatternTerm> {
        if self.at_nil()
            || self.at(&Token::LParen)
            || self.at(&Token::LBracket)
            || self.at_reified_open()
        {
            return self.error(format!(
                "a triple term's subject and object are terms or triple terms, found {}",
                self.describe()
            ));
        }
        self.var_or_term(ctx)
    }

    /// `[89] RDFTermData ::= iri | RDFLiteral | NumericLiteral | BooleanLiteral | BlankNode
    /// | NIL | TripleTermData`, `[90] VarOrRDFTerm ::= Var | iri | RDFLiteral |
    /// NumericLiteral | BooleanLiteral | BlankNode | NIL | TripleTerm`.
    fn var_or_term(&mut self, ctx: Place) -> Parse<PatternTerm> {
        if self.at_nil() {
            self.pos += 2;
            return Ok(iri_term(rdf::NIL));
        }
        if self.at_triple_term_open() {
            return self.triple_term(ctx);
        }
        match self.peek() {
            Some(Token::Variable(_)) if ctx == Place::Data => {
                self.error("variables are not allowed in a DATA block")
            }
            Some(Token::Variable(name)) => {
                let name = (*name).to_owned();
                self.pos += 1;
                Ok(PatternTerm::Variable(name))
            }
            Some(Token::Iri(_) | Token::PrefixedName(..)) => {
                Ok(PatternTerm::Term(Term::NamedNode(self.iri()?)))
            }
            Some(Token::BlankNodeLabel(_) | Token::Anon) => self.blank_node(),
            _ => match self.literal()? {
                Some(literal) => Ok(PatternTerm::Term(Term::Literal(literal.term()))),
                None => self.error(format!("expected an RDF term, found {}", self.describe())),
            },
        }
    }

    /// `[102] BlankNode ::= BLANK_NODE_LABEL | ANON`.
    fn blank_node(&mut self) -> Parse<PatternTerm> {
        match self.peek() {
            Some(Token::BlankNodeLabel(label)) => {
                let label = (*label).to_owned();
                self.pos += 1;
                Ok(PatternTerm::BlankNode(label))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(self.fresh())
            }
            _ => self.error(format!("expected a blank node, found {}", self.describe())),
        }
    }

    // ── terms ───────────────────────────────────────────────────────────────────────

    /// `[100] iri ::= IRIREF | PrefixedName`, `[101] PrefixedName ::= PNAME_LN | PNAME_NS`.
    fn iri(&mut self) -> Parse<NamedNode> {
        match self.peek().cloned() {
            Some(Token::Iri(reference)) => {
                let resolved = self.resolve(&reference)?;
                self.pos += 1;
                Ok(NamedNode::from(resolved.as_str()))
            }
            Some(Token::PrefixedName(prefix, local)) => {
                let Some(namespace) = self.prefixes.get(prefix) else {
                    return self.error(format!("undeclared prefix `{prefix}:`"));
                };
                let iri = format!("{namespace}{local}");
                purrdf_iri::parse(&iri).map_err(|e| SyntaxError {
                    message: format!(
                        "`{prefix}:{local}` expands to <{iri}>, which is not an IRI: {e}"
                    ),
                    offset: self.offset(),
                })?;
                self.pos += 1;
                Ok(NamedNode::from(iri.as_str()))
            }
            _ => self.error(format!("expected an IRI, found {}", self.describe())),
        }
    }

    /// Resolve an IRI reference against the base in scope (§7.4).
    fn resolve(&self, reference: &str) -> Parse<String> {
        self.base
            .resolve(reference)
            .map(|iri| iri.as_str().to_owned())
            .map_err(|e| SyntaxError {
                message: format!("<{reference}>: {e}"),
                offset: self.offset(),
            })
    }

    /// `[93] RDFLiteral ::= String ( LANG_DIR | '^^' iri )?`,
    /// `[94] NumericLiteral`, `[98] BooleanLiteral ::= 'true' | 'false'`; `None` when the
    /// current token starts none of them.
    fn literal(&mut self) -> Parse<Option<Lit>> {
        // A NumericLiteralPositive/Negative is ONE terminal: the sign touches the numeral.
        let signed = matches!(self.peek(), Some(Token::Plus | Token::Minus))
            && matches!(
                self.peek_at(1),
                Some(Token::Integer(_) | Token::Decimal(_) | Token::Double(_))
            )
            && self.adjacent(0);
        let sign = if signed {
            let sign = if self.at(&Token::Minus) { "-" } else { "+" };
            self.pos += 1;
            sign
        } else {
            ""
        };
        let number = |lexical: &str, datatype: &str| Lit {
            lexical: format!("{sign}{lexical}"),
            datatype: Some(datatype.to_owned()),
            language: None,
        };
        let literal = match self.peek().cloned() {
            Some(Token::Integer(lexical)) => number(lexical, xsd::INTEGER),
            Some(Token::Decimal(lexical)) => number(lexical, xsd::DECIMAL),
            Some(Token::Double(lexical)) => number(lexical, xsd::DOUBLE),
            Some(Token::Word(w)) if w == "true" || w == "false" => Lit {
                lexical: w.to_owned(),
                datatype: Some(xsd::BOOLEAN.to_owned()),
                language: None,
            },
            Some(Token::StringLit(value) | Token::LongStringLit(value)) => {
                self.pos += 1;
                let value = value.into_owned();
                if let Some(Token::LangTag(tag)) = self.peek().cloned() {
                    let language = self.lang_dir(tag)?;
                    self.pos += 1;
                    return Ok(Some(Lit {
                        lexical: value,
                        datatype: None,
                        language: Some(language),
                    }));
                }
                if self.eat(&Token::HatHat) {
                    let datatype = self.iri()?;
                    return Ok(Some(Lit {
                        lexical: value,
                        datatype: Some(datatype.as_str().to_owned()),
                        language: None,
                    }));
                }
                return Ok(Some(Lit {
                    lexical: value,
                    datatype: Some(xsd::STRING.to_owned()),
                    language: None,
                }));
            }
            _ => return Ok(None),
        };
        self.pos += 1;
        Ok(Some(literal))
    }

    /// `[125] LANG_DIR ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)* ('--' [a-zA-Z]+)?`, with the
    /// base direction one of RDF 1.2's two: "The base direction … MUST be one of `ltr` or
    /// `rtl`" (RDF 1.2 Concepts §3.3), and the tag held to the workspace's one concrete-
    /// syntax language-tag profile, the profile every other RDF reader here applies.
    fn lang_dir(&self, tag: &str) -> Parse<(String, Option<RdfTextDirection>)> {
        let (language, direction) = match tag.split_once("--") {
            Some((language, "ltr")) => (language, Some(RdfTextDirection::Ltr)),
            Some((language, "rtl")) => (language, Some(RdfTextDirection::Rtl)),
            Some((_, other)) => {
                return self.error(format!(
                    "`@{tag}`: the base direction `--{other}` is neither `ltr` nor `rtl` (RDF 1.2 \
                     base directions are exactly these two, in lower case)"
                ));
            }
            None => (tag, None),
        };
        if let Err(e) =
            langtag::parse_with(language, langtag::Profile::ConcreteSyntaxLangtagBounded)
        {
            return self.error(format!("`@{tag}`: invalid language tag: {e}"));
        }
        Ok((language.to_owned(), direction))
    }

    // ── expressions [103]–[118] ─────────────────────────────────────────────────────

    /// `[17] Constraint ::= BrackettedExpression | BuiltInCall | FunctionCall`,
    /// `[18] FunctionCall ::= iri ArgList`.
    fn constraint(&mut self) -> Parse<Expression> {
        match self.peek() {
            Some(Token::LParen) => self.bracketted(),
            Some(Token::Word(_)) if !self.at_word_literal() => self.builtin_call(),
            Some(Token::Iri(_) | Token::PrefixedName(..)) => {
                let iri = self.iri()?;
                if !self.at(&Token::LParen) {
                    return self.error(format!(
                        "a FILTER constraint that is an IRI must be a function call `iri(…)`, \
                         found {}",
                        self.describe()
                    ));
                }
                let args = self.arg_list()?;
                Ok(Expression::FunctionCall(
                    Function::Custom(algebra_iri(&iri)),
                    args,
                ))
            }
            _ => self.error(format!(
                "expected a FILTER constraint (a bracketted expression, a built-in call or a \
                 function call), found {}",
                self.describe()
            )),
        }
    }

    fn at_word_literal(&self) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if *w == "true" || *w == "false")
    }

    /// `[117] BrackettedExpression ::= '(' Expression ')'`.
    fn bracketted(&mut self) -> Parse<Expression> {
        self.expect(&Token::LParen, "(")?;
        let expression = self.expression()?;
        self.expect(&Token::RParen, ")")?;
        Ok(expression)
    }

    /// `[103] Expression ::= ConditionalOrExpression`,
    /// `[104] ConditionalOrExpression ::= ConditionalAndExpression ( '||'
    /// ConditionalAndExpression )*`.
    fn expression(&mut self) -> Parse<Expression> {
        let mut left = self.conditional_and()?;
        while self.eat(&Token::Or) {
            let right = self.conditional_and()?;
            left = Expression::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// `[105] ConditionalAndExpression ::= ValueLogical ( '&&' ValueLogical )*`,
    /// `[106] ValueLogical ::= RelationalExpression`.
    fn conditional_and(&mut self) -> Parse<Expression> {
        let mut left = self.relational()?;
        while self.eat(&Token::And) {
            let right = self.relational()?;
            left = Expression::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// `[107] RelationalExpression ::= NumericExpression ( '=' NumericExpression | '!='
    /// NumericExpression | '<' NumericExpression | '>' NumericExpression | '<='
    /// NumericExpression | '>=' NumericExpression | 'IN' ExpressionList | 'NOT' 'IN'
    /// ExpressionList )?`, `[108] NumericExpression ::= AdditiveExpression`.
    fn relational(&mut self) -> Parse<Expression> {
        let left = self.additive()?;
        let binary: Option<BinaryOp> = match self.peek() {
            Some(Token::Eq) => Some(Expression::Equal),
            Some(Token::Lt) => Some(Expression::Less),
            Some(Token::Gt) => Some(Expression::Greater),
            Some(Token::LtEq) => Some(Expression::LessOrEqual),
            Some(Token::GtEq) => Some(Expression::GreaterOrEqual),
            _ => None,
        };
        if let Some(op) = binary {
            self.pos += 1;
            let right = self.additive()?;
            return Ok(op(Box::new(left), Box::new(right)));
        }
        if self.eat(&Token::NotEq) {
            let right = self.additive()?;
            return Ok(Expression::Not(Box::new(Expression::Equal(
                Box::new(left),
                Box::new(right),
            ))));
        }
        if self.eat_kw("IN") {
            let list = self.expression_list()?;
            return Ok(Expression::In(Box::new(left), list));
        }
        if self.at_kw("NOT")
            && matches!(self.peek_at(1), Some(Token::Word(w)) if w.eq_ignore_ascii_case("IN"))
        {
            self.pos += 2;
            let list = self.expression_list()?;
            return Ok(Expression::Not(Box::new(Expression::In(
                Box::new(left),
                list,
            ))));
        }
        Ok(left)
    }

    /// `[109] AdditiveExpression ::= MultiplicativeExpression ( '+' MultiplicativeExpression
    /// | '-' MultiplicativeExpression | ( NumericLiteralPositive | NumericLiteralNegative )
    /// ( ( '*' UnaryExpression ) | ( '/' UnaryExpression ) )* )*`.
    ///
    /// The third alternative — `?a -1`, a signed numeral directly after an operand — is
    /// `?a + (-1)`, and `?a -1 * 2` is `?a + (-1 * 2)`. The lexer gives the sign its own
    /// token, so both read here as the binary operator: `?a - 1` and `?a - (1 * 2)` denote
    /// the same value as the grammar's reading in every numeric type, since negation is
    /// exact.
    fn additive(&mut self) -> Parse<Expression> {
        let mut left = self.multiplicative()?;
        loop {
            if self.eat(&Token::Plus) {
                let right = self.multiplicative()?;
                left = Expression::Add(Box::new(left), Box::new(right));
            } else if self.eat(&Token::Minus) {
                let right = self.multiplicative()?;
                left = Expression::Subtract(Box::new(left), Box::new(right));
            } else {
                return Ok(left);
            }
        }
    }

    /// `[110] MultiplicativeExpression ::= UnaryExpression ( '*' UnaryExpression | '/'
    /// UnaryExpression )*`.
    fn multiplicative(&mut self) -> Parse<Expression> {
        let mut left = self.unary()?;
        loop {
            if self.eat(&Token::Star) {
                let right = self.unary()?;
                left = Expression::Multiply(Box::new(left), Box::new(right));
            } else if self.eat(&Token::Slash) {
                let right = self.unary()?;
                left = Expression::Divide(Box::new(left), Box::new(right));
            } else {
                return Ok(left);
            }
        }
    }

    /// `[111] UnaryExpression ::= '!' PrimaryExpression | '+' PrimaryExpression | '-'
    /// PrimaryExpression | PrimaryExpression`.
    fn unary(&mut self) -> Parse<Expression> {
        if self.eat(&Token::Bang) {
            return Ok(Expression::Not(Box::new(self.primary()?)));
        }
        if self.eat(&Token::Plus) {
            return Ok(Expression::UnaryPlus(Box::new(self.primary()?)));
        }
        if self.eat(&Token::Minus) {
            return Ok(Expression::UnaryMinus(Box::new(self.primary()?)));
        }
        self.primary()
    }

    /// `[112] PrimaryExpression ::= BrackettedExpression | BuiltInCall | iriOrFunction |
    /// RDFLiteral | NumericLiteral | BooleanLiteral | Var | ExprTripleTerm`,
    /// `[113] iriOrFunction ::= iri ArgList?`.
    fn primary(&mut self) -> Parse<Expression> {
        if self.at_triple_term_open() {
            return self.expr_triple_term();
        }
        match self.peek() {
            Some(Token::LParen) => self.bracketted(),
            Some(Token::Variable(name)) => {
                let variable = Variable::new(*name);
                self.pos += 1;
                Ok(Expression::Variable(variable))
            }
            Some(Token::Iri(_) | Token::PrefixedName(..)) => {
                let iri = self.iri()?;
                if self.at(&Token::LParen) {
                    let args = self.arg_list()?;
                    Ok(Expression::FunctionCall(
                        Function::Custom(algebra_iri(&iri)),
                        args,
                    ))
                } else {
                    Ok(Expression::NamedNode(algebra_iri(&iri)))
                }
            }
            Some(Token::Word(_)) if !self.at_word_literal() => self.builtin_call(),
            _ => match self.literal()? {
                Some(literal) => Ok(Expression::Literal(literal.algebra())),
                None => self.error(format!("expected an expression, found {}", self.describe())),
            },
        }
    }

    /// `[114] ExprTripleTerm ::= '<<(' ExprTripleTermSubject Verb ExprTripleTermObject
    /// ')>>'`, `[115] ExprTripleTermSubject ::= iri | RDFLiteral | NumericLiteral |
    /// BooleanLiteral | Var`, `[116] ExprTripleTermObject ::= iri | RDFLiteral |
    /// NumericLiteral | BooleanLiteral | Var | ExprTripleTerm` — the SPARQL `TRIPLE`
    /// function of its three components.
    fn expr_triple_term(&mut self) -> Parse<Expression> {
        self.expect_triple_term_open()?;
        let subject = self.expr_triple_component(false)?;
        let predicate = match self.simple_verb(Place::Head)? {
            PatternTerm::Variable(name) => Expression::Variable(Variable::new(name)),
            PatternTerm::Term(Term::NamedNode(iri)) => Expression::NamedNode(algebra_iri(&iri)),
            other => unreachable!("a verb is an IRI or a variable, not {other:?}"),
        };
        let object = self.expr_triple_component(true)?;
        self.expect_triple_term_close()?;
        Ok(Expression::FunctionCall(
            Function::Triple,
            vec![subject, predicate, object],
        ))
    }

    fn expr_triple_component(&mut self, object: bool) -> Parse<Expression> {
        if object && self.at_triple_term_open() {
            return self.expr_triple_term();
        }
        match self.peek() {
            Some(Token::Variable(name)) => {
                let variable = Variable::new(*name);
                self.pos += 1;
                Ok(Expression::Variable(variable))
            }
            Some(Token::Iri(_) | Token::PrefixedName(..)) => {
                Ok(Expression::NamedNode(algebra_iri(&self.iri()?)))
            }
            _ => match self.literal()? {
                Some(literal) => Ok(Expression::Literal(literal.algebra())),
                None => self.error(format!(
                    "expected an IRI, a literal or a variable in a triple term, found {}",
                    self.describe()
                )),
            },
        }
    }

    /// `[19] ArgList ::= NIL | '(' Expression ( ',' Expression )* ')'`.
    fn arg_list(&mut self) -> Parse<Vec<Expression>> {
        self.expression_list()
    }

    /// `[20] ExpressionList ::= NIL | '(' Expression ( ',' Expression )* ')'`.
    fn expression_list(&mut self) -> Parse<Vec<Expression>> {
        if self.at_nil() {
            self.pos += 2;
            return Ok(Vec::new());
        }
        self.expect(&Token::LParen, "(")?;
        let mut list = vec![self.expression()?];
        while self.eat(&Token::Comma) {
            list.push(self.expression()?);
        }
        self.expect(&Token::RParen, ")")?;
        Ok(list)
    }

    /// `[118] BuiltInCall`: SRL's own list, each with the arity its production spells.
    fn builtin_call(&mut self) -> Parse<Expression> {
        let Some(Token::Word(name)) = self.peek().cloned() else {
            return self.error(format!(
                "expected a built-in call, found {}",
                self.describe()
            ));
        };
        let upper = name.to_ascii_uppercase();
        let Some(builtin) = builtin(&upper) else {
            let why = match upper.as_str() {
                "COALESCE" | "BOUND" => " (SPARQL 1.2 RL §5: \"There is no COALESCE, nor BOUND\")",
                "RAND" => " (SPARQL 1.2 RL §5: \"There is no RAND\")",
                "MD5" | "SHA1" | "SHA256" | "SHA384" | "SHA512" => {
                    " (SPARQL 1.2 RL §5: \"There are no hash functions\")"
                }
                "EXISTS" => {
                    " (SPARQL 1.2 RL §5: \"The syntax of NOT limits the inner body to triple \
                     patterns and filters, and does not allow nested patterns, unlike SPARQL \
                     FILTER NOT EXISTS\")"
                }
                _ => "",
            };
            return self.error(format!(
                "`{name}` is not a SPARQL 1.2 RL built-in call{why}"
            ));
        };
        self.pos += 1;
        let args = match builtin.arity {
            Arity::Nil => {
                if !self.at_nil() {
                    return self.error(format!("`{name}` takes no arguments: expected `()`"));
                }
                self.pos += 2;
                Vec::new()
            }
            Arity::List => self.expression_list()?,
            Arity::Range(min, _) if min > 0 && self.at_nil() => {
                return self.error(format!("`{name}` takes arguments: found `()`"));
            }
            Arity::Range(min, max) => {
                if self.at_nil() && min == 0 {
                    self.pos += 2;
                    Vec::new()
                } else {
                    self.expect(&Token::LParen, "(")?;
                    let mut args = vec![self.expression()?];
                    while self.eat(&Token::Comma) {
                        args.push(self.expression()?);
                    }
                    self.expect(&Token::RParen, ")")?;
                    if args.len() < min || args.len() > max {
                        return self.error(format!(
                            "`{name}` takes {} argument{}, found {}",
                            if min == max {
                                min.to_string()
                            } else {
                                format!("{min} to {max}")
                            },
                            if max == 1 { "" } else { "s" },
                            args.len()
                        ));
                    }
                    args
                }
            }
        };
        Ok(match builtin.form {
            Form::Call(function) => Expression::FunctionCall(function, args),
            Form::If => {
                let mut args = args.into_iter();
                let (Some(a), Some(b), Some(c)) = (args.next(), args.next(), args.next()) else {
                    unreachable!("IF's arity is checked above")
                };
                Expression::If(Box::new(a), Box::new(b), Box::new(c))
            }
            Form::SameTerm => {
                let mut args = args.into_iter();
                let (Some(a), Some(b)) = (args.next(), args.next()) else {
                    unreachable!("sameTerm's arity is checked above")
                };
                Expression::SameTerm(Box::new(a), Box::new(b))
            }
        })
    }
}

/// A binary expression constructor.
type BinaryOp = fn(Box<Expression>, Box<Expression>) -> Expression;

/// A literal as parsed, before it becomes a term or an expression constant.
#[derive(Debug, Clone)]
struct Lit {
    /// The lexical form.
    lexical: String,
    /// The datatype IRI; `None` for a language-tagged string.
    datatype: Option<String>,
    /// The language tag and base direction.
    language: Option<(String, Option<RdfTextDirection>)>,
}

impl Lit {
    /// As an RDF term.
    fn term(self) -> Literal {
        match (self.language, self.datatype) {
            (Some((language, None)), _) => {
                Literal::new_language_tagged_literal_unchecked(self.lexical, language)
            }
            (Some((language, Some(direction))), _) => {
                Literal::new_directional_language_tagged_literal_unchecked(
                    self.lexical,
                    language,
                    direction,
                )
            }
            (None, Some(datatype)) if datatype == xsd::STRING => {
                Literal::new_simple_literal(self.lexical)
            }
            (None, datatype) => Literal::new_typed_literal(
                self.lexical,
                NamedNode::from(datatype.as_deref().unwrap_or(xsd::STRING)),
            ),
        }
    }

    /// As a SPARQL expression constant.
    fn algebra(self) -> purrdf_sparql_algebra::Literal {
        match (self.language, self.datatype) {
            (Some((language, direction)), _) => purrdf_sparql_algebra::Literal::new_lang(
                self.lexical,
                language,
                direction.map(|d| match d {
                    RdfTextDirection::Ltr => BaseDirection::Ltr,
                    RdfTextDirection::Rtl => BaseDirection::Rtl,
                }),
            ),
            (None, Some(datatype)) if datatype == xsd::STRING => {
                purrdf_sparql_algebra::Literal::new_simple(self.lexical)
            }
            (None, datatype) => purrdf_sparql_algebra::Literal::new_typed(
                self.lexical,
                purrdf_sparql_algebra::NamedNode::new_unchecked(
                    datatype.as_deref().unwrap_or(xsd::STRING),
                ),
            ),
        }
    }
}

/// How a built-in's arguments are written.
#[derive(Debug, Clone, Copy)]
enum Arity {
    /// `NAME NIL`.
    Nil,
    /// `NAME ExpressionList` (CONCAT).
    List,
    /// `NAME '(' Expression ( ',' Expression ){min-1,max-1} ')'`; `min == 0` admits `NIL`
    /// as well (BNODE).
    Range(usize, usize),
}

/// What a built-in call builds.
#[derive(Debug, Clone)]
enum Form {
    /// A function call.
    Call(Function),
    /// `IF(c, t, f)`.
    If,
    /// `sameTerm(a, b)`.
    SameTerm,
}

/// A built-in: its form and arity.
#[derive(Debug, Clone)]
struct Builtin {
    /// What it builds.
    form: Form,
    /// How its arguments are written.
    arity: Arity,
}

/// SPARQL 1.2 RL `[118] BuiltInCall`, keyed by the upper-cased keyword ("Keywords are
/// case-insensitive").
fn builtin(upper: &str) -> Option<Builtin> {
    let call = |function: Function, min: usize, max: usize| Builtin {
        form: Form::Call(function),
        arity: Arity::Range(min, max),
    };
    let one = |function: Function| call(function, 1, 1);
    let two = |function: Function| call(function, 2, 2);
    let nil = |function: Function| Builtin {
        form: Form::Call(function),
        arity: Arity::Nil,
    };
    Some(match upper {
        "STR" => one(Function::Str),
        "LANG" => one(Function::Lang),
        "LANGMATCHES" => two(Function::LangMatches),
        "LANGDIR" => one(Function::LangDir),
        "DATATYPE" => one(Function::Datatype),
        "IRI" => one(Function::Iri),
        "URI" => one(Function::Uri),
        // 'BNODE' ( '(' Expression ')' | NIL )
        "BNODE" => call(Function::BNode, 0, 1),
        "ABS" => one(Function::Abs),
        "CEIL" => one(Function::Ceil),
        "FLOOR" => one(Function::Floor),
        "ROUND" => one(Function::Round),
        "CONCAT" => Builtin {
            form: Form::Call(Function::Concat),
            arity: Arity::List,
        },
        "SUBSTR" => call(Function::SubStr, 2, 3),
        "STRLEN" => one(Function::StrLen),
        "REPLACE" => call(Function::Replace, 3, 4),
        "UCASE" => one(Function::UCase),
        "LCASE" => one(Function::LCase),
        "ENCODE_FOR_URI" => one(Function::EncodeForUri),
        "CONTAINS" => two(Function::Contains),
        "STRSTARTS" => two(Function::StrStarts),
        "STRENDS" => two(Function::StrEnds),
        "STRBEFORE" => two(Function::StrBefore),
        "STRAFTER" => two(Function::StrAfter),
        "YEAR" => one(Function::Year),
        "MONTH" => one(Function::Month),
        "DAY" => one(Function::Day),
        "HOURS" => one(Function::Hours),
        "MINUTES" => one(Function::Minutes),
        "SECONDS" => one(Function::Seconds),
        "TIMEZONE" => one(Function::Timezone),
        "TZ" => one(Function::Tz),
        "NOW" => nil(Function::Now),
        "UUID" => nil(Function::Uuid),
        "STRUUID" => nil(Function::StrUuid),
        "IF" => Builtin {
            form: Form::If,
            arity: Arity::Range(3, 3),
        },
        "STRLANG" => two(Function::StrLang),
        "STRLANGDIR" => call(Function::StrLangDir, 3, 3),
        "STRDT" => two(Function::StrDt),
        "SAMETERM" => Builtin {
            form: Form::SameTerm,
            arity: Arity::Range(2, 2),
        },
        "ISIRI" => one(Function::IsIri),
        "ISURI" => one(Function::IsUri),
        "ISBLANK" => one(Function::IsBlank),
        "ISLITERAL" => one(Function::IsLiteral),
        "ISNUMERIC" => one(Function::IsNumeric),
        "HASLANG" => one(Function::HasLang),
        "HASLANGDIR" => one(Function::HasLangDir),
        "REGEX" => call(Function::Regex, 2, 3),
        "ISTRIPLE" => one(Function::IsTriple),
        "TRIPLE" => call(Function::Triple, 3, 3),
        "SUBJECT" => one(Function::Subject),
        "PREDICATE" => one(Function::Predicate),
        "OBJECT" => one(Function::Object),
        _ => return None,
    })
}

/// An IRI as an expression constant.
fn algebra_iri(iri: &NamedNode) -> purrdf_sparql_algebra::NamedNode {
    purrdf_sparql_algebra::NamedNode::new_unchecked(iri.as_str())
}

/// An IRI pattern term.
fn iri_term(iri: &str) -> PatternTerm {
    PatternTerm::Term(Term::NamedNode(NamedNode::from(iri)))
}

/// The triple term `<<( s p o )>>`: a ground term when every position is a constant, a
/// triple-term pattern otherwise.
fn make_triple(subject: PatternTerm, predicate: PatternTerm, object: PatternTerm) -> PatternTerm {
    match (subject, predicate, object) {
        (
            PatternTerm::Term(subject),
            PatternTerm::Term(Term::NamedNode(predicate)),
            PatternTerm::Term(object),
        ) => PatternTerm::Term(Term::Triple(Box::new(Triple {
            subject,
            predicate,
            object,
        }))),
        (subject, predicate, object) => {
            PatternTerm::Triple(Box::new(TriplePattern::new(subject, predicate, object)))
        }
    }
}

/// `reifier rdf:reifies <<( s p o )>>`.
fn reifies(reifier: PatternTerm, triple: &TriplePattern) -> TriplePattern {
    TriplePattern::new(
        reifier,
        iri_term(rdf::REIFIES),
        make_triple(
            triple.subject.clone(),
            triple.predicate.clone(),
            triple.object.clone(),
        ),
    )
}

/// A data-block position as a term: a blank node is a blank node of the data. The
/// grammar admits no variable in a data block, so none reaches here.
fn ground(position: PatternTerm) -> Term {
    match position {
        PatternTerm::Term(term) => term,
        PatternTerm::BlankNode(label) => Term::BlankNode(label),
        PatternTerm::Triple(inner) => {
            let predicate = match ground(inner.predicate) {
                Term::NamedNode(iri) => iri,
                other => unreachable!("a data-block verb is an IRI, not {other}"),
            };
            Term::Triple(Box::new(Triple {
                subject: ground(inner.subject),
                predicate,
                object: ground(inner.object),
            }))
        }
        PatternTerm::Variable(name) => {
            unreachable!("the DATA productions admit no variable, yet ?{name} reached a data block")
        }
    }
}
