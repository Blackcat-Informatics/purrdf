// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The non-Rust ratchet: repository tooling and tests are Rust.
//!
//! Two rules compare a TARGET tree against a BASE tree, both read from git:
//!
//! 1. **Explain.** Every non-Rust path present in TARGET and absent from BASE
//!    carries, within its first [`EXPLANATION_WINDOW`] lines, a comment
//!    `# Why not Rust: <reason>` or `// Why not Rust: <reason>` (any case)
//!    whose trimmed reason has at least [`MIN_REASON_CHARS`] characters.
//!    `vectors/` and `generated/` are exempt ([`EXEMPT_FROM_EXPLAIN`]), and so
//!    is a file holding nothing but whitespace.
//! 2. **Ratchet.** Every path that exists in BASE under one of the
//!    [`RATCHETED_ROOTS`] may not hold more non-Rust lines in TARGET than in
//!    BASE.
//!
//! Paths are identities: there is no rename or copy detection anywhere, so a
//! committer's `diff.renames` setting cannot change a verdict, and a file moved
//! to another path is a new path that rule 1 applies to. Contents come straight
//! from blobs (`git cat-file --batch`) or, for the working tree, from disk, so a
//! `-diff` or `binary` attribute cannot hide growth. BASE is normally the
//! merge-base of the target with the integration branch, so deleting a legacy
//! file in one commit and re-adding it, grown, in a later commit on the same
//! branch is still compared against the integration branch's copy.
//!
//! A path is non-Rust code when its extension is one of [`CODE_EXTENSIONS`], or
//! when it has any other extension (or none) and its first line is a shebang
//! naming a Python or JavaScript runtime. A shell script (by extension or
//! shebang) is not non-Rust code itself, but the bodies of the heredocs it
//! feeds to a Python or JavaScript runtime (`python3 - <<'PY'` … `PY`) are its
//! non-Rust lines, under both rules.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitCode, Stdio};

/// Rule 1 looks for the explanation in this many leading lines.
pub(crate) const EXPLANATION_WINDOW: usize = 40;

/// The explanation's reason, trimmed, has at least this many characters.
pub(crate) const MIN_REASON_CHARS: usize = 30;

/// The marker that opens an explanation, compared without regard to case.
const MARKER: &str = "why not rust:";

/// Extensions that make a path non-Rust code whatever its contents.
pub(crate) const CODE_EXTENSIONS: [(&str, Language); 11] = [
    ("py", Language::Python),
    ("pyw", Language::Python),
    ("pyi", Language::Python),
    ("mjs", Language::JavaScript),
    ("js", Language::JavaScript),
    ("cjs", Language::JavaScript),
    ("ts", Language::JavaScript),
    ("mts", Language::JavaScript),
    ("cts", Language::JavaScript),
    ("tsx", Language::JavaScript),
    ("jsx", Language::JavaScript),
];

/// Extensions that make a path a shell script, scanned for embedded heredocs.
const SHELL_EXTENSIONS: [&str; 5] = ["sh", "bash", "zsh", "ksh", "dash"];

/// Shell interpreters a shebang may name.
const SHELLS: [&str; 7] = ["sh", "bash", "dash", "zsh", "ksh", "ash", "mksh"];

/// JavaScript runtimes a shebang or a heredoc consumer may name.
const JS_RUNTIMES: [&str; 4] = ["node", "nodejs", "deno", "bun"];

/// Path prefixes rule 1 does not apply to: frozen vendor corpora and generator
/// output, neither of which is hand-written.
pub(crate) const EXEMPT_FROM_EXPLAIN: [&str; 2] = ["vectors/", "generated/"];

/// Path prefixes rule 2 applies to. `crates/*/tests/` is matched separately in
/// [`is_ratcheted`]. Shipped surfaces (`crates/rdf-wasm/js/index.mjs`,
/// `index.d.ts`, `crates/rdf-wasm/js/src/`, `docs/playground/`, and the Python
/// package under `bindings/python/python/`) lie outside every root on purpose.
pub(crate) const RATCHETED_ROOTS: [&str; 4] = [
    "scripts/",
    "bindings/python/tests/",
    "crates/rdf-wasm/js/tests/",
    "crates/rdf-wasm/js/bench/",
];

/// A non-Rust language family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Python,
    JavaScript,
}

