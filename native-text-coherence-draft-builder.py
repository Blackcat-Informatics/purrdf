# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this ignored Stage-only source-proposal assembler never invokes the shipping compiler or modifies source.
from pathlib import Path
import difflib
import re

stage = Path('.stage/sparql-eval-complete-bounded-workspace')
# Reuse the already-authored artifact splice helpers, without running its
# old whole-patch assembly against the concurrently integrated postimage.
exec((stage / 'native-text-owner-assembly.py').read_text().split('selected = [', 1)[0])
paths = [
    'crates/rdf-core/src/ir/term.rs', 'crates/text/src/relation.rs',
    'crates/text/src/index.rs', 'crates/text/src/score.rs',
    'crates/text/src/error.rs', 'crates/text/src/fixed.rs',
]
base = {path: Path(path).read_text() for path in paths}
post = dict(base)

# Describe the actual rejected field/type without recursively formatting a
# TermValue. This representation-only query belongs at the existing term home.
post[paths[0]] = insert_after_fn(post[paths[0]], 'is_blank', '''    /// A borrowed, allocation-free description of this RDF term's kind.
    /// Native diagnostics use it when the value's contents are unnecessary to
    /// explain a type refusal, avoiding recursive Debug's two spill work lists.
    #[must_use]
    pub const fn kind_description(&self) -> &'static str {
        match self {
            Self::Iri(_) => "an IRI",
            Self::Blank { .. } => "a blank node",
            Self::Literal { .. } => "a literal",
            Self::Triple { .. } => "a triple term",
        }
    }''')
relation = post[paths[1]]
relation = relation.replace('the needle at position {position} is {other:?};', 'the needle at position {position} is {};')
relation = relation.replace('the language at position {position} is {other:?};', 'the language at position {position} is {};')
relation = relation.replace('the rank at position {SEARCH_RANK} is {value:?};', 'the rank at position {SEARCH_RANK} is {};')
for name, value in [('needle_text_owned', 'other'), ('language_constraint_owned', 'other'), ('rank_bound_owned', 'value')]:
    body = fn_body(relation, name)
    # Exactly the type-refusal arm references this value. The literal datatype,
    # lexical and positive-domain diagnostics retain their existing borrowed data.
    empty_at = body.index('{};')
    closing = body.index('        ), workspace)', empty_at)
    body = body[:closing] + '            , ' + value + '.kind_description()\n' + body[closing:]
    relation = replace_fn(relation, name, body)

# Remove documentation orphaned by the old private functions' removal, and
# move the useful descriptions onto the corresponding actual owned functions.
orphan_start = relation.index('    /// Score each holding document in place, without ranking anything.', relation.index('    fn open_owned('))
orphan_end = relation.index('\n}', orphan_start)
relation = relation[:orphan_start] + relation[orphan_end:]
relation = relation.replace('    fn scored_in_place_owned(', '''    /// Score only held documents when rank is unobserved, through the same
    /// admitted summation the partition ranker uses.
    fn scored_in_place_owned(''', 1)
relation = relation.replace('    fn ranked_owned(', '''    /// The single partition-ranker entry; record actual work even on refusal.
    fn ranked_owned(''', 1)
relation = relation.replace('''/// Whether `row` agrees with every bound position of the invocation.


/// The invocation's bound values by flattened position, cloned out of the
/// borrow `args` lends for the duration of `open`.


/// A `?lang` cell: the tag itself, or the empty string for an untagged document.


''', '')
relation = relation.replace('''    /// Its partition.
    key: &'i PartitionKey,''', '''    /// Stable ordinal of its partition in the immutable caller index.
    partition: usize,''', 1)
relation = relation.replace('''            if !located.is_empty() { admitted(held.push(Holding { document, key, located }))?; }''', '''            if !located.is_empty() {
                let partition = self.index.partition_ordinal_of(document).ok_or_else(||
                    query_error(workspace, NativeDiagnosticKind::Internal,
                        format_args!("text index document {document} has no partition"))
                        .unwrap_or_else(|failure| failure))?;
                admitted(held.push(Holding { document, partition, located }))?;
            }''', 1)
relation = relation.replace('use crate::query_workspace::{QueryString, admitted};', 'use crate::query_workspace::{QueryString, admitted, query_error};', 1)
relation = relation.replace('keys.push(holding.key)?;', 'keys.push(holding.partition)?;', 1)
relation = relation.replace('keys.iter().any(|held| *held == key)', 'self.index.local_partition_ordinal(key).is_some_and(|ordinal| keys.contains(&ordinal))', 1)
relation = relation.replace('self.index.partition_key_of(document) == Some(key)', 'self.index.partition_ordinal_of(document) == Some(ordinal)', 1)
relation = relation.replace('[`distinct_terms`]', '[`distinct_terms_owned`]').replace('[`score_located`]', '[`score_located_owned`]')
post[paths[1]] = relation

