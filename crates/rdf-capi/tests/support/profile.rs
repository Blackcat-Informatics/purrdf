// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Profiling orchestration; target selection stays in Makefile and its gates.

use super::phases::{self, Recorder, invalid};
use purrdf_lex::json::record::{FromJson as _, ToJson as _};
use purrdf_lex::json::{self, Object, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

type IoResult<T> = std::io::Result<T>;
fn read(path: &Path) -> IoResult<Value> {
    json::read_with(
        &std::fs::read_to_string(path)?,
        json::Limits {
            unique_members: true,
            ..json::Limits::DEFAULT
        },
    )
    .map_err(|error| invalid(error.to_string()))
}
fn digest(bytes: &[u8]) -> String {
    purrdf_hash::hex::encode(purrdf_hash::blake3::hash(bytes).as_bytes())
}

struct Request {
    root: String,
    directory: String,
    cargo: String,
    lane: String,
    warmth: String,
    previous: Option<String>,
    jobs: u64,
    test_threads: u64,
    runner_class: String,
    dependency_cache: String,
    compiler_cache: String,
    page_cache: String,
}
purrdf_lex::json_record!(Request as "native profiling request" {
    "root" => root: required,
    "directory" => directory: required,
    "cargo" => cargo: required,
    "lane" => lane: required,
    "warmth" => warmth: required,
    "previous" => previous: optional,
    "jobs" => jobs: required,
    "test_threads" => test_threads: required,
    "runner_class" => runner_class: required,
    "dependency_cache" => dependency_cache: required,
    "compiler_cache" => compiler_cache: required,
    "page_cache" => page_cache: required,
});

fn lane_command(lane: &str) -> IoResult<Command> {
    let mut command = Command::new("make");
    match lane {
        "monolithic" => {
            command.arg("test");
        }
        "capi" => {
            command.arg("capi-check");
        }
        "lib" | "doc" | "integration-1" | "integration-2" | "integration-3" | "integration-4" => {
            command.arg("test-shard").arg(format!("SHARD={lane}"));
        }
        "downstream" => {
            command = Command::new("cargo");
            command.args([
                "test",
                "--manifest-path",
                "crates/jsonschema/tests/preserve_order_consumer/Cargo.toml",
                "--locked",
            ]);
        }
        _ => {
            return Err(invalid(
                "unknown lane; use existing native shards, monolithic, capi or downstream",
            ));
        }
    }
    Ok(command)
}

// Absolute physical roots differ between arms; logical source/package identities
// do not. Preserve the original evidence and normalize only comparison inputs.
fn normalize(value: &Value, roots: &[(&str, &str)]) -> Value {
    match value {
        Value::String(text) => {
            let mut text = text.clone();
            for (root, logical) in roots {
                text = text.replace(root, logical);
            }
            text.into()
        }
        Value::Array(values) => values
            .iter()
            .map(|value| normalize(value, roots))
            .collect::<Vec<_>>()
            .into(),
        Value::Object(object) => {
            let mut result = Object::new();
            for (key, value) in object.iter() {
                result.insert(key, normalize(value, roots));
            }
            result.into()
        }
        _ => value.clone(),
    }
}

fn output(command: &mut Command) -> IoResult<Vec<u8>> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(invalid(format!(
            "identity command failed: {}",
            output.status
        )));
    }
    Ok(output.stdout)
}
fn source(root: &Path) -> IoResult<Value> {
    let files = output(
        Command::new("git")
            .args([
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
                "--",
                ".",
                ":(exclude).stage",
                ":(exclude).worktrees",
            ])
            .current_dir(root),
    )?;
    let names = std::str::from_utf8(&files).map_err(|error| invalid(error.to_string()))?;
    let mut files = Object::new();
    for name in names.split_terminator('\0') {
        let bytes = std::fs::read(root.join(name))?;
        files.insert(name, digest(&bytes));
    }
    let c_smoke = std::fs::read_to_string(root.join("crates/rdf-capi/tests/c_smoke.rs"))?;
    Ok(Object::new()
        .with("files", Value::from(files))
        .with("c_smoke", c_smoke)
        .into())
}

fn configuration_projection(bytes: &[u8]) -> IoResult<Value> {
    // Cargo may include credential-bearing tables in the complete response.
    // Parse privately and retain ONLY these code-generation tables. Parse errors
    // deliberately do not echo response text or unrelated configuration.
    let text = std::str::from_utf8(bytes)
        .map_err(|_| invalid("Cargo configuration response is not UTF-8"))?;
    let configuration = json::read_with(
        text,
        json::Limits {
            unique_members: true,
            ..json::Limits::DEFAULT
        },
    )
    .map_err(|_| invalid("Cargo configuration response is malformed JSON"))?;
    if configuration.as_object().is_none() {
        return Err(invalid("Cargo configuration response must be an object"));
    }
    let mut values = Object::new();
    for key in ["build", "profile", "target"] {
        let state = if let Some(value) = configuration.get(key) {
            if value.as_object().is_none() {
                return Err(invalid(format!(
                    "Cargo configuration table {key} is not an object"
                )));
            }
            Object::new()
                .with("state", "present")
                .with("value", value.clone())
        } else {
            Object::new().with("state", "absent")
        };
        values.insert(key, Value::from(state));
    }
    Ok(values.into())
}

fn query_configuration(command: &mut Command) -> IoResult<Value> {
    // Querying an absent individual table exits101. The no-key query succeeds
    // with an object (possibly empty), so absence is bound to successful Cargo
    // output rather than guessed from an error or a catch-all fallback.
    let result = command
        .args([
            "-Z",
            "unstable-options",
            "config",
            "get",
            "--format",
            "json-value",
        ])
        .output()?;
    if !result.status.success() {
        return Err(invalid(format!(
            "Cargo config get --format json-value failed ({}); require a supported nightly Cargo and valid configuration; raw stdout/stderr withheld because they may contain credentials",
            result.status
        )));
    }
    configuration_projection(&result.stdout)
}