impl Language {
    const fn name(self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::JavaScript => "JavaScript/TypeScript",
        }
    }
}

/// What a path's contents are, for the two rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Non-Rust code: every line counts.
    Code(Language),
    /// A shell script: only the lines of heredocs fed to a Python or
    /// JavaScript runtime count.
    Shell,
    /// Anything else: no line counts.
    Other,
}

/// A path's classification and its non-Rust line count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Profile {
    pub(crate) kind: Kind,
    pub(crate) lines: usize,
}

impl Profile {
    /// Whether this path holds non-Rust code at all: a code file, or a shell
    /// script with at least one embedded line.
    fn has_non_rust_code(self) -> bool {
        matches!(self.kind, Kind::Code(_)) || self.lines > 0
    }

    /// Whether rule 1 asks this path for an explanation when it is new.
    fn needs_explanation(self, bytes: &[u8]) -> bool {
        match self.kind {
            Kind::Code(_) => bytes
                .iter()
                .any(|&byte| !purrdf_lex::terminals::is_ws(byte)),
            Kind::Shell => self.lines > 0,
            Kind::Other => false,
        }
    }

    fn describe(self) -> String {
        match self.kind {
            Kind::Code(language) => format!("{} file", language.name()),
            Kind::Shell => "shell script with embedded Python/JavaScript heredocs".to_owned(),
            Kind::Other => "file".to_owned(),
        }
    }

    const fn unit(self) -> &'static str {
        match self.kind {
            Kind::Shell => "embedded Python/JavaScript line(s)",
            Kind::Code(_) | Kind::Other => "line(s)",
        }
    }
}

/// The number of lines in `bytes`: newlines, plus one for an unterminated last
/// line.
pub(crate) fn line_count(bytes: &[u8]) -> usize {
    let mut newlines = 0;
    let mut rest = bytes;
    while let Some(at) = purrdf_lex::scan::find_byte(rest, b'\n') {
        newlines += 1;
        rest = &rest[at + 1..];
    }
    newlines + usize::from(!rest.is_empty())
}

/// The lower-cased extension of the path's file name, if it has one. A
/// dotfile's leading dot does not start an extension.
fn extension(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem_end = name.rfind('.')?;
    (stem_end > 0).then(|| name[stem_end + 1..].to_ascii_lowercase())
}

