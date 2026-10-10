// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Versioned BM25F profiles and corpus-bound prepared scoring.
//!
//! All products and quotients truncate toward zero at twelve decimal places.
//! Field frequencies are normalized and weighted before one saturation. The
//! single-field profile uses exactly the same operations. See `RANKING.md` for
//! the arithmetic, bound proof, and independent conformance reference.

use purrdf_core::{FastSet, SmallVec, TermValue};
use purrdf_hash::frame::frame_le;
use purrdf_xsd::{bigint::BigInt, exact::Integer, wide::mul_div};

use crate::fixed::{SCALE, Scaled};
use crate::{B, FINGERPRINT_BYTES, Fixed, K1, SCALE_DIGITS, TextError};

/// The arithmetic and query aggregation contract, independent of field choices.
pub const RANKING_PROFILE_ID: &str = "purrdf-bm25f-fixed-v2";
/// Revision of the complete ranking law, including intermediate rounding.
pub const RANKING_PROFILE_VERSION: u32 = 2;
/// Corpus construction used by the in-memory index: documents are
/// `(graph, subject, language)`, partitions are `(graph, language)`, direction
/// is merged, and zero-token documents are excluded. External stores provide
/// their own already-partitioned counts to the pure prepared scorer.
pub const INDEX_CORPUS_PROFILE_ID: &str = "purrdf-text-corpus-graph-language-v1";

/// Inline capacity is an allocation choice, never a field-count limit.
pub(crate) type FieldInputs = SmallVec<[FieldInput; 16]>;

/// A certified inclusive raw-score bound for one prepared query and corpus.
///
/// Constructed from the actual prepared IDFs and the saturation law, including
/// every truncation. It has no global score ceiling or reserved host tie bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScoreBound {
    maximum: Fixed,
    profile: [u8; FINGERPRINT_BYTES],
}

impl ScoreBound {
    /// Every score under this prepared query is in `[0, maximum]`.
    pub const fn maximum(self) -> Fixed {
        self.maximum
    }

    /// Significant bits of the certified maximum's nonnegative raw integer.
    pub const fn bits(self) -> u32 {
        128 - self.maximum.into_raw().unsigned_abs().leading_zeros()
    }

    /// The complete ranking law and caller field choices this bound certifies.
    pub const fn profile_fingerprint(self) -> [u8; FINGERPRINT_BYTES] {
        self.profile
    }

    /// Check the exact interval, rather than admitting every value of its width.
    ///
    /// # Errors
    /// Returns [`TextError::Domain`] for a negative or uncertified score.
    pub fn validate(self, score: Fixed) -> Result<Fixed, TextError> {
        if !(Fixed::ZERO..=self.maximum).contains(&score) {
            return Err(TextError::domain(format!(
                "score {} is outside the prepared interval [0, {}]",
                score.to_decimal_lexical(),
                self.maximum.to_decimal_lexical()
            )));
        }
        Ok(score)
    }
}

/// A pure scorer's value and its original query/corpus certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoundedScore {
    /// Exact twelve-fractional-digit value.
    pub value: Fixed,
    /// Inclusive certified bound, suitable for a caller's key-width decision.
    pub bound: ScoreBound,
}

/// One field's named, immutable scoring parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingField {
    /// Caller-chosen name; identity-bearing, never an ontology IRI.
    name: String,
    /// Weight applied after length normalization.
    weight: Fixed,
    /// Length normalization coefficient in the closed interval `[0, 1]`.
    b: Fixed,
}

impl RankingField {
    /// Validate a nonempty name, nonnegative weight, and `0 <= b <= 1`.
    ///
    /// # Errors
    /// Returns [`TextError::Config`] for invalid parameters.
    pub fn new(name: impl Into<String>, weight: Fixed, b: Fixed) -> Result<Self, TextError> {
        let name = name.into();
        if name.is_empty() || weight < Fixed::ZERO {
            return Err(TextError::config(
                "a field needs a name and a nonnegative weight",
            ));
        }
        if !(Fixed::ZERO..=Fixed::ONE).contains(&b) {
            return Err(TextError::config("a field's b must be in [0, 1]"));
        }
        Ok(Self { name, weight, b })
    }

