// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Solution-term identity: the [`SolutionTerm`] bound-value representation and the
//! per-query [`ScratchInterner`] for computed terms.
//!
//! ## The unification rule (the heart of the evaluator's hot path)
//!
//! A variable binding is either a term that exists in the queried dataset
//! ([`SolutionTerm::Existing`], a dataset-local [`TermId`]) or a term *minted*
//! during evaluation ([`SolutionTerm::Computed`], a [`ScratchId`] into the
//! per-query scratch table) — e.g. the result of `CONCAT`, arithmetic, or a
//! `VALUES`/template constant absent from the data.
//!
//! [`ScratchInterner::intern`] **first probes the dataset**
//! (`term_id_by_value`): if the value already exists, the binding is
//! promoted to [`SolutionTerm::Existing`] and never becomes `Computed`. The
//! consequence is load-bearing:
//!
//! - `Existing == Existing` is a raw [`TermId`] integer compare (the BGP/join hot
//!   path — zero string resolution).
//! - `Existing` vs `Computed` is **always unequal by construction**, because any
//!   value present in the dataset would have been promoted at mint time. No
//!   structural fallback is ever needed at join time.
//! - `Computed == Computed` is a [`ScratchId`] compare (the scratch interns by
//!   value, so equal values share an id).
//!
//! So [`SolutionTerm`] stays `Copy + Eq + Hash` and every join-key comparison is a
//! single integer compare. The one lookup cost (`term_id_by_value`) is paid only
//! when a term is *minted* (BIND/VALUES/aggregate output), never in BGP matching.
//!
//! ## The admission rule (why there are two doors)
//!
//! Minting is also the evaluator's only ingress for a whole `TermValue` built
//! OUTSIDE it — by a custom function, a service resolver, a property function or
//! a custom aggregate — so this is where a language tag that never met the IR
//! kernel would otherwise reach a results writer.
//!
//! [`ScratchInterner::intern_checked`] is the door for such a value: it asks the
//! grammar and returns [`None`] rather than minting a term no writer can spell.
//! [`ScratchInterner::intern`] is the plain door and asks nothing — it admits the
//! value as given, the way [`purrdf_core::TermValue::lang_literal`] and
//! `RdfLiteral::language_tagged` build one. Both are public; see their docs for
//! which is which and for the SPARQL 1.1 §17.2 reading of the [`None`].

use purrdf_core::{DatasetView, TermId, TermValue, ViewTermId};

use std::hash::Hash;

use hashbrown::HashTable;

/// The deterministic per-term charge for an id/index slot or reserved identity.
const TERM_OVERHEAD: u64 = 32;

/// An id into a [`ScratchInterner`]'s per-query table of computed terms.
///
/// Local to one query evaluation (like [`TermId`] is local to one dataset); never
/// serialized or compared across queries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ScratchId(u32);

impl ScratchId {
    #[inline]
    fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("scratch table cannot exceed u32::MAX entries"))
    }

    /// The raw table index behind this id. `pub(crate)` for
    /// [`crate::parallel::portable_row`], which must compare a `Computed` id
    /// against the parent's fork-time `computed_count()` to decide whether it
    /// was already valid in the parent's id space or freshly minted by the
    /// child after the fork.
    #[inline]
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    /// The raw `u32` table index, for encoding a `Computed` id into a
    /// [`ViewTermId::JoinKeyAtom`] join key (see [`SolutionTerm::join_key`]).
    #[inline]
    pub(crate) fn raw(self) -> u32 {
        self.0
    }
}

/// One bound value in a solution row.
///
/// See the [module docs](self) for the `Existing`/`Computed` unification rule that
/// keeps this `Copy` and join-comparable by a single integer compare.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SolutionTerm<I = TermId> {
    /// A term that exists in the queried dataset (the view's local id `I`).
    Existing(I),
    /// A term minted during evaluation, held in the [`ScratchInterner`].
    Computed(ScratchId),
}

impl<I: ViewTermId> SolutionTerm<I> {
    /// A total, collision-free encoding used as a single-column hash-join key, so a
    /// one-variable join key is a `Copy` [`ViewTermId::JoinKeyAtom`] with **no per-row
    /// `Vec` allocation**.
    ///
    /// `Existing` ids encode into the id space and `Computed` ids into a disjoint
    /// space (see [`ViewTermId`]'s join-key contract), so two terms encode to the same
    /// atom **iff** they are equal — the same invariant the `Existing`/`Computed`
    /// unification rule already guarantees for join correctness (a value present in the
    /// dataset is never also a `Computed` id). A collision here would be a *wrong join
    /// result*, not a slowdown. For `I = TermId` the atom is the historical packed
    /// `u64` (`Existing` in `[0, 2^32)`, `Computed` in `[2^32, 2^33)`), so the
    /// production hash-join is byte-identical.
    #[inline]
    pub(crate) fn join_key(self) -> I::JoinKeyAtom {
        match self {
            Self::Existing(id) => id.encode(),
            Self::Computed(sid) => I::encode_computed(sid.raw()),
        }
    }
}

/// The `I = TermId` monomorphization must keep its historical layout: `TermId` is a
/// `NonZeroU32`, so the two-variant enum has a spare discriminant value and
/// `Option<SolutionTerm<TermId>>` is niche-packed to the SAME size as
/// `SolutionTerm<TermId>` (no extra word). Genericizing over `I` must not grow the
/// production row cell.
const _: () = assert!(
    size_of::<Option<SolutionTerm<TermId>>>() == size_of::<SolutionTerm<TermId>>(),
    "Option<SolutionTerm<TermId>> must stay niche-packed to SolutionTerm<TermId>'s size"
);

/// Default-scope blank identities share immutable spellings and a native table.
/// Forking shares the original table; the first mutation admits a real copy.
#[derive(Debug, Default, Clone)]
struct BlankLabels {
    owner: Option<
        crate::workspace::SharedWorkspace<
            crate::AdmittedMap<purrdf_lex::allocation::SharedText, ()>,
        >,
    >,
}
impl BlankLabels {
    fn contains(&self, label: &str) -> bool {
        self.owner
            .as_ref()
            .is_some_and(|owner| owner.get(label).is_some())
    }
    fn iter(&self) -> impl Iterator<Item = &purrdf_lex::allocation::SharedText> {
        self.owner
            .iter()
            .flat_map(|owner| owner.iter().map(|(label, ())| label))
    }
    fn clear(&mut self) {
        if let Some(owner) = &mut self.owner
            && let Some(table) = owner.unique_mut()
        {
            table.clear();
            return;
        }
        self.owner = None;
    }
    fn capacity_bytes(&self) -> usize {
        self.owner.as_ref().map_or(0, |owner| {
            purrdf_core::hash::hash_table_allocation_bound::<(
                purrdf_lex::allocation::SharedText, (),
            )>(owner.capacity())
            .unwrap_or(usize::MAX)
            .saturating_add(crate::workspace::SharedWorkspace::<crate::AdmittedMap<
                purrdf_lex::allocation::SharedText, (),
            >>::control_bytes())
        })
    }
    fn insert_admitted(
        &mut self,
        label: &str,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<bool, crate::EvalError> {
        if self.contains(label) {
            return Ok(false);
        }
        let label = workspace.authored_text(&format_args!("{label}"))?;
        let unique = self
            .owner
            .as_mut()
            .and_then(crate::workspace::SharedWorkspace::unique_mut)
            .is_some();
        if !unique {
            let mut replacement = crate::AdmittedMap::default();
            if let Some(owner) = &self.owner {
                for (label, ()) in owner.iter() {
                    replacement.insert_admitted(label.clone(), (), workspace)?;
                }
            }
            self.owner = Some(crate::workspace::SharedWorkspace::new_admitted(
                replacement,
                workspace,
            )?);
        }
        self.owner
            .as_mut()
            .expect("a mutation has a native owner")
            .unique_mut()
            .expect("the replacement is uniquely owned")
            .insert_admitted(label, (), workspace)?;
        Ok(true)
    }
}
/// Identity and original-payload policies for the one store-once body.
struct InternValueHooks<H, Q, R, B> {
    hash_value: H,
    equal: Q,
    storage_error: R,
    track_value: B,
}

/// A per-query interner for terms computed during evaluation.
///
/// Interns by [`TermValue`] (dataset-independent), de-duplicating equal computed
/// values to one [`ScratchId`]. Stateless with respect to the dataset: the dataset
/// is passed to each operation so the interner does not hold a borrow that would
/// conflict with the evaluator's other dataset access.
#[derive(Debug, Default)]
pub struct ScratchInterner {
    /// The arena this one extends, frozen: its values are this arena's first
    /// [`Self::shared_len`] ids. A forked loop's worker reads the evaluation's arena
    /// through it rather than through a copy (see [`Self::over`]). `None` for an arena
    /// that holds every value itself.
    shared: Option<std::sync::Arc<Self>>,
    /// How many ids [`Self::shared`] holds.
    shared_len: usize,
    /// `ScratchId` index − [`Self::shared_len`] → the computed value.
    values: Vec<TermValue>,
    /// Store-once value index: cached hashes avoid rehashing nested terms during growth.
    index: HashTable<(ScratchId, u64)>,
    /// Default-scope identities already visible to a fresh blank allocator.
    /// Activated lazily, so ordinary computed terms keep their existing cost.
    blank_labels: BlankLabels,
    track_blank_labels: bool,
    /// The running total of [`value_bytes`] over computed values, plus the
    /// retained default-scope identity labels needed by fresh allocation.
    /// [`Self::minted_bytes`] reports this deterministic scratch-arena charge.
    minted_bytes: u64,
    /// This arena's already-accounted growth, independent of other contexts
    /// sharing the execution's governor. Atomic only to retain `Send + Sync`.
    charged_bytes: std::sync::atomic::AtomicU64,
    /// Values a forked loop's workers minted that the loop's ordered commit counted
    /// into [`Self::minted_bytes`] without storing them as terms
    /// ([`Self::count_worker_mint`]): what the loop run in order would hold here. A
    /// later intern of an equal value stores it without counting it again. `None` once
    /// stored. Not carried into a copy: a worker that mints one afresh is reconciled by
    /// its loop's commit.
    ghosts: Vec<Option<TermValue>>,
    /// [`Self::ghosts`]' value index.
    ghost_index: HashTable<usize>,
    // Capacity owners follow every arena allocation they cover in drop order.
    values_admission: Option<crate::WorkspaceAllocation>,
    index_admission: Option<crate::WorkspaceAllocation>,
}

impl Clone for ScratchInterner {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            shared_len: self.shared_len,
            values: self.values.clone(),
            index: self.index.clone(),
            blank_labels: self.blank_labels.clone(),
            track_blank_labels: self.track_blank_labels,
            minted_bytes: self.minted_bytes,
            // An independently evaluated copy owns its retention and charges
            // it at its own checkpoints. Transient pure row workers retain
            // their existing law: only outputs re-interned in the parent are
            // checkpointed, avoiding a thread-geometry-dependent receipt.
            charged_bytes: std::sync::atomic::AtomicU64::new(0),
            ghosts: Vec::new(),
            ghost_index: HashTable::new(),
            values_admission: None,
            index_admission: None,
        }
    }
}

