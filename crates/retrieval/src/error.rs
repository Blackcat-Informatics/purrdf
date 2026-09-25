// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed, structured failures of the retrieval-plan layer.
//!
//! Every failure carries the dimension that failed, so admission and decode
//! paths can name an exact reason rather than report a generic parse problem.
//! A version mismatch is its own variant precisely because a plan written by a
//! different build must be refused loudly, never silently reinterpreted under
//! the current layout.

/// A failure to plan a request, validate an IRI, or encode/decode a canonical
/// [`Plan`].
///
/// The planning variants are refusals with a named dimension — no producer
/// reaches the request, a request term is malformed, a stratum has no finite
/// depth, the request asks for more rows than a read can be planned for, or a
/// registered relation's declaration refused to be read — rather than a plausible
/// plan built over a fallback.
///
/// [`Plan`]: crate::Plan
#[derive(Debug)]
#[non_exhaustive]
pub enum PlanError {
    /// A plan's canonical encoding carries a version this build does not write.
    ///
    /// This is a refusal, not a warning: a plan whose layout a newer or older
    /// build defined cannot be decoded under this build's field order.
    VersionMismatch {
        /// The version found in the encoded plan.
        found: u16,
        /// The version this build writes and understands.
        expected: u16,
    },

    /// No registered producer accepts any term of the request.
    ///
    /// A request that reaches nothing is refused rather than answered by an
    /// empty plan: an empty plan is indistinguishable from a registry that
    /// needs no producers, and the caller asked for an answer.
    NoApplicableProducers,

    /// A request term is malformed and cannot be planned.
    InvalidRequestTerm {
        /// The offending term, carried whole so the caller can name it. Boxed so
        /// recording it does not inflate every `Result<_, PlanError>`.
        term: Box<crate::request::RequestTerm>,
        /// Why the term cannot be planned.
        reason: String,
    },

    /// The request's own row bound is a number no read this layer plans can reach.
    ///
    /// A [`ReadBound::Bounded`](crate::ReadBound::Bounded) states how many fused
    /// rows the answer is for, and where the producers' declarations license it
    /// that number *is* each stratum's depth (see [`plan`](crate::plan)). A depth
    /// is a 32-bit rank and a unit is emitted one row deeper than its depth, so the
    /// deepest depth that can be *read* is one shallower than the deepest a plan
    /// can express. A bound above that names a read no registry could serve.
    ///
    /// `ceiling` is therefore the deepest readable depth and not `u32::MAX`, and
    /// the distinction is the whole of this refusal's honesty: a bound of exactly
    /// `ceiling` plans, records that depth, and is emitted with the probe row one
    /// past it, while `u32::MAX` would leave the emitted bound no room for that row
    /// and the read would be reported exhausted however many rows the producer
    /// still held. Naming `u32::MAX` here would have stated a ceiling this layer
    /// cannot actually serve a read at.
    ///
    /// Refused at the request rather than where it happens to bind, because the
    /// alternative is a refusal that depends on which registry the request reaches
    /// — served silently wherever some declaration was smaller, and refused
    /// wherever it was not. It is the caller's own number, which is what separates
    /// it from a *declared* row bound past the same ceiling: that one is a fact
    /// about a producer's data rather than a request for rows, so it is recorded at
    /// the ceiling and the read's ending reports that the planned depth stopped it
    /// (see [`plan`](crate::plan)).
    ReadBoundBeyondDepthRange {
        /// The bound the request stated.
        requested: usize,
        /// The largest bound a read can be planned for: the deepest depth whose
        /// emitted bound can still carry the probe row.
        ceiling: u64,
    },

    /// A registered relation's declaration could not be read while planning.
    ///
    /// Every declaration read is panic-contained by the seam, so this reports a
    /// host relation whose own declaration panicked — a loud refusal rather than
    /// a producer silently dropped from the plan.
    RegistryDeclaration {
        /// The contained declaration failure, rendered.
        message: String,
    },

    /// The canonical encoding ended before a complete value was read.
    Truncated {
        /// The byte offset at which more data was needed.
        offset: usize,
    },

    /// A canonical discriminator byte named no known variant.
    InvalidTag {
        /// The field being decoded when the tag was read.
        what: &'static str,
        /// The tag byte that named no variant.
        tag: u8,
    },

    /// A canonical variable-length field was not valid UTF-8.
    InvalidUtf8 {
        /// The field being decoded.
        what: &'static str,
    },

    /// A canonical field did not parse as an IRI.
    InvalidIri {
        /// The offending IRI text.
        text: String,
        /// The kernel parser's refusal.
        source: purrdf_core::IriError,
    },

    /// The canonical encoding held bytes after the last field.
    TrailingBytes {
        /// The number of unconsumed bytes.
        extra: usize,
    },

