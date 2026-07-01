// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Factored claim-modality projection (#769, ME3 of #766).
//!
//! The closed 5-value `purrdf:StandpointModality` vocabulary fuses four separable
//! epistemic notions onto one axis. The standpoint slice now decomposes it into
//! six orthogonal canonical axes ([`StandpointAxes`]); this module is the
//! Rust-native lossless projection between the six-axis canonical form and the
//! retained legacy five values.
//!
//! # The two directions
//!
//! * [`expand_up`] — a legacy value → its canonical six axes. Total over the
//!   five; an unknown string is a LOUD [`UnknownLegacyModality`] error, never a
//!   silent `None` a caller could default away (symmetry with [`project_down`]).
//! * [`project_down`] — a six-axis tuple → the one legacy value it projects to,
//!   or a HARD [`ProjectionVerdict::Unsupported`]. A tuple with no legacy twin
//!   (e.g. `supportBoth`, `modalForceCounterfactual`, `assertoricRetract`,
//!   `truthStrategic`) is NEVER silently approximated to a nearby value — the
//!   `logic-compile`'s `compat.rs` "never silently approximate" doctrine (#767), transplanted to
//!   the purrdf: domain (Principle 9).
//!
//! # Single source of truth
//!
//! [`DECOMPOSITIONS`] is the one canonical table. It is cross-checked against the
//! committed `purrdf:decomposesToAxis` bundles in
//! `slices/core/standpoint/module.ttl` by `rust_table_matches_committed_turtle_bundle`,
//! so the Rust table and the Turtle declaration cannot silently diverge.

/// Determinism hinge for [`StandpointAxes::content_key`] — an ASCII unit
/// separator that cannot occur in a local name (mirrors `logic-compile`'s `ir.rs` `SEP`).
const SEP: &str = "\u{1f}";

/// The canonical six-axis decomposition of a standpoint modality (#769).
///
/// Axis values are local names (e.g. `"polarityDeny"`), not enums, so a new value
/// individual added to `module.ttl` joins the vocabulary without a Rust change —
/// the open-vocabulary discipline of `logic-compile`'s `ReasoningContract` IR. The `credence`
/// field carries the qualitative `purrdf:CredenceLevel` band (the projection key),
/// not the numeric `purrdf:claimCredence` decimal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandpointAxes {
    /// `purrdf:Polarity` value local name (`polarityAffirm` / `polarityDeny` / `polaritySuspend`).
    pub polarity: String,
    /// `purrdf:ModalForce` value local name.
    pub modal_force: String,
    /// `purrdf:CredenceLevel` qualitative band local name (the round-trip key).
    pub credence: String,
    /// `purrdf:AssertoricForce` value local name.
    pub assertoric_force: String,
    /// `purrdf:TruthDirectedness` value local name.
    pub truth_directedness: String,
    /// `purrdf:SupportStatus` value local name.
    pub support_status: String,
}

impl StandpointAxes {
    /// A deterministic content key over a FIXED field order — the projection's
    /// stable identity (mirrors `logic-compile`'s `ReasoningContract::content_key`).
    #[must_use]
    pub fn content_key(&self) -> String {
        // FIXED field order — do not reorder (it pins the key).
        [
            self.polarity.as_str(),
            self.modal_force.as_str(),
            self.credence.as_str(),
            self.assertoric_force.as_str(),
            self.truth_directedness.as_str(),
            self.support_status.as_str(),
        ]
        .join(SEP)
    }

    /// The six axis values as a set of local names — used by the committed-Turtle
    /// cross-check, where the `purrdf:decomposesToAxis` bundle is unordered. Local
    /// names are globally unique per axis, so set equality ⟺ field-wise equality.
    #[must_use]
    pub fn value_set(&self) -> std::collections::BTreeSet<String> {
        [
            &self.polarity,
            &self.modal_force,
            &self.credence,
            &self.assertoric_force,
            &self.truth_directedness,
            &self.support_status,
        ]
        .into_iter()
        .cloned()
        .collect()
    }
}