fn resolved_configuration(root: &Path, cargo: &Path) -> IoResult<Value> {
    query_configuration(Command::new(cargo).current_dir(root))
}
fn empty(path: &Path) -> IoResult<bool> {
    match std::fs::read_dir(path) {
        Ok(mut entries) => Ok(entries.next().is_none()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error),
    }
}
fn validate_request(request: &Request) -> IoResult<()> {
    lane_command(&request.lane)?;
    if request.jobs == 0 || request.test_threads == 0 || request.jobs > 8 {
        return Err(invalid(
            "explicit jobs 1..8 and positive libtest concurrency required",
        ));
    }
    for name in [
        &request.runner_class,
        &request.dependency_cache,
        &request.compiler_cache,
        &request.page_cache,
    ] {
        if name.trim().is_empty() {
            return Err(invalid(
                "runner and separate cache declarations must be nonempty",
            ));
        }
    }
    if !matches!(request.warmth.as_str(), "cold" | "warm")
        || (request.warmth == "warm") != request.previous.is_some()
    {
        return Err(invalid(
            "warm arm requires exactly one prior successful unchanged-arm receipt; cold arm has none",
        ));
    }
    let directory = Path::new(&request.directory);
    if !directory.is_absolute()
        || !directory.starts_with("/opt")
        || directory
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(invalid(
            "arm directory must be a task-owned absolute /opt disk path without parent components",
        ));
    }
    if !Path::new(&request.root).is_absolute() || !Path::new(&request.cargo).is_absolute() {
        return Err(invalid(
            "source root and captured real Cargo executable must be absolute",
        ));
    }
    Ok(())
}

pub(crate) fn run(input: &Path) -> IoResult<()> {
    let request = Request::from_json(&read(input)?).map_err(|error| invalid(error.to_string()))?;
    validate_request(&request)?;
    let root = std::fs::canonicalize(&request.root)?;
    let directory = PathBuf::from(&request.directory);
    std::fs::create_dir_all(&directory)?;
    let directory = std::fs::canonicalize(directory)?;
    if !directory.starts_with("/opt") || directory.starts_with(&root) {
        return Err(invalid(
            "arm directory resolves outside /opt or inside the source tree",
        ));
    }
    let executable = std::env::current_exe()?;
    if executable.starts_with(directory.join("target"))
        || executable.starts_with(directory.join("build"))
    {
        return Err(invalid(
            "controller must be built outside measured targets/build directories",
        ));
    }
    let receipt_path = directory.join(format!("{}-{}-receipt.json", request.lane, request.warmth));
    if receipt_path.exists() {
        return Err(invalid("refusing to overwrite an existing arm receipt"));
    }
    let target = directory.join("target");
    let build = directory.join("build");
    let mut recorder = Recorder::new(
        receipt_path,
        Object::new().with("request", request.to_json()).into(),
    )?;
    recorder.check("arm-preparation", || {
        if request.warmth == "cold" && (!empty(&target)? || !empty(&build)?) {
            return Err(invalid("cold arm requires empty task-owned target AND build directories; nothing is deleted"));
        }
        Ok(())
    })?;
    // A rustup proxy's basename selects Cargo, so retain its invocation path.
    let cargo = PathBuf::from(&request.cargo);
    // Pin environment overrides consistently; Cargo's config query records the
    // underlying config too. No profile, compiler-cache or codegen flag override.
    // Set these on each child instead of mutating this process's global environment.
    let mut configuration = Command::new(&cargo);
    configuration
        .env("CARGO_BUILD_JOBS", request.jobs.to_string())
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_BUILD_BUILD_DIR", &build)
        .current_dir(&root);
    let config = recorder.check("effective-build-configuration", || {
        let resolved = query_configuration(&mut configuration)?;
        // `cargo config get` lists file values and notes environment separately.
        // These explicit environment values win over configuration defaults.
        Ok::<Value, std::io::Error>(
            Object::new()
                .with("configured", resolved)
                .with("effective_jobs", request.jobs)
                .with("effective_test_threads", request.test_threads)
                .with("effective_target", target.display().to_string())
                .with("effective_build_dir", build.display().to_string())
                .into(),
        )
    })?;
    let configured = recorder.check("configuration-identity", || {
        resolved_configuration(&root, &cargo)
    })?;
    let context = recorder.check("tool-source-configuration-identity", || {
        phases::context(
            &root,
            request.cargo.as_str(),
            &std::env::var("CC").unwrap_or_else(|_| "cc".to_owned()),
        )
    })?;
    let source = recorder.check("logical-source-identity", || source(&root))?;
    let metadata = recorder.check("complete-target-inventory", || {
        let bytes = output(
            Command::new(&cargo)
                .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
                .current_dir(&root),
        )?;
        json::read(std::str::from_utf8(&bytes).map_err(|error| invalid(error.to_string()))?)
            .map_err(|error| invalid(error.to_string()))
    })?;
    let roots = [
        (
            root.to_str()
                .ok_or_else(|| invalid("non-UTF8 source root"))?,
            "$SOURCE",
        ),
        (
            directory
                .to_str()
                .ok_or_else(|| invalid("non-UTF8 arm root"))?,
            "$ARM",
        ),
    ];
    let identity: Value = Object::new()
        .with("context", normalize(&context, &roots))
        .with("source", source)
        .with("configuration", normalize(&configured, &roots))
        .with("effective_build_configuration", normalize(&config, &roots))
        .with("target_inventory", normalize(&metadata, &roots))
        .into();
    recorder.evidence("identity", identity.clone())?;
    recorder.check("unchanged-warm-arm", || {
        if let Some(previous) = &request.previous {
            let prior = read(Path::new(previous))?;
            valid_receipt(&prior)?;
            if prior.pointer("/context/identity") != Some(&identity) {
                return Err(invalid(
                    "warm arm source/tools/configuration/target inventory changed",
                ));
            }
            let prior_request = prior
                .pointer("/context/request")
                .ok_or_else(|| invalid("prior request missing"))?;
            for key in [
                "root",
                "directory",
                "cargo",
                "lane",
                "jobs",
                "test_threads",
                "runner_class",
                "dependency_cache",
                "compiler_cache",
                "page_cache",
            ] {
                if prior_request.get(key) != request.to_json().get(key) {
                    return Err(invalid(format!("warm arm changed {key}")));
                }
            }
        }
        Ok(())
    })?;
    let telemetry = directory.join(format!("{}-{}-cargo", request.lane, request.warmth));
    std::fs::create_dir(&telemetry)?;
    let bin = telemetry.join("bin");
    std::fs::create_dir(&bin)?;
    let shim = bin.join("cargo");
    std::fs::copy(executable, &shim)?;
    let inherited_path = std::env::var_os("PATH").ok_or_else(|| invalid("PATH missing"))?;
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&inherited_path));
    let mut command = lane_command(&request.lane)?;
    command
        .current_dir(&root)
        .env(
            "PATH",
            std::env::join_paths(paths).map_err(|error| invalid(error.to_string()))?,
        )
        .env("PURRDF_PROFILE_REAL_CARGO", request.cargo.as_str())
        .env("PURRDF_PROFILE_TELEMETRY", &telemetry)
        .env("PURRDF_PROFILE_CARGO", &shim)
        .env("CARGO_BUILD_JOBS", request.jobs.to_string())
        .env("RUST_TEST_THREADS", request.test_threads.to_string())
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_BUILD_BUILD_DIR", &build)
        .env("PURRDF_C_PHASE_RECEIPT", telemetry.join("c-phases.json"));
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invalid(error.to_string()))?
        .as_nanos();
    let result = recorder.run("actual-native-command", &mut command);
    let finished = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invalid(error.to_string()))?
        .as_nanos();
    recorder.evidence(
        "execution_window",
        Object::new()
            .with("started_unix_ns", started.to_string())
            .with("finished_unix_ns", finished.to_string())
            .with(
                "clock",
                "host wall clock; cross-runner synchronization is not independently certified",
            )
            .into(),
    )?;
    // A failed native command still retains compiler inventories and its truthful
    // failure phase. Collection errors also hard-fail instead of losing telemetry.
    let collection = recorder.check("cargo-telemetry-inventory", || collect(&telemetry, &roots));
    if let Ok(value) = &collection {
        recorder.evidence("cargo", value.clone())?;
    }
    result?;
    collection?;
    Ok(())
}