    /// A producer that placement accepted declares no row bound at the mode it
    /// will be invoked under.
    ///
    /// # Why this cannot happen, written down where a change would break it
    ///
    /// `plan` reads `PropertyFunctionRegistry::describe` **once**, and both
    /// steps below read that one snapshot, so there is no window for the
    /// registry to move between them. Within that snapshot:
    ///
    /// * `matching::place` admits an invocation only when
    ///   `descriptor.modes` holds a declared mode that `subsumes` the invoked
    ///   one, and refuses with `PlacementError::NoSatisfiableMode` otherwise;
    /// * `admission::declared_row_bound` reads the bound by filtering that same
    ///   `descriptor.modes` on that same `subsumes` predicate against that same
    ///   invoked mode, and returns `RowBound::Undeclared` only when the filter
    ///   is empty.
    ///
    /// Same collection, same predicate, same argument: a placement that
    /// succeeded has already proved the filter is non-empty. Reaching this
    /// variant therefore means those two readings disagree, which is a defect in
    /// this crate rather than anything a caller's registry did — so the message
    /// names the disagreement rather than diagnosing a registry that moved,
    /// which within one snapshot it cannot have. Anything narrowing what `place`
    /// admits, or widening what `declared_row_bound` filters out, breaks the
    /// argument, and this is where it is written.
    ///
    /// # Why it is refused rather than defaulted
    ///
    /// Every available default is a lie about a number the depth is derived
    /// from: zero declares an empty relation and floors the read to a single
    /// probing row, while [`u64::MAX`] declares an unbounded one. A missing
    /// declaration can refuse nothing and a zero is a measurement, so the
    /// absence is reported here rather than resolved into either. The guard
    /// stays as defence in depth precisely because the proof above is a proof
    /// about today's two functions.
    ///
    /// # Why it ends the whole plan
    ///
    /// The neighbouring failure — `place` returning `Err` — records
    /// [`ProducerDecision::Rejected`](crate::ProducerDecision) and planning
    /// continues, and the asymmetry is deliberate.
    ///
    /// A placement failure is a fact **about that producer**: its declarations
    /// do not admit this request's invocation,
    /// [`RejectionReason`](crate::RejectionReason) has a variant that says so,
    /// and the other producers are unaffected. This is not a fact about the
    /// producer at all. No rejection reason means "the planner could not read a
    /// number it had just proved was there", and recording
    /// [`UnsatisfiedConstraint`](crate::RejectionReason::UnsatisfiedConstraint)
    /// would report the producer as
    /// having failed a constraint it did not fail — the misattribution a
    /// recorded derivation exists to remove. Worse, it would drop the stratum
    /// from the plan looking exactly like a producer that honestly did not
    /// apply.
    ///
    /// And the condition is not local. Both readings run for **every** placed
    /// producer, so a disagreement between them is not confined to the one
    /// producer it became visible at: continuing would emit a plan whose other
    /// strata were derived by the same broken reading, under a registry
    /// fingerprint asserting it was planned against declarations it was not read
    /// from. One refusal that names the producer is the smaller harm than a plan
    /// that looks complete.
    UndeclaredRowBound {
        /// The stratum whose declaration went missing.
        stratum: String,
        /// The producer that was placed without one.
        producer: String,
    },

    /// A statistics snapshot named one subject twice.
    ///
    /// The snapshot is a record of what a provider reported about a subject, so
    /// two rows for one subject are two answers to one question with nothing
    /// saying which was used. Refused rather than resolved, because resolving it
    /// — keeping the first, the last, or the wider — would be inventing a rule
    /// the data does not carry.
    DuplicateStatisticsSubject {
        /// The repeated subject, as its recorded text.
        subject: String,
    },

    /// A canonical encoding recorded two derivations for one stratum.
    ///
    /// A derivation is the argument list a depth was computed from, so two of
    /// them are two explanations of one number — and the plan carries only one
    /// of those numbers, so at most one of the explanations can be the one it
    /// was derived by. Refused rather than resolved by the map that reads them,
    /// which would keep whichever arrived last and let a forged document carry
    /// an explanation the encoder never wrote.
    ///
    /// This is the derivation record's own refusal and not
    /// [`DuplicateStatisticsSubject`](Self::DuplicateStatisticsSubject). The two
    /// name different dimensions of one plan: a snapshot subject is something a
    /// provider was *asked about*, a stratum derivation is something a depth was
    /// *computed from*, and a reader repairing a document needs to know which of
    /// the plan's two records of a stratum it is holding.
    DuplicateStratumDerivation {
        /// The repeated stratum, as its recorded IRI text.
        stratum: String,
    },

    /// A canonical encoding recorded two depths for one stratum.
    ///
    /// A depth decides how deep that stratum is actually read, so two of them
    /// are two different reads with nothing saying which the plan describes. The
    /// map the decoder fills would keep whichever arrived last — a silent choice
    /// between two claims — so the encoding is refused instead.
    DuplicateStratumDepth {
        /// The repeated stratum, as its recorded IRI text.
        stratum: String,
    },

    /// A canonical encoding listed a keyed section's keys out of ascending
    /// order.
    ///
    /// Canonical bytes are the plan's identity, which requires the map from
    /// plans to encodings to run both ways: one plan, one encoding, and one
    /// encoding, one plan. A decoder that accepted any order would break the
    /// second half — the same plan would have as many valid encodings as its
    /// sections have permutations, each digesting to the plan's one id while
    /// being a document the encoder would never write. So the order is required
    /// on the way in, exactly as it is established on the way out.
    ///
    /// It is also what makes the check affordable. Comparing each key against
    /// the one before it refuses a repeat and a reordering in one linear pass,
    /// where scanning everything already read for a repeat is quadratic in the
    /// length of a document the decoder does not control.
    ///
    /// # Why the section is a field rather than three variants
    ///
    /// Out-of-order is one fact about an encoding, and it means the same thing
    /// wherever it occurs: these bytes are not canonical. The section says where,
    /// and carrying it as a typed field is how
    /// [`StatisticsEntryContradictsDerivation`](Self::StatisticsEntryContradictsDerivation)
    /// carries its dimension. A *repeated* key is the opposite case — what two
    /// rows for one key mean depends entirely on what the key indexes, so each
    /// section refuses a repeat by its own name.
    NonAscendingCanonicalKeys {
        /// Which keyed section was being decoded.
        section: CanonicalSection,
        /// The key read immediately before, as its recorded text.
        previous: String,
        /// The key that did not follow it, as its recorded text.
        key: String,
    },