/// One row of the canonical decomposition table.
pub(crate) struct Decomposition {
    /// The legacy `purrdf:StandpointModality` value local name.
    pub(crate) legacy: &'static str,
    pub(crate) polarity: &'static str,
    pub(crate) modal_force: &'static str,
    pub(crate) credence: &'static str,
    pub(crate) assertoric_force: &'static str,
    pub(crate) truth_directedness: &'static str,
    pub(crate) support_status: &'static str,
}

impl Decomposition {
    fn axes(&self) -> StandpointAxes {
        StandpointAxes {
            polarity: self.polarity.to_owned(),
            modal_force: self.modal_force.to_owned(),
            credence: self.credence.to_owned(),
            assertoric_force: self.assertoric_force.to_owned(),
            truth_directedness: self.truth_directedness.to_owned(),
            support_status: self.support_status.to_owned(),
        }
    }
}

/// The single canonical decomposition of each of the five legacy
/// `purrdf:StandpointModality` values — mirrored byte-for-byte by the
/// `purrdf:decomposesToAxis` bundles in `slices/core/standpoint/module.ttl`
/// (cross-checked by `rust_table_matches_committed_turtle_bundle`).
///
/// Semantic anchors: `bullshit` is `truthIndifferent` (Frankfurt indifference,
/// NOT a truth value), `refuted` is `polarityDeny` + `supportOpposed`
/// (frame-relative denial, never global-false — Principle 9).
const DECOMPOSITIONS: &[Decomposition] = &[
    Decomposition {
        legacy: "unequivocal",
        polarity: "polarityAffirm",
        modal_force: "modalForceNecessary",
        credence: "credenceCertain",
        assertoric_force: "assertoricAssert",
        truth_directedness: "truthAimed",
        support_status: "supportSupported",
    },
    Decomposition {
        legacy: "probable",
        polarity: "polarityAffirm",
        modal_force: "modalForceActual",
        credence: "credenceLikely",
        assertoric_force: "assertoricConjecture",
        truth_directedness: "truthAimed",
        support_status: "supportSupported",
    },
    Decomposition {
        legacy: "conceivable",
        polarity: "polaritySuspend",
        modal_force: "modalForcePossible",
        credence: "credencePossible",
        assertoric_force: "assertoricAssume",
        truth_directedness: "truthAimed",
        support_status: "supportNeither",
    },
    Decomposition {
        legacy: "refuted",
        polarity: "polarityDeny",
        modal_force: "modalForceNecessary",
        credence: "credenceCertain",
        assertoric_force: "assertoricAssert",
        truth_directedness: "truthAimed",
        support_status: "supportOpposed",
    },
    Decomposition {
        legacy: "bullshit",
        polarity: "polarityAffirm",
        modal_force: "modalForceActual",
        credence: "credenceUnspecified",
        assertoric_force: "assertoricAssert",
        truth_directedness: "truthIndifferent",
        support_status: "supportNeither",
    },
];

/// Returns the full canonical decomposition table — a crate-internal view used
/// by the standpoint-modality SPARQL emitter to build the projection query
/// without hard-coding the five rows a second time.
pub(crate) fn decompositions() -> &'static [Decomposition] {
    DECOMPOSITIONS
}

/// An unknown legacy modality string handed to [`expand_up`] — a hard error, so a
/// caller can never default a missing expansion away (Principle 9, no silent
/// approximation). Carries the offending local name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLegacyModality(pub String);

impl std::fmt::Display for UnknownLegacyModality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[UNKNOWN_LEGACY_MODALITY] {:?} is not one of the five legacy \
             purrdf:StandpointModality values (unequivocal, probable, conceivable, \
             refuted, bullshit); it has no canonical six-axis expansion",
            self.0
        )
    }
}

impl std::error::Error for UnknownLegacyModality {}

/// Expand a legacy `purrdf:StandpointModality` value (by local name) to its
/// canonical six axes. Total over the five legacy values.
///
/// # Errors
///
/// Returns [`UnknownLegacyModality`] for any string that is not one of the five
/// legacy values — a LOUD failure, never a silent `None` (Principle 9).
pub fn expand_up(legacy: &str) -> Result<StandpointAxes, UnknownLegacyModality> {
    DECOMPOSITIONS
        .iter()
        .find(|d| d.legacy == legacy)
        .map(Decomposition::axes)
        .ok_or_else(|| UnknownLegacyModality(legacy.to_owned()))
}

