// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What a host declares about the index behind a relation it registers, and the
//! wrapper that makes every invocation of that relation say it.
//!
//! # Why this is the one part of a relation the rows cannot carry
//!
//! Everything else about a relation registered from Python is visible in the
//! values crossing the boundary — the arity, the rows, the traversal envelope,
//! the indexed predicate. These two facts are not: a host that read its table out
//! of a search index mid-rebuild hands over exactly the same tuple of rows as a
//! host whose index was whole, and no query text, dataset snapshot or registry
//! fingerprint differs between the two runs. Only the host knows, so only the
//! host can say, and this is where it says it.
//!
//! # Neither field is ever minted here
//!
//! Both default to the kernel's own silence ([`IndexGeneration::Undeclared`] /
//! [`ServiceLevel::Undeclared`]), which a reader must not upgrade to "the index
//! was current" or "the index was whole" — there is no seam at which either could
//! be certified, so this binding invents neither. A relation declared without a
//! trailing attestation position therefore behaves exactly as it did before this
//! surface existed.
//!
//! # One mechanism, both registration surfaces
//!
//! Two Python surfaces let a host register a relation — the SPARQL
//! property-function lane (`py_store::query`) and the ranked-retrieval lane
//! (`py_retrieval`) — and the fact being carried is the same fact
//! about the same kind of index on both. A second copy of this wrapper would be a
//! second opinion about what silence means, so there is one, and each lane spells
//! only its own declaration syntax and its own subject in the diagnostics.

use std::sync::Arc;

use purrdf_sparql_eval::{
    BindingPattern, EvalError, IndexGeneration, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    ServiceLevel, Volatility,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

/// What the host declares about the index one relation's rows came from: which
/// version of it answered, and whether it was **not** whole.
///
/// # An incompleteness is witnessed or fatal
///
/// Declaring one does NOT make a query answer short and quiet. An entry point
/// whose outcome carries a witness — the governed lanes, and the retrieval
/// ladder's answer — reports the reason beside its rows; one whose outcome has
/// nowhere to put it (`query`, `update`) refuses the query with the kernel's own
/// `native-sparql-relation-incomplete` diagnostic. That is a property of the
/// entry point's return type, not a caller's choice.
#[derive(Debug, Clone)]
pub(crate) struct Attestation {
    /// The host's own spelling of the index version that produced the rows.
    generation: IndexGeneration,
    /// Whether the host declares that index was not whole, and why, verbatim.
    service: ServiceLevel,
}

impl Attestation {
    /// The attestation of a relation that declared nothing — silence on both
    /// halves, which is what every relation declared without a trailing position
    /// attests.
    pub(crate) const UNDECLARED: Self = Self {
        generation: IndexGeneration::Undeclared,
        service: ServiceLevel::Undeclared,
    };

    /// Whether this attestation says anything at all, i.e. whether wrapping the
    /// relation would change what it attests.
    fn is_silent(&self) -> bool {
        matches!(
            (&self.generation, &self.service),
            (IndexGeneration::Undeclared, ServiceLevel::Undeclared)
        )
    }

    /// Read one declaration's trailing attestation position:
    /// `(generation, incompleteness)`, each a `str` or `None`.
    ///
    /// `subject` names what is being declared, as the calling surface spells it
    /// (`property function <iri>`, `text producer <iri>`), so the two lanes
    /// report a member of the wrong type in their own idiom.
    ///
    /// `Ok(None)` is "this value is not an attestation at all" — not a two-member
    /// sequence — which is the caller's cue to report the whole declaration's
    /// accepted shapes rather than a diagnostic about a position the caller may
    /// never have meant to write.
    ///
    /// # A string is not a two-member sequence here
    ///
    /// Python strings are sequences of their own characters, so a two-character
    /// string would otherwise extract as a perfectly well-formed attestation whose
    /// two halves are its two letters — a misconfiguration that would be accepted
    /// in silence and then reported back on the receipt as though the host had
    /// said it.
    ///
    /// # Errors
    ///
    /// `TypeError` naming the field when the value IS a two-member sequence but a
    /// member is neither a `str` nor `None`.
    pub(crate) fn read(subject: &str, value: &Bound<'_, PyAny>) -> PyResult<Option<Self>> {
        if value.is_instance_of::<PyString>() || value.is_instance_of::<PyBytes>() {
            return Ok(None);
        }
        let Ok(members) = value.extract::<Vec<Bound<'_, PyAny>>>() else {
            return Ok(None);
        };
        let Ok([generation, incompleteness]) = <[Bound<'_, PyAny>; 2]>::try_from(members) else {
            return Ok(None);
        };
        let read = |member: &Bound<'_, PyAny>, field: &str| -> PyResult<Option<String>> {
            member.extract::<Option<String>>().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{subject}: an attestation's `{field}` must be a str or None"
                ))
            })
        };
        Ok(Some(Self {
            // Recorded verbatim on both halves. The kernel never parses either
            // string, and neither does this boundary: a generation is whatever the
            // host's index calls its versions, and a reason is whatever an
            // operator needs to read.
            generation: read(&generation, "generation")?
                .map_or(IndexGeneration::Undeclared, IndexGeneration::declared),
            service: read(&incompleteness, "incompleteness")?
                .map_or(ServiceLevel::Undeclared, |reason| {
                    ServiceLevel::Incomplete { reason }
                }),
        }))
    }

    /// `relation`, answering exactly as it does and attesting what the host
    /// declared about the index behind it.
    ///
    /// Wrapped only when the host actually declared something, so a relation that
    /// attested nothing is byte-for-byte the registration it was before the
    /// attestation position existed rather than a silent relation behind a
    /// wrapper.
    pub(crate) fn wrap(self, relation: Arc<dyn PropertyFunction>) -> Arc<dyn PropertyFunction> {
        if self.is_silent() {
            relation
        } else {
            Arc::new(AttestedRelation {
                inner: relation,
                attestation: self,
            })
        }
    }
}