    /// A canonical encoding listed one record's selectivity-term run out of
    /// ascending order.
    ///
    /// The run is the domain of a **sum**, so the order it is written in carries
    /// no information about the data: `[1, 0]` and `[0, 1]` name one term set
    /// contributing one aggregate. Both spellings would therefore be encodings of
    /// one plan, each digesting to that plan's single id — the same break in the
    /// plan-to-bytes biconditional that
    /// [`NonAscendingCanonicalKeys`](Self::NonAscendingCanonicalKeys) refuses at
    /// the level of a section's keys, one nesting level further in.
    ///
    /// It carries the record's own subject as well as the section, because a run
    /// is nested inside a keyed entry: the section alone says which of a plan's
    /// two selectivity records moved, and a plan has one such record per stratum
    /// and per snapshot row.
    NonAscendingSelectivityTerms {
        /// Which keyed section the run was nested in.
        section: CanonicalSection,
        /// The stratum or snapshot subject whose run it is, as its recorded text.
        subject: String,
        /// The index read immediately before.
        previous: u32,
        /// The index that did not follow it.
        request_term: u32,
    },

    /// A canonical encoding named one request term twice in one selectivity-term
    /// run.
    ///
    /// The run is the domain of a sum over the terms a provider answered for, and
    /// a domain is a set: one term contributed to the aggregate once or not at
    /// all. A repeat says the same term was counted twice into a total the
    /// arithmetic reached once, so the run and the number beside it describe two
    /// different measurements.
    ///
    /// One variant rather than one per section, which is where this parts company
    /// with [`DuplicateStratumDerivation`](Self::DuplicateStratumDerivation) and
    /// [`DuplicateStatisticsSubject`](Self::DuplicateStatisticsSubject). Those
    /// keys index different dimensions of a plan, so a repeat means a different
    /// thing in each. Both selectivity-term runs index the *same* dimension — the
    /// plan's own request — so a repeat is one fact, and the section and subject
    /// locate the record that carries it.
    DuplicateSelectivityTerm {
        /// Which keyed section the run was nested in.
        section: CanonicalSection,
        /// The stratum or snapshot subject whose run it is, as its recorded text.
        subject: String,
        /// The repeated index.
        request_term: u32,
    },

    /// A recorded depth is not the depth its own recorded inputs derive.
    ///
    /// Raised only by [`Plan::certify`](crate::Plan::certify). A plan records
    /// every input its depths were derived from precisely so this is a checkable
    /// claim rather than an asserted one; a mismatch means the plan was edited,
    /// forged, or written by a build whose arithmetic differed, and in all three
    /// cases the depth beside the inputs describes a read the inputs do not
    /// license.
    DepthNotDerivable {
        /// The stratum whose depth does not follow from its inputs.
        stratum: String,
        /// The depth the plan records.
        recorded: u32,
        /// The depth the plan's own recorded inputs derive.
        derived: u32,
    },

    /// A stratum carries a recorded depth with no recorded derivation.
    ///
    /// An unrecorded input cannot be checked, so a depth without its inputs is
    /// exactly the unverifiable claim the derivation record exists to abolish.
    DepthWithoutDerivation {
        /// The stratum whose derivation is missing.
        stratum: String,
    },

    /// A stratum carries a recorded derivation with no recorded depth.
    ///
    /// The mirror of [`DepthWithoutDerivation`](Self::DepthWithoutDerivation),
    /// and refused separately because it is a different edit: inputs for a
    /// stratum the plan does not read at all.
    DerivationWithoutDepth {
        /// The stratum whose depth is missing.
        stratum: String,
    },

    /// A stratum a depth was derived for is named by no statistics-snapshot row.
    ///
    /// The snapshot's whole claim is that it names every subject planning
    /// consulted, and a stratum is the subject planning consulted in order to
    /// *decide*. A snapshot missing one is a plan asserting it consulted nothing
    /// for a depth it recorded — so the omission is refused rather than repaired
    /// from the derivation, which would let the plan's two records drift apart
    /// silently in exactly the direction this check exists to catch.
    DerivationWithoutStatisticsEntry {
        /// The stratum the snapshot does not name.
        stratum: String,
    },

    /// A stratum's statistics-snapshot row contradicts its own recorded
    /// derivation.
    ///
    /// The row is written as a projection of the derivation, so the two agree by
    /// construction in any plan this build emitted. A plan in which they differ
    /// was edited or forged, and the two readings license different depths with
    /// nothing saying which is the measurement — so it is refused, naming the
    /// dimension that disagreed and both of its values.
    StatisticsEntryContradictsDerivation {
        /// The stratum whose two records disagree.
        stratum: String,
        /// Which statistic they disagree about.
        dimension: StatisticsDimension,
        /// What the snapshot entry records for that dimension.
        snapshot: String,
        /// What the derivation records for it.
        derivation: String,
    },