/// The verdict of projecting a six-axis tuple DOWN to a legacy value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionVerdict {
    /// The tuple matches exactly one legacy value's canonical decomposition; the
    /// carried string is that legacy local name.
    Legacy(&'static str),
    /// The tuple has no legacy equivalent — a HARD condition; the carried string
    /// is the human-readable reason. NEVER approximated to a nearby legacy value.
    Unsupported(String),
}

impl ProjectionVerdict {
    /// `true` iff the verdict is [`ProjectionVerdict::Legacy`].
    #[must_use]
    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy(_))
    }
}

/// Project a six-axis tuple DOWN to the one legacy `purrdf:StandpointModality`
/// value it represents, or a HARD [`ProjectionVerdict::Unsupported`].
///
/// EXACT match against [`DECOMPOSITIONS`]; any tuple with no legacy twin (e.g.
/// `supportBoth`, `modalForceCounterfactual`, `assertoricRetract`,
/// `truthStrategic`) returns `Unsupported` — it is never silently approximated to
/// a nearby legacy value (`logic-compile`'s `compat.rs` hard-verdict doctrine, Principle 9).
#[must_use]
pub fn project_down(axes: &StandpointAxes) -> ProjectionVerdict {
    for d in DECOMPOSITIONS {
        if d.polarity == axes.polarity
            && d.modal_force == axes.modal_force
            && d.credence == axes.credence
            && d.assertoric_force == axes.assertoric_force
            && d.truth_directedness == axes.truth_directedness
            && d.support_status == axes.support_status
        {
            return ProjectionVerdict::Legacy(d.legacy);
        }
    }
    ProjectionVerdict::Unsupported(format!(
        "[UNSUPPORTED_MODALITY_PROJECTION] six-axis tuple {} has no legacy \
         purrdf:StandpointModality equivalent; it is never silently approximated \
         to a nearby value (Principle 9 / never-approximate doctrine)",
        axes.content_key()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    /// The PURRDF vocabulary namespace — used only to build full IRIs when the
    /// committed-Turtle cross-check reads `module.ttl` (production code stores
    /// bare local names, the open-vocabulary discipline).
    const PURRDF: &str = "https://blackcatinformatics.ca/purrdf/";

    /// The five legacy values, for exhaustive iteration in tests.
    const LEGACY: [&str; 5] = [
        "unequivocal",
        "probable",
        "conceivable",
        "refuted",
        "bullshit",
    ];

    /// The repo root, anchored at this crate's manifest dir (`crates/slice/../..`).
    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root (crates/slice/../..) must exist")
    }

    #[test]
    fn round_trip_identity_holds_for_all_five() {
        // The lossless-projection acceptance criterion: legacy → six axes → legacy
        // is the identity for every seeded value.
        for legacy in LEGACY {
            let axes = expand_up(legacy).expect("legacy value expands");
            assert_eq!(
                project_down(&axes),
                ProjectionVerdict::Legacy(
                    DECOMPOSITIONS
                        .iter()
                        .find(|d| d.legacy == legacy)
                        .unwrap()
                        .legacy
                ),
                "round-trip identity must hold for {legacy}"
            );
        }
    }

    #[test]
    fn rust_table_matches_committed_turtle_bundle() {
        // The no-silent-divergence anchor: the Rust DECOMPOSITIONS table MUST equal
        // the purrdf:decomposesToAxis bundles committed in module.ttl. Parse the
        // committed Turtle and compare the unordered six-value set per legacy value
        // (local names are globally unique per axis, so set equality ⟺ field-wise).
        use crate::rdf_query::{Dataset, Object};

        let module = repo_root().join("slices/core/standpoint/module.ttl");
        if !module.exists() {
            eprintln!(
                "skipping committed standpoint-modality comparison; {} is absent",
                module.display()
            );
            return;
        }
        let bytes =
            std::fs::read(&module).unwrap_or_else(|e| panic!("read {}: {e}", module.display()));
        let ds = Dataset::parse_turtle(&bytes, &module.display().to_string())
            .expect("module.ttl parses");

        let decomposes = format!("{PURRDF}decomposesToAxis");
        for d in DECOMPOSITIONS {
            let subject = format!("{PURRDF}{}", d.legacy);
            let committed: BTreeSet<String> = ds
                .objects(&subject, &decomposes)
                .unwrap()
                .into_iter()
                .filter_map(|o| match o {
                    Object::Named(n) => n.strip_prefix(PURRDF).map(str::to_owned),
                    _ => None,
                })
                .collect();
            assert_eq!(
                committed,
                d.axes().value_set(),
                "purrdf:decomposesToAxis bundle for purrdf:{} in module.ttl must equal \
                 the Rust DECOMPOSITIONS row",
                d.legacy
            );
        }
    }

    #[test]
    fn no_legacy_combos_are_unsupported_not_approximated() {
        // Each of these single-axis deviations from a legacy tuple has no legacy
        // equivalent and MUST hard-fail rather than snap to a nearby value.
        let cases = [
            // supportBoth (Belnap glut) — admissible to author, no legacy twin.
            StandpointAxes {
                polarity: "polarityAffirm".into(),
                modal_force: "modalForceNecessary".into(),
                credence: "credenceCertain".into(),
                assertoric_force: "assertoricAssert".into(),
                truth_directedness: "truthAimed".into(),
                support_status: "supportBoth".into(),
            },
            // modalForceCounterfactual.
            StandpointAxes {
                polarity: "polarityAffirm".into(),
                modal_force: "modalForceCounterfactual".into(),
                credence: "credenceCertain".into(),
                assertoric_force: "assertoricAssert".into(),
                truth_directedness: "truthAimed".into(),
                support_status: "supportSupported".into(),
            },
            // assertoricRetract.
            StandpointAxes {
                polarity: "polarityAffirm".into(),
                modal_force: "modalForceNecessary".into(),
                credence: "credenceCertain".into(),
                assertoric_force: "assertoricRetract".into(),
                truth_directedness: "truthAimed".into(),
                support_status: "supportSupported".into(),
            },
            // truthStrategic.
            StandpointAxes {
                polarity: "polarityAffirm".into(),
                modal_force: "modalForceActual".into(),
                credence: "credenceLikely".into(),
                assertoric_force: "assertoricConjecture".into(),
                truth_directedness: "truthStrategic".into(),
                support_status: "supportSupported".into(),
            },
        ];
        for axes in cases {
            match project_down(&axes) {
                ProjectionVerdict::Unsupported(reason) => {
                    assert!(
                        reason.contains("UNSUPPORTED_MODALITY_PROJECTION"),
                        "unsupported reason must carry the rule tag, got: {reason}"
                    );
                }
                ProjectionVerdict::Legacy(l) => {
                    panic!(
                        "tuple {} must be Unsupported, not approximated to {l}",
                        axes.content_key()
                    )
                }
            }
        }
    }

    #[test]
    fn bullshit_is_indifferent_not_a_truth_value() {
        // Semantic-preservation guard: bullshit = truthDirectedness indifferent,
        // with polarity still affirm (NOT deny, NOT a truth value).
        let axes = expand_up("bullshit").unwrap();
        assert_eq!(axes.truth_directedness, "truthIndifferent");
        assert_eq!(axes.polarity, "polarityAffirm");
        assert_eq!(axes.credence, "credenceUnspecified");
    }

    #[test]
    fn refuted_is_deny_not_global_false() {
        // Semantic-preservation guard: refuted = polarity deny + support opposed,
        // a frame-relative denial, never a global-false verdict (Principle 9).
        let axes = expand_up("refuted").unwrap();
        assert_eq!(axes.polarity, "polarityDeny");
        assert_eq!(axes.support_status, "supportOpposed");
    }

    #[test]
    fn unknown_legacy_is_a_loud_error_not_silent_none() {
        let err = expand_up("plausible").unwrap_err();
        assert_eq!(err, UnknownLegacyModality("plausible".to_owned()));
        assert!(err.to_string().contains("UNKNOWN_LEGACY_MODALITY"));
    }

    #[test]
    fn content_key_is_order_stable_and_distinguishing() {
        let a = expand_up("unequivocal").unwrap();
        let b = expand_up("refuted").unwrap();
        assert_eq!(a.content_key(), a.content_key());
        assert_ne!(a.content_key(), b.content_key());
    }
}