    /// The field's identity-bearing name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The field's exact weight.
    pub const fn weight(&self) -> Fixed {
        self.weight
    }
    /// The field's length normalization coefficient.
    pub const fn b(&self) -> Fixed {
        self.b
    }
}

/// A validated ranking law, separate from tokenization and stored postings.
///
/// Every selected predicate must have a mapping or use the explicitly declared
/// unclassified field. Field order is identity-bearing. Mapping order is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingProfile {
    /// Fields, in scoring order.
    fields: Vec<RankingField>,
    /// Sorted predicate-to-field assignments.
    mappings: Vec<(TermValue, usize)>,
    /// Explicit destination for otherwise unmapped predicates.
    unclassified: Option<usize>,
    /// Normalize each field over its declared carriers rather than all documents.
    field_populations: bool,
    /// Digest of the complete canonical profile description.
    fingerprint: [u8; FINGERPRINT_BYTES],
}

impl RankingProfile {
    /// Construct a complete profile from explicit fields and predicate routing.
    ///
    /// # Errors
    /// Refuses zero fields, repeated field names or
    /// predicates, non-IRI predicates, and references to absent fields.
    pub fn new(
        fields: Vec<RankingField>,
        mut mappings: Vec<(TermValue, usize)>,
        unclassified: Option<usize>,
    ) -> Result<Self, TextError> {
        if fields.is_empty() {
            return Err(TextError::config(
                "a ranking profile needs at least one field",
            ));
        }
        let mut names = FastSet::default();
        for field in &fields {
            if !names.insert(field.name.as_str()) {
                return Err(TextError::config(format!(
                    "repeated field name {:?}",
                    field.name
                )));
            }
        }
        if unclassified.is_some_and(|field| field >= fields.len()) {
            return Err(TextError::config(
                "the unclassified field is not in this profile",
            ));
        }
        mappings.sort_by(|left, right| left.0.cmp(&right.0));
        for (at, (predicate, field)) in mappings.iter().enumerate() {
            validate_predicate(predicate)?;
            if *field >= fields.len() {
                return Err(TextError::config(
                    "a predicate mapping needs an IRI and an existing field",
                ));
            }
            if at > 0 && mappings[at - 1].0 == *predicate {
                return Err(TextError::config(format!(
                    "repeated mapped predicate {predicate:?}"
                )));
            }
        }
        let mut profile = Self {
            fields,
            mappings,
            unclassified,
            field_populations: false,
            fingerprint: [0; FINGERPRINT_BYTES],
        };
        profile.fingerprint =
            *purrdf_hash::blake3::hash(&profile.canonical_description()).as_bytes();
        Ok(profile)
    }

    /// The explicit single-field law: every selected predicate is unclassified
    /// text with weight one and `b = 0.75`. No vocabulary is supplied or invented.
    pub fn single_field() -> Self {
        Self::new(
            vec![RankingField::new("text", Fixed::ONE, B).expect("fixed valid parameters")],
            Vec::new(),
            Some(0),
        )
        .expect("one valid field with explicit unclassified routing")
    }

    /// Fields in the order consumed by prepared scoring inputs.
    pub fn fields(&self) -> &[RankingField] {
        &self.fields
    }
    /// Predicate assignments in canonical predicate order.
    pub fn mappings(&self) -> &[(TermValue, usize)] {
        &self.mappings
    }
    /// Explicit unclassified field, if declared.
    pub const fn unclassified(&self) -> Option<usize> {
        self.unclassified
    }
    /// Select length normalization over documents carrying each field.
    ///
    /// This changes the ranking fingerprint. Corpus preparation must supply
    /// the populations explicitly through [`PreparedCorpus::with_field_populations`].
    #[must_use]
    pub fn with_field_populations(mut self) -> Self {
        self.field_populations = true;
        self.fingerprint = *purrdf_hash::blake3::hash(&self.canonical_description()).as_bytes();
        self
    }

