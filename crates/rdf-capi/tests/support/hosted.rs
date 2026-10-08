// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Hosted campaign wiring. All measured selection and validation lives in profile.

use super::{phases::Recorder, profile};
use purrdf_lex::json::{self, Object, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

type IoResult<T> = std::io::Result<T>;
fn invalid(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}
const AFTER: [&str; 8] = [
    "lib",
    "doc",
    "integration-1",
    "integration-2",
    "integration-3",
    "integration-4",
    "capi",
    "downstream",
];
const BEFORE: [&str; 3] = ["monolithic", "capi", "downstream"];

fn selection(case: &str) -> IoResult<(&str, bool)> {
    if case == "profile-before-capi" {
        return Ok(("capi", true));
    }
    let (arm, lane) = case
        .split_once('-')
        .ok_or_else(|| invalid("missing hosted arm"))?;
    if (arm == "before" && BEFORE.contains(&lane)) || (arm == "after" && AFTER.contains(&lane)) {
        Ok((lane, false))
    } else {
        Err(invalid(
            "unsupported hosted arm/lane; full explicit inventory required",
        ))
    }
}
fn write(path: &Path, value: &Value) -> IoResult<()> {
    if path.exists() {
        return Err(invalid("refusing to overwrite hosted evidence"));
    }
    std::fs::write(path, json::write_pretty(value))
}
fn environment(key: &str) -> IoResult<String> {
    std::env::var(key).map_err(|_| invalid(format!("required hosted context {key} missing")))
}
fn campaign_identity(id: &str, attempt: &str) -> IoResult<String> {
    if [&id, &attempt]
        .iter()
        .any(|value| value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(invalid("invalid hosted campaign identity"));
    }
    Ok(format!("{id}-{attempt}"))
}
fn current_identity() -> IoResult<String> {
    campaign_identity(
        &environment("GITHUB_RUN_ID")?,
        &environment("GITHUB_RUN_ATTEMPT")?,
    )
}
fn campaign() -> IoResult<PathBuf> {
    Ok(Path::new("/opt/purrdf-native-profile").join(current_identity()?))
}
fn cases() -> Vec<String> {
    BEFORE
        .iter()
        .map(|lane| format!("before-{lane}"))
        .chain(AFTER.iter().map(|lane| format!("after-{lane}")))
        .chain(std::iter::once("profile-before-capi".to_owned()))
        .collect()
}
fn restore_at(directory: &Path, identity: &str) -> IoResult<()> {
    let prefix = format!("native-profile-{identity}-arm-");
    let downloads = directory.join("downloads");
    for path in [directory, downloads.as_path()] {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| invalid(format!("current-attempt evidence directory unavailable: {error}; rerun the FULL profiling workflow, not failed jobs only")))?;
        if !metadata.file_type().is_dir() {
            return Err(invalid(
                "current-attempt evidence directories must not be symlinks; rerun the FULL profiling workflow",
            ));
        }
    }
    let entries = std::fs::read_dir(&downloads).map_err(|error| invalid(format!("current-attempt arm downloads unavailable: {error}; rerun the FULL profiling workflow, not failed jobs only")))?;
    let mut moves = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| invalid("non-UTF8 artifact name"))?;
        let case = name.strip_prefix(&prefix).ok_or_else(|| invalid(format!("artifact {name} does not belong to current attempt {identity}; rerun the FULL profiling workflow")))?;
        selection(case).map_err(|_| {
            invalid(format!(
                "inadmitted artifact case {case}; rerun the FULL profiling workflow"
            ))
        })?;
        if !entry.file_type()?.is_dir() {
            return Err(invalid(
                "arm artifact must be a directory, never a symlink or file",
            ));
        }
        let destination = directory.join(case);
        match std::fs::symlink_metadata(&destination) {
            Ok(_) => return Err(invalid("refusing to overwrite restored arm evidence")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        moves.push((case.to_owned(), entry.path(), destination));
    }
    let mut expected = cases();
    expected.sort();
    moves.sort_by(|a, b| a.0.cmp(&b.0));
    if moves.iter().map(|item| &item.0).collect::<Vec<_>>() != expected.iter().collect::<Vec<_>>() {
        return Err(invalid(
            "current attempt lacks the exact complete arm inventory; rerun the FULL profiling workflow, not failed jobs only; prior attempts cannot substitute",
        ));
    }
    // Validate every name/case/destination before moving any evidence.
    for (_, source, destination) in moves {
        std::fs::rename(source, destination)?;
    }
    Ok(())
}
pub(crate) fn restore(directory: &Path) -> IoResult<()> {
    if directory != campaign()? {
        return Err(invalid("restore path is not the current campaign"));
    }
    restore_at(directory, &current_identity()?)
}
fn counterfactual(text: &str) -> IoResult<String> {
    let needle = "let profile = if cfg!(debug_assertions) {\n        \"test\"";
    if text.matches(needle).count() != 1 {
        return Err(invalid("exact current nested test profile branch missing"));
    }
    Ok(text.replacen(
        needle,
        "let profile = if cfg!(debug_assertions) {\n        \"dev\"",
        1,
    ))
}