fn telemetry_arguments(arguments: &[String]) -> Vec<String> {
    let mut delegated = arguments.to_vec();
    let verb = arguments.first().map(String::as_str);
    if matches!(verb, Some("build" | "test")) {
        let separator = arguments
            .iter()
            .position(|arg| arg == "--")
            .unwrap_or(arguments.len());
        if !arguments[..separator]
            .iter()
            .any(|arg| arg == "--timings" || arg.starts_with("--timings="))
        {
            delegated.insert(separator, "--timings".to_owned());
        }
        if !arguments[..separator]
            .iter()
            .any(|arg| arg == "--message-format" || arg.starts_with("--message-format="))
        {
            delegated.insert(
                separator,
                "--message-format=json-render-diagnostics".to_owned(),
            );
        }
        if !arguments[..separator]
            .iter()
            .any(|arg| arg == "--color" || arg.starts_with("--color="))
        {
            delegated.insert(separator, "--color=never".to_owned());
        }
    }
    delegated
}

pub(crate) fn cargo_shim(arguments: Vec<String>) -> IoResult<()> {
    let cargo = std::env::var_os("PURRDF_PROFILE_REAL_CARGO")
        .ok_or_else(|| invalid("real Cargo identity missing"))?;
    let directory = std::env::var_os("PURRDF_PROFILE_TELEMETRY")
        .ok_or_else(|| invalid("telemetry directory missing"))?;
    cargo_shim_at(&PathBuf::from(cargo), &PathBuf::from(directory), arguments)
}

fn cargo_shim_at(cargo: &Path, directory: &Path, arguments: Vec<String>) -> IoResult<()> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invalid(error.to_string()))?
        .as_nanos();
    let prefix = directory.join(format!("cargo-{}-{stamp}", std::process::id()));
    let measured = matches!(
        arguments.first().map(String::as_str),
        Some("build" | "test")
    );
    let target = if measured {
        Some(PathBuf::from(
            std::env::var_os("CARGO_TARGET_DIR")
                .ok_or_else(|| invalid("profiling Cargo target directory missing"))?,
        ))
    } else {
        None
    };
    let previous = target
        .as_ref()
        .map(|target| timing_snapshot(target))
        .transpose()?
        .unwrap_or_default();
    let delegated = telemetry_arguments(&arguments);
    let mut recorder = Recorder::new(
        prefix.with_extension("json"),
        Object::new()
            .with("original_argv", arguments)
            .with("delegated_argv", delegated.clone())
            .into(),
    )?;
    let mut command = Command::new(cargo);
    command
        .args(delegated)
        .current_dir(std::env::current_dir()?);
    let result = recorder.run(
        "rust-preparation-codegen-link-and-test-command",
        &mut command,
    );
    let receipt = read(&prefix.with_extension("json"))?;
    if let Some(target) = &target {
        let stderr = receipt
            .pointer("/phases/0/command/stderr")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let reported: Vec<PathBuf> = stderr
            .lines()
            .filter_map(|line| {
                line.split_once("Timing report saved to ")
                    .map(|(_, path)| PathBuf::from(path.trim()))
            })
            .collect();
        let timings = current_timings(target, &previous, &prefix, Some(&reported))?;
        recorder.evidence("timings", timings.clone().into())?;
        if result.is_ok() && timings.is_empty() {
            return Err(invalid(
                "successful Cargo preparation produced no current timing report",
            ));
        }
    }
    // Recorder preserves output even on failure. Relay the retained bytes for
    // nested Csmoke's strict Cargo artifact parser and ordinary caller behavior.
    if let Some(command) = receipt.pointer("/phases/0/command") {
        use std::io::Write as _;
        if let Some(text) = command.get("stdout").and_then(Value::as_str) {
            std::io::stdout().write_all(text.as_bytes())?;
        }
        if let Some(text) = command.get("stderr").and_then(Value::as_str) {
            std::io::stderr().write_all(text.as_bytes())?;
        }
    }
    result?;
    Ok(())
}

fn artifact_metadata(message: &Value) -> IoResult<()> {
    if message.get("package_id").and_then(Value::as_str).is_none()
        || message.get("fresh").and_then(Value::as_bool).is_none()
        || message.get("profile").and_then(Value::as_object).is_none()
        || message
            .pointer("/target/name")
            .and_then(Value::as_str)
            .is_none()
        || message
            .pointer("/target/src_path")
            .and_then(Value::as_str)
            .is_none()
    {
        return Err(invalid("malformed compiler artifact identity/profile"));
    }
    for pointer in ["/target/kind", "/features", "/filenames"] {
        let values = message
            .pointer(pointer)
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(format!("malformed artifact {pointer}")))?;
        if values.iter().any(|value| value.as_str().is_none()) {
            return Err(invalid(format!("non-string artifact {pointer}")));
        }
    }
    Ok(())
}