/// A deterministic byte size for one computed value: the payload it owns, plus a fixed
/// per-term constant for the id, the discriminant, and the table slot.
///
/// This is a *deterministic proxy*, not a true heap measurement, and deliberately so.
/// True heap bytes depend on allocator behaviour and on the interner's growth history,
/// so a ceiling compared against them would trip at a different point on a different
/// allocator — which is not a governor. This function is a pure function of the value,
/// so the same query over the same data mints the same number of bytes on every target.
///
/// `pub(crate)` (not private): [`crate::modifier`]'s aggregate phase-1 buffer
/// reuses this exact proxy to charge its own retained [`TermValue`] clones
/// against [`purrdf_core::ResourceDimension::ScratchBytes`] — those clones
/// never pass through [`ScratchInterner::intern`] (they come back out through
/// [`ScratchInterner::value_of`], which mints nothing), so `minted_bytes`
/// alone would never see them; see `crate::modifier::eval_aggregate`'s doc
/// comment for why that retained buffer needs its own charge.
pub(crate) fn value_bytes(value: &TermValue) -> u64 {
    value_bytes_with_storage(value).expect("resident scratch proxy traversal allocation")
}

fn value_bytes_with_storage(value: &TermValue) -> Result<u64, std::collections::TryReserveError> {
    // Every term of the value — the triple terms and their components, all the way
    // down — contributes its own payload and one `TERM_OVERHEAD`, summed with
    // saturating addition. The sum is walked over a work list rather than the call
    // stack, so a value of any nesting costs no more machine stack; saturating addition
    // of unsigned terms is associative, so the total is the same in any visit order.
    let mut total: u64 = 0;
    let mut pending: purrdf_lex::walk::WorkList<&TermValue, 16> =
        purrdf_lex::walk::WorkList::with(value);
    while let Some(term) = pending.pop() {
        let payload = match term {
            TermValue::Iri(iri) => iri.len() as u64,
            TermValue::Blank { label, .. } => label.len() as u64,
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                ..
            } => {
                (lexical_form.len() + datatype.len() + language.as_ref().map_or(0, String::len))
                    as u64
            }
            TermValue::Triple { s, p, o } => {
                pending.try_push(&**o)?;
                pending.try_push(&**p)?;
                pending.try_push(&**s)?;
                0
            }
        };
        total = total.saturating_add(payload).saturating_add(TERM_OVERHEAD);
    }
    Ok(total)
}

/// Whether `value` is a blank node the QUERY wrote inside a composite literal —
/// the one value [`ScratchInterner::intern`] must not resolve against the dataset.
///
/// A discriminant test on the value the interner is already holding, so it costs a
/// branch and runs strictly cheaper than the term-index probe it guards.
#[inline]
fn is_query_scoped_blank(value: &TermValue) -> bool {
    matches!(value, TermValue::Blank { scope, .. } if *scope == crate::convert::QUERY_BLANK_SCOPE)
}

/// The profile every language tag in this workspace is judged on — the parser's,
/// the codecs', the IR kernel's, and (since `eval_str_lang`) `STRLANG`'s.
///
/// Naming the same profile here is the whole point: a value that came out of the
/// dataset was admitted by `RdfLiteral::validate_components` on THIS profile, so
/// re-judging it cannot change the verdict, and [`ScratchInterner::intern_checked`]
/// therefore costs the internal callers nothing but a scan. Only a value minted
/// outside the kernel — which is to say, by an extension — can fail it.
///
/// `crate::substitute` names this same constant at the pre-binding ingress, so a
/// caller-supplied focus node is judged on the profile the query parser would
/// have judged the same term written in the query text on.
pub(crate) const LANGTAG_PROFILE: purrdf_iri::langtag::Profile =
    purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded;

/// Whether every language tag `value` carries is one the RDF concrete syntaxes
/// would have lexed, through every triple-term component.
///
/// A value that fails this is not a term any writer in the workspace can
/// serialize: the results writers spell a tag as `"x"@<tag>` (TSV), as
/// `"xml:lang": "<tag>"` (JSON) and as `xml:lang="<tag>"` (XML), and every one of
/// those is bytes no reader takes back. `Iri` and `Blank` carry no tag at all, so
/// they are a discriminant test; a literal with no tag is one more branch. The
/// walk is strictly cheaper than the [`hash_value`] the intern path already pays.
///
/// A triple term's components are visited subject, predicate, object, each fully
/// before the next, over a work list rather than the call stack; the first tag that
/// fails the grammar ends the walk.
pub(crate) fn language_tags_well_formed(value: &TermValue) -> bool {
    language_tags_with_storage(value).expect("resident language-tag traversal allocation")
}

fn language_tags_with_storage(
    value: &TermValue,
) -> Result<bool, std::collections::TryReserveError> {
    let mut pending: purrdf_lex::walk::WorkList<&TermValue, 16> =
        purrdf_lex::walk::WorkList::with(value);
    while let Some(term) = pending.pop() {
        match term {
            TermValue::Iri(_) | TermValue::Blank { .. } => {}
            TermValue::Literal { language, .. } => {
                if language.as_deref().is_some_and(|tag| {
                    !purrdf_iri::langtag::is_well_formed_with(tag, LANGTAG_PROFILE)
                }) {
                    return Ok(false);
                }
            }
            TermValue::Triple { s, p, o } => {
                pending.try_push(&**o)?;
                pending.try_push(&**p)?;
                pending.try_push(&**s)?;
            }
        }
    }
    Ok(true)
}

fn admit_identity_walk(
    value: &TermValue,
    workspace: &crate::WorkspaceCapability,
) -> Result<crate::WorkspaceAllocation, crate::EvalError> {
    let nodes = crate::retained::term_layout(value, workspace)?.nodes;
    let bytes =
        TermValue::hash_workspace_bound(nodes).ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
    workspace.charge(u64::try_from(bytes).map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?)
}