    /// Whether each field uses its own carrier population for normalization.
    pub const fn uses_field_populations(&self) -> bool {
        self.field_populations
    }
    /// Identity of the complete ranking law and all caller choices.
    pub const fn fingerprint(&self) -> [u8; FINGERPRINT_BYTES] {
        self.fingerprint
    }

    /// Resolve one selected predicate under this profile.
    ///
    /// # Errors
    /// An unmapped predicate without an explicit unclassified class is refused.
    pub fn field_for(&self, predicate: &TermValue) -> Result<usize, TextError> {
        validate_predicate(predicate)?;
        self.mappings
            .binary_search_by(|(iri, _)| iri.cmp(predicate))
            .ok()
            .map(|at| self.mappings[at].1)
            .or(self.unclassified)
            .ok_or_else(|| {
                TextError::config(format!(
                    "selected predicate {predicate:?} has no ranking field"
                ))
            })
    }

    /// Canonical, length-framed bytes used to identify the profile.
    pub fn canonical_description(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut text = |value: &str| frame_le(&mut bytes, value.as_bytes());
        text(RANKING_PROFILE_ID);
        text(INDEX_CORPUS_PROFILE_ID);
        text(if self.field_populations {
            "integer-ln-18-digits-20-terms;truncate-each-operation;relative=length*field_documents/total;distinct-query-terms-sorted;field-sum-then-saturate;exact-integer-intermediates;bound=sum(floor(idf*(k1+1)))"
        } else {
            "integer-ln-18-digits-20-terms;truncate-each-operation;relative=length*N/total;distinct-query-terms-sorted;field-sum-then-saturate;exact-integer-intermediates;bound=sum(floor(idf*(k1+1)))"
        });
        for number in [
            i128::from(RANKING_PROFILE_VERSION),
            i128::from(SCALE_DIGITS),
            K1.into_raw(),
            self.fields.len() as i128,
        ] {
            bytes.extend_from_slice(&number.to_le_bytes());
        }
        for field in &self.fields {
            frame_le(&mut bytes, field.name.as_bytes());
            bytes.extend_from_slice(&field.weight.into_raw().to_le_bytes());
            bytes.extend_from_slice(&field.b.into_raw().to_le_bytes());
        }
        bytes.extend_from_slice(&(self.mappings.len() as u64).to_le_bytes());
        for (predicate, field) in &self.mappings {
            let TermValue::Iri(iri) = predicate else {
                unreachable!("validated IRI mapping")
            };
            frame_le(&mut bytes, iri.as_bytes());
            bytes.extend_from_slice(&(*field as u64).to_le_bytes());
        }
        bytes.extend_from_slice(
            &self
                .unclassified
                .map_or(u64::MAX, |field| field as u64)
                .to_le_bytes(),
        );
        bytes
    }
}

/// Exact document facts for one field. These are validated even when the
/// frequency or weight is zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldInput {
    /// Occurrences of this query term in the field.
    pub term_frequency: u64,
    /// Total analyzed tokens in the field.
    pub length: u64,
}

/// Validated corpus statistics and their ranking profile.
///
/// Totals use `u128`: a u64 population times a u64 field length fits exactly.
/// Exact totals avoid rounding a sparse field's
/// average to zero. No caller-supplied cached IDF can enter this type.
#[derive(Debug)]
pub struct PreparedCorpus<'p> {
    /// Immutable law held for the lifetime of prepared queries.
    profile: &'p RankingProfile,
    /// Corpus population.
    documents: u64,
    /// Field token totals, in profile order.
    totals: Vec<u128>,
    /// Explicit carrier counts in field order; empty under the dense law.
    populations: Vec<u64>,
}

