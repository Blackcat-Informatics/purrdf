# Narrow API qualification preparation

Prepared, NOT RUN. Installed `cargo semver-checks check-release --help` actually exited0; installed version0.50.0. Help confirms explicit `--package`, `--baseline-rev`, `--release-type minor`, `--current-rustdoc` and `--baseline-rustdoc`. Reading help/version executed no crate build.

Latest accepted final PR body describes baseline origin/main for six packages (not comparator's default published-release baseline). Root's exact admitted main baseline is `2aa091423dab31a39bf2415abaec9d5c07b311e5`. Use this pinned ref to keep the actual candidate/base interaction explicit. Original five-package historical baseline `ab09fcaad8d0f393393c5bb77ad1174c28ba68c4` remains attributable history, not the latest combined comparator input. No original actual semver artifacts were recovered.

On root admission, use prescribed active SDK and eight Cargo jobs; do not install, change channels or pins. Candidate is the current 406 worktree after root's ordinary main composition. Run comparator package selections, either together or individually while preserving each package's actual output:

```sh
cargo semver-checks check-release -p purrdf-core -p purrdf-lex -p purrdf-shapes -p purrdf-shex -p purrdf-sparql-eval --baseline-rev 2aa091423dab31a39bf2415abaec9d5c07b311e5 --release-type minor
```

Validate's historical wrapper automatic-build baseline path refusal is not a comparator pass. If that constraint remains, use the existing supported ordinary same-active-SDK Rustdoc JSON route. Build candidate and exact baseline with distinct task-owned targets and source snapshots; root prepares the baseline snapshot without mutating protected main. Set CARGO_TARGET_DIR separately per source before each command (the active SDK route supports private target dirs; the Stage wrapper may rewrite them):

```sh
cargo rustdoc --locked -j 8 -p purrdf-validate --lib -- -Z unstable-options --output-format json -D warnings
```

Execute once from current candidate and once from exact `2aa091423` baseline source, preserving complete terminal results and actual emitted `<target>/doc/purrdf_validate.json`. The explicit comparison, from candidate, is:

```sh
cargo semver-checks check-release --current-rustdoc <candidate-target>/doc/purrdf_validate.json --baseline-rustdoc <baseline-target>/doc/purrdf_validate.json --release-type minor
```

Replace placeholders with the actually emitted files, not presumed filenames from an old archive. Bind both inputs to actual source/build receipts. If five-package automatic comparator encounters the same wrapper lookup admission, preserve its refusal and apply the same supported explicit-JSON route for those named crates, one package per JSON comparison. Native crate names are `purrdf_core`, `purrdf_lex`, `purrdf_shapes`, `purrdf_shex`, `purrdf_sparql_eval`, `purrdf_validate`.

Record actual checks/skips, result, profile/compiler and current/base source scope per package; skips are not passed tests. Do not claim the previous asserted 223/31 counts establish this candidate's result. A changed origin/main or later source patch requires explicit delta assessment, not relabeling these planned inputs.