    /// A recorded selectivity-term run names an index the plan's own request does
    /// not carry.
    ///
    /// Raised only by [`Plan::certify`](crate::Plan::certify). The run is the
    /// domain of a selectivity aggregate, written as indices into
    /// [`Plan::request_terms`](crate::Plan::request_terms) so that a reader can
    /// read the aggregate's derivation back onto the terms the caller wrote. An
    /// index addressing no term of that request names nothing at all: the
    /// aggregate's domain becomes unreadable, and the record that exists to make
    /// the sum checkable stops being checkable.
    ///
    /// It is the derivation's and the snapshot's member of the family the
    /// admission waist already enforces for
    /// [`ProducerBinding::request_terms`](crate::ProducerBinding::request_terms)
    /// and [`UnservedTerm::request_term`](crate::UnservedTerm::request_term). It
    /// is decided here rather than there because the plan carries its own request
    /// — so this is a property of the value alone, like every other question
    /// `certify` answers, and it holds for a plan reached by any path in rather
    /// than only for one being admitted against a registry.
    SelectivityTermOutOfRange {
        /// The stratum or snapshot subject whose run it is, as its recorded text.
        subject: String,
        /// The index that addresses no term.
        request_term: u32,
        /// How many terms the plan's own request carries.
        request_terms: usize,
    },

    /// A statistics-snapshot row names a subject nothing in the plan consulted.
    ///
    /// Raised only by [`Plan::certify`](crate::Plan::certify), and the mirror of
    /// [`DerivationWithoutStatisticsEntry`](Self::DerivationWithoutStatisticsEntry):
    /// that one refuses a consultation with no row, this one a row with no
    /// consultation. Both directions are needed, because the snapshot's claim
    /// runs both ways —
    /// [`StatisticsSnapshot`](crate::StatisticsSnapshot) states that it names
    /// every subject planning consulted *and* that a subject nothing consulted is
    /// absent rather than recorded as empty. Enforcing only the first leaves the
    /// second exactly where a forger would reach for it: a row can be added, and
    /// the plan then reads back as evidence about a consultation that never
    /// happened.
    ///
    /// Planning consults two kinds of subject and no others: a stratum, asked
    /// about in order to *decide* a depth, and the predicate of a request term,
    /// asked about in order to *report*. Both are read off the plan itself, so
    /// this is a property of the value alone, like every other question `certify`
    /// answers.
    UnconsultedStatisticsSubject {
        /// The subject nothing consulted, as its recorded text.
        subject: String,
    },
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionMismatch { found, expected } => write!(
                f,
                "unsupported plan version {found}; this build writes version {expected}"
            ),
            Self::NoApplicableProducers => {
                write!(f, "no registered producer accepts any term of the request")
            }
            Self::InvalidRequestTerm { reason, .. } => write!(f, "invalid request term: {reason}"),
            Self::ReadBoundBeyondDepthRange { requested, ceiling } => write!(
                f,
                "the request bounds the answer at {requested} fused rows, and no read can be planned that \
         deep: where the producers' declarations license it that bound is each stratum's own \
         depth, a depth is a 32-bit rank, and a read is emitted one row deeper than its depth — so \
         {ceiling} is the largest bound this layer can plan a read for, and a bound of exactly \
         that is served"
            ),
            Self::RegistryDeclaration { message } => {
                write!(f, "registry declaration failed while planning: {message}")
            }
            Self::Truncated { offset } => {
                write!(f, "plan canonical encoding is truncated at byte {offset}")
            }
            Self::InvalidTag { what, tag } => write!(
                f,
                "plan canonical encoding carries invalid tag {tag} for {what}"
            ),
            Self::InvalidUtf8 { what } => {
                write!(f, "plan canonical encoding carries invalid UTF-8 in {what}")
            }
            Self::InvalidIri { text, source } => write!(f, "invalid IRI {text:?}: {source}"),
            Self::TrailingBytes { extra } => {
                write!(f, "plan canonical encoding has {extra} trailing byte(s)")
            }
            Self::UndeclaredRowBound { stratum, producer } => write!(
                f,
                "producer {producer} was placed on stratum {stratum} but declares no row bound at the mode it is invoked under; placement and the row-bound read disagree about one snapshot of the registry's declarations"
            ),
            Self::DuplicateStatisticsSubject { subject } => write!(
                f,
                "statistics snapshot names subject {subject} more than once"
            ),
            Self::DuplicateStratumDerivation { stratum } => write!(
                f,
                "plan canonical encoding records a derivation for stratum {stratum} more than once"
            ),
            Self::DuplicateStratumDepth { stratum } => write!(
                f,
                "plan canonical encoding records a depth for stratum {stratum} more than once"
            ),
            Self::NonAscendingCanonicalKeys {
                section,
                previous,
                key,
            } => write!(
                f,
                "plan canonical encoding lists {section} key {key} after {previous}; a canonical encoding \
         orders them ascending"
            ),
            Self::NonAscendingSelectivityTerms {
                section,
                subject,
                previous,
                request_term,
            } => write!(
                f,
                "plan canonical encoding lists {section} {subject} selectivity term {request_term} after \
         {previous}; a canonical encoding orders them ascending"
            ),
            Self::DuplicateSelectivityTerm {
                section,
                subject,
                request_term,
            } => write!(
                f,
                "plan canonical encoding records {section} {subject} selectivity term {request_term} \
         twice; the aggregate is a sum over distinct terms"
            ),
            Self::DepthNotDerivable {
                stratum,
                recorded,
                derived,
            } => write!(
                f,
                "stratum {stratum} records depth {recorded}, but its recorded inputs derive depth {derived}"
            ),
            Self::DepthWithoutDerivation { stratum } => write!(
                f,
                "stratum {stratum} records a depth with no recorded derivation"
            ),
            Self::DerivationWithoutDepth { stratum } => write!(
                f,
                "stratum {stratum} records a derivation with no recorded depth"
            ),
            Self::DerivationWithoutStatisticsEntry { stratum } => write!(
                f,
                "stratum {stratum} records a derivation with no statistics snapshot entry"
            ),
            Self::StatisticsEntryContradictsDerivation {
                stratum,
                dimension,
                snapshot,
                derivation,
            } => write!(
                f,
                "stratum {stratum} records {dimension} {snapshot} in its statistics snapshot and {derivation} in its derivation"
            ),
            Self::SelectivityTermOutOfRange {
                subject,
                request_term,
                request_terms,
            } => write!(
                f,
                "subject {subject} records selectivity term {request_term}, but the plan carries \
         {request_terms} request term(s)"
            ),
            Self::UnconsultedStatisticsSubject { subject } => write!(
                f,
                "the statistics snapshot names subject {subject}, which is neither a stratum this plan \
         derives a depth for nor a predicate of any of its request terms"
            ),
        }
    }
}