impl<'p> PreparedCorpus<'p> {
    /// Validate population and exact field totals once per corpus.
    ///
    /// # Errors
    /// Refuses a mismatching field count or a field total exceeding the
    /// representable `documents * u64::MAX`, including nonzero empty-corpus totals.
    pub fn new(
        profile: &'p RankingProfile,
        documents: u64,
        totals: &[u128],
    ) -> Result<Self, TextError> {
        if profile.uses_field_populations() {
            return Err(TextError::data(
                "this ranking profile requires explicit field populations",
            ));
        }
        Self::validate(profile, documents, totals, &[])
    }

    /// Validate exact field totals and the number of documents carrying each field.
    ///
    /// The profile must select [`RankingProfile::with_field_populations`]. IDF
    /// remains corpus-wide; only length normalization uses these populations.
    /// A zero-token field may have a nonzero population. A nonzero total requires
    /// a nonzero population, and no population may exceed the corpus size.
    ///
    /// # Errors
    /// Refuses a dense profile, mismatching field counts, oversized populations,
    /// or a total exceeding the representable `field_documents * u64::MAX`.
    pub fn with_field_populations(
        profile: &'p RankingProfile,
        documents: u64,
        totals: &[u128],
        populations: &[u64],
    ) -> Result<Self, TextError> {
        if !profile.uses_field_populations() || populations.len() != profile.fields.len() {
            return Err(TextError::data(
                "field populations require their ranking mode and one count per field",
            ));
        }
        Self::validate(profile, documents, totals, populations)
    }

    /// Shared corpus admission before either constructor publishes statistics.
    fn validate(
        profile: &'p RankingProfile,
        documents: u64,
        totals: &[u128],
        populations: &[u64],
    ) -> Result<Self, TextError> {
        if totals.len() != profile.fields.len() {
            return Err(TextError::data(
                "corpus field count disagrees with the ranking profile",
            ));
        }
        for (at, &total) in totals.iter().enumerate() {
            let population = populations.get(at).copied().unwrap_or(documents);
            if population > documents || total > u128::from(population) * u128::from(u64::MAX) {
                return Err(TextError::data(
                    "a field population or token total exceeds its corpus bound",
                ));
            }
        }
        Ok(Self {
            profile,
            documents,
            totals: totals.to_vec(),
            populations: populations.to_vec(),
        })
    }

    /// Prepare IDFs for explicitly named, distinct query terms in strictly
    /// ascending lexical order. Names are identity keys, already analyzed by
    /// the caller; this arithmetic API does not select an analyzer.
    ///
    /// # Errors
    /// Refuses every document frequency outside the corpus,
    /// even when no document will subsequently be scored.
    pub fn prepare_query(
        &self,
        frequencies: &[(&str, u64)],
    ) -> Result<PreparedQuery<'_, 'p>, TextError> {
        let mut terms = Vec::with_capacity(frequencies.len());
        let total = self.totals.iter().fold(Integer::ZERO, |sum, &total| {
            sum + Integer::from_bigint(BigInt::from_u128(total))
        });
        let mut prior: Option<&str> = None;
        let mut frequency_sum = Integer::ZERO;
        let mut maximum = Fixed::ZERO;
        let saturation_ceiling = K1.checked_add(Fixed::ONE)?;
        for &(term, frequency) in frequencies {
            if term.is_empty() || prior.is_some_and(|prior| prior >= term) {
                return Err(TextError::data(
                    "prepared query terms must be nonempty, distinct and strictly sorted",
                ));
            }
            prior = Some(term);
            frequency_sum = frequency_sum + Integer::from_i128(i128::from(frequency));
            if frequency_sum > total {
                return Err(TextError::data(
                    "distinct query document frequencies exceed all tokens in their corpus",
                ));
            }
            let idf = inverse_document_frequency(self.documents, frequency)?;
            maximum = maximum.checked_add(idf.checked_mul(saturation_ceiling)?)?;
            terms.push((term.to_owned(), frequency, idf));
        }
        Ok(PreparedQuery {
            corpus: self,
            terms,
            bound: ScoreBound {
                maximum,
                profile: self.profile.fingerprint(),
            },
        })
    }
}