fn hardware(cpu: &str, memory: &str) -> IoResult<Value> {
    let mut values = Vec::new();
    let mut processors = 0_u64;
    for line in cpu.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            if key == "processor" {
                processors += 1;
            }
            if [
                "vendor_id",
                "cpu family",
                "model",
                "model name",
                "stepping",
                "flags",
                "cpu cores",
                "siblings",
                "cache size",
            ]
            .contains(&key)
            {
                values.push(format!("{key}:{}", value.trim()));
            }
        }
    }
    values.sort();
    values.dedup();
    let total = memory
        .lines()
        .find(|line| line.starts_with("MemTotal:"))
        .ok_or_else(|| invalid("observed physical memory missing"))?;
    if processors == 0 || values.is_empty() {
        return Err(invalid("observed CPU identity missing"));
    }
    Ok(Object::new()
        .with("cpu_identity", values)
        .with("logical_processors", processors)
        .with("physical_memory", total)
        .into())
}

fn workflow_contract(text: &str) -> IoResult<()> {
    let workflow = purrdf_lex::yaml::read(text).map_err(|error| invalid(error.to_string()))?;
    if workflow
        .pointer("/on/workflow_dispatch/inputs/native_profile/default")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(invalid("profiling dispatch must remain opt-in"));
    }
    let jobs = workflow
        .get("jobs")
        .ok_or_else(|| invalid("workflow jobs missing"))?;
    for job in [
        "workspace",
        "test-shard",
        "test",
        "capi",
        "native-profile-admission",
        "native-profile",
        "native-profile-comparison",
    ] {
        if jobs.get(job).is_none() {
            return Err(invalid(format!("required workflow job {job} missing")));
        }
    }
    let workflow_cases = jobs
        .pointer("/native-profile/strategy/matrix/case")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("profiling matrix missing"))?;
    let mut actual = workflow_cases
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| invalid("non-string hosted case"))
        })
        .collect::<IoResult<Vec<_>>>()?;
    actual.sort_unstable();
    let mut expected = cases();
    expected.sort();
    if actual != expected {
        return Err(invalid(
            "hosted matrix loses or duplicates an admitted comparison arm",
        ));
    }
    for job in [
        "native-profile-admission",
        "native-profile",
        "native-profile-comparison",
    ] {
        let steps = jobs
            .get(job)
            .and_then(|job| job.get("steps"))
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("hosted steps missing"))?;
        if !steps.iter().any(|step| {
            step.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
                && step.get("if").and_then(Value::as_str) == Some("always()")
        }) {
            return Err(invalid(format!(
                "{job} lacks failure-preserving evidence upload"
            )));
        }
    }
    let prefix = "native-profile-${{ github.run_id }}-${{ github.run_attempt }}";
    for (job, action, key, expected) in [
        (
            "native-profile-admission",
            "actions/upload-artifact@",
            "name",
            format!("{prefix}-admission"),
        ),
        (
            "native-profile",
            "actions/download-artifact@",
            "name",
            format!("{prefix}-admission"),
        ),
        (
            "native-profile",
            "actions/upload-artifact@",
            "name",
            format!("{prefix}-arm-${{{{ matrix.case }}}}"),
        ),
        (
            "native-profile-comparison",
            "actions/download-artifact@",
            "pattern",
            format!("{prefix}-arm-*"),
        ),
        (
            "native-profile-comparison",
            "actions/upload-artifact@",
            "name",
            format!("{prefix}-comparison"),
        ),
    ] {
        let steps = jobs
            .get(job)
            .and_then(|job| job.get("steps"))
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("artifact steps missing"))?;
        if !steps.iter().any(|step| {
            step.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.starts_with(action))
                && step
                    .get("with")
                    .and_then(|inputs| inputs.get(key))
                    .and_then(Value::as_str)
                    == Some(expected.as_str())
        }) {
            return Err(invalid(format!(
                "{job} must isolate {key} to the current run/attempt"
            )));
        }
    }
    Ok(())
}