impl ScratchInterner {
    pub(crate) fn owned_value_count(&self) -> usize {
        self.values.len()
    }
    /// Empty this interner while KEEPING the tables it has already grown.
    ///
    /// Observationally a fresh [`Self::new`] — every id it could answer is gone, the
    /// next mint is [`ScratchId`] zero again, and the minted-byte total the scratch
    /// ceiling is charged against restarts at zero — and that equality is the whole
    /// safety argument. A run that reads this interner after another run cleared it
    /// sees exactly what it would have seen from a brand-new one; what it does not
    /// pay for is the two allocations growing those tables from empty costs.
    ///
    /// # Exhaustiveness is the property, and the compiler holds it
    ///
    /// The destructuring `let` below names EVERY field with no `..` rest pattern, so
    /// a field added to [`ScratchInterner`] later and not cleared here does not
    /// compile. That matters more than it usually does: a table retained across runs
    /// and not cleared leaks one run's answer into the next, which is a silently
    /// wrong answer rather than a visible failure, and it is the exact defect a
    /// reviewer reading a hand-written list of `.clear()` calls cannot see is
    /// missing.
    ///
    /// `minted_bytes` is reset rather than carried for the same reason, and it is
    /// the field a careless clear would most plausibly keep "because it is only a
    /// counter": [`crate::eval::EvalCtx::charge_scratch_growth`] charges the
    /// DIFFERENCE between this total and this arena's own charged watermark, so
    /// carrying it across runs would silently re-charge one run's minting to the
    /// next and change where a `ScratchBytes` ceiling trips.
    pub(crate) fn clear(&mut self) {
        let Self {
            shared,
            shared_len,
            values,
            index,
            blank_labels,
            track_blank_labels,
            minted_bytes,
            charged_bytes,
            ghosts,
            ghost_index,
            values_admission: _,
            index_admission: _,
        } = self;
        *shared = None;
        *shared_len = 0;
        values.clear();
        index.clear();
        blank_labels.clear();
        *track_blank_labels = false;
        *minted_bytes = 0;
        charged_bytes.store(0, std::sync::atomic::Ordering::Relaxed);
        ghosts.clear();
        ghost_index.clear();
    }

    /// A conservative byte charge for the tables this interner is RETAINING — the
    /// capacity it keeps across a [`Self::clear`], not the values it held.
    ///
    /// Reported so a retained interner can be charged to
    /// [`crate::plan_memory::interner_memory_observer`] the way the per-worker
    /// interners are: retained capacity is retained memory whether or not anything
    /// is currently in it, and a host that can see a plan's bytes should be able to
    /// see these too. Excludes allocator overhead and the values' own payloads,
    /// which a cleared interner no longer owns.
    pub(crate) fn retained_capacity_bytes(&self) -> usize {
        self.values
            .capacity()
            .saturating_mul(size_of::<TermValue>())
            .saturating_add(
                self.index
                    .capacity()
                    .saturating_mul(size_of::<(ScratchId, u64)>()),
            )
            .saturating_add(self.blank_labels.capacity_bytes())
    }

