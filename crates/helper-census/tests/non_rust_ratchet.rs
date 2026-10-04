// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `helper-census --non-rust-ratchet` against real git repositories.
//!
//! Every refusal is paired with a valid neighbour that must still pass. The
//! `f1_`…`f8_` and `binary_` cases are the bypasses the shell implementation in
//! the pre-commit hook let through; each is reproduced here as a commit
//! sequence, so the test fails against any implementation that reintroduces
//! one.

use std::path::Path;
use std::process::{Command, Output};

use purrdf_testkit::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_helper-census");

/// A rule-1 explanation for Python and shell.
const HASH_REASON: &str =
    "# Why not Rust: pytest has to import the built CPython wheel to exercise it\n";

/// A rule-1 explanation for JavaScript and TypeScript.
const SLASH_REASON: &str =
    "// Why not Rust: the shipped npm package is exercised under Node itself\n";

/// `count` distinct lines, each starting with `prefix`.
fn lines(prefix: &str, count: usize) -> String {
    let mut text = String::new();
    for n in 0..count {
        text.push_str(prefix);
        text.push_str(&n.to_string());
        text.push('\n');
    }
    text
}

/// A shell script whose `python3` heredoc holds `embedded` lines.
fn tool_sh(embedded: usize) -> String {
    format!(
        "#!/usr/bin/env bash\nset -eu\npython3 - \"$1\" <<'PY'\n{}PY\necho done\n",
        lines("print(", embedded)
    )
}

/// Make `command` independent of the caller's git configuration and of any
/// repository a surrounding hook points at.
fn isolated(command: &mut Command) -> &mut Command {
    for var in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_WORK_TREE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_PREFIX",
        "PURRDF_RATCHET_BASE",
    ] {
        command.env_remove(var);
    }
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.org")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.org")
}

/// What one ratchet run reported.
#[derive(Debug)]
struct Outcome {
    accepted: bool,
    stderr: String,
}

