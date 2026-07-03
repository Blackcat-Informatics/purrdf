// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `IMPORT` resolution (ShEx 2.1 spec §11 / ShExJ `imports`).
//!
//! A schema may `IMPORT <iri>` other schemas; the imported schemas contribute
//! their labeled shape declarations to the importing schema. [`resolve_imports`]
//! walks the transitive closure of imports and folds every imported shape
//! declaration into a single flat [`Schema`] whose `imports` list is empty.
//!
//! * **Injected, no ambient I/O.** The bytes behind an import IRI are fetched
//!   by a caller-supplied [`ImportResolver`] returning an already-parsed
//!   [`Schema`]. The library never touches the filesystem or network, so the
//!   crate stays `wasm32`-clean. Because the resolver parses each imported
//!   document with that document's own IRI as base, per-document base-IRI
//!   resolution is satisfied at the injection boundary.
//! * **Cycle-tolerant.** Imports may form cycles (`A` imports `B` imports `A`)
//!   or a document may import itself; a visited-set on import IRIs bounds the
//!   walk to the finite set of distinct IRIs, and identical re-declarations are
//!   absorbed by dedup.
//! * **Root wins.** The importing schema's `start` and `startActs` are kept;
//!   imported schemas contribute only their labeled `shapes` (per the spec,
//!   `start` is not imported).
//! * **Merge policy.** A shape label may be declared once. A re-declaration
//!   that is byte-identical is deduplicated; a genuinely conflicting
//!   re-declaration is a hard [`ShexError::Import`] error.

use std::collections::{HashSet, VecDeque};

use crate::ast::{Schema, ShapeDecl};
use crate::error::{Result, ShexError};

/// A hook resolving an `IMPORT` IRI to its parsed [`Schema`].
///
/// Returning `None` means the import could not be resolved (unknown IRI,
/// unreadable document, parse failure) and surfaces as a
/// [`ShexError::Import`]. The resolver is expected to parse each document with
/// its own IRI as base so that relative IRIs resolve per-document.
pub type ImportResolver<'a> = dyn Fn(&str) -> Option<Schema> + 'a;

/// Flatten `root` and its transitive imports into a single import-free schema.
///
/// The returned schema keeps `root`'s `start` and `start_acts`, gathers the
/// labeled shape declarations of every transitively imported schema (root
/// first, then imports in breadth-first document order), and has an empty
/// `imports` list.
///
/// # Errors
///
/// Returns [`ShexError::Import`] when the resolver cannot supply an imported
/// IRI, or when two schemas declare the same shape label with different
/// definitions.
pub fn resolve_imports(root: Schema, resolver: &ImportResolver<'_>) -> Result<Schema> {
    let mut merged = Schema {
        imports: Vec::new(),
        start_acts: root.start_acts,
        start: root.start,
        shapes: Vec::new(),
    };
    // Track shapes already merged so identical re-declarations dedup and
    // conflicting ones hard-fail.
    let mut seen: HashSet<String> = HashSet::new();
    for decl in root.shapes {
        merge_decl(&mut merged.shapes, &mut seen, decl)?;
    }

    // Breadth-first over import IRIs; `visited` bounds cycles/self-imports.
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = root.imports.into_iter().collect();
    while let Some(iri) = queue.pop_front() {
        if !visited.insert(iri.clone()) {
            continue;
        }
        let imported = resolver(&iri).ok_or_else(|| ShexError::import(iri))?;
        for nested in imported.imports {
            if !visited.contains(&nested) {
                queue.push_back(nested);
            }
        }
        for decl in imported.shapes {
            merge_decl(&mut merged.shapes, &mut seen, decl)?;
        }
    }
    Ok(merged)
}

/// Merge one declaration, deduplicating identical labels and rejecting
/// conflicting ones.
fn merge_decl(
    shapes: &mut Vec<ShapeDecl>,
    seen: &mut HashSet<String>,
    decl: ShapeDecl,
) -> Result<()> {
    if seen.contains(&decl.id) {
        // A label may recur only if the definitions are identical.
        if shapes.iter().any(|existing| existing == &decl) {
            return Ok(());
        }
        return Err(ShexError::import(format!(
            "conflicting redefinition of shape {}",
            decl.id
        )));
    }
    seen.insert(decl.id.clone());
    shapes.push(decl);
    Ok(())
}
