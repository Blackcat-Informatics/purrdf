// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Hosted campaign wiring. All measured selection and validation lives in profile.

use super::{
    phases::{Recorder, invalid},
    profile,
};
use purrdf_lex::json::{self, Object, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

type IoResult<T> = std::io::Result<T>;
const AFTER: [&str; 9] = [
    "lib",
    "doc",
    "integration-1",
    "integration-2",
    "integration-3",
    "integration-4",
    "integration-5",
    "capi",
    "downstream",
];
const BEFORE: [&str; 3] = ["monolithic", "capi", "downstream"];
const EXECUTION: &str = r#"set -o pipefail
campaign="/opt/purrdf-native-profile/$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT"
mkdir -p "$campaign/$PROFILE_CASE"
cp "$campaign/setup-start.txt" "$campaign/$PROFILE_CASE/setup-start.txt"
cp "$campaign/resources/setup-disk-before.txt" "$campaign/$PROFILE_CASE/setup-disk-before.txt"
cp "$campaign/resources/setup-disk-after.txt" "$campaign/$PROFILE_CASE/setup-disk-after.txt"
cp "$campaign/resources/actual-rustc.txt" "$campaign/$PROFILE_CASE/actual-rustc.txt"
cp "$campaign/resources/actual-cargo.txt" "$campaign/$PROFILE_CASE/actual-cargo.txt"
"$PURRDF_HOSTED_CONTROLLER" hosted-run "$PROFILE_CASE" 2>&1 | tee "$campaign/$PROFILE_CASE/hosted-run.txt""#;
const RECLAMATION: &str = "\"$PURRDF_HOSTED_CONTROLLER\" hosted-reclaim \"$PROFILE_CASE\"";
const FIXTURE_SETUP: &str = "sudo mkdir -p /opt/purrdf-native-profile-tests\nsudo chown \"$(id -u):$(id -g)\" /opt/purrdf-native-profile-tests";

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
    serial_workflow(jobs)?;
    let steps = jobs
        .pointer("/test-shard/steps")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("native shard steps missing"))?;
    let execution = steps
        .iter()
        .position(|step| {
            step.get("run").and_then(Value::as_str)
                == Some("make test-shard SHARD=${{ matrix.shard }}")
        })
        .ok_or_else(|| invalid("native shard execution missing"))?;
    let provision = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| {
            step.get("run")
                .and_then(Value::as_str)
                .is_some_and(|run| run.trim() == FIXTURE_SETUP)
        })
        .collect::<Vec<_>>();
    if provision.len() != 1
        || provision[0].0 >= execution
        || provision[0].1.get("if").and_then(Value::as_str)
            != Some("matrix.shard == 'integration-5'")
    {
        return Err(invalid(
            "one owned fixture parent setup must precede only the native integration-5 shard",
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

fn serial_workflow(jobs: &Value) -> IoResult<()> {
    let job = jobs
        .get("native-profile")
        .ok_or_else(|| invalid("measured job missing"))?;
    if job.get("strategy").is_some()
        || job.get("timeout-minutes").and_then(Value::as_u64) != Some(360)
        || job.get("needs").and_then(Value::as_str) != Some("native-profile-admission")
    {
        return Err(invalid(
            "all measured cases require one bounded admitted runner",
        ));
    }
    for (name, candidate) in jobs
        .as_object()
        .ok_or_else(|| invalid("workflow jobs must be an object"))?
        .iter()
    {
        if name != "native-profile"
            && candidate
                .get("steps")
                .and_then(Value::as_array)
                .is_some_and(|steps| {
                    steps
                        .iter()
                        .any(|step| step.pointer("/env/PROFILE_CASE").is_some())
                })
        {
            return Err(invalid("measured cases cannot move to another runner"));
        }
    }
    let steps = job
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("measured steps missing"))?;
    if steps.iter().any(|step| {
        step.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|uses| uses.starts_with("actions/cache@"))
    }) {
        return Err(invalid(
            "measured runner must not restore artifact/compiler caches",
        ));
    }
    let setup = steps
        .iter()
        .filter(|step| step.get("id").and_then(Value::as_str) == Some("profile-setup"))
        .collect::<Vec<_>>();
    if setup.len() != 1 || !setup[0].get("run").and_then(Value::as_str).is_some_and(|run| {
        [
            "CARGO_TARGET_DIR=\"$campaign/controller-target\" CARGO_BUILD_BUILD_DIR=\"$campaign/controller-build\"",
            "cargo build --locked -p purrdf-capi --example native_ci_profile --profile test --jobs 8",
            "date +%s%N > \"$campaign/resources/setup-finished.txt\"",
        ].iter().all(|required| run.contains(required))
    }) {
        return Err(invalid("one shared controller build and setup endpoint must precede all measured cases"));
    }
    let setup_index = steps
        .iter()
        .position(|step| step.get("id").and_then(Value::as_str) == Some("profile-setup"))
        .ok_or_else(|| invalid("shared setup missing"))?;
    let fixture_setup = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| {
            step.get("run")
                .and_then(Value::as_str)
                .is_some_and(|run| run.contains(FIXTURE_SETUP))
        })
        .collect::<Vec<_>>();
    if fixture_setup.len() != 1
        || fixture_setup[0].0 >= setup_index
        || fixture_setup[0].1.get("if").is_some()
    {
        return Err(invalid(
            "owned fixture parent setup must be inside shared setup before measurements",
        ));
    }
    let fixture_run = fixture_setup[0]
        .1
        .get("run")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("fixture setup command missing"))?;
    let start = fixture_run
        .find("date +%s%N > \"$campaign/setup-start.txt\"")
        .ok_or_else(|| invalid("fixture provisioning must follow the observed setup start"))?;
    if start
        >= fixture_run
            .find(FIXTURE_SETUP)
            .ok_or_else(|| invalid("fixture provisioning missing"))?
    {
        return Err(invalid(
            "fixture provisioning must follow the observed setup start",
        ));
    }
    if steps[..=setup_index]
        .iter()
        .any(|step| step.pointer("/env/PROFILE_CASE").is_some())
    {
        return Err(invalid(
            "shared setup must complete before the first measured case",
        ));
    }
    let prefix = "native-profile-${{ github.run_id }}-${{ github.run_attempt }}";
    if !steps.iter().any(|step| {
        step.get("uses").and_then(Value::as_str).is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
            && step.get("if").and_then(Value::as_str) == Some("always()")
            && step.pointer("/with/name").and_then(Value::as_str) == Some(format!("{prefix}-resources").as_str())
            && step.pointer("/with/path").and_then(Value::as_str) == Some("/opt/purrdf-native-profile/${{ github.run_id }}-${{ github.run_attempt }}/resources/")
            && step.pointer("/with/if-no-files-found").and_then(Value::as_str) == Some("error")
    }) {
        return Err(invalid("shared setup and post-upload reclamation evidence must be retained"));
    }
    let mut actual = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let Some(case) = step.pointer("/env/PROFILE_CASE") else {
            continue;
        };
        let case = case
            .as_str()
            .ok_or_else(|| invalid("non-string hosted case"))?;
        let run = step.get("run").and_then(Value::as_str);
        if run == Some(RECLAMATION) {
            if index < 2
                || steps[index - 2]
                    .get("run")
                    .and_then(Value::as_str)
                    .map(str::trim_end)
                    != Some(EXECUTION)
            {
                return Err(invalid("reclamation must follow execution and its upload"));
            }
            continue;
        }
        if run.map(str::trim_end) != Some(EXECUTION) || step.get("if").is_some() {
            return Err(invalid(
                "measured case must execute the exact production recipe",
            ));
        }
        selection(case)?;
        actual.push(case);
        let upload = steps
            .get(index + 1)
            .ok_or_else(|| invalid("case upload missing"))?;
        let reclaim = steps
            .get(index + 2)
            .ok_or_else(|| invalid("case reclamation missing"))?;
        let upload_id = format!("upload_{}", case.replace('-', "_"));
        let artifact = format!("{prefix}-arm-{case}");
        let retained_paths = ["*.json", "*.txt", "*.diff", "*-cargo/*.json", "*-cargo/*.html"]
            .map(|suffix| format!("/opt/purrdf-native-profile/${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}/{case}/{suffix}"))
            .join("\n");
        if upload.get("id").and_then(Value::as_str) != Some(upload_id.as_str())
            || !upload
                .get("uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
            || upload.get("if").and_then(Value::as_str) != Some("always()")
            || upload.pointer("/with/name").and_then(Value::as_str) != Some(artifact.as_str())
            || upload
                .pointer("/with/path")
                .and_then(Value::as_str)
                .map(str::trim_end)
                != Some(retained_paths.as_str())
            || upload
                .pointer("/with/if-no-files-found")
                .and_then(Value::as_str)
                != Some("error")
            || reclaim.get("run").and_then(Value::as_str) != Some(RECLAMATION)
            || reclaim.pointer("/env/PROFILE_CASE").and_then(Value::as_str) != Some(case)
            || reclaim.get("if").is_some()
        {
            return Err(invalid(
                "case requires its exact failure-preserving upload then reclamation",
            ));
        }
        for (key, output) in [
            ("PURRDF_HOSTED_UPLOAD_ID", "artifact-id"),
            ("PURRDF_HOSTED_UPLOAD_DIGEST", "artifact-digest"),
        ] {
            let binding = format!("${{{{ steps.{upload_id}.outputs.{output} }}}}");
            if reclaim
                .get("env")
                .and_then(|env| env.get(key))
                .and_then(Value::as_str)
                != Some(binding.as_str())
            {
                return Err(invalid(
                    "reclamation must bind the actual successful upload outputs",
                ));
            }
        }
    }
    if actual != cases().iter().map(String::as_str).collect::<Vec<_>>() {
        return Err(invalid(
            "serial workflow loses, duplicates or reorders a measured case",
        ));
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

fn shared_setup(campaign: &Path) -> IoResult<Value> {
    let resources = campaign.join("resources");
    let mut endpoints = Vec::new();
    let mut files = Vec::new();
    for name in ["setup-start.txt", "setup-finished.txt"] {
        let path = resources.join(name);
        endpoints.push(
            std::fs::read_to_string(&path)?
                .trim()
                .parse::<u128>()
                .map_err(|error| invalid(error.to_string()))?,
        );
        files.push(super::phases::identity(&path)?);
    }
    if endpoints[0] > endpoints[1] {
        return Err(invalid("shared setup wall clock moved backwards"));
    }
    for name in [
        "setup-disk-before.txt",
        "setup-disk-after.txt",
        "actual-rustc.txt",
        "actual-cargo.txt",
    ] {
        files.push(super::phases::identity(&resources.join(name))?);
    }
    Ok(Object::new()
        .with("started_unix_ns", endpoints[0].to_string())
        .with("finished_unix_ns", endpoints[1].to_string())
        .with("observed_wall_window_ns", (endpoints[1] - endpoints[0]).to_string())
        .with("files", files)
        .with("interpretation", "runner provisioning and controller build happen once; not summed per case; case preparation excludes earlier measured cases")
        .into())
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
        .with("shared_setup", shared_setup(&campaign)?)
        .with("case_preparation_started_unix_ns", unix_ns()?)
        .with("case_order", cases())
        .with("image_os", environment("ImageOS")?).with("image_version", environment("ImageVersion")?)
        .with("runner_os", environment("RUNNER_OS")?).with("runner_arch", environment("RUNNER_ARCH")?)
        .with("observed_hardware", hardware)
        .with("dependency_cache", "no artifact cache restored; registry/download cache is shared sequentially and may exist")
        .with("compiler_cache", "no compiler cache requested; wrappers/configuration captured by controller")
        .with("page_cache", "uncontrolled hosted OS page cache shared sequentially; cold refers ONLY to empty private Cargo target/build directories")
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
            .with("effective_cargo_jobs", 8_u64)
            .with("effective_libtest_threads", parallelism)
            .into(),
    )?;
    recorder.evidence("setup_completed_unix_ns", unix_ns()?.into())?;
    for warmth in ["cold", "warm"] {
        let mut request = Object::new().with("root", root.display().to_string()).with("directory", directory.display().to_string())
            .with("cargo", cargo.clone()).with("lane", lane).with("warmth", warmth)
            .with("jobs", 8_u64).with("test_threads", parallelism)
            .with("runner_class", format!("{}:{}:{}:{}:{hardware_digest}:{prerequisite_digest}", environment("ImageOS")?, environment("ImageVersion")?, environment("RUNNER_OS")?, environment("RUNNER_ARCH")?))
            .with("dependency_cache", "no artifact cache restored; registry/download cache is shared sequentially and may exist")
            .with("compiler_cache", "no compiler cache requested; wrappers/configuration captured by controller")
            .with("page_cache", "uncontrolled hosted OS page cache shared sequentially; cold refers ONLY to empty private Cargo target/build directories");
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
    write(
        &directory.join("retained-validation.json"),
        &completed_case(&directory, lane)?,
    )?;
    Ok(())
}

fn completed_case(directory: &Path, lane: &str) -> IoResult<Value> {
    let cold = directory.join(format!("{lane}-cold-receipt.json"));
    let warm = directory.join(format!("{lane}-warm-receipt.json"));
    profile::completed_pair(
        &profile::read(&cold)?,
        &profile::read(&warm)?,
        directory,
        lane,
    )?;
    Ok(Object::new()
        .with("cold", super::phases::identity(&cold)?)
        .with("warm", super::phases::identity(&warm)?)
        .into())
}

fn real_directory(path: &Path) -> IoResult<()> {
    if !std::fs::symlink_metadata(path)?.file_type().is_dir()
        || std::fs::canonicalize(path)? != path
    {
        return Err(invalid(
            "owned directory must be real and exactly contained, never a symlink",
        ));
    }
    Ok(())
}

fn private_build_trees(directory: &Path) -> IoResult<[PathBuf; 2]> {
    real_directory(directory)?;
    let trees = [directory.join("target"), directory.join("build")];
    for tree in &trees {
        real_directory(tree)?;
    }
    Ok(trees)
}

fn upload_identity(id: &str, digest: &str) -> IoResult<Value> {
    let parsed = id
        .parse::<u64>()
        .map_err(|error| invalid(error.to_string()))?;
    if parsed == 0
        || parsed.to_string() != id
        || purrdf_hash::hex::decode_32_canonical(digest).is_none()
    {
        return Err(invalid(
            "successful upload requires its canonical artifact id and SHA-256 digest",
        ));
    }
    Ok(Object::new()
        .with("id", parsed)
        .with("sha256", digest)
        .into())
}

fn reclaim_at(directory: &Path, lane: &str, recorder: &mut Recorder) -> IoResult<()> {
    let trees = recorder.check("contained-private-build-trees", || {
        private_build_trees(directory)
    })?;
    let proof = recorder.check("retained-evidence-before-reclamation", || {
        let proof = completed_case(directory, lane)?;
        if profile::read(&directory.join("retained-validation.json"))? != proof {
            return Err(invalid(
                "retained case evidence changed after validation/upload",
            ));
        }
        Ok(proof)
    })?;
    recorder.evidence(
        "retained_validation",
        super::phases::identity(&directory.join("retained-validation.json"))?,
    )?;
    recorder.evidence("retained_receipts", proof.clone())?;
    recorder.run(
        "private-build-footprints",
        Command::new("du").arg("-sb").args(&trees),
    )?;
    recorder.run(
        "disk-before-reclamation",
        Command::new("df").arg("-B1").arg(directory),
    )?;
    for (tree, name) in trees
        .iter()
        .zip(["remove-private-target", "remove-private-build"])
    {
        recorder.check(name, || std::fs::remove_dir_all(tree))?;
    }
    recorder.check("retained-evidence-after-reclamation", || {
        if completed_case(directory, lane)? != proof {
            return Err(invalid(
                "retained receipts changed during private build reclamation",
            ));
        }
        Ok(())
    })?;
    recorder.run(
        "disk-after-reclamation",
        Command::new("df").arg("-B1").arg(directory),
    )?;
    Ok(())
}

/// Revalidate completed, uploaded proof before removing only this case's build caches.
pub(crate) fn reclaim(case: &str) -> IoResult<()> {
    let (lane, _) = selection(case)?;
    let campaign = campaign()?;
    real_directory(&campaign)?;
    let resources = campaign.join("resources");
    real_directory(&resources)?;
    let path = resources.join(format!("reclaim-{case}.json"));
    if path.exists() {
        return Err(invalid("refusing to overwrite a reclamation receipt"));
    }
    let upload = upload_identity(
        &environment("PURRDF_HOSTED_UPLOAD_ID")?,
        &environment("PURRDF_HOSTED_UPLOAD_DIGEST")?,
    )?;
    let context = Object::new()
        .with("case", case)
        .with("uploaded_artifact", upload)
        .with("artifact_name", format!("native-profile-{}-arm-{case}", current_identity()?))
        .with("disk_interpretation", "boundary du/df observations and reclaimed private build bytes; within-command disk high-water mark is not measured");
    let mut recorder = Recorder::new(path, context.into())?;
    reclaim_at(&campaign.join(case), lane, &mut recorder)
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
        .with("interpretation", "observed serial first-start to last-finish window includes intervening cases, validation, upload and reclamation; it is not an observed six-runner critical path and establishes no parallel speedup")
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
        for job in ["test-shard", "native-profile"] {
            for change in ["missing", "conditional", "late"] {
                let mut workflow = purrdf_lex::yaml::read(text).unwrap();
                let steps = workflow
                    .pointer_mut(&format!("/jobs/{job}/steps"))
                    .unwrap()
                    .as_array_mut()
                    .unwrap();
                let position = steps
                    .iter()
                    .position(|step| {
                        step.get("run")
                            .and_then(Value::as_str)
                            .is_some_and(|run| run.contains(FIXTURE_SETUP))
                    })
                    .unwrap();
                match change {
                    "missing" => {
                        steps.remove(position);
                    }
                    "conditional" => {
                        steps[position]
                            .as_object_mut()
                            .unwrap()
                            .insert("if", "matrix.shard == 'lib'");
                    }
                    "late" => {
                        let provision = steps.remove(position);
                        steps.push(provision);
                    }
                    _ => unreachable!(),
                }
                assert!(
                    workflow_contract(&purrdf_lex::yaml::write(&workflow)).is_err(),
                    "{job}: {change}"
                );
            }
        }
        assert!(
            workflow_contract(&text.replace(
                "date +%s%N > \"$campaign/setup-start.txt\"\n          sudo mkdir",
                "echo unobserved-setup\n          sudo mkdir",
            ))
            .is_err()
        );
        assert!(
            workflow_contract(&text.replace(
                "PROFILE_CASE: after-integration-4",
                "PROFILE_CASE: after-integration-3"
            ))
            .is_err()
        );
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
                "name: native-profile-${{ github.run_id }}-${{ github.run_attempt }}-arm-after-lib",
                "name: native-profile-after-lib",
            ),
            (
                "name: native-profile-${{ github.run_id }}-${{ github.run_attempt }}-comparison",
                "name: native-profile-comparison",
            ),
        ] {
            assert!(workflow_contract(&text.replace(current, stale)).is_err());
        }
        for (current, altered) in [
            ("    timeout-minutes: 360", "    timeout-minutes: 361"),
            ("PROFILE_CASE: after-lib", "PROFILE_CASE: after-unknown"),
            (
                "hosted-run \"$PROFILE_CASE\" 2>&1",
                "echo \"$PROFILE_CASE\" 2>&1",
            ),
            (
                "set -o pipefail\n          campaign=",
                "exit 0\n          campaign=",
            ),
            (
                "steps.upload_after_lib.outputs.artifact-id",
                "steps.upload_after_doc.outputs.artifact-id",
            ),
            (
                "steps.upload_after_lib.outputs.artifact-digest",
                "steps.upload_after_doc.outputs.artifact-digest",
            ),
            ("-arm-after-doc", "-arm-after-lib"),
            ("-resources\n", "-resources-stale\n"),
            ("/after-lib/*-cargo/*.html", "/after-lib/target/**"),
            (
                "resources/setup-finished.txt",
                "resources/setup-finished-stale.txt",
            ),
        ] {
            assert!(
                workflow_contract(&text.replace(current, altered)).is_err(),
                "{current}"
            );
        }
        assert!(workflow_contract(&text.replace("    timeout-minutes: 360\n", "    timeout-minutes: 360\n    strategy:\n      matrix:\n        case: [after-lib]\n")).is_err());
        assert!(workflow_contract(&text.replacen("      - name: Start task-owned disk setup evidence", "      - uses: actions/cache@fixture\n      - name: Start task-owned disk setup evidence", 1)).is_err());
        let split = format!(
            "{text}\n  another-measured-runner:\n    runs-on: ubuntu-latest\n    steps:\n      - env:\n          PROFILE_CASE: after-lib\n        run: echo wrong-runner\n"
        );
        assert!(workflow_contract(&split).is_err());
        let start = text
            .find("      - name: Execute validated cold and unchanged warm after-doc\n")
            .unwrap();
        let end = text
            .find("      - name: Execute validated cold and unchanged warm after-integration-1\n")
            .unwrap();
        assert!(workflow_contract(&format!("{}{}", &text[..start], &text[end..])).is_err());
    }

    #[test]
    fn shared_setup_requires_ordered_endpoints_and_all_owned_source_records() {
        let temporary = purrdf_testkit::temp_dir!().unwrap();
        let resources = temporary.path().join("resources");
        std::fs::create_dir(&resources).unwrap();
        assert!(shared_setup(temporary.path()).is_err());
        for (name, bytes) in [
            ("setup-start.txt", "10\n"),
            ("setup-finished.txt", "20\n"),
            ("setup-disk-before.txt", "fixture disk before"),
            ("setup-disk-after.txt", "fixture disk after"),
            ("actual-rustc.txt", "fixture compiler"),
            ("actual-cargo.txt", "fixture cargo"),
        ] {
            std::fs::write(resources.join(name), bytes).unwrap();
        }
        let setup = shared_setup(temporary.path()).unwrap();
        assert_eq!(
            setup.get("observed_wall_window_ns").and_then(Value::as_str),
            Some("10")
        );
        assert_eq!(
            setup.get("files").and_then(Value::as_array).unwrap().len(),
            6
        );
        for endpoint in ["invalid", "9"] {
            std::fs::write(resources.join("setup-finished.txt"), endpoint).unwrap();
            assert!(shared_setup(temporary.path()).is_err());
        }
        std::fs::write(resources.join("setup-finished.txt"), "20").unwrap();
        std::fs::remove_file(resources.join("actual-cargo.txt")).unwrap();
        assert!(shared_setup(temporary.path()).is_err());
    }

    fn retained_case(directory: &Path) {
        std::fs::create_dir(directory).unwrap();
        let mut cold = profile::tests::receipt(directory);
        let request = cold
            .pointer_mut("/context/request")
            .unwrap()
            .as_object_mut()
            .unwrap();
        request.insert(
            "root",
            directory.with_file_name("source").display().to_string(),
        );
        request.insert("directory", directory.display().to_string());
        request.insert("cargo", "/opt/fixture/cargo");
        let mut warm = cold.clone();
        let request = warm
            .pointer_mut("/context/request")
            .unwrap()
            .as_object_mut()
            .unwrap();
        request.insert("warmth", "warm");
        request.insert(
            "previous",
            directory
                .join("lib-cold-receipt.json")
                .display()
                .to_string(),
        );
        write(&directory.join("lib-cold-receipt.json"), &cold).unwrap();
        write(&directory.join("lib-warm-receipt.json"), &warm).unwrap();
        write(
            &directory.join("retained-validation.json"),
            &completed_case(directory, "lib").unwrap(),
        )
        .unwrap();
        for name in ["target", "build"] {
            std::fs::create_dir(directory.join(name)).unwrap();
            std::fs::write(
                directory.join(name).join("unit-fixture"),
                "private cache fixture",
            )
            .unwrap();
        }
    }

    #[test]
    fn uploaded_case_reclamation_preserves_proof_and_all_siblings() {
        let temporary =
            purrdf_testkit::TempDir::new_in("/opt/purrdf-native-profile-tests").unwrap();
        let directory = temporary.path().join("case");
        retained_case(&directory);
        for name in ["source", "controller-target", "sibling"] {
            std::fs::create_dir(temporary.path().join(name)).unwrap();
            std::fs::write(temporary.path().join(name).join("sentinel"), name).unwrap();
        }
        let proof = completed_case(&directory, "lib").unwrap();
        let mut recorder =
            Recorder::new(temporary.path().join("reclaim.json"), Object::new().into()).unwrap();
        reclaim_at(&directory, "lib", &mut recorder).unwrap();
        assert!(!directory.join("target").exists());
        assert!(!directory.join("build").exists());
        assert_eq!(completed_case(&directory, "lib").unwrap(), proof);
        for name in ["source", "controller-target", "sibling"] {
            assert_eq!(
                std::fs::read_to_string(temporary.path().join(name).join("sentinel")).unwrap(),
                name
            );
        }
        let receipt = profile::read(&temporary.path().join("reclaim.json")).unwrap();
        assert!(
            receipt
                .get("phases")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .all(|phase| phase.get("success").and_then(Value::as_bool) == Some(true))
        );
    }

    #[test]
    fn incomplete_or_changed_case_refuses_before_reclaiming_any_cache() {
        for change in [
            "missing-warm",
            "failed-warm",
            "wrong-directory",
            "wrong-previous",
            "invalid-jobs",
            "changed-source",
            "changed-proof",
            "changed-child",
            "wrong-lane",
        ] {
            let temporary =
                purrdf_testkit::TempDir::new_in("/opt/purrdf-native-profile-tests").unwrap();
            let directory = temporary.path().join("case");
            retained_case(&directory);
            let path = directory.join("lib-warm-receipt.json");
            let mut warm = profile::read(&path).unwrap();
            let mutation = match change {
                "missing-warm" => {
                    std::fs::remove_file(&path).unwrap();
                    None
                }
                "failed-warm" => Some(("/phases/0/success", false.into())),
                "wrong-directory" => Some(("/context/request/directory", "/opt/other-case".into())),
                "wrong-previous" => Some((
                    "/context/request/previous",
                    "/opt/other-cold-receipt.json".into(),
                )),
                "invalid-jobs" => Some(("/context/request/jobs", 0_u64.into())),
                "changed-source" => {
                    Some(("/context/identity/configuration", "different config".into()))
                }
                "changed-proof" => {
                    std::fs::write(directory.join("retained-validation.json"), "{}").unwrap();
                    None
                }
                "changed-child" => {
                    std::fs::write(directory.join("cargo-child.json"), "changed child bytes")
                        .unwrap();
                    None
                }
                "wrong-lane" => None,
                _ => unreachable!(),
            };
            if let Some((pointer, value)) = mutation {
                *warm.pointer_mut(pointer).unwrap() = value;
                std::fs::write(&path, json::write_pretty(&warm)).unwrap();
            }
            let mut recorder =
                Recorder::new(temporary.path().join("reclaim.json"), Object::new().into()).unwrap();
            assert!(
                reclaim_at(
                    &directory,
                    if change == "wrong-lane" { "doc" } else { "lib" },
                    &mut recorder
                )
                .is_err(),
                "{change}"
            );
            for name in ["target", "build"] {
                assert_eq!(
                    std::fs::read_to_string(directory.join(name).join("unit-fixture")).unwrap(),
                    "private cache fixture",
                    "{change}"
                );
            }
        }
    }

    #[test]
    fn private_cache_boundaries_and_upload_outputs_are_mandatory() {
        let temporary =
            purrdf_testkit::TempDir::new_in("/opt/purrdf-native-profile-tests").unwrap();
        let directory = temporary.path().join("case");
        retained_case(&directory);
        assert!(private_build_trees(&directory).is_ok());
        assert!(private_build_trees(temporary.path()).is_err());
        assert!(private_build_trees(&directory.join("target").join("..")).is_err());
        std::fs::remove_dir_all(directory.join("build")).unwrap();
        assert!(private_build_trees(&directory).is_err());
        assert!(directory.join("target/unit-fixture").exists());
        let digest = purrdf_hash::hex::encode(&[0xab; 32]);
        assert!(upload_identity("123", &digest).is_ok());
        for id in ["", "0", "01", "-1", "18446744073709551616"] {
            assert!(upload_identity(id, &digest).is_err());
        }
        for invalid_digest in [
            "",
            "sha256:not-a-digest",
            &digest[..63],
            &digest.to_uppercase(),
        ] {
            assert!(upload_identity("123", invalid_digest).is_err());
        }
        assert!(reclaim("after-unknown").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_case_or_cache_never_reclaims_foreign_bytes() {
        let temporary =
            purrdf_testkit::TempDir::new_in("/opt/purrdf-native-profile-tests").unwrap();
        let directory = temporary.path().join("case");
        retained_case(&directory);
        let foreign = temporary.path().join("foreign");
        std::fs::create_dir(&foreign).unwrap();
        std::fs::write(foreign.join("sentinel"), "protected").unwrap();
        let alias = temporary.path().join("alias");
        std::os::unix::fs::symlink(&directory, &alias).unwrap();
        assert!(private_build_trees(&alias).is_err());
        std::fs::remove_dir_all(directory.join("target")).unwrap();
        std::os::unix::fs::symlink(&foreign, directory.join("target")).unwrap();
        assert!(private_build_trees(&directory).is_err());
        assert_eq!(
            std::fs::read_to_string(foreign.join("sentinel")).unwrap(),
            "protected"
        );
        assert!(directory.join("build/unit-fixture").exists());
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