fn cargo_c_version(stdout: &str) -> IoResult<()> {
    let release = stdout
        .trim()
        .strip_prefix("cargo-c ")
        .ok_or_else(|| invalid("cargo-capi did not report its actual cargo-c version"))?;
    if !release.starts_with("0.10.23+") || release.contains(char::is_whitespace) {
        return Err(invalid(
            "cargo-capi version differs from the pinned 0.10.23 header generator",
        ));
    }
    Ok(())
}

fn executable(program: &str) -> IoResult<PathBuf> {
    let path = std::env::var_os("PATH").ok_or_else(|| invalid("PATH missing"))?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| invalid(format!("required executable {program} missing")))
}

pub(crate) fn run(case: &str) -> IoResult<()> {
    let (lane, legacy) = selection(case)?;
    let campaign = campaign()?;
    let directory = campaign.join(case);
    std::fs::create_dir_all(&directory)?;
    let original = std::fs::canonicalize(environment("GITHUB_WORKSPACE")?)?;
    workflow_contract(&std::fs::read_to_string(
        original.join(".github/workflows/ci.yaml"),
    )?)?;
    let cpu = std::fs::read_to_string("/proc/cpuinfo")?;
    let memory = std::fs::read_to_string("/proc/meminfo")?;
    let hardware = hardware(&cpu, &memory)?;
    let hardware_digest = purrdf_hash::hex::encode(
        purrdf_hash::blake3::hash(json::write_compact(&hardware).as_bytes()).as_bytes(),
    );
    std::fs::write(directory.join("actual-cpuinfo.txt"), cpu)?;
    std::fs::write(directory.join("actual-memory-before.txt"), memory)?;
    let setup: Value = Object::new().with("case", case)
        .with("setup_started_unix_ns", std::fs::read_to_string(campaign.join("setup-start.txt"))?.trim())
        .with("image_os", environment("ImageOS")?).with("image_version", environment("ImageVersion")?)
        .with("runner_os", environment("RUNNER_OS")?).with("runner_arch", environment("RUNNER_ARCH")?)
        .with("observed_hardware", hardware)
        .with("dependency_cache", "no artifact cache restored; registry/download cache may exist")
        .with("compiler_cache", "no compiler cache requested; wrappers/configuration captured by controller")
        .with("page_cache", "uncontrolled hosted OS page cache; cold refers ONLY to empty Cargo target/build directories")
        .into();
    let mut recorder = Recorder::new(directory.join("setup.json"), setup)?;
    let filesystem = recorder.run(
        "arm-filesystem",
        Command::new("stat")
            .args(["-f", "-c", "%T"])
            .arg(&directory),
    )?;
    recorder.check("disk-backed-arm", || {
        let kind = std::str::from_utf8(&filesystem.stdout)
            .map_err(|error| invalid(error.to_string()))?
            .trim();
        if kind.is_empty() || matches!(kind, "tmpfs" | "ramfs") {
            return Err(invalid("hosted arm must reside on disk, not RAM"));
        }
        Ok(())
    })?;
    recorder.check("admitted-compiler", || {
        let actual = Command::new("rustc").args(["-vV"]).output()?;
        if !actual.status.success()
            || actual.stdout != std::fs::read(campaign.join("admitted-rustc.txt"))?
        {
            return Err(invalid(
                "hosted runner compiler differs from campaign admission",
            ));
        }
        Ok(())
    })?;
    let mut prerequisite_tools = Vec::new();
    for (name, program, arguments) in [
        ("python", "python3", vec!["--version"]),
        ("node", "node", vec!["--version"]),
        ("binaryen", "wasm-opt", vec!["--version"]),
        ("cargo-c", "cargo-capi", vec!["--version"]),
    ] {
        let path = executable(program)?;
        let observed = recorder.run(name, Command::new(&path).args(arguments))?;
        let stdout =
            String::from_utf8(observed.stdout).map_err(|error| invalid(error.to_string()))?;
        if name == "cargo-c" {
            recorder.check("pinned-cargo-c-version", || cargo_c_version(&stdout))?;
        }
        recorder.evidence(
            &format!("{name}-executable"),
            super::phases::identity(&path)?,
        )?;
        prerequisite_tools.push(
            Object::new()
                .with("name", name)
                .with("stdout", stdout)
                .with(
                    "stderr",
                    String::from_utf8(observed.stderr)
                        .map_err(|error| invalid(error.to_string()))?,
                ),
        );
    }
    let prerequisite_tools: Value = prerequisite_tools
        .into_iter()
        .map(Value::from)
        .collect::<Vec<_>>()
        .into();
    let prerequisite_digest = purrdf_hash::hex::encode(
        purrdf_hash::blake3::hash(json::write_compact(&prerequisite_tools).as_bytes()).as_bytes(),
    );
    recorder.evidence("observed_prerequisite_tools", prerequisite_tools)?;
    let root = if legacy {
        let tree = campaign.join("nested-dev-source");
        recorder.run(
            "counterfactual-copy",
            Command::new("git")
                .args(["clone", "--shared", "--no-checkout"])
                .arg(&original)
                .arg(&tree),
        )?;
        recorder.run(
            "counterfactual-checkout",
            Command::new("git")
                .args(["checkout", "--detach"])
                .arg(environment("GITHUB_SHA")?)
                .current_dir(&tree),
        )?;
        let path = tree.join("crates/rdf-capi/tests/c_smoke.rs");
        recorder.check("exact-counterfactual", || {
            std::fs::write(&path, counterfactual(&std::fs::read_to_string(&path)?)?)
        })?;
        let output = Command::new("git")
            .args(["diff", "--binary", "--", "crates/rdf-capi/tests/c_smoke.rs"])
            .current_dir(&tree)
            .output()?;
        if !output.status.success() {
            return Err(invalid("counterfactual diff failed"));
        }
        std::fs::write(directory.join("counterfactual.diff"), output.stdout)?;
        tree
    } else {
        original
    };
    let cargo = environment("PURRDF_HOSTED_REAL_CARGO")?;
    let parallelism = std::thread::available_parallelism()?.get().min(8) as u64;
    recorder.evidence(
        "concurrency",
        Object::new()
            .with(
                "observed_available_parallelism",
                std::thread::available_parallelism()?.get() as u64,
            )
            .with("effective_cargo_jobs", parallelism)
            .with("effective_libtest_threads", parallelism)
            .into(),
    )?;
    recorder.evidence("setup_completed_unix_ns", unix_ns()?.into())?;
    for warmth in ["cold", "warm"] {
        let mut request = Object::new().with("root", root.display().to_string()).with("directory", directory.display().to_string())
            .with("cargo", cargo.clone()).with("lane", lane).with("warmth", warmth)
            .with("jobs", parallelism).with("test_threads", parallelism)
            .with("runner_class", format!("{}:{}:{}:{}:{hardware_digest}:{prerequisite_digest}", environment("ImageOS")?, environment("ImageVersion")?, environment("RUNNER_OS")?, environment("RUNNER_ARCH")?))
            .with("dependency_cache", "no artifact cache restored; registry/download cache may exist")
            .with("compiler_cache", "no compiler cache requested; wrappers/configuration captured by controller")
            .with("page_cache", "uncontrolled hosted OS page cache; cold refers ONLY to empty Cargo target/build directories");
        if warmth == "warm" {
            request.insert(
                "previous",
                directory
                    .join(format!("{lane}-cold-receipt.json"))
                    .display()
                    .to_string(),
            );
        }
        let path = directory.join(format!("request-{warmth}.json"));
        write(&path, &request.into())?;
        write(&directory.join(format!("corpus-state-{warmth}.json")), &Object::new()
            .with("xmlconf_present_before", root.join("target/conformance/xmlconf").exists())
            .with("scope", "the existing native XML fixture owns corpus acquisition/cache outside Cargo artifact directories; cold/warm never substitutes corpus existence for its verified acquisition")
            .into())?;
        std::fs::write(
            directory.join(format!("actual-memory-{warmth}-before.txt")),
            std::fs::read("/proc/meminfo")?,
        )?;
        profile::run(&path)?;
        std::fs::write(
            directory.join(format!("actual-memory-{warmth}-after.txt")),
            std::fs::read("/proc/meminfo")?,
        )?;
    }
    Ok(())
}