fn timing_snapshot(target: &Path) -> IoResult<Vec<(PathBuf, String)>> {
    let directory = target.join("cargo-timings");
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push((entry.path(), digest(&std::fs::read(entry.path())?)));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn current_timings(
    target: &Path,
    previous: &[(PathBuf, String)],
    prefix: &Path,
    reported: Option<&[PathBuf]>,
) -> IoResult<Vec<Value>> {
    let mut reports = Vec::new();
    for (path, hash) in timing_snapshot(target)? {
        if reported.is_some_and(|reported| !reported.contains(&path)) {
            continue;
        }
        if previous
            .iter()
            .any(|(old_path, old_hash)| *old_path == path && *old_hash == hash)
        {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| invalid("non-UTF8 timing report name"))?;
        let destination = prefix.with_file_name(format!(
            "{}-{filename}",
            prefix
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .ok_or_else(|| invalid("timing prefix malformed"))?
        ));
        std::fs::copy(path, &destination)?;
        reports.push(phases::identity(&destination)?);
    }
    Ok(reports)
}

fn collect(directory: &Path, roots: &[(&str, &str)]) -> IoResult<Value> {
    let mut receipts = Vec::new();
    let mut artifacts = Vec::new();
    let mut timings = Vec::new();
    let mut files = std::fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    files.sort_by_key(std::fs::DirEntry::file_name);
    for entry in files {
        if entry.file_name().to_string_lossy().starts_with("cargo-")
            && entry.path().extension().is_some_and(|ext| ext == "json")
        {
            let receipt = read(&entry.path())?;
            if let Some(reports) = receipt
                .pointer("/context/timings")
                .and_then(Value::as_array)
            {
                timings.extend(reports.iter().cloned());
            }
            let stdout = receipt
                .pointer("/phases/0/command/stdout")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("Cargo child output missing"))?;
            for line in stdout.lines().filter(|line| line.starts_with('{')) {
                let message = json::read_with(
                    line,
                    json::Limits {
                        unique_members: true,
                        ..json::Limits::DEFAULT
                    },
                )
                .map_err(|error| invalid(error.to_string()))?;
                if message.get("reason").and_then(Value::as_str) == Some("compiler-artifact") {
                    artifact_metadata(&message)?;
                    for key in [
                        "package_id",
                        "target",
                        "profile",
                        "features",
                        "filenames",
                        "fresh",
                    ] {
                        if message.get(key).is_none() {
                            return Err(invalid(format!("Cargo artifact {key} missing")));
                        }
                    }
                    if message.pointer("/target/name").and_then(Value::as_str) == Some("purrdf")
                        && message
                            .pointer("/target/kind")
                            .and_then(Value::as_array)
                            .is_some_and(|kinds| kinds.iter().any(|kind| kind == "cdylib"))
                    {
                        let package = message
                            .get("package_id")
                            .and_then(Value::as_str)
                            .ok_or_else(|| invalid("artifact package ID malformed"))?;
                        let filename = format!(
                            "{}purrdf{}",
                            std::env::consts::DLL_PREFIX,
                            std::env::consts::DLL_SUFFIX
                        );
                        let selected = phases::cdylib(stdout.as_bytes(), package, &filename)?;
                        if selected.message != message || !selected.path.is_file() {
                            return Err(invalid(
                                "actual selected C library is missing or ambiguous",
                            ));
                        }
                    }
                    artifacts.push(normalize(&message, roots));
                }
            }
            receipts.push(phases::identity(&entry.path())?);
        }
    }
    if receipts.is_empty() {
        return Err(invalid("no actual Cargo invocations captured"));
    }
    if timings.is_empty() {
        return Err(invalid("Cargo did not retain a timing report"));
    }
    let mut collected = Object::new()
        .with("receipts", receipts)
        .with("compiler_artifacts", artifacts)
        .with("timings", timings);
    let c_receipt = directory.join("c-phases.json");
    if c_receipt.exists() {
        collected.insert("c_phases", phases::identity(&c_receipt)?);
    }
    Ok(collected.into())
}

fn valid_receipt(receipt: &Value) -> IoResult<u128> {
    if receipt.get("schema").and_then(Value::as_str) != Some("purrdf-process-phases-v1") {
        return Err(invalid("unknown or missing receipt schema"));
    }
    let phases = receipt
        .get("phases")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("phase inventory missing"))?;
    let mut elapsed = None;
    for phase in phases {
        if phase.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(invalid(
                "failed/incomplete phase cannot qualify a comparison",
            ));
        }
        if phase.get("name").and_then(Value::as_str) == Some("actual-native-command") {
            if elapsed.is_some() {
                return Err(invalid("duplicate actual command phase"));
            }
            elapsed = Some(
                phase
                    .get("elapsed_ns")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("elapsed time missing"))?
                    .parse::<u128>()
                    .map_err(|error| invalid(error.to_string()))?,
            );
        }
    }
    let elapsed = elapsed.ok_or_else(|| invalid("actual native command did not complete"))?;
    for key in ["request", "identity", "cargo"] {
        if receipt.pointer(&format!("/context/{key}")).is_none() {
            return Err(invalid(format!("missing {key} evidence")));
        }
    }
    let cargo = receipt
        .pointer("/context/cargo")
        .ok_or_else(|| invalid("Cargo evidence missing"))?;
    for key in ["receipts", "compiler_artifacts", "timings"] {
        if cargo
            .get(key)
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            return Err(invalid(format!("missing/empty Cargo {key}")));
        }
    }
    for artifact in cargo
        .get("compiler_artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("artifact inventory missing"))?
    {
        artifact_metadata(artifact)?;
    }
    for key in ["receipts", "timings"] {
        for identity in cargo
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("identity inventory malformed"))?
        {
            let path = identity
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("retained evidence path missing"))?;
            if phases::identity(Path::new(path))? != *identity {
                return Err(invalid("retained evidence bytes changed"));
            }
            if key == "receipts" {
                let child = read(Path::new(path))?;
                if child.pointer("/phases/0/success").and_then(Value::as_bool) != Some(true) {
                    return Err(invalid("failed Cargo child cannot qualify"));
                }
            }
        }
    }
    if matches!(
        receipt
            .pointer("/context/request/lane")
            .and_then(Value::as_str),
        Some("capi" | "monolithic")
    ) {
        let identity = cargo
            .get("c_phases")
            .ok_or_else(|| invalid("actual C compile/link/runtime receipt missing"))?;
        let path = identity
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("C phase path missing"))?;
        if phases::identity(Path::new(path))? != *identity {
            return Err(invalid("C phase evidence changed"));
        }
        let c = read(Path::new(path))?;
        let phases = c
            .get("phases")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("C phases missing"))?;
        if phases.len() != 16
            || phases
                .iter()
                .any(|phase| phase.get("success").and_then(Value::as_bool) != Some(true))
        {
            return Err(invalid("C smoke did not complete all original phases"));
        }
    }
    Ok(elapsed)
}