impl std::error::Error for PlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidIri { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl PlanError {
    /// The pinned, machine-readable name of this refusal.
    ///
    /// A refusal's message is prose and may be reworded; this is the contract. A
    /// caller branching on which refusal it received — a host receiving plans
    /// from somewhere it does not control, a binding rendering them into its own
    /// vocabulary — reads this and never `to_string`.
    ///
    /// # It lives here rather than at each caller
    ///
    /// The enum is `#[non_exhaustive]`, so a match written in any other crate
    /// needs a wildcard arm, and a wildcard arm over a *name* has nothing honest
    /// to return: it would have to invent a word for a refusal it cannot name,
    /// handed to the caller at the one moment the caller is trying to find out
    /// which refusal it got. Inside this module the match is exhaustive, so a
    /// variant added without a name is a compile error here — the same discipline
    /// [`DepthCause`](crate::DepthCause) gets from being a closed enum.
    ///
    /// Every name is fixed once issued, for the reason a canonical tag is: a
    /// caller that branches on the string is broken by a rename exactly as
    /// silently as by a renumbered discriminator.
    #[must_use]
    pub fn refusal(&self) -> &'static str {
        match self {
            Self::VersionMismatch { .. } => "version",
            Self::NoApplicableProducers => "no-applicable-producers",
            Self::InvalidRequestTerm { .. } => "invalid-request-term",
            Self::ReadBoundBeyondDepthRange { .. } => "read-bound-beyond-depth-range",
            Self::RegistryDeclaration { .. } => "registry-declaration",
            Self::Truncated { .. } => "truncated",
            Self::InvalidTag { .. } => "invalid-tag",
            Self::InvalidUtf8 { .. } => "invalid-utf8",
            Self::InvalidIri { .. } => "invalid-iri",
            Self::TrailingBytes { .. } => "trailing-bytes",
            Self::UndeclaredRowBound { .. } => "undeclared-row-bound",
            Self::DuplicateStatisticsSubject { .. } => "duplicate-statistics-subject",
            Self::DuplicateStratumDerivation { .. } => "duplicate-stratum-derivation",
            Self::DuplicateStratumDepth { .. } => "duplicate-stratum-depth",
            Self::NonAscendingCanonicalKeys { .. } => "non-ascending-keys",
            Self::NonAscendingSelectivityTerms { .. } => "non-ascending-selectivity-terms",
            Self::DuplicateSelectivityTerm { .. } => "duplicate-selectivity-term",
            Self::SelectivityTermOutOfRange { .. } => "selectivity-term-out-of-range",
            Self::UnconsultedStatisticsSubject { .. } => "unconsulted-statistics-subject",
            Self::DepthNotDerivable { .. } => "depth-not-derivable",
            Self::DepthWithoutDerivation { .. } => "depth-without-derivation",
            Self::DerivationWithoutDepth { .. } => "derivation-without-depth",
            Self::DerivationWithoutStatisticsEntry { .. } => "derivation-without-statistics-entry",
            Self::StatisticsEntryContradictsDerivation { .. } => {
                "statistics-entry-contradicts-derivation"
            }
        }
    }
}

