// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Host process receipts. Cargo, never a receipt, decides artifact freshness.

use purrdf_lex::json::record::ToJson as _;
use purrdf_lex::json::{self, Object, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

type IoResult<T> = std::io::Result<T>;
pub(crate) fn invalid(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

#[derive(Debug)]
pub(crate) struct Artifact {
    pub(crate) path: PathBuf,
    pub(crate) message: Value,
}

/// Parse admitted Cargo frames once. JSON-only callers set `with_harness` false;
/// test children may additionally emit ordinary libtest/doc output.
pub(crate) fn cargo_messages(messages: &[u8], with_harness: bool) -> IoResult<Vec<Value>> {
    let text = std::str::from_utf8(messages).map_err(|error| invalid(error.to_string()))?;
    text.lines()
        .filter(|line| {
            // Cargo frames begin at column zero; libtest/doc output is separate.
            // Malformed admitted frames remain errors, never ignored failures.
            if with_harness {
                line.starts_with('{')
            } else {
                !line.trim().is_empty()
            }
        })
        .map(|line| {
            json::read_with(
                line,
                json::Limits {
                    unique_members: true,
                    ..json::Limits::DEFAULT
                },
            )
            .map_err(|error| invalid(error.to_string()))
        })
        .collect()
}

/// Select from an already admitted complete invocation, never across children.
pub(crate) fn cdylib(messages: &[Value], package_id: &str, filename: &str) -> IoResult<Artifact> {
    let mut selected = None;
    for message in messages {
        if message.get("reason").and_then(Value::as_str) != Some("compiler-artifact")
            || message.get("package_id").and_then(Value::as_str) != Some(package_id)
        {
            continue;
        }
        let kinds = message
            .pointer("/target/kind")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("artifact target kind missing"))?;
        if !kinds.iter().any(|kind| kind == "cdylib") {
            continue;
        }
        if kinds.iter().any(|kind| kind.as_str().is_none()) {
            return Err(invalid("non-string artifact target kind"));
        }
        if message.pointer("/target/name").and_then(Value::as_str) != Some("purrdf")
            || message
                .pointer("/profile/opt_level")
                .and_then(Value::as_str)
                .is_none()
            || message
                .pointer("/profile/debug_assertions")
                .and_then(Value::as_bool)
                .is_none()
            || message
                .pointer("/profile/overflow_checks")
                .and_then(Value::as_bool)
                .is_none()
            || message
                .pointer("/profile/test")
                .and_then(Value::as_bool)
                .is_none()
            || message.get("features").and_then(Value::as_array).is_none()
            || message.get("fresh").and_then(Value::as_bool).is_none()
        {
            return Err(invalid("malformed CAPI artifact metadata"));
        }
        if message
            .get("features")
            .and_then(Value::as_array)
            .is_some_and(|features| features.iter().any(|feature| feature.as_str().is_none()))
        {
            return Err(invalid("non-string Cargo feature"));
        }
        for value in message
            .get("filenames")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("artifact filenames missing"))?
        {
            let path = PathBuf::from(
                value
                    .as_str()
                    .ok_or_else(|| invalid("non-string artifact filename"))?,
            );
            if !path.is_absolute() {
                return Err(invalid("Cargo artifact path is not absolute"));
            }
            if path.file_name().and_then(|name| name.to_str()) == Some(filename) {
                if selected.is_some() {
                    return Err(invalid("ambiguous CAPI cdylib"));
                }
                selected = Some(Artifact {
                    path,
                    message: message.clone(),
                });
            }
        }
    }
    selected.ok_or_else(|| invalid("Cargo did not report the exact CAPI cdylib"))
}

pub(crate) fn identity(path: &Path) -> IoResult<Value> {
    let bytes = std::fs::read(path)?;
    Ok(Object::new()
        .with("path", path.display().to_string())
        .with("bytes", bytes.len() as u64)
        .with(
            "blake3",
            purrdf_hash::hex::encode(purrdf_hash::blake3::hash(&bytes).as_bytes()),
        )
        .into())
}