    /// Preserve concrete input identities when a user function starts a fresh
    /// computed-term table. Its counter is shared with the calling evaluation.
    pub(crate) fn fresh_for_user_function_admitted(
        &mut self,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, crate::EvalError> {
        self.track_blank_labels_admitted(workspace)?;
        let blank_labels = self.blank_labels.clone();
        let minted_bytes = blank_labels
            .iter()
            .map(|label| {
                u64::try_from(label.len())
                    .unwrap_or(u64::MAX)
                    .saturating_add(TERM_OVERHEAD)
            })
            .fold(0_u64, u64::saturating_add);
        Ok(Self {
            blank_labels,
            track_blank_labels: true,
            minted_bytes,
            ..Self::default()
        })
    }

    fn track_blank_labels_admitted(
        &mut self,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        if !self.track_blank_labels {
            let shared = self.shared.as_deref();
            for value in shared
                .into_iter()
                .flat_map(|shared| (0..shared.computed_count()).map(|i| shared.value_at(i)))
                .chain(&self.values)
            {
                reserve_value_blanks_admitted(
                    value,
                    &mut self.blank_labels,
                    &mut self.minted_bytes,
                    workspace,
                )?;
            }
            self.track_blank_labels = true;
        }
        Ok(())
    }

    /// Reserve a scoped concrete input without resolving it against a dataset.
    #[cfg(test)]
    pub(crate) fn reserve_blank_identity(&mut self, label: &str, scope: purrdf_core::BlankScope) {
        self.reserve_blank_identity_admitted(label, scope, &crate::WorkspaceCapability::resident())
            .expect("resident blank identity index");
    }
    pub(crate) fn reserve_blank_identity_admitted(
        &mut self,
        label: &str,
        scope: purrdf_core::BlankScope,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        if scope == purrdf_core::BlankScope::DEFAULT {
            self.track_blank_labels_admitted(workspace)?;
            reserve_blank_label_admitted(
                label,
                &mut self.blank_labels,
                &mut self.minted_bytes,
                workspace,
            )?;
        }
        Ok(())
    }

    /// A lazily indexed membership check, independent of term interning order.
    #[cfg(test)]
    pub(crate) fn blank_label_is_reserved(&mut self, label: &str) -> bool {
        self.blank_label_is_reserved_admitted(label, &crate::WorkspaceCapability::resident())
            .expect("resident blank identity index")
    }
    pub(crate) fn blank_label_is_reserved_admitted(
        &mut self,
        label: &str,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<bool, crate::EvalError> {
        self.track_blank_labels_admitted(workspace)?;
        Ok(self.blank_labels.contains(label))
    }

    /// Intern a dataset-independent value to a [`SolutionTerm`], **promoting** it to
    /// [`SolutionTerm::Existing`] if `dataset` already contains the term.
    ///
    /// This is the unification rule: a value already in the data never becomes a
    /// `Computed` id, so cross-case join keys are unequal by construction.
    ///
    /// # The one value that is never promoted
    ///
    /// A blank node at the RESERVED query blank-node scope, `BlankScope(u32::MAX)`,
    /// was written inside a `cdt:List` / `cdt:Map` literal in the QUERY, and a query
    /// is its own blank-node document: such a node is distinct from every node of
    /// the queried data by definition, not by luck of the label. Promotion is
    /// therefore skipped for it, so the separation holds even against a dataset that
    /// (absurdly) interned a blank under the reserved scope. Every other value —
    /// including a blank a composite literal of the DATA names — takes the promotion
    /// path unchanged, which is what keeps a bare `_:b` and the `_:b` embedded in a
    /// `cdt:List` literal of the same document one node.
    ///
    /// No dataset may use that scope; `crate::convert`'s `QUERY_BLANK_SCOPE` states
    /// the reservation and is where query text is bound into it.
    ///
    /// # This door does not ask the grammar
    ///
    /// `intern` admits `value` exactly as it is handed over. In particular it
    /// does **not** judge a literal's language tag: a caller that builds a
    /// [`TermValue::Literal`] by hand with `language: Some("en us")` gets a
    /// [`SolutionTerm`] carrying that tag straight back, and it will be written
    /// out as `"x"@en us` by the results writers — bytes no reader takes back.
    ///
    /// That is deliberate, and it is the same category as the infallible
    /// constructors [`purrdf_core::TermValue::lang_literal`] and
    /// `RdfLiteral::language_tagged`, which also build a tagged literal out of
    /// whatever string the caller names: the caller wrote the tag, so the caller
    /// owns it. A `TermValue` that came out of an [`RdfDataset`] or any codec
    /// reader was already judged on `LANGTAG_PROFILE` by
    /// `RdfLiteral::validate_components`, so for those values — which is every
    /// value the evaluator itself mints — there is nothing left to ask.
    ///
    /// For a value that came from OUTSIDE the evaluator, use
    /// [`Self::intern_checked`], which asks the grammar and refuses. The
    /// extension seams in this crate (`crate::user_fn`, `crate::row_ingest`,
    /// `crate::property_fn_eval`, `crate::modifier::eval_custom_aggregate`, and
    /// the composite-datatype lifters `crate::cdt_fn` / `crate::cdt_unfold` /
    /// `crate::list_fn`) all go through that door.
    ///
    /// # The algebra's own ground terms stay on this door
    ///
    /// A `VALUES` cell (`crate::modifier::eval_values`) and a zero-length path
    /// endpoint (`crate::path`) are `purrdf_sparql_algebra::GroundTerm`s, not
    /// extension-computed values, and they intern here on purpose — twice over.
    ///
    /// They are gated before they arrive, on this same profile every time:
    /// query text by the SPARQL parser; a hand-built
    /// `purrdf_sparql_algebra::Query` by that crate's algebra validator, which
    /// [`crate::PreparedQuery::rewritten`] runs and which refuses an "invalid
    /// language tag in query algebra"; a `SparqlRequest` pre-binding by
    /// `crate::substitute`'s ingress, which REFUSES with a diagnostic rather
    /// than passing the value on — and which has to, because substitution
    /// rewrites an ALREADY-admitted plan, so the algebra validator never sees
    /// its `VALUES` cell; and a SEP-0007 Values-Insertion row by the fact that
    /// its cells are already-admitted solution terms being put back.
    ///
    /// And at those two positions a [`None`] would not be §17.2's unbound
    /// result. An unbound `VALUES` cell is `UNDEF`, compatible with everything,
    /// so it deletes the pre-binding's constraint and returns MORE rows; an
    /// unbound path endpoint has no binding to give up at all, only the whole
    /// zero-length row §18.5.1 requires. A refusal may cost a binding — never a
    /// constraint and never a row — so where it could only cost one of those,
    /// the gate belongs upstream and this door stays plain.
    ///
    /// [`RdfDataset`]: purrdf_core::RdfDataset
    ///
    /// # Depth
    ///
    /// A triple term is walked here — looked up, hashed, compared — over work lists, so
    /// a value nested to any depth costs no machine stack: its depth is bounded by memory
    /// alone, and every value the evaluator builds or receives at run time enters through
    /// this door or [`Self::intern_checked`] without being measured first.
    pub fn try_intern<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> Result<SolutionTerm<D::Id>, D::ReadError> {
        self.intern_value(dataset, value)
    }

    pub(crate) fn try_intern_admitted<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<SolutionTerm<D::Id>, crate::EvalError> {
        let _working = admit_identity_walk(&value, workspace)?;
        self.intern_value_with(
            dataset,
            value,
            |dataset, value| {
                dataset
                    .term_id_by_value(value)
                    .map_err(crate::EvalError::source_read)
            },
            |arena| arena.admit_storage_growth(workspace),
            InternValueHooks {
                hash_value: |value: &TermValue| workspace.term_hash(value),
                equal: |left: &TermValue, right: &TermValue| workspace.terms_equal(left, right),
                storage_error: |_| crate::EvalError::AllocationFailed {
                    construct: "computed term identity traversal",
                },
                track_value: |value: &TermValue, labels: &mut BlankLabels, bytes: &mut u64| {
                    reserve_value_blanks_admitted(value, labels, bytes, workspace)
                },
            },
        )
    }

    pub(crate) fn try_intern_checked_admitted<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Option<SolutionTerm<D::Id>>, crate::EvalError> {
        let _working = admit_identity_walk(&value, workspace)?;
        if !language_tags_with_storage(&value).map_err(|_| crate::EvalError::AllocationFailed {
            construct: "computed term language traversal",
        })? {
            return Ok(None);
        }
        self.try_intern_admitted(dataset, value, workspace)
            .map(Some)
    }

    pub(crate) fn try_intern_checked_with_admission<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
        workspace: &crate::WorkspaceCapability,
        lookup: impl FnOnce(&TermValue) -> Result<Option<D::Id>, crate::EvalError>,
    ) -> Result<Option<SolutionTerm<D::Id>>, crate::EvalError> {
        let _working = admit_identity_walk(&value, workspace)?;
        if !language_tags_with_storage(&value).map_err(|_| crate::EvalError::AllocationFailed {
            construct: "computed term language traversal",
        })? {
            return Ok(None);
        }
        self.intern_value_with(
            dataset,
            value,
            |_, value| lookup(value),
            |arena| arena.admit_storage_growth(workspace),
            InternValueHooks {
                hash_value: |value: &TermValue| workspace.term_hash(value),
                equal: |left: &TermValue, right: &TermValue| workspace.terms_equal(left, right),
                storage_error: |_| crate::EvalError::AllocationFailed {
                    construct: "computed term identity traversal",
                },
                track_value: |value: &TermValue, labels: &mut BlankLabels, bytes: &mut u64| {
                    reserve_value_blanks_admitted(value, labels, bytes, workspace)
                },
            },
        )
        .map(Some)
    }

    fn admit_storage_growth(
        &mut self,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        if !workspace.is_bounded() {
            return Ok(());
        }
        if self.computed_count() >= u32::MAX as usize {
            return Err(crate::EvalError::WorkspaceBoundOverflow);
        }
        if self.values.len() == self.values.capacity() {
            let capacity = self
                .values
                .capacity()
                .checked_mul(2)
                .ok_or(crate::EvalError::WorkspaceBoundOverflow)?
                .max(1);
            let bytes = capacity
                .checked_mul(size_of::<TermValue>())
                .ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
            let admission = workspace.charge(
                u64::try_from(bytes).map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?,
            )?;
            self.values
                .try_reserve_exact(capacity - self.values.len())
                .map_err(|_| crate::EvalError::AllocationFailed {
                    construct: "computed term arena",
                })?;
            self.values_admission = Some(admission);
        }
        if self.index.len() == self.index.capacity() {
            let capacity = self
                .index
                .len()
                .checked_add(1)
                .ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
            let bytes =
                purrdf_core::hash::hash_table_allocation_bound::<(ScratchId, u64)>(capacity)
                    .ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
            let admission = workspace.charge(
                u64::try_from(bytes).map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?,
            )?;
            self.index.try_reserve(1, |(_, hash)| *hash).map_err(|_| {
                crate::EvalError::AllocationFailed {
                    construct: "computed term index",
                }
            })?;
            self.index_admission = Some(admission);
        }
        Ok(())
    }

    /// [`Self::intern`], but it asks the language grammar first and returns
    /// [`None`] instead of minting a term no writer in the workspace could
    /// spell.
    ///
    /// This is the door for a whole `TermValue` the evaluator did not build.
    /// Four public extension seams hand it such values — a `UserFunction`
    /// (`crate::user_fn`), a `ServiceResolver`'s rows (`crate::row_ingest`), a
    /// `PropertyFunction`'s rows (`crate::property_fn_eval`), and a
    /// `CustomAggregate`'s result (`crate::modifier::eval_custom_aggregate`) —
    /// and none of those traits constrains the language string at all. So do the
    /// composite-datatype lifters `crate::cdt_fn` and `crate::cdt_unfold`, whose
    /// members are RDF terms parsed back out of a literal's LEXICAL FORM rather
    /// than terms the kernel ever admitted.
    ///
    /// `crate::list_fn` goes through this door too, but for uniformity rather
    /// than need: it walks an `rdf:List` in the dataset, so its members ARE
    /// kernel-admitted and the judgement there is vacuous. Its own `intern` says
    /// so rather than borrowing the lifters' justification.
    ///
    /// A value carrying a tag `LANGTAG_PROFILE` refuses is **not interned**: the
    /// arena is untouched, no id is minted, and no scratch bytes are charged.
    /// That is not a dropped term, and it is not a dropped row either — it is
    /// the "expression error" of SPARQL 1.1 §17.2, whose specified outcome is an
    /// unbound result, which is exactly what `crate::expr::eval_str_lang`
    /// already returns (`Ok(None)`) when `STRLANG` is handed the same garbage.
    /// Each caller maps the [`None`] onto the unbound outcome its own position
    /// requires — **one binding, never a whole row or solution** — and none of
    /// them may turn it back into a term.
    ///
    /// The tags are judged through every level of an RDF-1.2 triple term, because a
    /// tag one level down is still a tag that reaches the writer. [`TermValue::Iri`]
    /// and [`TermValue::Blank`] carry none, so for them this is a discriminant
    /// test and the two doors are the same door.
    pub fn try_intern_checked<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> Result<Option<SolutionTerm<D::Id>>, D::ReadError> {
        if !language_tags_well_formed(&value) {
            return Ok(None);
        }
        self.intern_value(dataset, value).map(Some)
    }

    /// Intern an IRI. Infallible by construction: an IRI carries no language tag,
    /// so [`Self::intern_checked`]'s gate would have nothing to judge.
    pub fn try_intern_iri<D: DatasetView>(
        &mut self,
        dataset: &D,
        iri: String,
    ) -> Result<SolutionTerm<D::Id>, D::ReadError> {
        self.intern_value(dataset, TermValue::Iri(iri))
    }

    /// Intern a blank node. Infallible by construction: a blank node carries no
    /// language tag, so [`Self::intern_checked`]'s gate would have nothing to
    /// judge.
    pub fn try_intern_blank<D: DatasetView>(
        &mut self,
        dataset: &D,
        label: String,
        scope: purrdf_core::BlankScope,
    ) -> Result<SolutionTerm<D::Id>, D::ReadError> {
        self.intern_value(dataset, TermValue::Blank { label, scope })
    }

    /// Intern a datatyped literal from its parts. Infallible by construction: the
    /// value is built here with `language: None`, so [`Self::intern_checked`]'s
    /// gate would have nothing to judge. This is how the evaluator mints its own
    /// `xsd:string` / `xsd:integer` / `xsd:boolean` results without inventing a
    /// failure branch that cannot be taken.
    pub fn try_intern_datatyped<D: DatasetView>(
        &mut self,
        dataset: &D,
        lexical_form: String,
        datatype: String,
    ) -> Result<SolutionTerm<D::Id>, D::ReadError> {
        self.intern_value(
            dataset,
            TermValue::Literal {
                lexical_form,
                datatype,
                language: None,
                direction: None,
            },
        )
    }

    /// Count `value`, minted by a forked loop's worker in its copy of this arena, as the
    /// loop's ordered commit replays the worker's mints (`crate::row_checkpoint`): when
    /// this arena holds no equal value, its bytes join [`Self::minted_bytes`] as the
    /// in-order loop's intern would add them, and it is kept as a ghost so that a later
    /// intern of it is not counted twice. The worker's own intern already promoted it
    /// and passed it through the language gate.
    pub(crate) fn count_worker_mint(&mut self, hash: u64, value: TermValue) {
        debug_assert_eq!(
            hash,
            purrdf_hash::fixed::hash_one(&value),
            "the value's own hash"
        );
        if self.find(hash, &value).is_some()
            || self
                .ghost_index
                .find(hash, |&slot| self.ghosts[slot].as_ref() == Some(&value))
                .is_some()
        {
            return;
        }
        if self.track_blank_labels {
            reserve_value_blanks(&value, &mut self.blank_labels, &mut self.minted_bytes);
        }
        self.minted_bytes = self.minted_bytes.saturating_add(value_bytes(&value));
        let slot = self.ghosts.len();
        self.ghosts.push(Some(value));
        let ghosts = &self.ghosts;
        self.ghost_index.insert_unique(hash, slot, |&slot| {
            ghosts[slot]
                .as_ref()
                .map_or(0, purrdf_hash::fixed::hash_one)
        });
    }

    /// Whether `value` (of `hash`) was a ghost, which is then stored and no longer one.
    fn take_ghost(&mut self, hash: u64, value: &TermValue) -> bool {
        if self.ghost_index.is_empty() {
            return false;
        }
        let ghosts = &self.ghosts;
        let Ok(entry) = self
            .ghost_index
            .find_entry(hash, |&slot| ghosts[slot].as_ref() == Some(value))
        else {
            return false;
        };
        let (slot, _) = entry.remove();
        self.ghosts[slot] = None;
        true
    }

    /// The promotion + store-once body — the one `values.push` in the crate.
    /// Every public door above funnels here, so the promotion rule and the
    /// store-once rule have exactly one implementation.
    fn intern_value<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> Result<SolutionTerm<D::Id>, D::ReadError> {
        self.intern_value_with(
            dataset,
            value,
            D::term_id_by_value,
            |_| Ok(()),
            InternValueHooks {
                hash_value: |value: &TermValue| Ok(purrdf_hash::fixed::hash_one(value)),
                equal: |left: &TermValue, right: &TermValue| Ok(left == right),
                storage_error: |error| {
                    panic!("resident scratch proxy traversal allocation: {error}")
                },
                track_value: |value: &TermValue, labels: &mut BlankLabels, bytes: &mut u64| {
                    reserve_value_blanks(value, labels, bytes);
                    Ok(())
                },
            },
        )
    }

    fn intern_value_with<D, E, H, Q, R, B>(
        &mut self,
        dataset: &D,
        value: TermValue,
        lookup: impl FnOnce(&D, &TermValue) -> Result<Option<D::Id>, E>,
        admit: impl FnOnce(&mut Self) -> Result<(), E>,
        hooks: InternValueHooks<H, Q, R, B>,
    ) -> Result<SolutionTerm<D::Id>, E>
    where
        D: DatasetView,
        H: FnOnce(&TermValue) -> Result<u64, E>,
        Q: FnMut(&TermValue, &TermValue) -> Result<bool, E>,
        R: FnOnce(std::collections::TryReserveError) -> E,
        B: FnOnce(&TermValue, &mut BlankLabels, &mut u64) -> Result<(), E>,
    {
        let InternValueHooks {
            hash_value,
            mut equal,
            storage_error,
            track_value,
        } = hooks;
        // A computed identity already passed promotion in this execution.
        // Reusing it must not repeat the source lookup or retain a duplicate.
        let hash = hash_value(&value)?;
        if let Some(sid) = self.find_with(hash, &value, &mut equal)? {
            return Ok(SolutionTerm::Computed(sid));
        }
        if !is_query_scoped_blank(&value)
            && let Some(id) = lookup(dataset, &value)?
        {
            return Ok(SolutionTerm::Existing(id));
        }
        admit(self)?;
        let sid = ScratchId::from_index(self.computed_count());
        // A ghost's bytes, its labels included, were counted when it became one.
        if !self.take_ghost(hash, &value) {
            if self.track_blank_labels {
                track_value(&value, &mut self.blank_labels, &mut self.minted_bytes)?;
            }
            self.minted_bytes = self
                .minted_bytes
                .saturating_add(value_bytes_with_storage(&value).map_err(storage_error)?);
        }
        self.values.push(value);
        self.index
            .insert_unique(hash, (sid, hash), |(_, hash)| *hash);
        Ok(SolutionTerm::Computed(sid))
    }

    /// The deterministic byte size retained in this arena: each computed value's
    /// owned payload plus a fixed per-term constant, and each reserved allocation
    /// label's payload plus that same constant. The total depends on admitted
    /// identities rather than allocator growth.
    ///
    /// This is the meter behind the scratch-arena resource ceiling. It is its own
    /// dimension because arena growth is independent of any row or cell count:
    /// `CONCAT`, `GROUP_CONCAT`, `REPLACE`, and the list constructors mint arbitrarily
    /// large owned values, so a query can exhaust memory with a row count and a cell
    /// count that both sit comfortably inside their ceilings.
    ///
    /// Monotone: de-duplicated interns add nothing, and nothing ever removes a value, so
    /// a caller can charge the difference since its last reading without double-counting.
    #[must_use]
    pub fn minted_bytes(&self) -> u64 {
        self.minted_bytes
    }

    /// The growth [`Self::claim_uncharged_growth`] would claim now, without claiming it.
    pub(crate) fn uncharged_growth(&self) -> u64 {
        self.minted_bytes.saturating_sub(
            self.charged_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Claim only this arena's growth for the execution's shared scratch meter.
    /// Repeated checkpoints cannot charge the same bytes twice; a cleared or
    /// independently evaluated copied arena starts its own accounting from zero.
    pub(crate) fn claim_uncharged_growth(&self) -> u64 {
        let charged = self
            .charged_bytes
            .fetch_max(self.minted_bytes, std::sync::atomic::Ordering::Relaxed);
        self.minted_bytes.saturating_sub(charged)
    }

    /// A different execution receipt accounts this arena's full retained value.
    /// Attaching the same receipt preserves its existing watermark instead.
    pub(crate) fn reset_charged_growth(&mut self) {
        *self.charged_bytes.get_mut() = 0;
    }

    /// Materialize a [`SolutionTerm`] to an owned, dataset-independent [`TermValue`].
    ///
    /// `Existing` ids are resolved from the dataset (recursively for RDF-1.2 triple
    /// terms, expanding the literal datatype id to its IRI string); `Computed` ids
    /// are read from the scratch table. This is the egress boundary used to build
    /// `SparqlResult` rows.
    pub fn try_value_of<D: DatasetView>(
        &self,
        dataset: &D,
        term: SolutionTerm<D::Id>,
    ) -> Result<TermValue, purrdf_core::TermLookupError<D::ReadError>> {
        match term {
            SolutionTerm::Existing(id) => dataset.term_value(id),
            SolutionTerm::Computed(sid) => Ok(self.value_at(sid.index()).clone()),
        }
    }

    pub(crate) fn try_owned_value_of<D: DatasetView>(
        &self,
        dataset: &D,
        term: SolutionTerm<D::Id>,
        workspace: &crate::WorkspaceCapability,
        source_error: impl Fn(D::ReadError) -> crate::EvalError,
    ) -> Result<crate::WorkspaceTerm, crate::EvalError> {
        if let SolutionTerm::Computed(id) = term {
            return workspace.clone_term(self.computed_value(id));
        }
        let SolutionTerm::Existing(id) = term else {
            unreachable!("computed terms returned above")
        };
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        // This ordered carrier keeps the actual payload alive beneath its
        // original frame until every fallible publication step has succeeded.
        let mut value = {
            let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
            Some(dataset.term_value_with_memory(
                id,
                &mut memory,
                |error| match error {
                    purrdf_core::TermLookupError::Read(error) => source_error(error),
                    purrdf_core::TermLookupError::ForeignId => {
                        if workspace.is_bounded() {
                            crate::NativeDiagnostic::error(
                                crate::NativeDiagnosticKind::Internal,
                                "foreign dataset term id",
                                workspace,
                            )
                        } else {
                            crate::EvalError::source_read("foreign dataset term id")
                        }
                    }
                },
                |error, memory| {
                    memory
                        .admission_mut()
                        .storage_error(error, "resolved term value")
                },
            )?)
        };
        frame.finish_term(|| {
            value
                .take()
                .expect("original term checked before publication")
        })
    }

    pub(crate) fn try_value_of_admitted<D: DatasetView>(
        &self,
        dataset: &D,
        term: SolutionTerm<D::Id>,
        workspace: &crate::WorkspaceCapability,
        source_error: impl Fn(D::ReadError) -> crate::EvalError,
    ) -> Result<TermValue, crate::EvalError> {
        if !workspace.is_bounded() {
            return self
                .try_value_of(dataset, term)
                .map_err(crate::EvalError::source_read);
        }
        let (payload, temporary) = match term {
            SolutionTerm::Existing(_) => {
                let bytes =
                    dataset
                        .max_owned_term_bytes()
                        .ok_or(crate::EvalError::WorkspaceUnpriced(
                            "a view without an owned term certificate",
                        ))?;
                // Every non-root node owns one TermValue box. This remains a
                // safe node bound when a provider omits inline root storage.
                let nodes = usize::try_from(bytes / size_of::<TermValue>() as u64)
                    .map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?
                    .checked_add(1)
                    .ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
                (
                    bytes,
                    purrdf_core::ir::fold_workspace_bound::<D::Id, TermValue>(nodes),
                )
            }
            SolutionTerm::Computed(id) => {
                let layout = crate::retained::term_layout(self.computed_value(id), workspace)?;
                (layout.bytes, TermValue::clone_workspace_bound(layout.nodes))
            }
        };
        let temporary = temporary.ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
        let _temporary = workspace.charge(
            u64::try_from(temporary).map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?,
        )?;
        workspace.retain(payload)?;
        match term {
            SolutionTerm::Existing(id) => dataset.term_value_with_storage(
                id,
                |error| match error {
                    purrdf_core::TermLookupError::Read(error) => source_error(error),
                    purrdf_core::TermLookupError::ForeignId => crate::NativeDiagnostic::error(
                        crate::NativeDiagnosticKind::Internal,
                        "foreign dataset term id",
                        workspace,
                    ),
                },
                |_| crate::EvalError::AllocationFailed {
                    construct: "materialized dataset term",
                },
            ),
            SolutionTerm::Computed(id) => self.computed_value(id).try_clone().map_err(|_| {
                crate::EvalError::AllocationFailed {
                    construct: "materialized computed term",
                }
            }),
        }
    }

    /// Borrow the computed value behind a [`ScratchId`] (no clone) — the hot-path
    /// twin of [`Self::value_of`] for callers that only need to *inspect* a
    /// computed term (e.g. the comparison fast path in `expr`).
    #[must_use]
    pub fn computed_value(&self, sid: ScratchId) -> &TermValue {
        self.value_at(sid.index())
    }

    /// The value of id `index`, in this arena or the one it extends.
    fn value_at(&self, index: usize) -> &TermValue {
        match index.checked_sub(self.shared_len) {
            Some(local) => &self.values[local],
            None => self.shared.as_deref().map_or_else(
                || unreachable!("an id below shared_len is shared"),
                |shared| shared.value_at(index),
            ),
        }
    }

    /// The id of a value equal to `value` (whose hash is `hash`), in this arena or the
    /// one it extends.
    fn find(&self, hash: u64, value: &TermValue) -> Option<ScratchId> {
        let result: Result<_, core::convert::Infallible> =
            self.find_with(hash, value, &mut |left, right| Ok(left == right));
        match result {
            Ok(value) => value,
            Err(error) => match error {},
        }
    }

    fn find_with<E>(
        &self,
        hash: u64,
        value: &TermValue,
        equal: &mut impl FnMut(&TermValue, &TermValue) -> Result<bool, E>,
    ) -> Result<Option<ScratchId>, E> {
        if let Some(sid) = self
            .shared
            .as_deref()
            .map(|shared| shared.find_with(hash, value, equal))
            .transpose()?
            .flatten()
        {
            return Ok(Some(sid));
        }
        let mut failure = None;
        let found = self
            .index
            .find(hash, |(sid, _)| {
                if failure.is_some() {
                    return false;
                }
                match equal(self.value_at(sid.index()), value) {
                    Ok(same) => same,
                    Err(error) => {
                        failure = Some(error);
                        false
                    }
                }
            })
            .map(|(sid, _)| *sid);
        match failure {
            Some(error) => Err(error),
            None => Ok(found),
        }
    }

    /// An arena extending `shared`, frozen: it holds `shared`'s values as its own first
    /// ids without copying them, and mints after them, as a copy of `shared` would. A
    /// forked loop's worker takes one per block of the loop's items, so a block's worker
    /// costs no copy of the evaluation's arena however large that has grown.
    pub(crate) fn over(shared: &std::sync::Arc<Self>) -> Self {
        let shared_len = shared.computed_count();
        Self {
            // No id can reach an ancestor with zero terms. Retain its byte/blank-label
            // state below, but avoid that empty lookup layer. Keep the local tables
            // fresh even if the snapshot has a large index retained after clear().
            shared: (shared_len > 0).then(|| std::sync::Arc::clone(shared)),
            shared_len,
            blank_labels: shared.blank_labels.clone(),
            track_blank_labels: shared.track_blank_labels,
            minted_bytes: shared.minted_bytes,
            ..Self::default()
        }
    }

    /// The number of distinct computed terms minted so far (diagnostics/tests).
    #[must_use]
    pub fn computed_count(&self) -> usize {
        self.shared_len + self.values.len()
    }

    /// Move out every computed value minted at or after index `base`, in mint order and
    /// with its hash, leaving the arena holding the values before it. A forked loop's worker hands its
    /// mints to the loop's ordered commit this way once its chunk is done
    /// (`crate::row_checkpoint`); ids at or past `base` are dangling afterwards, so the
    /// worker reads none of them again.
    pub(crate) fn take_values_from(&mut self, base: usize) -> Vec<(u64, TermValue)> {
        if base >= self.computed_count() {
            return Vec::new();
        }
        debug_assert!(base >= self.shared_len, "a worker takes only its own mints");
        self.index.retain(|(sid, _)| sid.index() < base);
        self.values
            .split_off(base - self.shared_len)
            .into_iter()
            .map(|value| (purrdf_hash::fixed::hash_one(&value), value))
            .collect()
    }

    /// Make room for `additional` more [ghosts](Self::count_worker_mint).
    pub(crate) fn reserve_ghosts(&mut self, additional: usize) {
        self.ghosts.reserve(additional);
        let ghosts = &self.ghosts;
        self.ghost_index.reserve(additional, |&slot| {
            ghosts[slot]
                .as_ref()
                .map_or(0, purrdf_hash::fixed::hash_one)
        });
    }
    /// Promote and intern a value in a resident dataset.
    pub fn intern<D: DatasetView<ReadError = core::convert::Infallible>>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> SolutionTerm<D::Id> {
        match self.try_intern(dataset, value) {
            Ok(term) => term,
            Err(error) => match error {},
        }
    }
    /// Judge language tags and intern a value in a resident dataset.
    pub fn intern_checked<D: DatasetView<ReadError = core::convert::Infallible>>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> Option<SolutionTerm<D::Id>> {
        match self.try_intern_checked(dataset, value) {
            Ok(term) => term,
            Err(error) => match error {},
        }
    }
    /// Intern an IRI in a resident dataset.
    pub fn intern_iri<D: DatasetView<ReadError = core::convert::Infallible>>(
        &mut self,
        dataset: &D,
        iri: String,
    ) -> SolutionTerm<D::Id> {
        match self.try_intern_iri(dataset, iri) {
            Ok(term) => term,
            Err(error) => match error {},
        }
    }
    /// Intern a blank node in a resident dataset.
    pub fn intern_blank<D: DatasetView<ReadError = core::convert::Infallible>>(
        &mut self,
        dataset: &D,
        label: String,
        scope: purrdf_core::BlankScope,
    ) -> SolutionTerm<D::Id> {
        match self.try_intern_blank(dataset, label, scope) {
            Ok(term) => term,
            Err(error) => match error {},
        }
    }
    /// Intern a typed literal in a resident dataset.
    pub fn intern_datatyped<D: DatasetView<ReadError = core::convert::Infallible>>(
        &mut self,
        dataset: &D,
        lexical: String,
        datatype: String,
    ) -> SolutionTerm<D::Id> {
        match self.try_intern_datatyped(dataset, lexical, datatype) {
            Ok(term) => term,
            Err(error) => match error {},
        }
    }
    /// Materialize a term in a resident dataset; foreign ids violate this contract.
    pub fn value_of<D: DatasetView<ReadError = core::convert::Infallible>>(
        &self,
        dataset: &D,
        term: SolutionTerm<D::Id>,
    ) -> TermValue {
        self.try_value_of(dataset, term)
            .expect("an id the view handed the evaluator resolves to a value")
    }
}

fn reserve_blank_label_admitted(
    label: &str,
    labels: &mut BlankLabels,
    bytes: &mut u64,
    workspace: &crate::WorkspaceCapability,
) -> Result<(), crate::EvalError> {
    if labels.insert_admitted(label, workspace)? {
        *bytes = bytes
            .saturating_add(u64::try_from(label.len()).unwrap_or(u64::MAX))
            .saturating_add(TERM_OVERHEAD);
    }
    Ok(())
}

fn reserve_value_blanks(value: &TermValue, labels: &mut BlankLabels, bytes: &mut u64) {
    reserve_value_blanks_admitted(
        value,
        labels,
        bytes,
        &crate::WorkspaceCapability::resident(),
    )
    .expect("resident blank identity discovery");
}
fn reserve_value_blanks_admitted(
    value: &TermValue,
    labels: &mut BlankLabels,
    bytes: &mut u64,
    workspace: &crate::WorkspaceCapability,
) -> Result<(), crate::EvalError> {
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let mut failure = None;
    let visited = value.visit_blank_identities_with_memory(
        |label, scope, _memory| {
            if scope == purrdf_core::BlankScope::DEFAULT
                && let Err(error) = reserve_blank_label_admitted(label, labels, bytes, workspace)
            {
                failure = Some(error);
                return core::ops::ControlFlow::Break(());
            }
            core::ops::ControlFlow::Continue(())
        },
        &mut purrdf_lex::allocation::Memory::new(&mut frame),
    );
    if let Some(error) = failure {
        return Err(error);
    }
    let _ = visited.map_err(|error| frame.storage_error(error, "blank identity discovery"))?;
    Ok(())
}

/// Resolve a dataset-local id to an owned, dataset-independent [`TermValue`]
/// through [`DatasetView::term_value`] (the C0.8 boundary), for an id the view
/// itself handed to the evaluator.
///
/// # Panics
///
/// On an id the view did not mint: a literal whose datatype does not resolve to an
/// IRI. Every id the evaluator holds was read out of the view it is resolved
/// against.
#[cfg(test)]
pub(crate) fn term_id_to_value<D: DatasetView<ReadError = core::convert::Infallible>>(
    dataset: &D,
    id: D::Id,
) -> TermValue {
    dataset
        .term_value(id)
        .expect("an id the view handed the evaluator resolves to a value")
}

/// Typed materialization for operationally fallible views.
pub(crate) fn try_term_id_to_value<D: DatasetView>(
    dataset: &D,
    id: D::Id,
) -> Result<TermValue, crate::EvalError> {
    dataset
        .term_value(id)
        .map_err(crate::EvalError::source_read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::TermBox;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};

    fn dataset_with_one_iri() -> std::sync::Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn empty_shared_arena_flattens_without_copying_retained_index_or_losing_mint_state() {
        use std::sync::Arc;
        let ds = dataset_with_one_iri();
        let mut parent = ScratchInterner::default();
        // Workspace reuse keeps the grown index while removing all stored terms.
        for index in 0..512 {
            parent.intern(&ds, TermValue::integer(index));
        }
        parent.clear();
        assert_eq!(parent.computed_count(), 0);
        assert!(parent.index.capacity() >= 512);
        parent.reserve_blank_identity("input", purrdf_core::BlankScope::DEFAULT);
        let ghost = TermValue::Blank {
            label: "ghost".to_owned(),
            scope: purrdf_core::BlankScope::DEFAULT,
        };
        parent.count_worker_mint(purrdf_hash::fixed::hash_one(&ghost), ghost.clone());
        assert!(parent.ghosts.iter().any(Option::is_some));
        let inherited = parent.minted_bytes();
        assert_eq!(parent.claim_uncharged_growth(), inherited);
        let mut copy = parent.clone();
        let snapshot = Arc::new(parent.clone());
        assert!(snapshot.index.capacity() >= 512);
        let mut worker = ScratchInterner::over(&snapshot);
        assert!(worker.shared.is_none());
        assert_eq!(
            worker.index.capacity(),
            0,
            "no retained bucket copy per worker"
        );
        assert_eq!(worker.minted_bytes(), inherited);
        assert!(worker.blank_label_is_reserved("input"));
        assert!(worker.blank_label_is_reserved("ghost"));
        assert!(worker.ghosts.is_empty() && copy.ghosts.is_empty());
        assert_eq!(
            worker.claim_uncharged_growth(),
            copy.claim_uncharged_growth()
        );
        for value in [ghost.clone(), TermValue::integer(9_000), ghost.clone()] {
            let expected = copy.intern(&ds, value.clone());
            let actual = worker.intern(&ds, value);
            assert_eq!(actual, expected);
            assert_eq!(worker.value_of(&ds, actual), copy.value_of(&ds, expected));
            assert_eq!(worker.minted_bytes(), copy.minted_bytes());
            assert_eq!(
                worker.claim_uncharged_growth(),
                copy.claim_uncharged_growth()
            );
        }
        let mints = worker.take_values_from(0);
        assert_eq!(
            mints,
            copy.take_values_from(0),
            "same ordered harvest and hashes"
        );
        for (hash, value) in mints {
            parent.count_worker_mint(hash, value);
        }
        let before = parent.minted_bytes();
        parent.intern(&ds, ghost.clone());
        assert_eq!(
            parent.minted_bytes(),
            before,
            "parent ghost counted only once"
        );
        let nonempty = Arc::new(parent);
        let mut child = ScratchInterner::over(&nonempty);
        assert!(Arc::ptr_eq(
            child.shared.as_ref().expect("inherited term"),
            &nonempty
        ));
        assert_eq!(
            child.intern(&ds, ghost),
            SolutionTerm::Computed(ScratchId::from_index(0))
        );
        assert_eq!(child.computed_count(), 1);
        assert_eq!(child.take_values_from(1), []);
    }

    #[test]
    fn existing_value_is_promoted_not_computed() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::default();
        let term = scratch.intern(&ds, TermValue::Iri("https://example.org/s".to_owned()));
        // The value is in the dataset → it MUST resolve to an Existing id, and the
        // scratch table stays empty (the promotion rule).
        assert!(matches!(term, SolutionTerm::Existing(_)));
        assert_eq!(scratch.computed_count(), 0);
    }

    #[test]
    fn novel_value_is_computed_and_deduped() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::default();
        let novel = TermValue::Literal {
            lexical_form: "hello world".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        };
        let a = scratch.intern(&ds, novel.clone());
        let b = scratch.intern(&ds, novel);
        // Absent from the dataset → Computed, and the two interns share one id.
        assert!(matches!(a, SolutionTerm::Computed(_)));
        assert_eq!(a, b);
        assert_eq!(scratch.computed_count(), 1);
    }

    #[test]
    fn existing_and_computed_are_never_equal() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::default();
        let existing = scratch.intern(&ds, TermValue::Iri("https://example.org/s".to_owned()));
        let computed = scratch.intern(
            &ds,
            TermValue::Iri("https://example.org/NOT-PRESENT".to_owned()),
        );
        assert!(matches!(existing, SolutionTerm::Existing(_)));
        assert!(matches!(computed, SolutionTerm::Computed(_)));
        // The whole point of the unification rule: cross-case is unequal.
        assert_ne!(existing, computed);
    }