/// Prepared, corpus-bound term statistics. IDFs are computed once per query
/// term and corpus, never accepted from a caller or recomputed per document.
#[derive(Debug)]
pub struct PreparedQuery<'c, 'p> {
    /// Corpus and profile that give the cached IDFs their meaning.
    corpus: &'c PreparedCorpus<'p>,
    /// `(term key, document_frequency, IDF)` in verified canonical term order.
    terms: Vec<(String, u64, Fixed)>,
    /// Certified for these exact IDFs, corpus and immutable profile.
    bound: ScoreBound,
}

impl PreparedQuery<'_, '_> {
    /// The actual prepared query's certified maximum and key width.
    pub const fn score_bound(&self) -> ScoreBound {
        self.bound
    }

    /// Validated document frequency and computed IDF for a query term.
    pub fn term_statistics(&self, ordinal: usize) -> Option<(u64, Fixed)> {
        self.terms
            .get(ordinal)
            .map(|(_, frequency, idf)| (*frequency, *idf))
    }

    /// One term's BM25F contribution after validating every field.
    ///
    /// # Errors
    /// Refuses a missing term, field count mismatch, impossible or out-of-bound
    /// document facts, undefined normalization, or an out-of-bound score.
    pub fn contribution(&self, ordinal: usize, fields: &[FieldInput]) -> Result<Fixed, TextError> {
        if self.corpus.documents == 0 {
            return Err(TextError::data(
                "an empty corpus contains no document to score",
            ));
        }
        let (df, idf) = self
            .term_statistics(ordinal)
            .ok_or_else(|| TextError::data("query term ordinal is absent"))?;
        if fields.len() != self.corpus.profile.fields.len() {
            return Err(TextError::data(
                "document field count disagrees with its ranking profile",
            ));
        }
        let mut pseudo = Scaled::ZERO;
        for (at, ((input, field), &total)) in fields
            .iter()
            .zip(&self.corpus.profile.fields)
            .zip(&self.corpus.totals)
            .enumerate()
        {
            let population = self.corpus.populations.get(at).copied();
            let documents = population.unwrap_or(self.corpus.documents);
            let remaining = if population.is_some_and(|count| count < self.corpus.documents)
                && input.length == 0
            {
                documents
            } else {
                documents.saturating_sub(1)
            };
            validate_field(*input, total, documents, remaining, df)?;
            if input.term_frequency == 0 {
                continue;
            }
            // Positive tf implies positive length and total. Form this ratio
            // from exact counts so an average below one raw unit stays usable.
            let count_product = u128::from(input.length) * u128::from(documents);
            let relative = Fixed::from_raw(
                i128::try_from(
                    mul_div(count_product, SCALE as u128, total)
                        .expect("length <= total bounds the ratio by a u64 population at scale"),
                )
                .expect("a u64 population at scale fits i128"),
            );
            let normalization = Fixed::ONE
                .checked_sub(field.b)?
                .checked_add(field.b.checked_mul(relative)?)?;
            if normalization <= Fixed::ZERO {
                return Err(TextError::domain(
                    "field normalization rounds to zero under this profile",
                ));
            }
            let frequency = Scaled::from_fixed(from_count(input.term_frequency));
            pseudo = pseudo.add(
                &frequency
                    .div(&Scaled::from_fixed(normalization))?
                    .mul(&Scaled::from_fixed(field.weight)),
            );
        }
        let saturation = pseudo
            .mul(&Scaled::from_fixed(K1.checked_add(Fixed::ONE)?))
            .div(&pseudo.add(&Scaled::from_fixed(K1)))?
            .into_fixed()?;
        self.bound.validate(idf.checked_mul(saturation)?)
    }