/// Which keyed section of a canonical encoding was being decoded.
///
/// Carried by
/// [`PlanError::NonAscendingCanonicalKeys`](PlanError::NonAscendingCanonicalKeys)
/// so a refusal says which of a plan's three keyed sequences was not ordered.
/// They are written by different encoders, read into different containers and
/// keyed on different things, and a caller repairing a document needs to know
/// which one moved.
///
/// The two sections that nest a selectivity-term run inside each entry carry it
/// for the same reason, on
/// [`PlanError::NonAscendingSelectivityTerms`](PlanError::NonAscendingSelectivityTerms)
/// and
/// [`PlanError::DuplicateSelectivityTerm`](PlanError::DuplicateSelectivityTerm):
/// a plan records one run per stratum and one per snapshot row, so the section
/// and the entry's own subject together are what locate it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CanonicalSection {
    /// The per-stratum depths, keyed by stratum IRI.
    StratumDepths,
    /// The per-stratum depth derivations, keyed by stratum IRI.
    StratumDerivations,
    /// The statistics snapshot's entries, keyed by subject.
    StatisticsEntries,
}

impl core::fmt::Display for CanonicalSection {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Self::StratumDepths => "stratum depth",
            Self::StratumDerivations => "stratum derivation",
            Self::StatisticsEntries => "statistics snapshot",
        };
        formatter.write_str(name)
    }
}

/// Which statistic a plan's two records of one stratum disagree about.
///
/// Carried by
/// [`PlanError::StatisticsEntryContradictsDerivation`](PlanError::StatisticsEntryContradictsDerivation)
/// so the refusal names a dimension rather than reporting that "the statistics"
/// differ: the three are measured separately, edited separately, and lead to
/// different depths, and a caller repairing a plan needs to know which one moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatisticsDimension {
    /// The row count the provider reported for the subject.
    Cardinality,
    /// The aggregate selectivity, in parts per million, that was applied.
    SelectivityPpm,
    /// The request-term indices that aggregate was summed over.
    SelectivityTerms,
}

impl core::fmt::Display for StatisticsDimension {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Self::Cardinality => "cardinality",
            Self::SelectivityPpm => "selectivity in parts per million",
            Self::SelectivityTerms => "selectivity terms",
        };
        formatter.write_str(name)
    }
}

/// A failure raised while building a profile or fusing ranked streams.
///
/// Every variant is a refusal, never a repaired or partial answer: an invalid
/// profile is rejected where it is supplied, a malformed stream is returned to
/// its producer, and an arithmetic intermediate that does not fit is reported
/// rather than wrapped.
#[derive(Debug)]
#[non_exhaustive]
pub enum FusionError {
    /// A checked arithmetic step left the range it is computed in: a
    /// fixed-point addition, the profile's derived ceiling
    /// (`max_weight × stratum count`), or a stratum count too large for the
    /// `u32` the canonical profile encoding writes. A wrapped value would be a
    /// wrong order presented as a right one, so it is refused.
    Overflow,

    /// A ranked stream violated the input protocol.
    Protocol(Box<crate::ranked_stream::ProtocolError>),

    /// The reciprocal-rank smoothing constant `K` was zero. `K >= 1` keeps a
    /// rank-one item's contribution finite and below one.
    InvalidK {
        /// The rejected value.
        k: u32,
    },

    /// A contribution was asked for at rank zero, or a depth of zero was asked
    /// to be reached.
    ///
    /// Ranks are 1-based, and a depth is a count of them read from rank one, so
    /// zero names no rank under either reading. The two share this variant
    /// because they are the same fact about the same 1-based axis; a depth-zero
    /// request is rejected rather than answered vacuously, which is what
    /// returning the lightest weight there is would be.
    InvalidRank {
        /// The rejected rank, or the rejected depth expressed as the rank it
        /// would have to reach.
        rank: u64,
    },

    /// A class-width tolerance of zero was offered as a tolerance.
    ///
    /// An indifference class always contains its own rank, so the narrowest
    /// class that exists is one rank wide and a tolerance of zero admits no
    /// class at all. It is the same shape as a smoothing constant of zero —
    /// a question with no evaluable content — and it is refused for the same
    /// reason rather than quietly read as one, which would answer the most
    /// favourable question available instead of the one that was asked.
    ///
    /// It does **not** share [`InvalidRank`](Self::InvalidRank), even though
    /// both are "this count may not be zero". That variant is about the 1-based
    /// rank axis, and its message says `rank must be at least 1`; a tolerance
    /// is a width measured *across* that axis rather than a position on it, and
    /// reporting a rejected tolerance as a rejected rank would send a caller to
    /// inspect an argument that was never at fault.
    InvalidWidth {
        /// The rejected tolerance.
        max_width: u64,
    },

    /// A fusion profile carried no stratum weights.
    EmptyWeights,

    /// A stratum weight was not strictly positive.
    NonPositiveWeight {
        /// The stratum whose weight was rejected, as its canonical IRI text.
        stratum: String,
        /// The rejected weight.
        weight: purrdf_text::Fixed,
    },

    /// A weight handed to the threshold-crossing derivation was not strictly
    /// positive.
    ///
    /// Separate from [`Self::NonPositiveWeight`], which names the stratum whose
    /// declaration was rejected while a profile was being built. This one is
    /// raised by
    /// [`crossing_rank_at`](crate::crossing_rank_at) over a bare list of
    /// weights, where there is no stratum to name and inventing one would
    /// attribute the refusal to a declaration nobody made. The premise it
    /// defends is the derivation's own: the threshold is non-increasing in the
    /// rank only while every weight is positive, and a bisection over a
    /// non-monotone predicate returns an arbitrary rank rather than a slightly
    /// wrong one.
    NonPositiveCrossingWeight {
        /// The rejected weight.
        weight: purrdf_text::Fixed,
    },