fn is_python(word: &str) -> bool {
    ["python", "pypy"].iter().any(|prefix| {
        word.strip_prefix(prefix).is_some_and(|rest| {
            rest.bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
    })
}

/// The language a program name runs, if it is a Python or JavaScript runtime.
fn runtime_language(program: &str) -> Option<Language> {
    let base = program.rsplit('/').next().unwrap_or(program);
    if is_python(base) {
        Some(Language::Python)
    } else if JS_RUNTIMES.contains(&base) {
        Some(Language::JavaScript)
    } else {
        None
    }
}

/// The interpreter a `#!` first line names, looking through `env`, its
/// options (`-S`, `-i`, and `-u NAME`/`-C DIR` with their arguments) and its
/// `VAR=value` assignments.
fn shebang_interpreter(bytes: &[u8]) -> Option<String> {
    let first = bytes.split(|&byte| byte == b'\n').next()?;
    let line = String::from_utf8_lossy(first.strip_prefix(b"#!")?);
    let mut words = line.split_whitespace();
    let program = words.next()?;
    let base = program.rsplit('/').next().unwrap_or(program);
    if base != "env" {
        return Some(base.to_owned());
    }
    while let Some(word) = words.next() {
        if matches!(word, "-u" | "--unset" | "-C" | "--chdir") {
            words.next();
        } else if !word.starts_with('-') && !word.contains('=') {
            return Some(word.rsplit('/').next().unwrap_or(word).to_owned());
        }
    }
    None
}

/// Classify `bytes` stored at `path` and count its non-Rust lines.
pub(crate) fn profile(path: &str, bytes: &[u8]) -> Profile {
    let extension = extension(path);
    if extension.as_deref() == Some("rs") {
        return Profile {
            kind: Kind::Other,
            lines: 0,
        };
    }
    if let Some(&(_, language)) = CODE_EXTENSIONS
        .iter()
        .find(|(known, _)| extension.as_deref() == Some(*known))
    {
        return Profile {
            kind: Kind::Code(language),
            lines: line_count(bytes),
        };
    }
    let interpreter = shebang_interpreter(bytes);
    if let Some(language) = interpreter.as_deref().and_then(runtime_language) {
        return Profile {
            kind: Kind::Code(language),
            lines: line_count(bytes),
        };
    }
    let shell = extension
        .as_deref()
        .is_some_and(|known| SHELL_EXTENSIONS.contains(&known))
        || interpreter
            .as_deref()
            .is_some_and(|name| SHELLS.contains(&name));
    if shell {
        Profile {
            kind: Kind::Shell,
            lines: embedded_lines(bytes),
        }
    } else {
        Profile {
            kind: Kind::Other,
            lines: 0,
        }
    }
}

/// A heredoc opened on a line and not yet closed.
struct Heredoc {
    delimiter: String,
    strip_tabs: bool,
    counted: bool,
}

/// Whether a shell line names a Python or JavaScript runtime as a word (not as
/// a `$variable` or `${parameter}`).
fn line_runs_runtime(line: &str) -> bool {
    let bytes = line.as_bytes();
    let word_byte = |byte: u8| byte.is_ascii_alphanumeric() || b"_./-".contains(&byte);
    let mut start = 0;
    while start < bytes.len() {
        if !word_byte(bytes[start]) {
            start += 1;
            continue;
        }
        let mut end = start;
        while end < bytes.len() && word_byte(bytes[end]) {
            end += 1;
        }
        let sigil = start > 0 && matches!(bytes[start - 1], b'$' | b'{');
        if !sigil && runtime_language(&line[start..end]).is_some() {
            return true;
        }
        start = end;
    }
    false
}

/// The heredocs a shell line opens, in order: `<<WORD`, `<<-WORD`, `<<'WORD'`,
/// `<<"WORD"`, `<<\WORD`. A here-string (`<<<`) and a shift inside `$(( ))`
/// open nothing.
fn heredocs_opened(line: &str, counted: bool) -> Vec<Heredoc> {
    let mut opened = Vec::new();
    if line.trim_start().starts_with('#') {
        return opened;
    }
    let bytes = line.as_bytes();
    let mut at = 0;
    while let Some(found) = line[at..].find("<<") {
        let operator = at + found;
        let mut cursor = operator + 2;
        at = cursor;
        if bytes.get(cursor) == Some(&b'<') || (operator > 0 && bytes[operator - 1] == b'<') {
            at = cursor + 1;
            continue;
        }
        let before = &line[..operator];
        if before.matches("((").count() > before.matches("))").count() {
            // A shift inside an arithmetic expression, `$(( x << y ))`.
            continue;
        }
        let strip_tabs = bytes.get(cursor) == Some(&b'-');
        if strip_tabs {
            cursor += 1;
        }
        while bytes
            .get(cursor)
            .is_some_and(|&byte| byte == b' ' || byte == b'\t')
        {
            cursor += 1;
        }
        let (delimiter, end) = match bytes.get(cursor) {
            Some(&quote @ (b'\'' | b'"')) => {
                let body = cursor + 1;
                let Some(close) = line[body..].find(char::from(quote)) else {
                    break;
                };
                (line[body..body + close].to_owned(), body + close + 1)
            }
            Some(b'\\') => bare_word(line, cursor + 1),
            Some(_) => bare_word(line, cursor),
            None => break,
        };
        let starts_like_a_word = delimiter
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
        if starts_like_a_word {
            opened.push(Heredoc {
                delimiter,
                strip_tabs,
                counted,
            });
        }
        at = end.max(at);
    }
    opened
}

/// The unquoted word starting at `start`, and where it ends.
fn bare_word(line: &str, start: usize) -> (String, usize) {
    let end = line[start..]
        .find(|c: char| c.is_whitespace() || ";|&<>()'\"`".contains(c))
        .map_or(line.len(), |offset| start + offset);
    (line[start..end].to_owned(), end)
}

/// The number of lines inside heredocs that a shell script feeds to a Python
/// or JavaScript runtime. Delimiter lines are not counted.
pub(crate) fn embedded_lines(bytes: &[u8]) -> usize {
    let text = String::from_utf8_lossy(bytes);
    let mut pending: Vec<Heredoc> = Vec::new();
    let mut count = 0;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(open) = pending.first() {
            let candidate = if open.strip_tabs {
                line.trim_start_matches('\t')
            } else {
                line
            };
            if candidate == open.delimiter {
                pending.remove(0);
            } else if open.counted {
                count += 1;
            }
            continue;
        }
        pending = heredocs_opened(line, line_runs_runtime(line));
    }
    count
}

/// Whether the first [`EXPLANATION_WINDOW`] lines hold a `# Why not Rust:` or
/// `// Why not Rust:` comment with a long enough reason.
pub(crate) fn has_explanation(bytes: &[u8]) -> bool {
    bytes
        .split(|&byte| byte == b'\n')
        .take(EXPLANATION_WINDOW)
        .any(|line| {
            let line = String::from_utf8_lossy(line);
            let line = line.trim_start();
            let Some(comment) = line.strip_prefix("//").or_else(|| line.strip_prefix('#')) else {
                return false;
            };
            let comment = comment.trim_start();
            comment
                .get(..MARKER.len())
                .is_some_and(|marker| marker.eq_ignore_ascii_case(MARKER))
                && comment[MARKER.len()..].trim().chars().count() >= MIN_REASON_CHARS
        })
}

/// Whether rule 2 applies to `path`.
pub(crate) fn is_ratcheted(path: &str) -> bool {
    if RATCHETED_ROOTS.iter().any(|root| path.starts_with(root)) {
        return true;
    }
    let mut parts = path.splitn(4, '/');
    parts.next() == Some("crates")
        && parts.next().is_some_and(|name| !name.is_empty())
        && parts.next() == Some("tests")
        && parts.next().is_some_and(|rest| !rest.is_empty())
}

fn is_exempt_from_explain(path: &str) -> bool {
    EXEMPT_FROM_EXPLAIN
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

/// Where the BASE tree comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Base {
    /// This commit, as given.
    Rev(String),
    /// The merge-base of this ref with the target (with `HEAD` and every
    /// `MERGE_HEAD` for the index and working-tree targets).
    MergeBaseWith(String),
}

/// Where the TARGET tree comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// The staged index (`GIT_INDEX_FILE` is honoured).
    Index,
    /// Tracked and untracked, not ignored, files on disk.
    Worktree,
    /// A commit.
    Rev(String),
}