struct Policy {
    before: Vec<String>,
    after: Vec<String>,
    change: String,
    output: String,
}
purrdf_lex::json_record!(Policy as "profiling comparison policy" {
    "before" => before: required,
    "after" => after: required,
    "change" => change: required,
    "output" => output: required,
});
fn same(a: &Value, b: &Value, pointer: &str) -> IoResult<()> {
    let left = a
        .pointer(pointer)
        .ok_or_else(|| invalid(format!("missing comparison input {pointer}")))?;
    if b.pointer(pointer) != Some(left) {
        return Err(invalid(format!("comparison mismatch {pointer}")));
    }
    Ok(())
}
fn sources_match(a: &Value, b: &Value, change: &str) -> IoResult<()> {
    let a = a
        .pointer("/context/identity/source")
        .ok_or_else(|| invalid("source identity missing"))?;
    let b = b
        .pointer("/context/identity/source")
        .ok_or_else(|| invalid("source identity missing"))?;
    if change != "nested-profile" {
        if a != b {
            return Err(invalid("logical source bytes differ"));
        }
        return Ok(());
    }
    let before = a
        .get("c_smoke")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("counterfactual source missing"))?;
    let after = b
        .get("c_smoke")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("counterfactual source missing"))?;
    let needle = "let profile = if cfg!(debug_assertions) {\n        \"dev\"";
    if before.matches(needle).count() != 1
        || before.replacen(
            needle,
            "let profile = if cfg!(debug_assertions) {\n        \"test\"",
            1,
        ) != after
    {
        return Err(invalid(
            "counterfactual must change ONLY the debug nested C profile dev to test",
        ));
    }
    let mut left = a
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("source file inventory missing"))?
        .clone();
    let mut right = b
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("source file inventory missing"))?
        .clone();
    left.remove("crates/rdf-capi/tests/c_smoke.rs");
    right.remove("crates/rdf-capi/tests/c_smoke.rs");
    if left != right {
        return Err(invalid("counterfactual also changed other source files"));
    }
    Ok(())
}
fn validate_pair(a: &Value, b: &Value, change: &str) -> IoResult<()> {
    valid_receipt(a)?;
    valid_receipt(b)?;
    for key in [
        "warmth",
        "jobs",
        "test_threads",
        "runner_class",
        "dependency_cache",
        "compiler_cache",
        "page_cache",
    ] {
        same(a, b, &format!("/context/request/{key}"))?;
    }
    // Context source contains Git HEAD/index/diff by design. Physical roots and
    // permitted counterfactuals make those audit fields unequal; byte identities
    // and exact narrowly admitted delta below are the comparison authority.
    for key in [
        "tools",
        "host_os",
        "host_arch",
        "available_parallelism",
        "inputs",
    ] {
        same(a, b, &format!("/context/identity/context/{key}"))?;
    }
    for key in [
        "configuration",
        "effective_build_configuration",
        "target_inventory",
    ] {
        same(a, b, &format!("/context/identity/{key}"))?;
    }
    same(a, b, "/context/identity/context/environment")?;
    sources_match(a, b, change)
}
fn collection(paths: &[String]) -> IoResult<Vec<Value>> {
    if paths.is_empty() {
        return Err(invalid("empty comparison arm"));
    }
    paths
        .iter()
        .map(|path| {
            let receipt = read(Path::new(path))?;
            valid_receipt(&receipt)?;
            Ok(receipt)
        })
        .collect()
}
fn lanes(receipts: &[Value]) -> IoResult<Vec<String>> {
    let mut lanes: Vec<String> = receipts
        .iter()
        .map(|receipt| {
            receipt
                .pointer("/context/request/lane")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| invalid("lane missing"))
        })
        .collect::<IoResult<_>>()?;
    lanes.sort();
    if lanes.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("duplicate comparison lane"));
    }
    Ok(lanes)
}

fn coverage(receipts: &[Value]) -> IoResult<Vec<String>> {
    let mut targets = Vec::new();
    for receipt in receipts {
        for artifact in receipt
            .pointer("/context/cargo/compiler_artifacts")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("compiled target coverage missing"))?
        {
            let mut target = Object::new();
            for key in ["package_id", "target", "features"] {
                target.insert(
                    key,
                    artifact
                        .get(key)
                        .ok_or_else(|| invalid(format!("compiled coverage {key} missing")))?
                        .clone(),
                );
            }
            targets.push(json::write_compact(&target.into()));
        }
    }
    targets.sort();
    targets.dedup();
    Ok(targets)
}