    /// Score one document against all prepared terms, in their canonical order.
    /// An empty query is zero in a nonempty corpus. An empty corpus contains
    /// no document to score.
    ///
    /// # Errors
    /// Refuses a term count mismatch, invalid field input, or a score outside
    /// the exact profile bound.
    pub fn score(&self, terms: &[Vec<FieldInput>]) -> Result<BoundedScore, TextError> {
        if self.corpus.documents == 0 {
            return Err(TextError::data(
                "an empty corpus contains no document to score",
            ));
        }
        if terms.len() != self.terms.len() {
            return Err(TextError::data(
                "document term count disagrees with its prepared query",
            ));
        }
        let mut score = Fixed::ZERO;
        let mut frequencies = SmallVec::<[u64; 16]>::new();
        frequencies.resize(self.corpus.profile.fields.len(), 0);
        for (ordinal, fields) in terms.iter().enumerate() {
            if fields
                .iter()
                .map(|field| field.length)
                .ne(terms[0].iter().map(|field| field.length))
            {
                return Err(TextError::data(
                    "one document has inconsistent field lengths across query terms",
                ));
            }
            let contribution = self.contribution(ordinal, fields)?;
            for (sum, field) in frequencies.iter_mut().zip(fields) {
                *sum = sum.checked_add(field.term_frequency).ok_or_else(|| {
                    TextError::data(
                        "distinct query term frequencies exceed the document field length",
                    )
                })?;
                if *sum > field.length {
                    return Err(TextError::data(
                        "distinct query term frequencies exceed the document field length",
                    ));
                }
            }
            score = score.checked_add(contribution)?;
        }
        Ok(BoundedScore {
            value: self.bound.validate(score)?,
            bound: self.bound,
        })
    }
}

/// Validate raw facts before considering zero contribution shortcuts.
fn validate_field(
    input: FieldInput,
    total: u128,
    documents: u64,
    remaining: u64,
    df: u64,
) -> Result<(), TextError> {
    if input.term_frequency > input.length || u128::from(input.length) > total {
        return Err(TextError::data(
            "field frequency, length, and corpus total are inconsistent",
        ));
    }
    if total - u128::from(input.length) > u128::from(remaining) * u128::from(u64::MAX) {
        return Err(TextError::data(
            "the remaining corpus cannot hold the declared field total",
        ));
    }
    if (documents == 0 && input.length != 0) || (df == 0 && input.term_frequency != 0) {
        return Err(TextError::data(
            "a nonempty document or term cannot belong to an empty corpus or posting list",
        ));
    }
    Ok(())
}

/// Shifted IDF, checked before any zero-contribution shortcuts.
/// At large corpus sizes a positive real value may round to exact zero.
fn inverse_document_frequency(documents: u64, frequency: u64) -> Result<Fixed, TextError> {
    if frequency > documents {
        return Err(TextError::data(
            "document frequency exceeds its corpus population",
        ));
    }
    let half = Fixed::from_raw(500_000_000_000);
    let numerator = from_count(documents - frequency).checked_add(half)?;
    let denominator = from_count(frequency).checked_add(half)?;
    Fixed::ONE
        .checked_add(numerator.checked_div(denominator)?)?
        .ln()
}

/// Every u64 corpus count fits exactly in i128 at the public scale.
fn from_count(value: u64) -> Fixed {
    Fixed::from_raw(i128::from(value) * SCALE)
}

/// Require an absolute, lexically valid RDF predicate IRI.
fn validate_predicate(predicate: &TermValue) -> Result<(), TextError> {
    let TermValue::Iri(iri) = predicate else {
        return Err(TextError::config("a ranking predicate must be an IRI"));
    };
    let parsed = purrdf_core::parse_iri(iri).map_err(|error| {
        TextError::config(format!("invalid ranking predicate {iri:?}: {error}"))
    })?;
    if !parsed.has_scheme() {
        return Err(TextError::config(format!(
            "ranking predicate {iri:?} must be absolute"
        )));
    }
    Ok(())
}