impl Target {
    /// Parse the value of `--target`.
    pub(crate) fn parse(spec: &str) -> Result<Self, String> {
        match spec {
            "index" => Ok(Self::Index),
            "worktree" => Ok(Self::Worktree),
            _ => match spec.strip_prefix("rev:") {
                Some(rev) if !rev.is_empty() => Ok(Self::Rev(rev.to_owned())),
                _ => Err(format!(
                    "--target takes `index`, `worktree` or `rev:<REV>`, not `{spec}`"
                )),
            },
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::Index => "the staged index".to_owned(),
            Self::Worktree => "the working tree".to_owned(),
            Self::Rev(rev) => format!("commit {rev}"),
        }
    }
}

/// The git plumbing the ratchet reads through, rooted at one work tree.
struct Git {
    root: PathBuf,
}

impl Git {
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.root).args(args);
        command
    }

    /// Run git; a nonzero exit is `Ok(None)`.
    fn try_output(&self, args: &[&str]) -> Result<Option<Vec<u8>>, String> {
        let output = self
            .command(args)
            .stderr(Stdio::null())
            .output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        Ok(output.status.success().then_some(output.stdout))
    }

    /// Run git; a nonzero exit is an error naming the command.
    fn output(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let output = self
            .command(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(format!(
                "`git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    /// The commit `rev` names, if it names one.
    fn commit(&self, rev: &str) -> Result<Option<String>, String> {
        let spec = format!("{rev}^{{commit}}");
        Ok(self
            .try_output(&["rev-parse", "-q", "--verify", &spec])?
            .map(|out| String::from_utf8_lossy(&out).trim().to_owned()))
    }

    /// `HEAD` (absent on an unborn branch) and every commit in `MERGE_HEAD`.
    fn index_heads(&self) -> Result<Vec<String>, String> {
        let mut heads: Vec<String> = self.commit("HEAD")?.into_iter().collect();
        let merge_head = self.output(&["rev-parse", "--git-path", "MERGE_HEAD"])?;
        let merge_head = PathBuf::from(String::from_utf8_lossy(&merge_head).trim());
        let merge_head = if merge_head.is_absolute() {
            merge_head
        } else {
            self.root.join(merge_head)
        };
        if let Ok(text) = std::fs::read_to_string(&merge_head) {
            for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
                heads.push(
                    self.commit(line)?
                        .ok_or_else(|| format!("MERGE_HEAD names `{line}`, not a commit"))?,
                );
            }
        }
        Ok(heads)
    }

    /// Regular-file blobs of a tree-ish, by path. Symlinks and submodules are
    /// not files with contents of their own and are skipped.
    fn tree(&self, rev: &str) -> Result<BTreeMap<String, String>, String> {
        let out = self.output(&["ls-tree", "-r", "-z", "--full-tree", rev])?;
        let mut entries = BTreeMap::new();
        for record in out.split(|&byte| byte == 0).filter(|r| !r.is_empty()) {
            let record = String::from_utf8_lossy(record);
            let (meta, path) = record
                .split_once('\t')
                .ok_or_else(|| format!("unreadable ls-tree record `{record}`"))?;
            let mut fields = meta.split(' ');
            let (mode, kind, oid) = (fields.next(), fields.next(), fields.next());
            if kind == Some("blob") && matches!(mode, Some("100644" | "100755")) {
                let oid = oid.ok_or_else(|| format!("unreadable ls-tree record `{record}`"))?;
                entries.insert(path.to_owned(), oid.to_owned());
            }
        }
        Ok(entries)
    }

    /// Regular-file blobs of the index, by path. An unmerged entry is an error.
    fn index(&self) -> Result<BTreeMap<String, String>, String> {
        let out = self.output(&["ls-files", "-s", "-z"])?;
        let mut entries = BTreeMap::new();
        for record in out.split(|&byte| byte == 0).filter(|r| !r.is_empty()) {
            let record = String::from_utf8_lossy(record);
            let (meta, path) = record
                .split_once('\t')
                .ok_or_else(|| format!("unreadable ls-files record `{record}`"))?;
            let mut fields = meta.split(' ');
            let (mode, oid, stage) = (fields.next(), fields.next(), fields.next());
            if stage != Some("0") {
                return Err(format!(
                    "{path} is unmerged in the index; resolve the conflict first"
                ));
            }
            if matches!(mode, Some("100644" | "100755")) {
                let oid = oid.ok_or_else(|| format!("unreadable ls-files record `{record}`"))?;
                entries.insert(path.to_owned(), oid.to_owned());
            }
        }
        Ok(entries)
    }

    /// Paths of the working tree that may differ from `base`: those `git diff`
    /// (without rename detection) names, and every untracked, unignored file.
    fn worktree_candidates(&self, base: Option<&str>) -> Result<Vec<String>, String> {
        let listing = match base {
            Some(base) => self.output(&[
                "diff",
                "--no-renames",
                "--no-ext-diff",
                "--name-only",
                "-z",
                base,
                "--",
            ])?,
            None => self.output(&["ls-files", "-z", "--cached"])?,
        };
        let untracked = self.output(&["ls-files", "-z", "--others", "--exclude-standard"])?;
        let mut paths: Vec<String> = listing
            .split(|&byte| byte == 0)
            .chain(untracked.split(|&byte| byte == 0))
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect();
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

/// A long-lived `git cat-file --batch`, read one blob at a time.
struct Blobs {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Blobs {
    fn open(git: &Git) -> Result<Self, String> {
        let mut child = git
            .command(&["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot run git cat-file: {error}"))?;
        let input = child.stdin.take().ok_or("git cat-file has no stdin")?;
        let output = BufReader::new(child.stdout.take().ok_or("git cat-file has no stdout")?);
        Ok(Self {
            child,
            input,
            output,
        })
    }

    fn read(&mut self, oid: &str) -> Result<Vec<u8>, String> {
        let io = |error: std::io::Error| format!("git cat-file: {error}");
        writeln!(self.input, "{oid}").map_err(io)?;
        self.input.flush().map_err(io)?;
        let mut header = String::new();
        self.output.read_line(&mut header).map_err(io)?;
        let mut fields = header.split_whitespace();
        let (_, kind, size) = (fields.next(), fields.next(), fields.next());
        if kind != Some("blob") {
            return Err(format!(
                "git cat-file: {oid} is not a blob ({})",
                header.trim()
            ));
        }
        let size: usize = size
            .and_then(|size| size.parse().ok())
            .ok_or_else(|| format!("git cat-file: unreadable header `{}`", header.trim()))?;
        let mut contents = vec![0; size + 1];
        self.output.read_exact(&mut contents).map_err(io)?;
        contents.pop();
        Ok(contents)
    }
}

impl Drop for Blobs {
    fn drop(&mut self) {
        // Every read above checked its own header and length, so the batch
        // process's exit status adds nothing; it is stopped and reaped.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One refusal.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Violation {
    /// Rule 1: a new non-Rust path without an explanation.
    Unexplained { path: String, profile: Profile },
    /// Rule 2: a ratcheted path that grew.
    Grew {
        path: String,
        base: usize,
        target: Profile,
    },
}

/// What one run found.
#[derive(Debug)]
pub(crate) struct Verdict {
    pub(crate) base: Option<String>,
    pub(crate) checked: usize,
    pub(crate) violations: Vec<Violation>,
}

fn resolve_base(git: &Git, base: &Base, target: &Target) -> Result<Option<String>, String> {
    match base {
        Base::Rev(rev) => git
            .commit(rev)?
            .map(Some)
            .ok_or_else(|| format!("--base `{rev}` does not name a commit")),
        Base::MergeBaseWith(reference) => {
            let integration = git.commit(reference)?.ok_or_else(|| {
                format!(
                    "the integration ref `{reference}` does not name a commit; fetch it \
                     (`git fetch origin`) or name another with PURRDF_RATCHET_BASE"
                )
            })?;
            let heads = match target {
                Target::Rev(rev) => vec![
                    git.commit(rev)?
                        .ok_or_else(|| format!("--target rev:{rev} does not name a commit"))?,
                ],
                Target::Index | Target::Worktree => git.index_heads()?,
            };
            if heads.is_empty() {
                // An unborn branch: nothing is in BASE, so every path is new.
                return Ok(None);
            }
            let mut args = vec!["merge-base", integration.as_str()];
            args.extend(heads.iter().map(String::as_str));
            let found = git.try_output(&args)?.ok_or_else(|| {
                format!("the target shares no history with `{reference}`; there is no merge-base")
            })?;
            Ok(Some(String::from_utf8_lossy(&found).trim().to_owned()))
        }
    }
}

/// Compare TARGET against BASE under both rules.
pub(crate) fn evaluate(root: &Path, base: &Base, target: &Target) -> Result<Verdict, String> {
    let git = Git {
        root: root.to_path_buf(),
    };
    let base_commit = resolve_base(&git, base, target)?;
    let base_tree = match &base_commit {
        Some(commit) => git.tree(commit)?,
        None => BTreeMap::new(),
    };
    let mut blobs = Blobs::open(&git)?;
    // Every TARGET path whose contents may differ from BASE, with a way to
    // read them. A path whose blob BASE holds unchanged passes both rules.
    let changed: Vec<(String, Option<String>)> = match target {
        Target::Index | Target::Rev(_) => {
            let entries = match target {
                Target::Rev(rev) => git.tree(rev)?,
                _ => git.index()?,
            };
            entries
                .into_iter()
                .filter(|(path, oid)| base_tree.get(path) != Some(oid))
                .map(|(path, oid)| (path, Some(oid)))
                .collect()
        }
        Target::Worktree => git
            .worktree_candidates(base_commit.as_deref())?
            .into_iter()
            .map(|path| (path, None))
            .collect(),
    };
    let mut verdict = Verdict {
        base: base_commit,
        checked: 0,
        violations: Vec::new(),
    };
    for (path, oid) in changed {
        let bytes = match oid {
            Some(oid) => blobs.read(&oid)?,
            None => {
                let on_disk = root.join(&path);
                match std::fs::symlink_metadata(&on_disk) {
                    Ok(meta) if meta.is_file() => std::fs::read(&on_disk)
                        .map_err(|error| format!("{}: {error}", on_disk.display()))?,
                    // Deleted, a symlink or a directory (a submodule): no contents.
                    _ => continue,
                }
            }
        };
        let after = profile(&path, &bytes);
        match base_tree.get(&path) {
            None => {
                if after.has_non_rust_code() {
                    verdict.checked += 1;
                }
                if after.needs_explanation(&bytes)
                    && !is_exempt_from_explain(&path)
                    && !has_explanation(&bytes)
                {
                    verdict.violations.push(Violation::Unexplained {
                        path,
                        profile: after,
                    });
                }
            }
            Some(base_oid) if is_ratcheted(&path) => {
                let before = profile(&path, &blobs.read(base_oid)?);
                if after.has_non_rust_code() || before.has_non_rust_code() {
                    verdict.checked += 1;
                }
                if after.lines > before.lines {
                    verdict.violations.push(Violation::Grew {
                        path,
                        base: before.lines,
                        target: after,
                    });
                }
            }
            Some(_) => {}
        }
    }
    Ok(verdict)
}

/// The `--non-rust-ratchet` mode: evaluate and report.
pub(crate) fn run(root: &Path, base: &Base, target: &Target) -> Result<ExitCode, String> {
    let verdict = evaluate(root, base, target)?;
    let base_name = verdict.base.as_deref().map_or_else(
        || "the empty tree (unborn branch)".to_owned(),
        |commit| commit.chars().take(12).collect(),
    );
    let base_how = match base {
        Base::Rev(rev) => format!("--base {rev}"),
        Base::MergeBaseWith(reference) => format!("merge-base with {reference}"),
    };
    if verdict.violations.is_empty() {
        println!(
            "OK: non-Rust ratchet: {} changed non-Rust path(s) in {} against {base_name} \
             ({base_how}); nothing legacy grew and every new file says why it is not Rust",
            verdict.checked,
            target.describe()
        );
        return Ok(ExitCode::SUCCESS);
    }
    eprintln!(
        "helper-census --non-rust-ratchet: {} refusal(s) in {} against {base_name} ({base_how}).",
        verdict.violations.len(),
        target.describe()
    );
    eprintln!(
        "Repository tooling and tests are written in Rust (see AGENTS.md, Hard constraints)."
    );
    let unexplained: Vec<_> = verdict
        .violations
        .iter()
        .filter_map(|violation| match violation {
            Violation::Unexplained { path, profile } => Some((path, profile)),
            Violation::Grew { .. } => None,
        })
        .collect();
    if !unexplained.is_empty() {
        eprintln!();
        eprintln!("New non-Rust code without an explanation of why it cannot be Rust:");
        for (path, profile) in &unexplained {
            eprintln!(
                "  {path}: new {}, {} {}, no explanation in its first {EXPLANATION_WINDOW} lines",
                profile.describe(),
                profile.lines,
                profile.unit()
            );
        }
        eprintln!(
            "  Write it in Rust (helper-census, an xtask, or a Rust test). If it truly cannot be,"
        );
        eprintln!(
            "  add within its first {EXPLANATION_WINDOW} lines a comment whose reason has at least \
             {MIN_REASON_CHARS} characters:"
        );
        eprintln!("    # Why not Rust: <the concrete reason>     (Python, shell)");
        eprintln!("    // Why not Rust: <the concrete reason>    (JavaScript/TypeScript)");
        eprintln!("  A moved or copied file is a new path: git rename detection is not consulted.");
    }
    let grew: Vec<_> = verdict
        .violations
        .iter()
        .filter_map(|violation| match violation {
            Violation::Grew { path, base, target } => Some((path, base, target)),
            Violation::Unexplained { .. } => None,
        })
        .collect();
    if !grew.is_empty() {
        eprintln!();
        eprintln!("Legacy non-Rust tooling and tests that grew (they may only shrink):");
        for (path, base, target) in &grew {
            eprintln!(
                "  {path}: {base} -> {} {} (+{})",
                target.lines,
                target.unit(),
                target.lines - **base
            );
        }
        eprintln!(
            "  Put the new logic in Rust instead, or offset it by porting or deleting at least as"
        );
        eprintln!("  many lines of the same file.");
    }
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::{
        Kind, Language, embedded_lines, has_explanation, is_ratcheted, line_count, profile,
    };

    #[test]
    fn line_count_counts_an_unterminated_last_line() {
        assert_eq!(line_count(b""), 0);
        assert_eq!(line_count(b"a"), 1);
        assert_eq!(line_count(b"a\n"), 1);
        assert_eq!(line_count(b"a\nb"), 2);
        assert_eq!(line_count(b"\n\n"), 2);
    }

    #[test]
    fn extensions_and_shebangs_classify() {
        for path in [
            "a.py", "a.pyw", "a.pyi", "a.PY", "x/a.d.ts", "a.tsx", "a.jsx", "a.cts",
        ] {
            assert!(
                matches!(profile(path, b"x\n").kind, Kind::Code(_)),
                "{path}"
            );
        }
        let python = profile("tool", b"#!/usr/bin/env python3\nprint()\n");
        assert_eq!(python.kind, Kind::Code(Language::Python));
        assert_eq!(python.lines, 2);
        for shebang in [
            "#!/usr/bin/python3.12",
            "#!/usr/bin/env node",
            "#!/usr/bin/env -S deno run --allow-read",
            "#! /usr/local/bin/bun",
            "#!/usr/bin/env PYTHONSAFEPATH=1 python",
            "#!/usr/bin/env -u PYTHONPATH python3",
            "#!/usr/bin/env -i -C /tmp node",
        ] {
            let text = format!("{shebang}\nx\n");
            assert!(
                matches!(profile("tool.unknown", text.as_bytes()).kind, Kind::Code(_)),
                "{shebang}"
            );
        }
        assert_eq!(profile("tool", b"#!/bin/bash\necho\n").kind, Kind::Shell);
        assert_eq!(profile("README", b"plain\n").kind, Kind::Other);
        assert_eq!(profile(".eslintrc", b"x\n").kind, Kind::Other);
        assert_eq!(profile("lib.rs", b"#![allow(x)]\n").kind, Kind::Other);
        assert_eq!(profile("x.json", b"{}\n").kind, Kind::Other);
    }

    #[test]
    fn heredocs_fed_to_a_runtime_count() {
        let script = b"#!/bin/bash\n\
            python3 - \"$a\" <<'PY'\nimport sys\nprint(sys.argv)\nPY\n\
            cat <<EOF\nnot counted\nEOF\n\
            node <<\"JS\"\nconsole.log(1)\nJS\n\
            cat <<-END | python3\n\tprint(2)\n\tEND\n\
            echo $((1<<3))\n\
            (( n = 2 )) && python3 - <<PY\nprint(n)\nPY\n\
            read x <<< \"$python\"\n\
            echo \"$python\" <<EOF\nignored\nEOF\n";
        assert_eq!(embedded_lines(script), 2 + 1 + 1 + 1);
        assert_eq!(profile("a.sh", script).lines, 5);
    }

    #[test]
    fn an_explanation_needs_the_marker_and_a_reason() {
        let reason = "pytest must import the built wheel, a CPython-only surface";
        assert!(has_explanation(
            format!("# Why not Rust: {reason}\n").as_bytes()
        ));
        assert!(has_explanation(
            format!("  // WHY NOT RUST: {reason}\n").as_bytes()
        ));
        assert!(!has_explanation(b"# Why not Rust: too short\n"));
        assert!(!has_explanation(
            format!("x = 1  # Why not Rust: {reason}\n").as_bytes()
        ));
        let late = format!("{}# Why not Rust: {reason}\n", "x\n".repeat(40));
        assert!(!has_explanation(late.as_bytes()));
        let in_window = format!("{}# Why not Rust: {reason}\n", "x\n".repeat(39));
        assert!(has_explanation(in_window.as_bytes()));
    }

    #[test]
    fn ratcheted_roots_are_exact() {
        for path in [
            "scripts/a.py",
            "scripts/cleanroom/a.py",
            "bindings/python/tests/t.py",
            "crates/shapes/tests/oracle.mjs",
            "crates/text/tests/reference/bm25f.py",
            "crates/rdf-wasm/js/tests/a.mjs",
            "crates/rdf-wasm/js/bench/b.mjs",
        ] {
            assert!(is_ratcheted(path), "{path}");
        }
        for path in [
            "crates/rdf-wasm/js/index.mjs",
            "crates/rdf-wasm/js/index.d.ts",
            "crates/rdf-wasm/js/src/a.mjs",
            "docs/playground/app.mjs",
            "bindings/python/python/purrdf/__init__.py",
            "crates/tests",
            "crates//tests/a.py",
        ] {
            assert!(!is_ratcheted(path), "{path}");
        }
    }
}