pub(crate) fn compare(input: &Path) -> IoResult<()> {
    let policy = Policy::from_json(&read(input)?).map_err(|error| invalid(error.to_string()))?;
    if !matches!(
        policy.change.as_str(),
        "none" | "nested-profile" | "native-partition"
    ) {
        return Err(invalid("unadmitted experimental change"));
    }
    let before = collection(&policy.before)?;
    let after = collection(&policy.after)?;
    let before_lanes = lanes(&before)?;
    let after_lanes = lanes(&after)?;
    if policy.change == "native-partition" {
        let expected_before = ["capi", "downstream", "monolithic"].map(str::to_owned);
        let expected_after = [
            "capi",
            "doc",
            "downstream",
            "integration-1",
            "integration-2",
            "integration-3",
            "integration-4",
            "lib",
        ]
        .map(str::to_owned);
        if before_lanes != expected_before || after_lanes != expected_after {
            return Err(invalid(
                "partition comparison requires complete monolithic+dedicated C+downstream versus all six shards+dedicated C+downstream",
            ));
        }
        for receipt in before.iter().chain(&after) {
            validate_pair(&before[0], receipt, "none")?;
        }
    } else {
        if before_lanes != after_lanes {
            return Err(invalid("phase selection differs"));
        }
        for a in &before {
            let lane = a.pointer("/context/request/lane");
            let b = after
                .iter()
                .find(|b| b.pointer("/context/request/lane") == lane)
                .ok_or_else(|| invalid("matching lane missing"))?;
            validate_pair(a, b, &policy.change)?;
        }
    }
    if coverage(&before)? != coverage(&after)? {
        return Err(invalid(
            "actual combined compiled target/features coverage differs",
        ));
    }
    let before_ns = before.iter().try_fold(0_u128, |sum, receipt| {
        sum.checked_add(valid_receipt(receipt)?)
            .ok_or_else(|| invalid("aggregate duration overflow"))
    })?;
    let after_ns = after.iter().try_fold(0_u128, |sum, receipt| {
        sum.checked_add(valid_receipt(receipt)?)
            .ok_or_else(|| invalid("aggregate duration overflow"))
    })?;
    // This is aggregate actual command work. Separate runners' scheduling/start
    // timestamps are necessary for hosted critical path and are never invented.
    let summary: Value = Object::new().with("schema", "purrdf-native-comparison-v1")
        .with("change", policy.change).with("before", policy.before).with("after", policy.after)
        .with("before_aggregate_command_ns", before_ns.to_string()).with("after_aggregate_command_ns", after_ns.to_string())
        .with("aggregate_reduction_observed", after_ns < before_ns)
        .with("critical_path", "NOT MEASURED: aggregate durations do not establish hosted six-runner wall time")
        .with("rust_cost", "combined preparation/code generation/linking and test execution; see Cargo timing reports, never subtraction-derived pure link time").into();
    std::fs::write(policy.output, json::write_pretty(&summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_configuration_absence_is_bound_and_private_values_are_excluded() {
        let absent = configuration_projection(b"{}").unwrap();
        for key in ["build", "profile", "target"] {
            assert_eq!(
                absent
                    .pointer(&format!("/{key}/state"))
                    .and_then(Value::as_str),
                Some("absent")
            );
        }
        let configured = configuration_projection(br#"{"build":{"jobs":4},"profile":{"dev":{"opt-level":3}},"target":{"x86_64-unknown-linux-gnu":{"rustflags":["-Ctarget-cpu=x86-64"]}},"registries":{"private":{"token":"DO_NOT_PERSIST"}}}"#).unwrap();
        assert_eq!(
            configured.pointer("/build/state").and_then(Value::as_str),
            Some("present")
        );
        assert_eq!(
            configured
                .pointer("/build/value/jobs")
                .and_then(Value::as_u64),
            Some(4)
        );
        assert!(!json::write_compact(&configured).contains("DO_NOT_PERSIST"));
        assert!(configured.get("registries").is_none());
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let mut before = receipt(holder.path());
        *before
            .pointer_mut("/context/identity/configuration")
            .unwrap() = absent;
        let mut after = before.clone();
        *after
            .pointer_mut("/context/identity/configuration")
            .unwrap() = configured;
        assert!(validate_pair(&before, &after, "none").is_err());
    }

    #[test]
    fn cargo_configuration_malformed_responses_never_become_absence() {
        for bytes in [
            b"".as_slice(),
            b"[]",
            b"null",
            b"{bad secret",
            b"{\"build\":null}",
            b"{\"build\":4}",
            b"{\"build\":{},\"build\":{}}",
            &[255],
        ] {
            let error = configuration_projection(bytes).unwrap_err().to_string();
            assert!(!error.contains("secret"));
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn real_sdk_configuration_query_admits_absence_and_refuses_private_errors() {
        // Resolve the actual SDK Cargo rather than a managed wrapper that may
        // inject configuration. These subprocesses query only; none compiles.
        let sysroot = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(sysroot.status.success());
        let cargo = PathBuf::from(std::str::from_utf8(&sysroot.stdout).unwrap().trim())
            .join("bin")
            .join(format!("cargo{}", std::env::consts::EXE_SUFFIX));
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let root = holder.path();
        let query = || {
            query_configuration(
                Command::new(&cargo)
                    .env_clear()
                    .env("CARGO_HOME", root.join("cargo-home"))
                    .env("PATH", cargo.parent().unwrap())
                    .current_dir(root),
            )
        };
        let absent = query().unwrap();
        assert_eq!(
            absent.pointer("/build/state").and_then(Value::as_str),
            Some("absent")
        );
        std::fs::create_dir(root.join(".cargo")).unwrap();
        let config = root.join(".cargo/config.toml");
        std::fs::write(&config, "[build]\njobs=4\n[profile.dev]\nopt-level=3\n[target.x86_64-unknown-linux-gnu]\nrustflags=['-Ctarget-cpu=x86-64']\n[registries.private]\ntoken='DO_NOT_PERSIST'\n").unwrap();
        let configured = query().unwrap();
        assert_eq!(
            configured
                .pointer("/build/value/jobs")
                .and_then(Value::as_u64),
            Some(4)
        );
        assert!(!json::write_compact(&configured).contains("DO_NOT_PERSIST"));
        std::fs::write(config, "DO_NOT_PERSIST = [ malformed private config").unwrap();
        let error = query().unwrap_err().to_string();
        assert!(error.contains("Cargo config get --format json-value failed"));
        assert!(error.contains("supported nightly Cargo and valid configuration"));
        assert!(!error.contains("DO_NOT_PERSIST"));
    }

    fn receipt(root: &Path) -> Value {
        let child = root.join("cargo-child.json");
        let timing = root.join("cargo-timing.html");
        std::fs::write(&child, r#"{"phases":[{"success":true}]}"#).unwrap();
        std::fs::write(&timing, "fixture Cargo timing report").unwrap();
        let cargo: Value = Object::new()
            .with("receipts", vec![phases::identity(&child).unwrap()])
            .with("timings", vec![phases::identity(&timing).unwrap()])
            .with(
                "compiler_artifacts",
                vec![Value::from(
                    Object::new()
                        .with("package_id", "purrdf")
                        .with(
                            "target",
                            Value::from(
                                Object::new()
                                    .with("name", "all targets")
                                    .with("src_path", "$SOURCE/lib.rs")
                                    .with("kind", vec!["lib"]),
                            ),
                        )
                        .with("features", Vec::<Value>::new())
                        .with("filenames", vec!["$ARM/library"])
                        .with("profile", Value::from(Object::new().with("opt_level", "3")))
                        .with("fresh", true),
                )],
            )
            .into();
        let context: Value = Object::new()
            .with(
                "tools",
                vec!["rustc nightly identity", "cargo identity", "cc identity"],
            )
            .with("host_os", "linux")
            .with("host_arch", "x86_64")
            .with("available_parallelism", 8_u64)
            .with("inputs", vec!["same lock/manifests/config bytes"])
            .with(
                "environment",
                vec!["RUSTFLAGS=-D warnings", "CARGO_BUILD_JOBS=8"],
            )
            .into();
        let source: Value = Object::new()
            .with(
                "files",
                Value::from(
                    Object::new()
                        .with("lib.rs", "content")
                        .with("crates/rdf-capi/tests/c_smoke.rs", "content"),
                ),
            )
            .with(
                "c_smoke",
                "let profile = if cfg!(debug_assertions) {\n        \"test\"\n    };",
            )
            .into();
        let identity: Value = Object::new()
            .with("context", context)
            .with("source", source)
            .with("configuration", "config flags")
            .with(
                "effective_build_configuration",
                "jobs8 target=$ARM/target build=$ARM/build",
            )
            .with("target_inventory", vec!["all workspace target inventory"])
            .into();
        let request: Value = Object::new()
            .with("warmth", "cold")
            .with("lane", "lib")
            .with("jobs", 8_u64)
            .with("test_threads", 2_u64)
            .with("runner_class", "bounded-local")
            .with("dependency_cache", "preprovisioned unchanged")
            .with("compiler_cache", "enabled unchanged")
            .with("page_cache", "uncontrolled unchanged declaration")
            .into();
        Object::new()
            .with("schema", "purrdf-process-phases-v1")
            .with(
                "context",
                Value::from(
                    Object::new()
                        .with("identity", identity)
                        .with("request", request)
                        .with("cargo", cargo),
                ),
            )
            .with(
                "phases",
                vec![Value::from(
                    Object::new()
                        .with("name", "actual-native-command")
                        .with("success", true)
                        .with("elapsed_ns", "100"),
                )],
            )
            .into()
    }

    #[test]
    fn rejects_missing_failed_mismatched_tools_config_concurrency_cache_and_coverage() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let a = receipt(holder.path());
        validate_pair(&a, &a, "none").unwrap();
        for pointer in [
            "/context/request/jobs",
            "/context/request/test_threads",
            "/context/request/warmth",
            "/context/request/dependency_cache",
            "/context/request/compiler_cache",
            "/context/request/page_cache",
            "/context/request/runner_class",
            "/context/identity/context/tools",
            "/context/identity/context/environment",
            "/context/identity/configuration",
            "/context/identity/effective_build_configuration",
            "/context/identity/target_inventory",
            "/context/identity/source/files/lib.rs",
        ] {
            let mut b = a.clone();
            *b.pointer_mut(pointer).unwrap() = "mismatch".into();
            assert!(validate_pair(&a, &b, "none").is_err(), "accepted {pointer}");
        }
        let mut b = a;
        *b.pointer_mut("/phases/0/success").unwrap() = false.into();
        assert!(valid_receipt(&b).is_err());
        *b.pointer_mut("/phases/0/success").unwrap() = true.into();
        *b.pointer_mut("/phases/0/name").unwrap() = "cancelled-before-completion".into();
        assert!(valid_receipt(&b).is_err());
        assert!(valid_receipt(&Value::Null).is_err());
    }

    #[test]
    fn admits_exact_nested_profile_delta_and_refuses_extra_source_changes() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let after = receipt(holder.path());
        let mut before = after.clone();
        *before
            .pointer_mut("/context/identity/source/c_smoke")
            .unwrap() = "let profile = if cfg!(debug_assertions) {\n        \"dev\"\n    };".into();
        *before
            .pointer_mut("/context/identity/source/files/crates~1rdf-capi~1tests~1c_smoke.rs")
            .unwrap() = "before digest".into();
        validate_pair(&before, &after, "nested-profile").unwrap();
        assert!(validate_pair(&before, &after, "none").is_err());
        *before
            .pointer_mut("/context/identity/source/files/lib.rs")
            .unwrap() = "unrelated change".into();
        assert!(validate_pair(&before, &after, "nested-profile").is_err());
    }

    #[test]
    fn physical_roots_normalize_without_losing_package_or_target_identity() {
        let a: Value = Object::new()
            .with(
                "package",
                "path+file:///opt/arm-a/source/crates/rdf#purrdf-rdf@3.0.1",
            )
            .with("artifact", "/opt/arm-a/artifacts/target/debug/lib.so")
            .into();
        let b: Value = Object::new()
            .with(
                "package",
                "path+file:///opt/arm-b/source/crates/rdf#purrdf-rdf@3.0.1",
            )
            .with("artifact", "/opt/arm-b/artifacts/target/debug/lib.so")
            .into();
        assert_eq!(
            normalize(
                &a,
                &[
                    ("/opt/arm-a/source", "$SOURCE"),
                    ("/opt/arm-a/artifacts", "$ARM")
                ]
            ),
            normalize(
                &b,
                &[
                    ("/opt/arm-b/source", "$SOURCE"),
                    ("/opt/arm-b/artifacts", "$ARM")
                ]
            )
        );
        assert_ne!(
            normalize(&a, &[("/opt/arm-a", "$ARM")]),
            normalize(&b, &[("/opt/arm-b/source", "$SOURCE")])
        );
    }

    #[test]
    fn telemetry_preserves_selection_and_harness_arguments_and_nonbuild_commands() {
        let argv = [
            "test",
            "--workspace",
            "--locked",
            "--test",
            "[a-d]*",
            "--",
            "--exact",
            "--skip",
            "c_abi_smoke",
        ]
        .map(str::to_owned);
        let delegated = telemetry_arguments(&argv);
        let separator = delegated.iter().position(|arg| arg == "--").unwrap();
        assert_eq!(&delegated[separator..], &argv[5..]);
        assert!(delegated.contains(&"--timings".to_owned()));
        assert!(delegated.contains(&"--message-format=json-render-diagnostics".to_owned()));
        let argv = ["metadata", "--locked", "--format-version", "1"].map(str::to_owned);
        assert_eq!(telemetry_arguments(&argv), argv);
        let argv = [
            "build",
            "--profile",
            "test",
            "--message-format=json-render-diagnostics",
            "--timings",
            "--color=never",
        ]
        .map(str::to_owned);
        assert_eq!(telemetry_arguments(&argv), argv);
    }

    #[test]
    fn complete_comparison_records_aggregate_cost_but_never_invents_runner_wall_time() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let before_path = holder.path().join("before.json");
        let after_path = holder.path().join("after.json");
        let policy_path = holder.path().join("policy.json");
        let output_path = holder.path().join("comparison.json");
        let before = receipt(holder.path());
        std::fs::write(&before_path, json::write_pretty(&before)).unwrap();
        let mut after = before;
        *after.pointer_mut("/phases/0/elapsed_ns").unwrap() = "90".into();
        std::fs::write(&after_path, json::write_pretty(&after)).unwrap();
        let policy = Policy {
            before: vec![before_path.display().to_string()],
            after: vec![after_path.display().to_string()],
            change: "none".to_owned(),
            output: output_path.display().to_string(),
        };
        std::fs::write(&policy_path, json::write_pretty(&policy.to_json())).unwrap();
        compare(&policy_path).unwrap();
        let summary = read(&output_path).unwrap();
        assert_eq!(
            summary
                .get("after_aggregate_command_ns")
                .and_then(Value::as_str),
            Some("90")
        );
        assert_eq!(
            summary
                .get("aggregate_reduction_observed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            summary
                .get("critical_path")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("NOT MEASURED")
        );
        std::fs::remove_file(&output_path).unwrap();
        *after.pointer_mut("/phases/0/success").unwrap() = false.into();
        std::fs::write(&after_path, json::write_pretty(&after)).unwrap();
        assert!(compare(&policy_path).is_err());
        assert!(!output_path.exists());
        let policy = Policy {
            change: "native-partition".to_owned(),
            ..policy
        };
        std::fs::write(&policy_path, json::write_pretty(&policy.to_json())).unwrap();
        assert!(compare(&policy_path).is_err());
    }

    #[test]
    fn actual_delegate_failures_and_missing_requests_are_hard_and_retained() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let root = holder.path();
        assert!(run(&root.join("missing-request.json")).is_err());
        assert!(
            cargo_shim_at(
                &root.join("missing-Cargo"),
                root,
                vec!["--version".to_owned()]
            )
            .is_err()
        );
        let receipts = std::fs::read_dir(root)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(receipts.len(), 1);
        let receipt = read(&receipts[0].path()).unwrap();
        assert_eq!(
            receipt
                .pointer("/phases/0/success")
                .and_then(Value::as_bool),
            Some(false)
        );
        if std::env::var_os("PURRDF_PROFILE_REAL_CARGO").is_none() {
            assert!(cargo_shim(vec!["--version".to_owned()]).is_err());
        }
        let success = purrdf_testkit::temp_dir!().unwrap();
        cargo_shim_at(
            Path::new(env!("CARGO")),
            success.path(),
            vec!["--version".to_owned()],
        )
        .unwrap();
        let entry = std::fs::read_dir(success.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let receipt = read(&entry.path()).unwrap();
        assert_eq!(
            receipt
                .pointer("/phases/0/success")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            receipt.pointer("/context/original_argv"),
            receipt.pointer("/context/delegated_argv")
        );
        assert!(
            receipt
                .pointer("/phases/0/command/stdout")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("cargo ")
        );
    }

    #[test]
    fn cold_build_and_target_are_independent_and_warm_admission_is_explicit() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let target = holder.path().join("target");
        let build = holder.path().join("build");
        assert!(empty(&target).unwrap() && empty(&build).unwrap());
        std::fs::create_dir(&build).unwrap();
        std::fs::write(build.join("cached-unit"), "not cold").unwrap();
        assert!(empty(&target).unwrap());
        assert!(!empty(&build).unwrap());
        let mut request = Request {
            root: "/opt/source".to_owned(),
            directory: "/opt/task/arm".to_owned(),
            cargo: "/usr/bin/cargo".to_owned(),
            lane: "lib".to_owned(),
            warmth: "cold".to_owned(),
            previous: None,
            jobs: 8,
            test_threads: 2,
            runner_class: "bounded local".to_owned(),
            dependency_cache: "preprovisioned".to_owned(),
            compiler_cache: "unchanged enabled".to_owned(),
            page_cache: "uncontrolled".to_owned(),
        };
        validate_request(&request).unwrap();
        request.warmth = "warm".to_owned();
        assert!(validate_request(&request).is_err());
        request.previous = Some("/opt/task/cold-receipt.json".to_owned());
        validate_request(&request).unwrap();
        request.directory = "/opt/task/../shared".to_owned();
        assert!(validate_request(&request).is_err());
        request.directory = "/tmp/arm".to_owned();
        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn lost_or_changed_compiler_receipts_cannot_qualify_observed_reduction() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let receipt = receipt(holder.path());
        valid_receipt(&receipt).unwrap();
        std::fs::write(
            holder.path().join("cargo-child.json"),
            r#"{"phases":[{"success":false}]}"#,
        )
        .unwrap();
        assert!(valid_receipt(&receipt).is_err());
        std::fs::remove_file(holder.path().join("cargo-child.json")).unwrap();
        assert!(valid_receipt(&receipt).is_err());
    }

    #[test]
    fn warm_arm_retains_old_reports_but_attributes_only_new_or_changed_reports() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let target = holder.path().join("target");
        let timings = target.join("cargo-timings");
        std::fs::create_dir_all(&timings).unwrap();
        let old = timings.join("cold-report.html");
        let latest = timings.join("cargo-timing.html");
        std::fs::write(&old, "old cold data").unwrap();
        std::fs::write(&latest, "previous current data").unwrap();
        let before = timing_snapshot(&target).unwrap();
        let prefix = holder.path().join("warm-cargo-invocation");
        assert_eq!(
            current_timings(&target, &before, &prefix, None).unwrap(),
            [] as [Value; 0]
        );
        std::fs::write(&latest, "new warm data").unwrap();
        std::fs::write(timings.join("new-warm-report.html"), "new warm data").unwrap();
        let reports = current_timings(&target, &before, &prefix, None).unwrap();
        assert_eq!(reports.len(), 2);
        let attributed = current_timings(
            &target,
            &before,
            &holder.path().join("nested-invocation"),
            Some(&[timings.join("new-warm-report.html")]),
        )
        .unwrap();
        assert_eq!(attributed.len(), 1);
        assert!(
            attributed[0]
                .get("path")
                .unwrap()
                .as_str()
                .unwrap()
                .ends_with("nested-invocation-new-warm-report.html")
        );
        assert!(reports.iter().all(|report| {
            !report
                .get("path")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("cold-report")
        }));
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "old cold data");
        assert_eq!(
            std::fs::read_to_string(
                holder
                    .path()
                    .join("warm-cargo-invocation-cargo-timing.html")
            )
            .unwrap(),
            "new warm data"
        );
    }
}