index = post[paths[2]]
# A local borrow maps to its exact array slot in constant time. Address checks
# are used only to prove pointer identity; no address enters emitted ordering,
# identity or bytes. get+ptr::eq prevents an external object being misclassified.
index = insert_after_fn(index, 'partition_key_of', '''    /// The stored partition ordinal of one document, without comparing terms.
    pub(crate) fn partition_ordinal_of(&self, document: u32) -> Option<usize> {
        self.documents.get(document as usize).map(|document| document.partition as usize)
    }

    /// Resolve an actual index-owned key borrow to its stable ordinal.
    pub(crate) fn local_partition_ordinal(&self, partition: &PartitionKey) -> Option<usize> {
        let first = &self.partitions.first()?.0;
        let stride = core::mem::size_of::<(PartitionKey, PartitionStats)>();
        let distance = core::ptr::from_ref(partition).addr()
            .checked_sub(core::ptr::from_ref(first).addr())?;
        if distance.checked_rem(stride)? != 0 { return None; }
        let ordinal = distance.checked_div(stride)?;
        let candidate = &self.partitions.get(ordinal)?.0;
        core::ptr::eq(candidate, partition).then_some(ordinal)
    }

    /// Resolve an external key through fallible native graph-term comparison.
    /// A failed comparator's search result is discarded before publication.
    pub(crate) fn partition_key_owned(
        &self, partition: &PartitionKey, workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<&PartitionKey>, purrdf_sparql_eval::EvalError> {
        if let Some(ordinal) = self.local_partition_ordinal(partition) {
            return Ok(Some(&self.partitions[ordinal].0));
        }
        let mut failure = None;
        let result = self.partitions.binary_search_by(|(key, _)| {
            if failure.is_some() { return Ordering::Equal; }
            let graph = match (key.graph(), partition.graph()) {
                (None, None) => Ok(Ordering::Equal),
                (None, Some(_)) => Ok(Ordering::Less),
                (Some(_), None) => Ok(Ordering::Greater),
                (Some(left), Some(right)) => workspace.terms_cmp(left, right),
            };
            match graph {
                Ok(graph) => graph.then_with(|| key.language().cmp(&partition.language())),
                Err(error) => { failure = Some(error); Ordering::Equal }
            }
        });
        match failure {
            Some(error) => Err(error),
            None => Ok(result.ok().map(|ordinal| &self.partitions[ordinal].0)),
        }
    }''')
body = fn_body(index, 'partition_index')
body = body.replace('        self.partitions', '''        if let Some(ordinal) = self.local_partition_ordinal(partition) {
            return u32::try_from(ordinal).ok();
        }
        self.partitions''', 1)
index = replace_fn(index, 'partition_index', body)
body = fn_body(index, 'prepared_corpus_owned')
old = 'let at = self.partition_index(partition).ok_or_else(|| query_error(workspace, NativeDiagnosticKind::Data, "ranking partition is absent").unwrap_or_else(|failure| failure))? as usize;'
new = '''let partition = crate::query_workspace::admitted(self.partition_key_owned(partition, workspace))?
            .ok_or_else(|| query_error(workspace, NativeDiagnosticKind::Data,
                "ranking partition is absent").unwrap_or_else(|failure| failure))?;
        let at = self.local_partition_ordinal(partition).expect("resolved index-owned partition");'''
assert old in body
body = body.replace(old, new, 1)
index = replace_fn(index, 'prepared_corpus_owned', body)
post[paths[2]] = index

score = post[paths[3]]
body = fn_body(score, 'rank_terms_owned')
body = body.replace('    let candidates =', '''    let Some(partition) = admitted(index.partition_key_owned(partition, workspace))? else {
        return Ok(AdmittedVec::new(workspace));
    };
    let candidates =''', 1)
score = replace_fn(score, 'rank_terms_owned', body)
# Source search found exactly one score_located( occurrence: its definition.
score = replace_fn(score, 'score_located', '')
post[paths[3]] = score

error = post[paths[4]].replace('TextError::Html(_) | TextError::Substring(_) | TextError::Phonetic(_) => {',
    'TextError::Html(_) | TextError::Substring(_) | TextError::Phonetic(_)\n                | TextError::Capacity(_) | TextError::Diagnostic(_) => {', 1)
post[paths[4]] = error

# Decimal lexical bytes have one renderer. The resident API allocates its own
# result, while operational literal construction borrows this same Display.
post[paths[5]] = replace_fn(post[paths[5]], 'to_decimal_lexical', '''    pub fn to_decimal_lexical(self) -> String { self.decimal_display().to_string() }''')

patch = ''.join(''.join(difflib.unified_diff(
    base[path].splitlines(True), post[path].splitlines(True),
    fromfile='a/' + path, tofile='b/' + path,
)) for path in paths)
(stage / 'native-text-coherence-draft.patch').write_text(patch)
print('native-text-coherence-draft.patch: standard diff against shipping postimage; six homes; no source write')
