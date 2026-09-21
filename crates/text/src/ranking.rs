// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Versioned BM25F profiles and corpus-bound prepared scoring.
//!
//! All products and quotients truncate toward zero at twelve decimal places.
//! Field frequencies are normalized and weighted before one saturation. The
//! single-field profile uses exactly the same operations. See `RANKING.md` for
//! the arithmetic, bound proof, and independent conformance reference.

use purrdf_core::TermValue;

use crate::{B, FINGERPRINT_BYTES, Fixed, K1, SCALE_DIGITS, TextError};

/// The arithmetic and query aggregation contract, independent of field choices.
pub const RANKING_PROFILE_ID: &str = "purrdf-bm25f-fixed-v1";
/// Revision of the complete ranking law, including intermediate rounding.
pub const RANKING_PROFILE_VERSION: u32 = 1;
/// Corpus construction used by the in-memory index: documents are
/// `(graph, subject, language)`, partitions are `(graph, language)`, direction
/// is merged, and zero-token documents are excluded. External stores provide
/// their own already-partitioned counts to the pure prepared scorer.
pub const INDEX_CORPUS_PROFILE_ID: &str = "purrdf-text-corpus-graph-language-v1";

/// Maximum number of fields in a ranking profile.
pub const MAX_FIELDS: usize = 16;
/// Maximum number of distinct analyzed query terms.
pub const QUERY_TERMS_MAX: usize = 1024;
/// Maximum corpus size accepted by the pure scorer.
pub const DOCUMENTS_MAX: u64 = 1 << 40;
/// Maximum length of any field in a document.
pub const FIELD_LENGTH_MAX: u64 = 1 << 24;
/// Maximum frequency of one term in one field.
pub const TERM_FREQUENCY_MAX: u64 = 1 << 24;
/// Maximum field weight; keeps every intermediate representable.
pub const FIELD_WEIGHT_MAX: Fixed = Fixed::from_raw((1_i128 << 24) * 1_000_000_000_000);
/// Inclusive maximum score, in the public fixed-point representation.
pub const SCORE_MAX: Fixed = Fixed::from_raw(65_536 * 1_000_000_000_000);
/// Bits needed for every admitted nonnegative raw score, derived from the bound.
pub const SCORE_BITS: u32 = 128 - SCORE_MAX.into_raw().unsigned_abs().leading_zeros();

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
    /// Validate a nonempty name, nonnegative bounded weight, and `0 <= b <= 1`.
    ///
    /// # Errors
    /// Returns [`TextError::Config`] for invalid parameters.
    pub fn new(name: impl Into<String>, weight: Fixed, b: Fixed) -> Result<Self, TextError> {
        let name = name.into();
        if name.is_empty() || weight < Fixed::ZERO || weight > FIELD_WEIGHT_MAX {
            return Err(TextError::config(
                "a field needs a name and a weight in [0, 2^24]",
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
    /// Digest of the complete canonical profile description.
    fingerprint: [u8; FINGERPRINT_BYTES],
}

impl RankingProfile {
    /// Construct a complete profile from explicit fields and predicate routing.
    ///
    /// # Errors
    /// Refuses zero or more than sixteen fields, repeated field names or
    /// predicates, non-IRI predicates, and references to absent fields.
    pub fn new(
        fields: Vec<RankingField>,
        mut mappings: Vec<(TermValue, usize)>,
        unclassified: Option<usize>,
    ) -> Result<Self, TextError> {
        if fields.is_empty() || fields.len() > MAX_FIELDS {
            return Err(TextError::config(
                "a ranking profile needs between 1 and 16 fields",
            ));
        }
        for (at, field) in fields.iter().enumerate() {
            if fields[..at].iter().any(|prior| prior.name == field.name) {
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
            fingerprint: [0; FINGERPRINT_BYTES],
        };
        profile.fingerprint = *blake3::hash(&profile.canonical_description()).as_bytes();
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

    /// Refuse scores outside the exact inclusive bound, including negative
    /// scores and values that fit [`SCORE_BITS`] but exceed [`SCORE_MAX`].
    ///
    /// # Errors
    /// Returns [`TextError::Domain`] for a score outside the profile.
    pub fn validate_score(&self, score: Fixed) -> Result<Fixed, TextError> {
        if !(Fixed::ZERO..=SCORE_MAX).contains(&score) {
            return Err(TextError::domain(format!(
                "score {} is outside [0, {}]",
                score.to_decimal_lexical(),
                SCORE_MAX.to_decimal_lexical()
            )));
        }
        Ok(score)
    }

    /// Canonical, length-framed bytes used to identify the profile.
    pub fn canonical_description(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut text = |value: &str| {
            bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
            bytes.extend_from_slice(value.as_bytes());
        };
        text(RANKING_PROFILE_ID);
        text(INDEX_CORPUS_PROFILE_ID);
        text(
            "integer-ln-18-digits-20-terms;truncate-each-operation;relative=length*N/total;distinct-query-terms-sorted;field-sum-then-saturate",
        );
        for number in [
            i128::from(RANKING_PROFILE_VERSION),
            i128::from(SCALE_DIGITS),
            K1.into_raw(),
            MAX_FIELDS as i128,
            QUERY_TERMS_MAX as i128,
            i128::from(DOCUMENTS_MAX),
            i128::from(FIELD_LENGTH_MAX),
            i128::from(TERM_FREQUENCY_MAX),
            FIELD_WEIGHT_MAX.into_raw(),
            SCORE_MAX.into_raw(),
            i128::from(SCORE_BITS),
            self.fields.len() as i128,
        ] {
            bytes.extend_from_slice(&number.to_le_bytes());
        }
        for field in &self.fields {
            bytes.extend_from_slice(&(field.name.len() as u64).to_le_bytes());
            bytes.extend_from_slice(field.name.as_bytes());
            bytes.extend_from_slice(&field.weight.into_raw().to_le_bytes());
            bytes.extend_from_slice(&field.b.into_raw().to_le_bytes());
        }
        bytes.extend_from_slice(&(self.mappings.len() as u64).to_le_bytes());
        for (predicate, field) in &self.mappings {
            let TermValue::Iri(iri) = predicate else {
                unreachable!("validated IRI mapping")
            };
            bytes.extend_from_slice(&(iri.len() as u64).to_le_bytes());
            bytes.extend_from_slice(iri.as_bytes());
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
/// Totals use `u128`: the declared corpus and length bounds permit exactly
/// `2^64` tokens in a field. Exact totals avoid rounding a sparse field's
/// average to zero. No caller-supplied cached IDF can enter this type.
#[derive(Debug)]
pub struct PreparedCorpus<'p> {
    /// Immutable law held for the lifetime of prepared queries.
    profile: &'p RankingProfile,
    /// Corpus population.
    documents: u64,
    /// Field token totals, in profile order.
    totals: Vec<u128>,
}

impl<'p> PreparedCorpus<'p> {
    /// Validate population and exact field totals once per corpus.
    ///
    /// # Errors
    /// Refuses an oversized corpus, mismatching field count, or a field total
    /// exceeding `documents * FIELD_LENGTH_MAX`, including nonzero empty-corpus totals.
    pub fn new(
        profile: &'p RankingProfile,
        documents: u64,
        totals: &[u128],
    ) -> Result<Self, TextError> {
        if documents > DOCUMENTS_MAX || totals.len() != profile.fields.len() {
            return Err(TextError::data(
                "corpus population or field count exceeds the ranking profile",
            ));
        }
        let maximum = u128::from(documents) * u128::from(FIELD_LENGTH_MAX);
        if totals.iter().any(|&total| total > maximum) {
            return Err(TextError::data(
                "a corpus field total exceeds documents times the field length bound",
            ));
        }
        Ok(Self {
            profile,
            documents,
            totals: totals.to_vec(),
        })
    }

    /// Prepare IDFs for explicitly named, distinct query terms in strictly
    /// ascending lexical order. Names are identity keys, already analyzed by
    /// the caller; this arithmetic API does not select an analyzer.
    ///
    /// # Errors
    /// Refuses too many terms and every document frequency outside the corpus,
    /// even when no document will subsequently be scored.
    pub fn prepare_query(
        &self,
        frequencies: &[(&str, u64)],
    ) -> Result<PreparedQuery<'_, 'p>, TextError> {
        if frequencies.len() > QUERY_TERMS_MAX {
            return Err(TextError::data(
                "query exceeds 1024 distinct analyzed terms",
            ));
        }
        let mut terms = Vec::with_capacity(frequencies.len());
        let total: u128 = self.totals.iter().sum();
        let mut prior: Option<&str> = None;
        let mut frequency_sum = 0_u128;
        for &(term, frequency) in frequencies {
            if term.is_empty() || prior.is_some_and(|prior| prior >= term) {
                return Err(TextError::data(
                    "prepared query terms must be nonempty, distinct and strictly sorted",
                ));
            }
            prior = Some(term);
            frequency_sum += u128::from(frequency);
            if frequency_sum > total {
                return Err(TextError::data(
                    "distinct query document frequencies exceed all tokens in their corpus",
                ));
            }
            terms.push((
                term.to_owned(),
                frequency,
                inverse_document_frequency(self.documents, frequency)?,
            ));
        }
        Ok(PreparedQuery {
            corpus: self,
            terms,
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
}

impl PreparedQuery<'_, '_> {
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
        let mut pseudo = Fixed::ZERO;
        for ((input, field), &total) in fields
            .iter()
            .zip(&self.corpus.profile.fields)
            .zip(&self.corpus.totals)
        {
            validate_field(*input, total, self.corpus.documents, df)?;
            if input.term_frequency == 0 {
                continue;
            }
            // Positive tf implies positive length and total. Form this ratio
            // from exact counts so an average below one raw unit stays usable.
            let numerator =
                u128::from(input.length) * u128::from(self.corpus.documents) * 1_000_000_000_000;
            let relative = Fixed::from_raw(
                i128::try_from(numerator / total).expect("bounded counts fit i128"),
            );
            let normalization = Fixed::ONE
                .checked_sub(field.b)?
                .checked_add(field.b.checked_mul(relative)?)?;
            if normalization <= Fixed::ZERO {
                return Err(TextError::domain(
                    "field normalization rounds to zero under this profile",
                ));
            }
            let frequency = from_count(input.term_frequency)?;
            pseudo = pseudo.checked_add(
                frequency
                    .checked_div(normalization)?
                    .checked_mul(field.weight)?,
            )?;
        }
        let saturation = pseudo
            .checked_mul(K1.checked_add(Fixed::ONE)?)?
            .checked_div(pseudo.checked_add(K1)?)?;
        self.corpus
            .profile
            .validate_score(idf.checked_mul(saturation)?)
    }

    /// Score one document against all prepared terms, in their canonical order.
    /// An empty query is zero in a nonempty corpus. An empty corpus contains
    /// no document to score.
    ///
    /// # Errors
    /// Refuses a term count mismatch, invalid field input, or a score outside
    /// the exact profile bound.
    pub fn score(&self, terms: &[Vec<FieldInput>]) -> Result<Fixed, TextError> {
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
        let mut frequencies = [0_u64; MAX_FIELDS];
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
                *sum += field.term_frequency;
                if *sum > field.length {
                    return Err(TextError::data(
                        "distinct query term frequencies exceed the document field length",
                    ));
                }
            }
            score = score.checked_add(contribution)?;
        }
        self.corpus.profile.validate_score(score)
    }
}

/// Validate raw facts before considering zero contribution shortcuts.
fn validate_field(
    input: FieldInput,
    total: u128,
    documents: u64,
    df: u64,
) -> Result<(), TextError> {
    if input.length > FIELD_LENGTH_MAX || input.term_frequency > TERM_FREQUENCY_MAX {
        return Err(TextError::data(
            "document field length or term frequency exceeds 2^24",
        ));
    }
    if input.term_frequency > input.length || u128::from(input.length) > total {
        return Err(TextError::data(
            "field frequency, length, and corpus total are inconsistent",
        ));
    }
    if documents > 0
        && total - u128::from(input.length)
            > u128::from(documents - 1) * u128::from(FIELD_LENGTH_MAX)
    {
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
    let numerator = from_count(documents - frequency)?.checked_add(half)?;
    let denominator = from_count(frequency)?.checked_add(half)?;
    Fixed::ONE
        .checked_add(numerator.checked_div(denominator)?)?
        .ln()
}

/// A bounded corpus count, exactly at the public scale.
fn from_count(value: u64) -> Result<Fixed, TextError> {
    Fixed::from_integer(i64::try_from(value).map_err(|_| TextError::overflow("count exceeds i64"))?)
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
