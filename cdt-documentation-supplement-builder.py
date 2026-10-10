# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: ignored Stage-only proposal assembly reads the integrated source and writes no shipping paths or build outputs.
from pathlib import Path
import difflib
import re

stage = Path('.stage/sparql-eval-complete-bounded-workspace')
exec((stage / 'native-text-owner-assembly.py').read_text().split('selected = [', 1)[0])
paths = [
    'crates/lex/src/allocation.rs', 'crates/cdt/src/lib.rs', 'crates/cdt/src/tree.rs',
    'crates/cdt/src/term.rs', 'crates/cdt/src/value.rs', 'crates/iri/src/parse.rs',
]
base = {p: Path(p).read_text() for p in paths}
post = dict(base)

p = post[paths[0]]
docs = {
    'new': '''    /// Start one native computation with no owned heap capacity.
    /// All nonempty buffers passed to this memory must have been allocated through
    /// it, or explicitly included with `add_bytes` under the same original grant.
    #[must_use]
''',
    'live_bytes': '''    /// The currently admitted requested layouts of payload and working buffers.
    /// Read the surviving total only after construction scratch has been released.
    #[must_use]
''',
    'admission_mut': '''    /// Borrow the original admission for a native payload factory above this leaf.
''',
    'add_bytes': '''    /// Admit a native payload's concrete layout before calling its fallible factory.
    ///
    /// # Errors
    /// Returns checked live-byte overflow or the caller's original refusal marker.
''',
    'release_bytes': '''    /// Release a concrete layout only after destroying its original allocation.
    ///
    /// # Errors
    /// Returns an unbalanced live-byte invariant or the caller's refusal marker.
''',
    'reserve': '''    /// Replace a vector buffer while both old and new layouts remain admitted.
    /// The old contents survive admission or allocator refusal unchanged. The buffer
    /// must already belong to this memory's live total if it is nonempty.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
''',
    'push': '''    /// Append one value after admitting and fallibly allocating any buffer growth.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
''',
    'release_vec': '''    /// Destroy a vector before releasing its original buffer's admission.
    /// Independently priced owned element payloads require their matching release;
    /// this method releases only the vector's concrete backing array layout.
    ///
    /// # Errors
    /// Returns layout overflow, an unbalanced live-byte invariant or refusal.
''',
    'release_string': '''    /// Destroy owned UTF-8 storage before releasing its original buffer admission.
    ///
    /// # Errors
    /// Returns an unbalanced live-byte invariant or the caller's refusal marker.
''',
    'reserve_string': '''    /// Replace UTF-8 storage while old and new allocations both remain admitted.
    /// The old text survives admission or allocator refusal unchanged.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
''',
    'push_char': '''    /// Append one scalar after admitting any physical UTF-8 buffer growth.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
''',
    'string': '''    /// Copy borrowed text into an admitted, fallibly allocated UTF-8 destination.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
''',
}
for name, doc in docs.items():
    start, end = fn_span(p, name)
    begin = start
    # Replace just existing contiguous docs/attributes; retain the source body.
    while begin > 0:
        previous_end = begin - 1
        previous_start = p.rfind('\n', 0, previous_end) + 1
        line = p[previous_start:previous_end].strip()
        if line.startswith('///') or line.startswith('#[must_use]'):
            begin = previous_start
        else: break
    p = p[:begin] + doc + p[start:]
post[paths[0]] = p

p = post[paths[1]]
p = p.replace('All whole-tree operations — scan, render, `==`, ordering, `Clone`, `Debug` and\n//! `Drop` — are iterative over explicit heap work lists.',
'''Scan, render, `==`, ordering, `Clone` and `Debug` use explicit work lists.
//! `Drop` instead reuses evacuated slots in existing boxes, so it needs neither
//! an allocation nor stack proportional to nesting.''')
post[paths[1]] = p
p = post[paths[2]]
p = p.replace('//! The owning value tree\'s `Drop`, `Clone` and `Debug`, each over an explicit heap\n//! work list rather than the compiler\'s recursive glue.',
'''//! The owning value tree's iterative `Drop`, `Clone` and `Debug`.
//! Destruction threads ancestors through existing slots without allocating;
//! copying and debugging use explicit work lists instead of recursive glue.''')
p = p.replace('//! would be. What bounds it', '//! would be. What bounds it')
p = p.replace('//! would be checking for the very thing it caused', '//! would be checking for the very thing it caused')
p = p.replace('//! whole tree is a loop over a heap stack:',
'''//! whole tree is iterative; destruction reuses its existing slots and the other
//! walks use a heap work list:''')
post[paths[2]] = p
p = post[paths[3]]
p = p.replace('/// implements `Drop` iteratively ([`crate::tree`]);', '/// implements allocation-free iterative `Drop` ([`crate::tree`]);')
p = p.replace('/// heap work list, and equality and ordering are the loops in [`crate::ops`].',
'''/// work lists for copying/debugging and existing evacuated slots for destruction;
/// equality and ordering are the loops in [`crate::ops`].''')
post[paths[3]] = p
p = post[paths[4]]
p = p.replace('/// tree\'s own `Drop`, `Clone` and `Debug` — is iterative over a heap worklist, so a',
'''/// tree's own `Clone` and `Debug` — is iterative over a heap worklist, so a''')
p = p.replace('/// level costs heap and never stack (see [`crate::tree`]).',
'''/// level costs heap and never stack (see [`crate::tree`]). Destruction reuses
/// the existing tree slots and needs neither a worklist allocation nor more stack.''')
post[paths[4]] = p
p = post[paths[5]]
p = p.replace('/// Absolute/relative/invalid verdict through the same scanner, without owning\n/// the parsed text or rendering an error which this caller will discard.',
'''/// The same scanner's allocation-free absolute/relative/invalid verdict.
/// `Some(true)` is absolute, `Some(false)` is a valid relative reference, and
/// `None` is malformed. Neither parsed text nor a discarded diagnostic is owned;
/// public diagnostic-producing parse APIs retain their original messages.''')
post[paths[5]] = p

chunks = []
for p in paths:
    if base[p] != post[p]:
        chunks.append('diff --git a/' + p + ' b/' + p + '\n')
        chunks.extend(difflib.unified_diff(base[p].splitlines(True), post[p].splitlines(True),
            fromfile='a/' + p, tofile='b/' + p, n=3))
(stage / 'native-cdt-documentation-supplement.patch').write_text(''.join(chunks))
(stage / 'lex-allocation-draft.rs').write_text(post[paths[0]])
print('Stage documentation supplement:', sum(base[p] != post[p] for p in paths), 'homes; no source mutation/compiler/tests')
