# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this ignored Stage-only analysis artifact assembles source proposals without invoking the forbidden compiler/build lane.
from pathlib import Path
import sys
import difflib
import re

stage = Path('.stage/sparql-eval-complete-bounded-workspace')
notes = (stage / 'native-text-owner-source-units.md').read_text()
parts = dict(re.findall(r'### ([a-z_]+)\n```rust\n(.*?)\n```', notes, re.S))
paths = ['crates/sparql-eval/src/error.rs', 'crates/sparql-eval/src/workspace.rs', 'crates/sparql-eval/src/lib.rs']
base = {path: Path(path).read_text() for path in paths}
post = dict(base)
error = post[paths[0]]
if '--text' in sys.argv:
    # Diagnostic proposal was delivered separately and its source is now
    # concurrently integrated. Never regenerate duplicate diagnostic hunks.
    exec((stage / 'native-text-owner-assembly.py').read_text())
    sys.exit(0)
error = error.replace('/// An error raised while evaluating a SPARQL query.', parts['native_diagnostic'] + '\n\n/// An error raised while evaluating a SPARQL query.', 1)
error = error.replace('pub enum EvalError {', 'pub enum EvalError {\n    /// An immutable native diagnostic retaining its exact allocation grant.\n    NativeDiagnostic(NativeDiagnostic),\n    /// A native formatter violated its certified stable output length.\n    UnstableNativeDiagnostic,', 1)
error = error.replace('Self::Function(message) => Self::FunctionOperational(message),', 'Self::Function(message) => Self::FunctionOperational(message),\n            Self::NativeDiagnostic(message) => Self::NativeDiagnostic(message.preserve_function_failure()),', 1)
marker = '        match self {'
at = error.index(marker, error.index('    pub fn diagnostic_code(&self)')) + len(marker)
routing = '\n            Self::NativeDiagnostic(message) => match message.kind() {\n                NativeDiagnosticKind::Internal => Some(Self::INTERNAL_CODE),\n                NativeDiagnosticKind::FunctionOperational => Some(Self::FUNCTION_OPERATIONAL_CODE),\n                NativeDiagnosticKind::Function | NativeDiagnosticKind::Data | NativeDiagnosticKind::Config => None,\n            },\n            Self::UnstableNativeDiagnostic => Some("native-sparql-unstable-native-diagnostic"),'
error = error[:at] + routing + error[at:]
at = error.index(marker, error.index('impl core::fmt::Display for EvalError')) + len(marker)
error = error[:at] + '\n            Self::NativeDiagnostic(message) => core::fmt::Display::fmt(message, f),\n            Self::UnstableNativeDiagnostic => f.write_str("native diagnostic formatter violated its certified output length"),' + error[at:]
post[paths[0]] = error
workspace = post[paths[1]]
at = workspace.index('pub(crate) fn render(')
workspace = workspace[:at] + parts['format_helpers'] + '\n\n' + workspace[at:]
at = workspace.index('pub(crate) fn render(')
workspace = workspace[:at] + '''pub(crate) fn render(
    workspace: &WorkspaceCapability, value: &EvalError, construct: &'static str,
) -> Result<(String, WorkspaceAllocation), EvalError> {
    let length = display_len(value)?;
    let admission = workspace.charge(u64::try_from(length).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
    let text = format_exact(value, length, construct)?;
    Ok((text, admission))
}
'''
post[paths[1]] = workspace
post[paths[2]] += '\n/// Immutable admission-carrying native producer diagnostics.\npub use error::{NativeDiagnostic, NativeDiagnosticKind};\n'

def patch_for(selected):
    return ''.join(''.join(difflib.unified_diff(base.get(path, '').splitlines(True), post[path].splitlines(True), fromfile='a/' + path, tofile='b/' + path)) for path in selected)

patch = patch_for(paths)
(stage / 'native-diagnostic-owner-draft.patch').write_text(patch)
header = '// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>\n// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0\n\n// Stage proposal; insert at sparql-eval/src/error.rs. Not applied or compiled.\n// Uses the one workspace display_len / format_exact home supplied by its patch.\n\n'
(stage / 'native-diagnostic-owner-draft.rs').write_text(header + parts['native_diagnostic'] + '\n')
print('native-diagnostic-owner-draft.patch: standard unified diff; 3 source homes; no shipping write')