pub(crate) fn unix_ns() -> IoResult<String> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invalid(error.to_string()))?
        .as_nanos()
        .to_string())
}

fn execution_window(paths: &[String]) -> IoResult<Value> {
    let mut first = u128::MAX;
    let mut last = 0;
    let mut windows: Vec<Value> = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(path)?;
        let receipt = json::read(&text).map_err(|error| invalid(error.to_string()))?;
        let interval = receipt
            .pointer("/context/execution_window")
            .ok_or_else(|| invalid("actual hosted execution timestamps missing"))?;
        let time = |key: &str| {
            interval
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("execution timestamp missing"))?
                .parse::<u128>()
                .map_err(|error| invalid(error.to_string()))
        };
        let start = time("started_unix_ns")?;
        let finish = time("finished_unix_ns")?;
        if finish < start {
            return Err(invalid("host wall clock moved backwards"));
        }
        first = first.min(start);
        last = last.max(finish);
        windows.push(
            Object::new()
                .with("receipt", path.as_str())
                .with("interval", interval.clone())
                .into(),
        );
    }
    if windows.is_empty() {
        return Err(invalid("empty hosted execution window"));
    }
    Ok(Object::new().with("lanes", windows).with("observed_wall_window_ns", (last - first).to_string())
        .with("interpretation", "observed first-start to last-finish window includes runner scheduling; arms are not a synchronized barrier experiment; cross-runner clock synchronization is not independently certified; do not infer speedup from this window")
        .into())
}