#[derive(Clone)]
struct Phase {
    name: String,
    elapsed_ns: String,
    command: Value,
    success: bool,
    error: Option<String>,
}
purrdf_lex::json_record!(Phase as "process phase" {
    "name" => name: required,
    "elapsed_ns" => elapsed_ns: required,
    "command" => command: required,
    "success" => success: required,
    "error" => error: optional,
});

pub(crate) struct Recorder {
    path: PathBuf,
    context: Value,
    phases: Vec<Phase>,
}
impl Recorder {
    pub(crate) fn new(path: PathBuf, context: Value) -> IoResult<Self> {
        let recorder = Self {
            path,
            context,
            phases: Vec::new(),
        };
        recorder.save()?;
        Ok(recorder)
    }
    fn save(&self) -> IoResult<()> {
        let document: Value = Object::new()
            .with("schema", "purrdf-process-phases-v1")
            .with("context", self.context.clone())
            .with(
                "phases",
                self.phases.iter().map(Phase::to_json).collect::<Vec<_>>(),
            )
            .into();
        std::fs::write(&self.path, json::write_pretty(&document))
    }
    pub(crate) fn evidence(&mut self, name: &str, value: Value) -> IoResult<()> {
        self.context
            .as_object_mut()
            .ok_or_else(|| invalid("receipt context must be an object"))?
            .insert(name, value);
        self.save()
    }
    pub(crate) fn check<T>(
        &mut self,
        name: &str,
        check: impl FnOnce() -> IoResult<T>,
    ) -> IoResult<T> {
        let start = Instant::now();
        let result = check();
        self.record(
            name,
            start,
            Value::Null,
            result.as_ref().err().map(ToString::to_string),
        )?;
        result
    }
    fn record(
        &mut self,
        name: &str,
        start: Instant,
        command: Value,
        error: Option<String>,
    ) -> IoResult<()> {
        self.phases.push(Phase {
            name: name.to_owned(),
            elapsed_ns: start.elapsed().as_nanos().to_string(),
            command,
            success: error.is_none(),
            error,
        });
        self.save()
    }
    pub(crate) fn run(&mut self, name: &str, command: &mut Command) -> IoResult<Output> {
        let start = Instant::now();
        let argv: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let env: Vec<Value> = command
            .get_envs()
            .map(|(key, value)| {
                Object::new()
                    .with("key", key.to_string_lossy().into_owned())
                    .with(
                        "value",
                        value.map(|value| value.to_string_lossy().into_owned()),
                    )
                    .into()
            })
            .collect();
        let mut invocation = Object::new()
            .with(
                "program",
                command.get_program().to_string_lossy().into_owned(),
            )
            .with("argv", argv)
            .with("env", env)
            .with(
                "cwd",
                command
                    .get_current_dir()
                    .map(|path| path.display().to_string()),
            );
        let result = command.output();
        if let Ok(output) = &result {
            invocation = invocation
                .with("exit_code", output.status.code())
                .with("exit_status", output.status.to_string())
                .with(
                    "stdout",
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                )
                .with(
                    "stderr",
                    String::from_utf8_lossy(&output.stderr).into_owned(),
                );
        }
        let failure = match &result {
            Ok(output) if output.status.success() => None,
            Ok(output) => Some(format!(
                "{name}: {}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )),
            Err(error) => Some(format!("{name}: {error}")),
        };
        self.record(name, start, invocation.into(), failure.clone())?;
        if let Some(error) = failure {
            return Err(invalid(error));
        }
        result
    }
}

fn configuration_inputs(root: &Path) -> IoResult<Vec<Value>> {
    let mut inputs = Vec::new();
    for path in [
        root.join("Cargo.lock"),
        root.join("Cargo.toml"),
        root.join("rust-toolchain.toml"),
    ] {
        inputs.push(identity(&path)?);
    }
    let mut paths = Vec::new();
    for ancestor in root.ancestors() {
        for name in ["config", "config.toml"] {
            paths.push(ancestor.join(".cargo").join(name));
        }
    }
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .ok_or_else(|| invalid("Cargo home cannot be resolved"))?;
    for name in ["config", "config.toml"] {
        paths.push(cargo_home.join(name));
    }
    paths.sort();
    paths.dedup();
    for path in paths {
        match identity(&path) {
            Ok(value) => inputs.push(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(inputs)
}

fn tool_identities(root: &Path, cargo: &str, cc: &str) -> IoResult<Vec<Value>> {
    let mut tools: Vec<Value> = Vec::new();
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    for (program, args) in [
        (cargo, vec!["--version"]),
        (rustc.as_str(), vec!["-Vv"]),
        (cc, vec!["--version"]),
    ] {
        let output = Command::new(program)
            .args(args)
            .current_dir(root)
            .output()?;
        if !output.status.success() {
            return Err(invalid(format!("tool identity failed: {program}")));
        }
        tools.push(
            Object::new()
                .with("program", program)
                .with(
                    "identity",
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                )
                .into(),
        );
    }
    Ok(tools)
}

fn untracked_paths(bytes: &[u8]) -> IoResult<Vec<&str>> {
    if !bytes.is_empty() && bytes.last() != Some(&0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Git untracked path inventory is not NUL-terminated",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|error| std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("cannot record non-UTF-8 Git source paths exactly; rename the path to UTF-8 before profiling: {error}")))?;
    text.split_terminator('\0')
        .map(|path| {
            if path.is_empty() {
                Err(invalid("empty Git source path"))
            } else {
                Ok(path)
            }
        })
        .collect()
}

fn source_state(root: &Path, inputs: &mut Vec<Value>) -> IoResult<Value> {
    let mut source = Object::new();
    for (name, args) in [
        ("head", vec!["rev-parse", "HEAD"]),
        ("tracked", vec!["ls-files", "-s"]),
        ("changes", vec!["diff", "HEAD", "--binary"]),
        (
            "untracked",
            vec![
                "ls-files",
                "-z",
                "--others",
                "--exclude-standard",
                "--",
                ".",
                ":(exclude).stage",
                ":(exclude).worktrees",
            ],
        ),
    ] {
        let output = Command::new("git").args(args).current_dir(root).output()?;
        if !output.status.success() {
            return Err(invalid(format!("source identity failed: {name}")));
        }
        source = source.with(name, String::from_utf8_lossy(&output.stdout).into_owned());
        if name == "untracked" {
            for path in untracked_paths(&output.stdout)? {
                inputs.push(identity(&root.join(path))?);
            }
        }
    }
    Ok(source.into())
}

pub(crate) fn context(root: &Path, cargo: &str, cc: &str) -> IoResult<Value> {
    let mut inputs = configuration_inputs(root)?;
    let tools = tool_identities(root, cargo, cc)?;
    let source = source_state(root, &mut inputs)?;
    let mut environment: Vec<(String, String)> = std::env::vars()
        .filter(|(key, _)| {
            key.starts_with("CARGO_PROFILE_")
                || key.starts_with("CARGO_TARGET_")
                || key.starts_with("CARGO_BUILD_")
                || [
                    "CARGO_BUILD_JOBS",
                    "CARGO_TARGET_DIR",
                    "CARGO_BUILD_BUILD_DIR",
                    "CARGO_ENCODED_RUSTFLAGS",
                    "CARGO_HOME",
                    "CARGO_INCREMENTAL",
                    "RUSTUP_TOOLCHAIN",
                    "RUSTC_BOOTSTRAP",
                    "RUSTFLAGS",
                    "RUSTC",
                    "RUSTC_WRAPPER",
                    "RUSTC_WORKSPACE_WRAPPER",
                    "RUST_TEST_THREADS",
                    "CC",
                    "CFLAGS",
                ]
                .contains(&key.as_str())
        })
        .collect();
    environment.sort();
    let environment: Vec<Value> = environment
        .into_iter()
        .map(|(key, value)| Object::new().with("key", key).with("value", value).into())
        .collect();
    Ok(Object::new()
        .with("inputs", inputs)
        .with("environment", environment)
        .with("tools", tools)
        .with("source", source)
        .with(
            "available_parallelism",
            std::thread::available_parallelism()?.get() as u64,
        )
        .with("host_os", std::env::consts::OS)
        .with("host_arch", std::env::consts::ARCH)
        .into())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn nul_inventory_preserves_literal_paths_and_refuses_lossy_decoding() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let root = holder.path();
        #[cfg(unix)]
        let names = vec![
            "space and Unicode café.rs",
            "quote\"and\\slash.rs",
            "line\nbreak.rs",
        ];
        #[cfg(not(unix))]
        let names = vec!["space and Unicode café.rs"];
        let mut inventory = Vec::new();
        for (index, name) in names.iter().enumerate() {
            std::fs::write(root.join(name), index.to_string()).unwrap();
            inventory.extend_from_slice(name.as_bytes());
            inventory.push(0);
        }
        let decoded = untracked_paths(&inventory).unwrap();
        assert_eq!(decoded, names);
        for (index, name) in decoded.iter().enumerate() {
            let input = identity(&root.join(name)).unwrap();
            assert_eq!(
                input.get("blake3").and_then(Value::as_str),
                Some(
                    purrdf_hash::hex::encode(
                        purrdf_hash::blake3::hash(index.to_string().as_bytes()).as_bytes()
                    )
                    .as_str()
                )
            );
            std::fs::remove_file(root.join(name)).unwrap();
        }
        assert!(untracked_paths(b"unterminated").is_err());
        assert_eq!(
            untracked_paths(&[0xff, 0]).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert!(untracked_paths(b"\0").is_err());
        assert_eq!(untracked_paths(b"").unwrap(), [] as [&str; 0]);
    }

    pub(crate) fn artifact(package: &str, fresh: Value, filenames: Value) -> String {
        json::write_compact(
            &Object::new()
                .with("reason", "compiler-artifact")
                .with("package_id", package)
                .with(
                    "target",
                    Value::from(
                        Object::new()
                            .with("name", "purrdf")
                            .with("src_path", "/fixture/lib.rs")
                            .with("kind", vec!["cdylib", "rlib"]),
                    ),
                )
                .with(
                    "profile",
                    Value::from(
                        Object::new()
                            .with("opt_level", "3")
                            .with("debug_assertions", true)
                            .with("overflow_checks", true)
                            .with("test", false),
                    ),
                )
                .with("features", Vec::<Value>::new())
                .with("fresh", fresh)
                .with("filenames", filenames)
                .into(),
        )
    }

    #[test]
    fn mixed_harness_frames_preserve_strict_artifact_selection() {
        let exact = artifact("capi", false.into(), vec!["/exact/libpurrdf.so"].into());
        let mixed = format!(
            "{{\"reason\":\"build-started\"}}\n{exact}\n{{\"reason\":\"build-finished\",\"success\":true}}\nrunning 5 tests\ntest example ... ok\ntest result: ok. 5 passed\n"
        );
        assert!(cargo_messages(mixed.as_bytes(), false).is_err());
        let messages = cargo_messages(mixed.as_bytes(), true).unwrap();
        assert_eq!(messages.len(), 3);
        let selected = cdylib(&messages, "capi", "libpurrdf.so").unwrap();
        assert_eq!(selected.message, json::read(&exact).unwrap());
        assert_eq!(selected.path, Path::new("/exact/libpurrdf.so"));
        for suffix in ["{malformed", "{\"reason\":1,\"reason\":2}"] {
            assert!(cargo_messages(format!("{mixed}{suffix}\n").as_bytes(), true).is_err());
        }
        let duplicate = cargo_messages(format!("{mixed}{exact}\n").as_bytes(), true).unwrap();
        assert!(cdylib(&duplicate, "capi", "libpurrdf.so").is_err());
        assert!(cdylib(&messages, "unrelated", "libpurrdf.so").is_err());
        assert!(cdylib(&messages, "capi", "wrong.so").is_err());
    }

    #[test]
    fn selects_exact_package_and_retains_freshness_metadata() {
        let unrelated = artifact("other", true.into(), vec!["/other/libpurrdf.so"].into());
        let exact = artifact("capi", false.into(), vec!["/exact/libpurrdf.so"].into());
        let messages = format!("{unrelated}\n{exact}\n");
        let selected = cdylib(
            &cargo_messages(messages.as_bytes(), false).unwrap(),
            "capi",
            "libpurrdf.so",
        )
        .unwrap();
        assert_eq!(selected.path, Path::new("/exact/libpurrdf.so"));
        assert_eq!(
            selected.message.get("fresh").and_then(Value::as_bool),
            Some(false)
        );
        let warm = artifact("capi", true.into(), vec!["/exact/libpurrdf.so"].into());
        assert_eq!(
            cdylib(
                &cargo_messages(warm.as_bytes(), false).unwrap(),
                "capi",
                "libpurrdf.so",
            )
            .unwrap()
            .message
            .get("fresh")
            .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn rejects_malformed_missing_unrelated_and_ambiguous_artifacts() {
        let valid = artifact("capi", true.into(), vec!["/exact/libpurrdf.so"].into());
        for input in [
            "{".to_owned(),
            String::new(),
            artifact("other", true.into(), vec!["/exact/libpurrdf.so"].into()),
            artifact("capi", "true".into(), vec!["/exact/libpurrdf.so"].into()),
            artifact("capi", true.into(), vec!["/exact/wrong.so"].into()),
            artifact("capi", true.into(), vec![1_u64].into()),
            format!("{valid}\n{valid}"),
            valid.replace("\"name\":\"purrdf\"", "\"name\":\"unrelated\""),
            valid.replace("\"features\":[]", "\"features\":[4]"),
            valid.replace("/exact/libpurrdf.so", "libpurrdf.so"),
            valid.replace("\"fresh\":true", "\"fresh\":true,\"fresh\":false"),
        ] {
            assert!(
                cargo_messages(input.as_bytes(), false)
                    .and_then(|messages| cdylib(&messages, "capi", "libpurrdf.so"))
                    .is_err(),
                "accepted {input}"
            );
        }
        assert!(cargo_messages(&[0xff], false).is_err());
    }

    #[test]
    fn child_and_validation_failures_are_recorded_and_receipt_failures_are_hard() {
        let holder = purrdf_testkit::temp_dir!().unwrap();
        let path = holder.path().join("failure-fixture.json");
        let mut recorder = Recorder::new(path.clone(), Object::new().into()).unwrap();
        assert!(
            recorder
                .run(
                    "missing-child",
                    &mut Command::new(path.with_extension("nonexistent-program"))
                )
                .is_err()
        );
        let mut rustc = Command::new("rustc");
        rustc.arg("--definitely-not-a-rustc-option");
        assert!(recorder.run("nonzero-child", &mut rustc).is_err());
        assert!(
            recorder
                .check("output-validation", || Err::<(), _>(invalid("bad output")))
                .is_err()
        );
        let receipt = json::read(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let phases = receipt.get("phases").unwrap().as_array().unwrap();
        assert_eq!(phases.len(), 3);
        assert!(
            phases
                .iter()
                .all(|phase| phase.get("success").and_then(Value::as_bool) == Some(false))
        );
        assert!(
            phases[1]
                .pointer("/command/exit_code")
                .unwrap()
                .as_number()
                .is_some()
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(
            recorder
                .run(
                    "successful-child-unwritable-receipt",
                    Command::new("rustc").arg("--version")
                )
                .is_err()
        );
        assert!(
            recorder
                .check("successful-check-unwritable-receipt", || Ok(()))
                .is_err()
        );
        assert!(Recorder::new(path.clone(), Object::new().into()).is_err());
        std::fs::remove_dir(path).unwrap();
    }
}