    #[test]
    fn value_of_round_trips_existing_and_computed() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::default();

        let iri = TermValue::Iri("https://example.org/s".to_owned());
        let existing = scratch.intern(&ds, iri.clone());
        assert_eq!(scratch.value_of(&ds, existing), iri);

        let lit = TermValue::Literal {
            lexical_form: "42".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        };
        let computed = scratch.intern(&ds, lit.clone());
        assert_eq!(scratch.value_of(&ds, computed), lit);
    }

    #[test]
    fn value_of_resolves_a_literal_from_the_dataset() {
        // A literal interned in the dataset resolves back to the same value space,
        // exercising the datatype-id → IRI expansion in `term_id_to_value`.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_literal(RdfLiteral {
            lexical_form: "42".to_owned(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
            language: None,
            direction: None,
        });
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("freeze");

        let scratch = ScratchInterner::default();
        let value = scratch.value_of(&ds, SolutionTerm::Existing(o));
        assert_eq!(
            value,
            TermValue::Literal {
                lexical_form: "42".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                language: None,
                direction: None,
            }
        );
    }

    /// One `TermValue` per tag, straight at the choke point, both halves.
    ///
    /// The accepted list is not decoration: `x-purrdf-afrikaans` and
    /// `x-gmeow-english` are tags this workspace's own artifacts carry, and
    /// `en-fr-jura` / `fr-be-fbcl` are tags approved W3C corpora carry, so
    /// refusing any of them would break data that is already valid — the mirror
    /// bug of the escape this gate closes.
    #[test]
    fn the_language_tag_gate_refuses_only_ungrammatical_tags() {
        let ds = dataset_with_one_iri();
        let tagged = |tag: &str| TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some(tag.to_owned()),
            direction: None,
        };

        for tag in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "de-CH-x-phonebk",
            "i-enochian",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "en-fr-jura",
            "fr-be-fbcl",
            "abcdefgh",
            "en-x-cantbethislong",
        ] {
            let mut scratch = ScratchInterner::default();
            assert!(
                scratch.intern_checked(&ds, tagged(tag)).is_some(),
                "{tag} is a tag real data carries and must still bind"
            );
            assert_eq!(scratch.computed_count(), 1, "{tag} must have been minted");
        }

        for tag in [
            "en us",
            "1",
            "9-9",
            "123-456",
            "en-",
            "-",
            "!!!",
            "abcdefghi",
            "",
        ] {
            let mut scratch = ScratchInterner::default();
            assert!(
                scratch.intern_checked(&ds, tagged(tag)).is_none(),
                "{tag:?} must not become a solution term"
            );
            // Refused means NOT interned: the arena is untouched, so a refusal
            // costs no scratch bytes and mints no id a later row could hit.
            assert_eq!(scratch.computed_count(), 0, "{tag:?} must mint nothing");
            assert_eq!(scratch.minted_bytes(), 0, "{tag:?} must charge nothing");
        }
    }

    /// A triple term is judged through its components: an ungrammatical tag one
    /// level down is still a tag that would reach a writer.
    #[test]
    fn the_gate_recurses_through_triple_terms() {
        let ds = dataset_with_one_iri();
        let quoted = |tag: &str| TermValue::Triple {
            s: TermBox::new(TermValue::Iri("https://example.org/s".to_owned())),
            p: TermBox::new(TermValue::Iri("https://example.org/p".to_owned())),
            o: TermBox::new(TermValue::Literal {
                lexical_form: "x".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some(tag.to_owned()),
                direction: None,
            }),
        };
        let mut scratch = ScratchInterner::default();
        assert!(scratch.intern_checked(&ds, quoted("en-US")).is_some());
        assert!(scratch.intern_checked(&ds, quoted("en us")).is_none());
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The whole-term walks of this module — the byte proxy, the language-tag check
    //! and the id-to-value resolution — checked against recursive references over
    //! generated shapes, and the first two over a term a hundred thousand levels deep
    //! on a thread with a 128 KiB stack.
    //!
    //! The resolution has no deep case: a dataset's triple terms nest a bounded number
    //! of levels, so the deepest term it can hand back is far shallower than the
    //! generated shapes already cover.

    use super::{LANGTAG_PROFILE, language_tags_well_formed, term_id_to_value, value_bytes};
    use purrdf_core::{
        BlankScope, RdfDataset, RdfDatasetBuilder, TermBox, TermFactory as _, TermId, TermRef,
        TermValue,
    };
    use std::sync::Arc;

    const EX: &str = "http://example.org/";
    use purrdf_iri::vocab::rdf::LANG_STRING as RDF_LANG_STRING;
    use purrdf_xsd::datatype::XSD_INTEGER;
    use purrdf_xsd::datatype::XSD_STRING;
    const DEPTH: usize = 100_000;
    const SMALL_STACK: usize = 128 * 1024;

    /// A deterministic choice sequence.
    struct Choices {
        state: purrdf_testkit::rng::SplitMix64,
    }

    impl Choices {
        const fn new(seed: u64) -> Self {
            Self {
                state: purrdf_testkit::rng::SplitMix64::new(seed),
            }
        }

        /// One choice below `n`.
        fn choose(&mut self, n: usize) -> usize {
            self.state.below_usize(n)
        }
    }

    fn typed(lexical: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        }
    }

    fn tagged(lexical: &str, tag: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some(tag.to_owned()),
            direction: None,
        }
    }

    /// A generated value of every kind — tags well-formed and not — with triple terms
    /// nested while `budget` lasts.
    fn value(choices: &mut Choices, budget: &mut usize) -> TermValue {
        match choices.choose(if *budget > 0 { 5 } else { 4 }) {
            0 => TermValue::Iri(format!("{EX}i{}", choices.choose(3))),
            1 => TermValue::Blank {
                label: ["a", "bb"][choices.choose(2)].to_owned(),
                scope: BlankScope::DEFAULT,
            },
            2 => typed(["x", "12"][choices.choose(2)], XSD_STRING),
            3 => tagged("x", ["en", "en-US", "en-", "de"][choices.choose(4)]),
            _ => {
                *budget -= 1;
                TermValue::Triple {
                    s: TermBox::new(value(choices, budget)),
                    p: TermBox::new(value(choices, budget)),
                    o: TermBox::new(value(choices, budget)),
                }
            }
        }
    }

    /// A generated value a dataset admits: IRIs, blank nodes and typed literals, and
    /// triple terms whose subject is an IRI or a blank node and whose predicate is an
    /// IRI, nested through the object while `budget` lasts.
    fn admissible(choices: &mut Choices, budget: &mut usize) -> TermValue {
        match choices.choose(if *budget > 0 { 5 } else { 4 }) {
            0 => TermValue::Iri(format!("{EX}i{}", choices.choose(3))),
            1 => TermValue::Blank {
                label: ["a", "bb"][choices.choose(2)].to_owned(),
                scope: BlankScope::DEFAULT,
            },
            2 => typed(["x", "y"][choices.choose(2)], XSD_STRING),
            3 => typed(["1", "2"][choices.choose(2)], XSD_INTEGER),
            _ => {
                *budget -= 1;
                let s = if choices.choose(2) == 0 {
                    TermValue::Iri(format!("{EX}s"))
                } else {
                    TermValue::Blank {
                        label: "s".to_owned(),
                        scope: BlankScope::DEFAULT,
                    }
                };
                TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(TermValue::Iri(format!("{EX}p"))),
                    o: TermBox::new(admissible(choices, budget)),
                }
            }
        }
    }

    /// The recursive reference for [`value_bytes`].
    fn bytes_reference(value: &TermValue) -> u64 {
        let payload = match value {
            TermValue::Iri(iri) => iri.len() as u64,
            TermValue::Blank { label, .. } => label.len() as u64,
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                ..
            } => {
                (lexical_form.len() + datatype.len() + language.as_ref().map_or(0, String::len))
                    as u64
            }
            TermValue::Triple { s, p, o } => bytes_reference(s)
                .saturating_add(bytes_reference(p))
                .saturating_add(bytes_reference(o)),
        };
        payload.saturating_add(32)
    }

    /// The recursive reference for [`language_tags_well_formed`].
    fn tags_reference(value: &TermValue) -> bool {
        match value {
            TermValue::Iri(_) | TermValue::Blank { .. } => true,
            TermValue::Literal { language, .. } => language
                .as_deref()
                .is_none_or(|tag| purrdf_iri::langtag::is_well_formed_with(tag, LANGTAG_PROFILE)),
            TermValue::Triple { s, p, o } => {
                tags_reference(s) && tags_reference(p) && tags_reference(o)
            }
        }
    }

    /// The recursive reference for [`term_id_to_value`].
    fn term_id_to_value_by_recursion(dataset: &RdfDataset, id: TermId) -> TermValue {
        match dataset.resolve(id) {
            TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
            TermRef::Blank { label, scope } => TermValue::Blank {
                label: label.to_owned(),
                scope,
            },
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let TermRef::Iri(datatype) = dataset.resolve(datatype) else {
                    panic!("a literal's datatype is an IRI");
                };
                TermValue::Literal {
                    lexical_form: lexical.to_owned(),
                    datatype: datatype.to_owned(),
                    language: language.map(str::to_owned),
                    direction,
                }
            }
            TermRef::Triple { s, p, o } => TermValue::Triple {
                s: TermBox::new(term_id_to_value_by_recursion(dataset, s)),
                p: TermBox::new(term_id_to_value_by_recursion(dataset, p)),
                o: TermBox::new(term_id_to_value_by_recursion(dataset, o)),
            },
        }
    }

    /// A dataset holding `values`, each as the object of a `:s :p` quad, and their ids.
    fn dataset_of(values: &[TermValue]) -> (Arc<RdfDataset>, Vec<TermId>) {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri(&format!("{EX}s"));
        let p = builder.intern_iri(&format!("{EX}p"));
        let ids: Vec<TermId> = values
            .iter()
            .map(|value| {
                let id = builder.intern_value(value);
                builder.push_quad(s, p, id, None);
                id
            })
            .collect();
        (builder.freeze().expect("the generated values freeze"), ids)
    }

    /// A triple-term chain `depth` levels deep over `innermost`.
    fn chain(depth: usize, innermost: TermValue) -> TermValue {
        let mut term = innermost;
        for _ in 0..depth {
            term = TermValue::Triple {
                s: TermBox::new(TermValue::Iri(format!("{EX}s"))),
                p: TermBox::new(TermValue::Iri(format!("{EX}p"))),
                o: TermBox::new(term),
            };
        }
        term
    }

    #[test]
    fn the_byte_proxy_and_the_tag_check_agree_with_the_recursive_references() {
        for seed in 0..200_u64 {
            let mut choices = Choices::new(seed);
            let mut budget = 6;
            let term = value(&mut choices, &mut budget);
            assert_eq!(value_bytes(&term), bytes_reference(&term), "seed {seed}");
            assert_eq!(
                language_tags_well_formed(&term),
                tags_reference(&term),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn the_resolution_agrees_with_the_recursive_reference_and_round_trips() {
        let mut choices = Choices::new(11);
        let values: Vec<TermValue> = (0..200)
            .map(|_| {
                let mut budget = 6;
                admissible(&mut choices, &mut budget)
            })
            .collect();
        let (dataset, ids) = dataset_of(&values);
        for (value, id) in values.iter().zip(ids) {
            let resolved = term_id_to_value(&*dataset, id);
            assert_eq!(resolved, term_id_to_value_by_recursion(&dataset, id));
            assert_eq!(resolved, *value);
        }
    }

    #[test]
    fn a_hundred_thousand_level_term_is_measured_and_checked_on_a_128_kib_stack() {
        purrdf_stack::on_stack(SMALL_STACK, || {
            let well_formed = chain(DEPTH, tagged("x", "en"));
            let s_len = format!("{EX}s").len() as u64;
            let p_len = format!("{EX}p").len() as u64;
            let level = (s_len + 32) + (p_len + 32) + 32;
            let leaf = (1 + RDF_LANG_STRING.len() + 2) as u64 + 32;
            let expected = leaf + level * DEPTH as u64;
            assert_eq!(value_bytes(&well_formed), expected);
            assert!(language_tags_well_formed(&well_formed));

            let ill_formed = chain(DEPTH, tagged("x", "en-"));
            assert!(!language_tags_well_formed(&ill_formed));
        })
        .expect("spawn");
    }
}