    /// A stream emitted under a stratum the profile declares no weight for.
    UnknownStratum {
        /// The undeclared stratum, as its canonical IRI text.
        stratum: String,
    },

    /// Two streams were tagged with the same stratum.
    DuplicateStratum {
        /// The repeated stratum, as its canonical IRI text.
        stratum: String,
    },

    /// The streams of one fusion do not descend from the same pinned plan.
    ///
    /// A fused answer names **one** plan, so streams that name two cannot be
    /// fused under a single trailer: whichever identity the answer carried
    /// would be right about some of its rows and wrong about the others. The
    /// refusal is narrow on purpose — streams that all name the same plan fuse,
    /// and streams that name no plan at all fuse too, producing an answer that
    /// simply names no plan. Only a disagreement is refused.
    PlanIdMismatch {
        /// The plan the fusion's other streams name.
        expected: crate::id::PlanId,
        /// The plan this stream names, or `None` when a stream that should have
        /// named one named nothing.
        got: Option<crate::id::PlanId>,
    },

    /// A stream's depth was planned for one row bound and the fusion was run at
    /// another.
    ///
    /// # Why this is a sibling of [`PlanIdMismatch`](Self::PlanIdMismatch) and not
    /// a case of it
    ///
    /// Both are refusals read off the streams before a row is pulled, and both
    /// exist because a provenance that travels with the rows is the only one worth
    /// having. They are different facts with different repairs, though.
    /// `PlanIdMismatch` is a disagreement *among the streams*: two of them descend
    /// from different plans, and the repair is to stop mixing them. This one is a
    /// disagreement between the streams — which may agree perfectly — and the
    /// caller's own `top_k` argument, and the repair is either to pass the bound the
    /// plan was built for or to plan again at the bound that is wanted. Folding it
    /// into the other variant would report a provenance conflict for a call whose
    /// provenance is consistent, and would send a reader to inspect the streams
    /// instead of the argument.
    ///
    /// # Why a smaller bound is refused too
    ///
    /// A `top_k` above what was planned cannot be served: under declarations that
    /// let the planner narrow a depth to the planned bound, the rows past it were
    /// never read. A `top_k` below it would answer correctly, but out of a read
    /// deeper than the question needed — and the depths the bundle records, the
    /// resolution it reports and the identity it carries would all describe a
    /// different request. One rule, in both directions, keeps the bound the plan
    /// recorded and the bound the answer was assembled under the same number.
    ReadBoundMismatch {
        /// The bound the streams' plan was built for.
        planned: crate::fuse::TopK,
        /// The bound this fusion was asked to run at.
        requested: crate::fuse::TopK,
    },

    /// A fusion profile's canonical bytes could not be decoded.
    MalformedProfile(String),

    /// A candidate received more contributions than there are strata.
    ///
    /// This is an **invariant violation, not a policy refusal**. A profile's
    /// contribution maximum is its stratum count
    /// ([`FusionProfile::max_contributions`](crate::FusionProfile::max_contributions)),
    /// derived rather than declared, and a candidate may surface at most once
    /// per stratum — so reaching `max + 1` means one of two things happened:
    /// a stream emitted the same candidate twice under a stratum whose
    /// per-stream uniqueness check did not see it, or the stream set handed to
    /// [`FusionStream::new`](crate::FusionStream::new) tagged two streams with
    /// the same stratum. [`fuse`](crate::fuse) refuses the second before a row
    /// is read ([`DuplicateStratum`](Self::DuplicateStratum)), so this arrives
    /// only from a hand-built fusion.
    ///
    /// It is not something a corpus can provoke. A candidate that surfaces in
    /// *every* stratum is the fusion working, not a bound being crossed, and
    /// this error can never name it.
    ///
    /// Checked immediately after the contribution that crossed the bound is
    /// recorded, so `count` is exactly `max + 1`, never a later, larger tally,
    /// and the offending candidate is named because "some candidate" is not a
    /// report anybody can act on.
    MaxContributionsExceeded {
        /// The candidate that exceeded the bound, as its canonical term text.
        item: String,
        /// The contribution count reached.
        count: u32,
        /// The profile's stratum count, which is its contribution maximum.
        max: u32,
    },