pub(crate) fn compare(directory: &Path) -> IoResult<()> {
    if directory != campaign()? {
        return Err(invalid(
            "downloaded campaign path differs from original evidence paths",
        ));
    }
    for warmth in ["cold", "warm"] {
        for (change, before, after) in [
            (
                "native-partition",
                BEFORE
                    .iter()
                    .map(|lane| (format!("before-{lane}"), *lane))
                    .collect::<Vec<_>>(),
                AFTER
                    .iter()
                    .map(|lane| (format!("after-{lane}"), *lane))
                    .collect::<Vec<_>>(),
            ),
            (
                "nested-profile",
                vec![("profile-before-capi".to_owned(), "capi")],
                vec![("after-capi".to_owned(), "capi")],
            ),
        ] {
            let paths = |items: &[(String, &str)]| {
                items
                    .iter()
                    .map(|(case, lane)| {
                        directory
                            .join(case)
                            .join(format!("{lane}-{warmth}-receipt.json"))
                            .display()
                            .to_string()
                    })
                    .collect::<Vec<_>>()
            };
            let policy = Object::new()
                .with("before", paths(&before))
                .with("after", paths(&after))
                .with("change", change)
                .with(
                    "output",
                    directory
                        .join(format!("{change}-{warmth}-comparison.json"))
                        .display()
                        .to_string(),
                );
            let input = directory.join(format!("{change}-{warmth}-policy.json"));
            write(&input, &policy.into())?;
            profile::compare(&input)?;
            // Only emit scheduling observations AFTER the shared validator has
            // admitted all source/config/tool/cache/coverage/phase inputs.
            write(
                &directory.join(format!("{change}-{warmth}-execution-comparison.json")),
                &Object::new()
                    .with("before", execution_window(&paths(&before))?)
                    .with("after", execution_window(&paths(&after))?)
                    .into(),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hosted_inventory_and_exact_counterfactual_are_closed() {
        assert!(run("after-unknown").is_err());
        assert!(compare(Path::new("/tmp/unsupported-campaign")).is_err());
        assert!(restore(Path::new("/tmp/unsupported-campaign")).is_err());
        for lane in BEFORE {
            assert_eq!(selection(&format!("before-{lane}")).unwrap(), (lane, false));
        }
        for lane in AFTER {
            assert_eq!(selection(&format!("after-{lane}")).unwrap(), (lane, false));
        }
        assert_eq!(selection("profile-before-capi").unwrap(), ("capi", true));
        for bad in [
            "before-lib",
            "after-monolithic",
            "after",
            "after-capi-extra",
            "profile-after-capi",
        ] {
            assert!(selection(bad).is_err());
        }
        let current = "x\nlet profile = if cfg!(debug_assertions) {\n        \"test\"\n} y";
        let old = counterfactual(current).unwrap();
        assert_eq!(old.replace("\"dev\"", "\"test\""), current);
        assert!(counterfactual(&old).is_err());
        assert!(counterfactual(&format!("{current}{current}")).is_err());
    }
    #[test]
    fn physical_runner_identity_ignores_frequency_but_refuses_missing_observations() {
        let cpu = "processor : 0\nmodel name : actual-model\nflags : sse sse2\ncpu MHz : 2000\n";
        let memory = "MemTotal: 1000 kB\nCached: 50 kB\n";
        let admitted = hardware(cpu, memory).unwrap();
        assert_eq!(
            admitted,
            hardware(&cpu.replace("2000", "3000"), &memory.replace("50", "80")).unwrap()
        );
        assert_ne!(
            admitted,
            hardware(&cpu.replace("actual-model", "other-model"), memory).unwrap()
        );
        assert_ne!(
            admitted,
            hardware(cpu, &memory.replace("1000", "2000")).unwrap()
        );
        assert!(hardware("", memory).is_err());
        assert!(hardware(cpu, "Cached: 50 kB").is_err());
    }
    #[test]
    fn actual_workflow_preserves_full_optional_inventory_and_failure_uploads() {
        let text = include_str!("../../../../.github/workflows/ci.yaml");
        workflow_contract(text).unwrap();
        assert!(workflow_contract(&text.replace("after-integration-4, ", "")).is_err());
        assert!(workflow_contract(&text.replace("if: always()", "if: success()")).is_err());
        assert!(workflow_contract(&text.replacen("native_profile:\n        description: Capture matched native compilation and C phase cold/warm arms\n        type: boolean\n        default: false", "native_profile:\n        type: boolean\n        default: true", 1)).is_err());
        for (current, stale) in [
            (
                "name: native-profile-${{ github.run_id }}-${{ github.run_attempt }}-admission",
                "name: native-profile-admission",
            ),
            (
                "pattern: native-profile-${{ github.run_id }}-${{ github.run_attempt }}-arm-*",
                "pattern: native-profile-*",
            ),
            (
                "name: native-profile-${{ github.run_id }}-${{ github.run_attempt }}-arm-${{ matrix.case }}",
                "name: native-profile-${{ matrix.case }}",
            ),
            (
                "name: native-profile-${{ github.run_id }}-${{ github.run_attempt }}-comparison",
                "name: native-profile-comparison",
            ),
        ] {
            assert!(workflow_contract(&text.replace(current, stale)).is_err());
        }
    }
    #[test]
    fn cargo_c_identity_refuses_successful_generic_help_and_wrong_release() {
        cargo_c_version("cargo-c 0.10.23+cargo-0.97.0\n").unwrap();
        for output in [
            "cargo-c 0.10.22+cargo-0.96.0",
            "cargo-c\nUsage: cargo capi",
            "cargo-c 0.10.23\nUsage: cargo capi",
            "cargo-c 0.10.230+cargo-0.97.0",
            "",
        ] {
            assert!(cargo_c_version(output).is_err());
        }
    }
    #[test]
    fn execution_windows_refuse_missing_and_reversed_clocks() {
        let temporary = purrdf_testkit::temp_dir!().unwrap();
        let path = temporary.path().join("receipt.json");
        let paths = vec![path.display().to_string()];
        std::fs::write(
            &path,
            r#"{"context":{"execution_window":{"started_unix_ns":"10","finished_unix_ns":"20"}}}"#,
        )
        .unwrap();
        let observed = execution_window(&paths).unwrap();
        assert_eq!(
            observed
                .get("observed_wall_window_ns")
                .and_then(Value::as_str),
            Some("10")
        );
        std::fs::write(
            &path,
            r#"{"context":{"execution_window":{"started_unix_ns":"20","finished_unix_ns":"10"}}}"#,
        )
        .unwrap();
        assert!(execution_window(&paths).is_err());
        std::fs::write(&path, "{}").unwrap();
        assert!(execution_window(&paths).is_err());
        assert!(execution_window(&[]).is_err());
    }
    fn downloaded_fixture(root: &Path, identity: &str) {
        let downloads = root.join("downloads");
        std::fs::create_dir_all(&downloads).unwrap();
        for case in cases() {
            let path = downloads.join(format!("native-profile-{identity}-arm-{case}"));
            std::fs::create_dir(&path).unwrap();
            std::fs::write(path.join("sentinel"), format!("{identity}:{case}")).unwrap();
        }
    }
    #[test]
    fn restores_only_complete_current_attempt_and_preserves_prior_evidence() {
        let temporary = purrdf_testkit::temp_dir!().unwrap();
        let old = temporary
            .path()
            .join(campaign_identity("314", "1").unwrap());
        let current = temporary
            .path()
            .join(campaign_identity("314", "2").unwrap());
        downloaded_fixture(&old, "314-1");
        downloaded_fixture(&current, "314-2");
        restore_at(&current, "314-2").unwrap();
        for case in cases() {
            assert_eq!(
                std::fs::read_to_string(current.join(&case).join("sentinel")).unwrap(),
                format!("314-2:{case}")
            );
            assert_eq!(
                std::fs::read_to_string(
                    old.join("downloads")
                        .join(format!("native-profile-314-1-arm-{case}"))
                        .join("sentinel")
                )
                .unwrap(),
                format!("314-1:{case}")
            );
        }
        assert!(restore_at(&current, "314-2").is_err());
        for bad in [("314", ""), ("314", "2/../1"), ("", "2")] {
            assert!(campaign_identity(bad.0, bad.1).is_err());
        }
    }
    #[test]
    fn restored_destinations_are_never_overwritten() {
        let temporary = purrdf_testkit::temp_dir!().unwrap();
        downloaded_fixture(temporary.path(), "314-2");
        let existing = temporary.path().join("after-lib");
        std::fs::create_dir(&existing).unwrap();
        std::fs::write(existing.join("sentinel"), "preserved").unwrap();
        assert!(restore_at(temporary.path(), "314-2").is_err());
        assert_eq!(
            std::fs::read_to_string(existing.join("sentinel")).unwrap(),
            "preserved"
        );
        for case in cases() {
            assert!(
                temporary
                    .path()
                    .join("downloads")
                    .join(format!("native-profile-314-2-arm-{case}"))
                    .exists()
            );
        }
    }
    #[cfg(unix)]
    #[test]
    fn restoration_never_follows_an_alias_to_prior_evidence() {
        let temporary = purrdf_testkit::temp_dir!().unwrap();
        let prior = temporary.path().join("314-1");
        downloaded_fixture(&prior, "314-2");
        let current = temporary.path().join("314-2");
        std::fs::create_dir(&current).unwrap();
        std::os::unix::fs::symlink(prior.join("downloads"), current.join("downloads")).unwrap();
        assert!(restore_at(&current, "314-2").is_err());
        for case in cases() {
            assert!(
                prior
                    .join("downloads")
                    .join(format!("native-profile-314-2-arm-{case}"))
                    .exists()
            );
            assert!(!current.join(case).exists());
        }
    }
    #[test]
    fn stale_wrong_prefix_wrong_case_and_partial_attempt_refuse_before_any_move() {
        for (seed, extra, missing) in [
            ("314-1", None, false),
            ("314-2", Some("native-profile-314-1-arm-after-lib"), false),
            ("314-2", Some("native-profile-314-2-arm-admission"), false),
            ("314-2", Some("wrong-prefix-314-2-arm-after-lib"), false),
            ("314-2", None, true),
        ] {
            let temporary = purrdf_testkit::temp_dir!().unwrap();
            downloaded_fixture(temporary.path(), seed);
            if let Some(extra) = extra {
                std::fs::create_dir(temporary.path().join("downloads").join(extra)).unwrap();
            }
            if missing {
                std::fs::remove_dir_all(
                    temporary
                        .path()
                        .join("downloads/native-profile-314-2-arm-after-lib"),
                )
                .unwrap();
            }
            let error = restore_at(temporary.path(), "314-2")
                .unwrap_err()
                .to_string();
            assert!(error.contains("FULL profiling workflow"), "{error}");
            for case in cases() {
                assert!(!temporary.path().join(case).exists());
            }
            assert!(temporary.path().join("downloads").exists());
        }
    }
}