impl Outcome {
    fn from_output(output: &Output) -> Self {
        Self {
            accepted: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    #[track_caller]
    fn assert_accepted(&self) {
        assert!(self.accepted, "expected acceptance, got:\n{}", self.stderr);
    }

    /// Assert a refusal whose report contains every needle.
    #[track_caller]
    fn assert_refused(&self, needles: &[&str]) {
        assert!(!self.accepted, "expected a refusal, but the ratchet passed");
        for needle in needles {
            assert!(
                self.stderr.contains(needle),
                "the refusal does not mention `{needle}`:\n{}",
                self.stderr
            );
        }
    }
}

/// A scratch repository whose `main` (mirrored to `origin/main`) holds the
/// legacy layout, with the work branch `work` checked out.
struct Repo {
    dir: TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: purrdf_testkit::temp_dir!("non-rust-ratchet-").expect("scratch repository"),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&["config", "core.hooksPath", "no-hooks"]);
        repo.write("scripts/legacy.py", &lines("x = ", 10));
        repo.write("scripts/legacy.cjs", &lines("// c", 5));
        repo.write("scripts/legacy.mts", &lines("// m", 5));
        repo.write("scripts/tool.sh", &tool_sh(3));
        repo.write("bindings/python/tests/test_legacy.py", &lines("# t", 5));
        repo.write("crates/demo/tests/oracle.mjs", &lines("// o", 5));
        repo.write("crates/rdf-wasm/js/tests/suite.mjs", &lines("// s", 5));
        repo.write("crates/rdf-wasm/js/bench/bench.mjs", &lines("// b", 5));
        repo.write("crates/rdf-wasm/js/index.mjs", &lines("// i", 5));
        repo.write("crates/demo/src/lib.rs", "//! Demo.\n");
        repo.commit("legacy layout");
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.git(&["checkout", "-q", "-b", "work"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    #[track_caller]
    fn git(&self, args: &[&str]) -> String {
        let output = isolated(Command::new("git").arg("-C").arg(self.path()).args(args))
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, contents).expect("write");
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path().join(relative)).expect("read")
    }

    fn append(&self, relative: &str, extra: &str) {
        let mut contents = self.read(relative);
        contents.push_str(extra);
        self.write(relative, &contents);
    }

    /// `git mv`, creating the destination's directory first.
    fn mv(&self, from: &str, to: &str) {
        let parent = self.path().join(to);
        std::fs::create_dir_all(parent.parent().expect("a parent")).expect("mkdir");
        self.git(&["mv", from, to]);
    }

    fn stage(&self) {
        self.git(&["add", "-A"]);
    }

    fn commit(&self, message: &str) {
        self.stage();
        self.git(&["commit", "-q", "--allow-empty", "-m", message]);
    }

    fn ratchet(&self, base: &[&str], target: &str) -> Outcome {
        let mut command = Command::new(BIN);
        command
            .arg("--root")
            .arg(self.path())
            .arg("--non-rust-ratchet")
            .args(base)
            .args(["--target", target]);
        Outcome::from_output(&isolated(&mut command).output().expect("the ratchet runs"))
    }

    /// The check the pre-commit and pre-merge-commit hooks run: the staged
    /// index against the merge-base with `origin/main`.
    fn check_staged(&self) -> Outcome {
        self.ratchet(&["--merge-base-with", "origin/main"], "index")
    }

    /// The check `make check` runs.
    fn check_worktree(&self) -> Outcome {
        self.ratchet(&["--merge-base-with", "origin/main"], "worktree")
    }

    /// The check CI runs on a pull request's head.
    fn check_head(&self) -> Outcome {
        self.ratchet(&["--merge-base-with", "origin/main"], "rev:HEAD")
    }
}

#[test]
fn f1_a_script_moved_out_of_scripts_and_grown_is_refused() {
    let repo = Repo::new();
    repo.mv("scripts/legacy.py", "tools/legacy.py");
    repo.append("tools/legacy.py", "grown = 1\n");
    repo.stage();
    repo.check_staged()
        .assert_refused(&["tools/legacy.py", "new Python file", "11 line(s)"]);
    repo.commit("move and grow");
    repo.check_head().assert_refused(&["tools/legacy.py"]);
}

#[test]
fn f1_a_script_renamed_inside_scripts_and_grown_is_refused() {
    let repo = Repo::new();
    repo.mv("scripts/legacy.py", "scripts/renamed.py");
    repo.append("scripts/renamed.py", "grown = 1\n");
    repo.stage();
    repo.check_staged().assert_refused(&["scripts/renamed.py"]);
}

#[test]
fn f1_neighbour_a_moved_script_that_says_why_not_rust_is_accepted() {
    let repo = Repo::new();
    repo.mv("scripts/legacy.py", "tools/legacy.py");
    let moved = format!("{HASH_REASON}{}", repo.read("tools/legacy.py"));
    repo.write("tools/legacy.py", &moved);
    repo.stage();
    repo.check_staged().assert_accepted();
}

/// Copy `scripts/legacy.py` to `tools/copy.py` and trim the source by one
/// line, so git's copy detection (which only looks at modified sources) pairs
/// the two.
fn copy_and_trim(repo: &Repo, header: &str) {
    let source = repo.read("scripts/legacy.py");
    repo.write("tools/copy.py", &format!("{header}{source}"));
    let trimmed = source.split_once('\n').map_or("", |(_, rest)| rest);
    repo.write("scripts/legacy.py", trimmed);
}

#[test]
fn f2_rename_and_copy_configuration_cannot_change_the_verdict() {
    let mut reports = Vec::new();
    for setting in ["copies", "true", "false"] {
        let repo = Repo::new();
        repo.git(&["config", "diff.renames", setting]);
        copy_and_trim(&repo, "");
        repo.stage();
        let outcome = repo.check_staged();
        outcome.assert_refused(&["tools/copy.py", "new Python file"]);
        reports.push(outcome.stderr);
    }
    // The report names only paths and counts, never a commit, so with the same
    // fixture it is byte-identical under every setting once the base is masked.
    let masked: Vec<String> = reports
        .iter()
        .map(|report| report.lines().skip(1).collect::<Vec<_>>().join("\n"))
        .collect();
    assert_eq!(masked[0], masked[1]);
    assert_eq!(masked[1], masked[2]);
}

#[test]
fn f2_neighbour_an_explained_copy_is_accepted_under_every_setting() {
    for setting in ["copies", "true", "false"] {
        let repo = Repo::new();
        repo.git(&["config", "diff.renames", setting]);
        copy_and_trim(&repo, HASH_REASON);
        repo.stage();
        repo.check_staged().assert_accepted();
    }
}

#[test]
fn f3_an_explained_file_over_64_kib_is_accepted() {
    let repo = Repo::new();
    let big = format!("{HASH_REASON}{}", lines("value_with_a_long_name = ", 4_000));
    assert!(big.len() > 64 * 1024, "the fixture must exceed 64 KiB");
    repo.write("bindings/python/tests/test_big.py", &big);
    repo.stage();
    repo.check_staged().assert_accepted();
    repo.check_worktree().assert_accepted();
}

#[test]
fn f3_neighbour_an_unexplained_file_over_64_kib_is_refused() {
    let repo = Repo::new();
    repo.write(
        "bindings/python/tests/test_big.py",
        &lines("value_with_a_long_name = ", 4_000),
    );
    repo.stage();
    repo.check_staged()
        .assert_refused(&["bindings/python/tests/test_big.py", "4000 line(s)"]);
}

#[test]
fn f4_mts_growth_in_scripts_is_refused() {
    let repo = Repo::new();
    repo.append("scripts/legacy.mts", "// more\n");
    repo.stage();
    repo.check_staged()
        .assert_refused(&["scripts/legacy.mts: 5 -> 6 line(s) (+1)"]);
}

#[test]
fn f4_cjs_and_mts_growth_in_crate_tests_is_refused() {
    let repo = Repo::new();
    repo.write("crates/demo/tests/old.cjs", &lines("// c", 3));
    repo.write("crates/demo/tests/old.mts", &lines("// m", 3));
    repo.commit("legacy crate-test JS");
    repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    repo.append("crates/demo/tests/old.cjs", "// more\n");
    repo.append("crates/demo/tests/old.mts", "// more\n");
    repo.stage();
    repo.check_staged().assert_refused(&[
        "crates/demo/tests/old.cjs: 3 -> 4 line(s) (+1)",
        "crates/demo/tests/old.mts: 3 -> 4 line(s) (+1)",
    ]);
}

#[test]
fn f4_cjs_growth_in_scripts_is_refused() {
    let repo = Repo::new();
    repo.append("scripts/legacy.cjs", "// more\n");
    repo.stage();
    repo.check_staged()
        .assert_refused(&["scripts/legacy.cjs: 5 -> 6 line(s) (+1)"]);
}

#[test]
fn f4_neighbour_cjs_and_mts_shrinking_is_accepted() {
    let repo = Repo::new();
    repo.write("scripts/legacy.cjs", &lines("// c", 4));
    repo.write("scripts/legacy.mts", &lines("// m", 2));
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn f5_deleting_then_readding_a_grown_legacy_file_is_refused() {
    let repo = Repo::new();
    repo.git(&["rm", "-q", "scripts/legacy.py"]);
    repo.check_staged().assert_accepted();
    repo.commit("delete the legacy script");
    let readded = format!("{HASH_REASON}{}", lines("x = ", 10));
    repo.write("scripts/legacy.py", &readded);
    repo.stage();
    repo.check_staged()
        .assert_refused(&["scripts/legacy.py: 10 -> 11 line(s) (+1)"]);
    repo.commit("re-add it, grown");
    repo.check_head()
        .assert_refused(&["scripts/legacy.py: 10 -> 11 line(s)"]);
}

#[test]
fn f5_neighbour_readding_no_larger_is_accepted() {
    let repo = Repo::new();
    repo.git(&["rm", "-q", "scripts/legacy.py"]);
    repo.commit("delete the legacy script");
    repo.write(
        "scripts/legacy.py",
        &format!("{HASH_REASON}{}", lines("x = ", 9)),
    );
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn f6_new_pyw_pyi_cts_tsx_jsx_files_need_an_explanation() {
    let repo = Repo::new();
    let paths = [
        "tools/a.pyw",
        "tools/b.pyi",
        "tools/c.cts",
        "tools/d.tsx",
        "tools/e.jsx",
    ];
    for path in paths {
        repo.write(path, "code()\n");
    }
    repo.stage();
    repo.check_staged().assert_refused(&paths);
}

#[test]
fn new_files_under_the_classic_code_extensions_need_an_explanation() {
    let repo = Repo::new();
    let paths = [
        "tools/f.py",
        "tools/g.mjs",
        "tools/h.js",
        "tools/i.cjs",
        "tools/j.ts",
        "tools/k.mts",
    ];
    for path in paths {
        repo.write(path, "code()\n");
    }
    repo.stage();
    repo.check_staged().assert_refused(&paths);
}

#[test]
fn f6_neighbour_explained_files_under_every_code_extension_are_accepted() {
    let repo = Repo::new();
    for (path, reason) in [
        ("tools/a.pyw", HASH_REASON),
        ("tools/b.pyi", HASH_REASON),
        ("tools/c.cts", SLASH_REASON),
        ("tools/d.tsx", SLASH_REASON),
        ("tools/e.jsx", SLASH_REASON),
    ] {
        repo.write(path, &format!("{reason}code()\n"));
    }
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn f6_extensionless_shebang_scripts_need_an_explanation() {
    let repo = Repo::new();
    let scripts = [
        ("scripts/py", "#!/usr/bin/env python\n"),
        ("scripts/py3", "#!/usr/bin/python3\n"),
        ("scripts/nd", "#!/usr/bin/env node\n"),
        ("scripts/dn", "#!/usr/bin/env -S deno run --allow-read\n"),
        ("scripts/bn", "#!/usr/bin/env bun\n"),
        ("scripts/odd.tool", "#!/usr/bin/env python3\n"),
    ];
    for (path, shebang) in scripts {
        repo.write(path, &format!("{shebang}work()\n"));
    }
    repo.stage();
    repo.check_staged()
        .assert_refused(&scripts.map(|(path, _)| path));
}

#[test]
fn f6_neighbour_shebang_scripts_with_a_reason_and_plain_shell_are_accepted() {
    let repo = Repo::new();
    repo.write(
        "scripts/py",
        &format!("#!/usr/bin/env python3\n{HASH_REASON}work()\n"),
    );
    repo.write(
        "scripts/nd",
        &format!("#!/usr/bin/env node\n{SLASH_REASON}work()\n"),
    );
    repo.write("scripts/plain", "#!/bin/sh\necho no embedded code\n");
    repo.write("scripts/notes", "plain text, no shebang\n");
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn f6_growing_a_python_heredoc_in_a_legacy_shell_script_is_refused() {
    let repo = Repo::new();
    repo.write("scripts/tool.sh", &tool_sh(4));
    repo.stage();
    repo.check_staged()
        .assert_refused(&["scripts/tool.sh: 3 -> 4 embedded Python/JavaScript line(s) (+1)"]);
}

#[test]
fn f6_adding_a_node_heredoc_to_a_legacy_shell_script_is_refused() {
    let repo = Repo::new();
    repo.append("scripts/tool.sh", "node <<'JS'\nconsole.log(1)\nJS\n");
    repo.stage();
    repo.check_staged()
        .assert_refused(&["scripts/tool.sh: 3 -> 4 embedded"]);
}

#[test]
fn f6_neighbour_shell_growth_outside_runtime_heredocs_and_shrinking_are_accepted() {
    let repo = Repo::new();
    let shrunk = tool_sh(2);
    repo.write(
        "scripts/tool.sh",
        &format!("{shrunk}echo more shell\ncat <<'EOF'\nnot python\nEOF\n"),
    );
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn f6_a_new_shell_script_embedding_python_needs_an_explanation() {
    let repo = Repo::new();
    repo.write("tools/run.sh", &tool_sh(2));
    repo.stage();
    repo.check_staged().assert_refused(&[
        "tools/run.sh",
        "shell script with embedded Python/JavaScript heredocs",
    ]);
}

#[test]
fn f6_neighbour_a_new_shell_script_embedding_python_with_a_reason_is_accepted() {
    let repo = Repo::new();
    let script = tool_sh(2).replacen('\n', &format!("\n{HASH_REASON}"), 1);
    repo.write("tools/run.sh", &script);
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn f7_f8_every_enforcement_point_runs_the_ratchet() {
    let root = purrdf_testkit::paths::workspace_root();
    let read = |relative: &str| {
        std::fs::read_to_string(root.join(relative))
            .unwrap_or_else(|error| panic!("{relative}: {error}"))
    };
    let hook = read(".githooks/pre-commit");
    assert!(hook.contains("--non-rust-ratchet"), "the pre-commit hook");
    assert!(hook.contains("--target index"), "the hook checks the index");
    assert!(
        !hook.contains("--diff-filter=A -z") && !hook.contains("--numstat"),
        "the shell implementation is gone"
    );
    let merge = read(".githooks/pre-merge-commit");
    assert!(
        merge
            .lines()
            .any(|line| line.trim_start().starts_with("exec ") && line.contains("pre-commit")),
        "pre-merge-commit hands over to pre-commit"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(root.join(".githooks/pre-merge-commit"))
            .expect("pre-merge-commit")
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "pre-merge-commit is executable");
    }
    let makefile = read("Makefile");
    let check = makefile
        .split("\ncheck:")
        .nth(1)
        .and_then(|rest| rest.split("\n\n").next())
        .expect("a check target");
    assert!(
        check.contains("--non-rust-ratchet") && check.contains("--target worktree"),
        "make check runs the ratchet on the working tree"
    );
    let ci = read(".github/workflows/ci.yaml");
    assert!(ci.contains("--non-rust-ratchet"), "CI runs the ratchet");
    assert!(ci.contains("fetch-depth: 0"), "CI fetches the merge-base");
}

#[test]
fn f8_a_merge_bringing_in_an_unexplained_file_is_refused() {
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "side"]);
    repo.write("scripts/side.py", "side = 1\n");
    repo.commit("an unexplained script on a side branch");
    repo.git(&["checkout", "-q", "work"]);
    repo.write("crates/demo/src/lib.rs", "//! Demo, changed.\n");
    repo.commit("work");
    repo.git(&["merge", "-q", "--no-ff", "--no-commit", "side"]);
    repo.check_staged().assert_refused(&["scripts/side.py"]);
}

#[test]
fn f8_neighbour_merging_the_integration_branch_is_accepted() {
    let repo = Repo::new();
    repo.write("crates/demo/src/lib.rs", "//! Demo, changed.\n");
    repo.commit("work");
    // The integration branch moves on with a script that predates the ratchet.
    repo.git(&["checkout", "-q", "main"]);
    repo.write("scripts/upstream.py", "upstream = 1\n");
    repo.commit("upstream adds a script");
    repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    repo.git(&["checkout", "-q", "work"]);
    repo.git(&["merge", "-q", "--no-ff", "--no-commit", "origin/main"]);
    repo.check_staged().assert_accepted();
}

#[test]
fn binary_attributes_cannot_hide_growth() {
    let repo = Repo::new();
    repo.write(
        ".gitattributes",
        "*.py -diff\n*.mjs binary\n*.sh -text -diff\n",
    );
    repo.commit("mark legacy code binary");
    repo.append("scripts/legacy.py", "grown = 1\n");
    repo.append("crates/rdf-wasm/js/tests/suite.mjs", "// grown\n");
    repo.write("scripts/tool.sh", &tool_sh(5));
    repo.stage();
    repo.check_staged().assert_refused(&[
        "scripts/legacy.py: 10 -> 11",
        "crates/rdf-wasm/js/tests/suite.mjs: 5 -> 6",
        "scripts/tool.sh: 3 -> 5",
    ]);
}

#[test]
fn binary_neighbour_shrinking_under_binary_attributes_is_accepted() {
    let repo = Repo::new();
    repo.write(".gitattributes", "*.py -diff\n*.mjs binary\n");
    repo.write("scripts/legacy.py", &lines("x = ", 3));
    repo.write("crates/rdf-wasm/js/tests/suite.mjs", &lines("// s", 4));
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn growth_under_every_ratcheted_root_is_refused() {
    let repo = Repo::new();
    for path in [
        "bindings/python/tests/test_legacy.py",
        "crates/demo/tests/oracle.mjs",
        "crates/rdf-wasm/js/tests/suite.mjs",
        "crates/rdf-wasm/js/bench/bench.mjs",
    ] {
        repo.append(path, "// grown\n");
    }
    repo.stage();
    repo.check_staged().assert_refused(&[
        "bindings/python/tests/test_legacy.py: 5 -> 6",
        "crates/demo/tests/oracle.mjs: 5 -> 6",
        "crates/rdf-wasm/js/tests/suite.mjs: 5 -> 6",
        "crates/rdf-wasm/js/bench/bench.mjs: 5 -> 6",
        "Put the new logic in Rust",
    ]);
}

#[test]
fn shipped_index_mjs_growth_is_accepted() {
    let repo = Repo::new();
    repo.append("crates/rdf-wasm/js/index.mjs", &lines("// more", 50));
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn vectors_and_generated_additions_need_no_explanation() {
    let repo = Repo::new();
    repo.write("vectors/vendor/suite.py", "vendored = 1\n");
    repo.write("generated/out.mjs", "export const generated = 1;\n");
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn a_change_without_non_rust_code_is_accepted() {
    let repo = Repo::new();
    repo.write(
        "crates/demo/src/lib.rs",
        "//! Demo, changed.\npub fn f() {}\n",
    );
    repo.write("README.md", "# Demo\n");
    repo.stage();
    repo.check_staged().assert_accepted();
    repo.commit("rust only");
    repo.check_head().assert_accepted();
    repo.check_worktree().assert_accepted();
}

#[test]
fn an_unchanged_legacy_tree_is_accepted() {
    let repo = Repo::new();
    repo.check_staged().assert_accepted();
    repo.check_head().assert_accepted();
    repo.check_worktree().assert_accepted();
}

#[test]
fn an_explained_new_test_file_is_accepted() {
    let repo = Repo::new();
    repo.write(
        "bindings/python/tests/test_new.py",
        &format!("{HASH_REASON}def test_x():\n    pass\n"),
    );
    repo.write(
        "crates/rdf-wasm/js/tests/new.mjs",
        &format!("{SLASH_REASON}export {{}};\n"),
    );
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn a_short_or_misplaced_reason_is_refused() {
    let repo = Repo::new();
    repo.write("tools/short.py", "# Why not Rust: because\nx = 1\n");
    repo.write(
        "tools/late.py",
        &format!("{}{HASH_REASON}", lines("x = ", 40)),
    );
    repo.stage();
    repo.check_staged()
        .assert_refused(&["tools/short.py", "tools/late.py"]);
}

#[test]
fn a_whitespace_only_python_file_needs_no_explanation() {
    let repo = Repo::new();
    repo.write("bindings/python/python/purrdf/sub/__init__.py", "");
    repo.write("tools/blank.py", "\n  \n");
    repo.stage();
    repo.check_staged().assert_accepted();
}

#[test]
fn a_whitespace_only_neighbour_with_code_is_refused() {
    let repo = Repo::new();
    repo.write("bindings/python/python/purrdf/sub/__init__.py", "x = 1\n");
    repo.stage();
    repo.check_staged()
        .assert_refused(&["bindings/python/python/purrdf/sub/__init__.py"]);
}

#[test]
fn the_working_tree_target_sees_unstaged_and_untracked_files() {
    let repo = Repo::new();
    repo.append("scripts/legacy.py", "grown = 1\n");
    repo.write("tools/untracked.py", "x = 1\n");
    repo.write(".gitignore", "ignored/\n");
    repo.write("ignored/scratch.py", "x = 1\n");
    repo.check_worktree()
        .assert_refused(&["scripts/legacy.py: 10 -> 11", "tools/untracked.py"]);
    assert!(
        !repo.check_worktree().stderr.contains("ignored/scratch.py"),
        "an ignored file is not part of the working tree target"
    );
    // Nothing is staged, so the index the hooks check is unchanged and passes.
    repo.check_staged().assert_accepted();
}

#[test]
fn an_explicit_base_compares_against_that_commit() {
    let repo = Repo::new();
    repo.append("scripts/legacy.py", "grown = 1\n");
    repo.commit("grow");
    repo.ratchet(&["--base", "HEAD~1"], "rev:HEAD")
        .assert_refused(&["scripts/legacy.py: 10 -> 11"]);
    repo.ratchet(&["--base", "HEAD"], "rev:HEAD")
        .assert_accepted();
}

#[test]
fn a_missing_integration_ref_is_an_error_that_says_so() {
    let repo = Repo::new();
    repo.ratchet(&["--merge-base-with", "origin/nowhere"], "index")
        .assert_refused(&["origin/nowhere", "does not name a commit"]);
}