    /// No weight at all reaches the requested depth under this decay rule.
    ///
    /// This is a property of the rule's arithmetic, not a budget or a policy,
    /// and it is exact rather than conservative.
    /// [`DecayRule::ReciprocalRank`](crate::DecayRule::ReciprocalRank) truncates
    /// the reciprocal *before* applying the weight, so once `trunc(S / D)` and
    /// `trunc(S / (D + 1))` are equal the two ranks have already merged at the
    /// point the weight arrives and no weight can part them again. A caller that
    /// needs depth past `saturates_at` names
    /// [`DecayRule::WeightedReciprocalRank`](crate::DecayRule::WeightedReciprocalRank),
    /// which folds the weight into the numerator and whose reachable depth does
    /// grow with it.
    ///
    /// Reaching this is not an error in the answer: a profile read past its
    /// separating range still produces a correct, deterministic result at a
    /// coarser rank resolution. It is only a refusal to claim a depth the
    /// arithmetic cannot deliver.
    ///
    /// `saturates_at` is a **maximum over every weight**, not a reading taken at
    /// one of them. No weight was named in the question and none is named in the
    /// answer: the depth is quoted for the best weight there is, which under
    /// [`DecayRule::ReciprocalRank`](crate::DecayRule::ReciprocalRank) is any
    /// weight at or above one — below one the weight binds and the separating
    /// depth is shorter, at and above one the inner truncation binds instead and
    /// every such weight shares the same depth. Reading it as a property of some
    /// particular weight would invite the repair that does not exist, which is to
    /// raise that weight.
    ///
    /// `saturates_at` is always a measured rank and never a saturating case
    /// standing in for one: the rule was observed to stop separating, so there
    /// is an exact rank to report, and it is strictly below the depth asked
    /// for — that is what makes this refusal different from the one below. A
    /// depth that no *plan* can carry is a different fact and carries a
    /// different variant ([`DepthBeyondPlanRange`](Self::DepthBeyondPlanRange)),
    /// because nothing about the rule's arithmetic failed there.
    DepthUnreachable {
        /// The depth that was asked for.
        depth: u64,
        /// The deepest 1-based depth **any** weight separates every adjacent
        /// pair within under this rule: the maximum of the per-weight
        /// separating depths, not the depth of some one weight. Exact rather
        /// than conservative, and the pair immediately past it carries one
        /// contribution whatever weight is applied — so no heavier weight
        /// reaches further, which is why the remedy is the other decay rule and
        /// never a larger number here.
        saturates_at: u64,
    },

    /// The requested depth is deeper than a plan is able to record.
    ///
    /// A plan carries a per-stratum depth as a `u32`, so `limit` is the deepest
    /// depth anything downstream can name. This is a limit of that encoding and
    /// says nothing about the decay rule: under
    /// [`DecayRule::WeightedReciprocalRank`](crate::DecayRule::WeightedReciprocalRank)
    /// the arithmetic separates ranks well past this point given a heavy enough
    /// weight, and a weight quoted for such a depth would buy a depth no plan
    /// could ever ask for. It is therefore refused as a range, never reported as
    /// the rule saturating ([`DepthUnreachable`](Self::DepthUnreachable)).
    DepthBeyondPlanRange {
        /// The depth that was asked for.
        depth: u64,
        /// The deepest depth a plan's 32-bit depth field can carry.
        limit: u64,
    },
}

impl std::fmt::Display for FusionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => write!(f, "fusion overflowed the fixed-point range"),
            Self::Protocol(err) => write!(f, "ranked-stream protocol violation: {err}"),
            Self::InvalidK { k } => write!(f, "fusion profile K must be at least 1, got {k}"),
            Self::InvalidRank { rank } => write!(f, "rank must be at least 1, got {rank}"),
            Self::InvalidWidth { max_width } => write!(
                f,
                "class-width tolerance must be at least 1, got {max_width}; a tolerance of zero is not a tolerance, because a class always contains its own rank"
            ),
            Self::EmptyWeights => {
                write!(f, "fusion profile must declare at least one stratum weight")
            }
            Self::NonPositiveWeight { stratum, weight } => {
                write!(f, "stratum {stratum} has non-positive weight {weight:?}")
            }
            Self::NonPositiveCrossingWeight { weight } => write!(
                f,
                "the threshold-crossing derivation was handed the non-positive weight {weight:?}; its search rests on a non-increasing threshold, which a non-positive weight does not give"
            ),
            Self::UnknownStratum { stratum } => {
                write!(f, "fusion profile declares no weight for stratum {stratum}")
            }
            Self::DuplicateStratum { stratum } => {
                write!(f, "two streams are tagged with stratum {stratum}")
            }
            Self::PlanIdMismatch { expected, got } => write!(
                f,
                "fused streams descend from different pinned plans: expected {expected}, got {got:?}"
            ),
            Self::ReadBoundMismatch { planned, requested } => write!(
                f,
                "streams were planned for a bound of {planned} fused rows and the fusion was run at {requested}; a depth derived for one bound does not serve another"
            ),
            Self::MalformedProfile(err) => write!(f, "malformed fusion profile: {err}"),
            Self::MaxContributionsExceeded { item, count, max } => write!(
                f,
                "candidate {item} received {count} contributions across {max} strata; a candidate may surface at most once per stratum"
            ),
            Self::DepthUnreachable {
                depth,
                saturates_at,
            } => write!(
                f,
                "no weight separates ranks to depth {depth} under this decay rule; it separates to depth {saturates_at} and no further"
            ),
            Self::DepthBeyondPlanRange { depth, limit } => write!(
                f,
                "depth {depth} is deeper than a plan can record; a plan carries a per-stratum depth as a 32-bit rank, so {limit} is the deepest expressible depth"
            ),
        }
    }
}

impl std::error::Error for FusionError {}

impl From<crate::ranked_stream::ProtocolError> for FusionError {
    /// Wrap a producer's protocol violation without growing `FusionError` to the
    /// producer error's own size.
    fn from(error: crate::ranked_stream::ProtocolError) -> Self {
        Self::Protocol(Box::new(error))
    }
}