/// A relation that answers exactly as `inner` does and attests what the host
/// declared about the index behind it.
///
/// A wrapper rather than a field on each relation kind: the attestation is a
/// property of the INDEX the rows were read from, which is the same fact whichever
/// spelling carried the rows, and the kernel reads it through the cursor rather
/// than through the relation. Every declaration the planner reads — volatility,
/// arity, modes, per-invocation cardinality, the admitted access patterns — is
/// delegated verbatim, so wrapping cannot change which plan a query gets or which
/// rows it produces. Only the two cursor attestations differ.
struct AttestedRelation {
    /// The relation the rows actually come from.
    inner: Arc<dyn PropertyFunction>,
    /// What every invocation of it attests.
    attestation: Attestation,
}

impl PropertyFunction for AttestedRelation {
    fn volatility(&self) -> Volatility {
        self.inner.volatility()
    }

    fn arity(&self) -> PfArity {
        self.inner.arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        self.inner.modes()
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.inner.rows_per_invocation(mode)
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(AttestedCursor {
            inner: self.inner.open(args, ceiling)?,
            attestation: self.attestation.clone(),
        }))
    }

    fn admits(&self, invocation: BindingPattern) -> bool {
        // Delegated rather than left to the provided default: if `inner` ever
        // narrows which access patterns it serves, a wrapper that recomputed the
        // default from `modes` would admit calls the relation itself refuses.
        self.inner.admits(invocation)
    }
}

/// [`AttestedRelation`]'s cursor: `inner`'s rows, and on each axis the host's
/// declaration where it made one and `inner`'s own where it did not.
///
/// # Silence on an axis is delegation, not erasure
///
/// The two axes are read independently because a host may hold a fact on one and
/// not the other, and because some relations this binding builds already attest a
/// generation of their own — the shipped ranked text relation declares the content
/// digest of the index it searched. A wrapper that answered `Undeclared` on the
/// axis its host left silent would delete that digest as the price of declaring an
/// incompleteness beside it, which is a fact lost to say a fact. A host that does
/// name a generation REPLACES the relation's, because the kernel pins exactly one
/// per invocation and two distinct ones are the diagnostic for an index that moved
/// under the query.
struct AttestedCursor {
    /// The wrapped relation's own cursor for this invocation.
    inner: Box<dyn PfCursor>,
    /// What the host declared about this invocation, at both instants the
    /// evaluator asks.
    attestation: Attestation,
}

impl PfCursor for AttestedCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        self.inner.next()
    }

    fn take_work(&mut self) -> u64 {
        // Forwarded, not zeroed: the wrapped relation's reported internal work is
        // what its caller's budget is charged for, and swallowing it here would
        // make a governed receipt describe a cheaper execution than the one that
        // ran.
        self.inner.take_work()
    }

    fn generation(&self) -> IndexGeneration {
        match &self.attestation.generation {
            IndexGeneration::Undeclared => self.inner.generation(),
            declared @ IndexGeneration::Declared(_) => declared.clone(),
        }
    }

    fn service_level(&self) -> ServiceLevel {
        // The same declaration at both instants the evaluator asks. A host's
        // attestation is fixed before evaluation began, so it cannot discover a
        // shortfall mid-drain the way a live index-backed relation can — and a
        // relation that can is exactly what the delegated arm is for.
        match &self.attestation.service {
            ServiceLevel::Undeclared => self.inner.service_level(),
            declared @ ServiceLevel::Incomplete { .. } => declared.clone(),
        }
    }
}
