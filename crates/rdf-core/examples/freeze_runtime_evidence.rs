// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Manual hosted freeze evidence admission, linking and measurement.
//! Compiled against the candidate's libraries; both arms use that same testkit.
//! Successful collection is not a latency acceptance decision.

use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use purrdf_lex::json::{self, Value};
use purrdf_testkit::bench::{Estimates, Throughput};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ORIGINAL: &str = "4a23c91eab218cf497c4caa4f40ef264c33cba05";
const ARMS: [&str; 2] = ["original", "candidate"];
const CASES: [(&str, u64); 4] = [
    ("freeze_default", 10_300_000),
    ("materialize_default", 10_300_000),
    ("freeze_named", 11_700_000),
    ("materialize_named", 11_700_000),
];

fn require(ok: bool, message: impl Into<String>) -> Result<()> {
    if !ok {
        return Err(io::Error::other(message.into()).into());
    }
    Ok(())
}

fn parse(text: &str) -> Result<Value> {
    Ok(json::read_with(
        text,
        json::Limits {
            unique_members: true,
            ..json::Limits::DEFAULT
        },
    )?)
}

fn array(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| io::Error::other("expected JSON array").into())
}

fn field<'a>(value: &'a Value, pointer: &str) -> Result<&'a Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| io::Error::other(format!("missing {pointer}")).into())
}

fn string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str> {
    field(value, pointer)?
        .as_str()
        .ok_or_else(|| io::Error::other(format!("expected string at {pointer}")).into())
}

fn run(command: &mut Command, log: &Path) -> Result<()> {
    fs::write(log.with_extension("command"), format!("{command:?}\n"))?;
    let output = File::create(log)?;
    let status = command
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(output))
        .status()?;
    fs::write(log.with_extension("status"), format!("{status}\n"))?;
    require(
        status.success(),
        format!("command failed; inspect {}", log.display()),
    )
}

fn digest(path: &Path) -> Result<String> {
    Ok(purrdf_hash::hex::encode(
        purrdf_hash::blake3::hash(&fs::read(path)?).as_bytes(),
    ))
}

fn record(path: &Path, receipt: &mut File) -> Result<String> {
    let hash = digest(path)?;
    writeln!(receipt, "{hash}  {}", path.display())?;
    receipt.flush()?;
    Ok(hash)
}

fn profile(value: &Value) -> Result<()> {
    require(
        string(value, "/name")? == "bench"
            && string(value, "/opt_level")? == "3"
            && string(value, "/lto")? == "fat"
            && field(value, "/codegen_units")?.as_u64() == Some(1)
            && field(value, "/debuginfo")?.as_u64() == Some(0)
            && field(value, "/debug_assertions")?.as_bool() == Some(false)
            && field(value, "/overflow_checks")?.as_bool() == Some(false)
            && field(value, "/incremental")?.as_bool() == Some(false)
            && string(value, "/panic")? == "unwind"
            && string(value, "/strip/resolved")? == "None",
        "wrong effective bench profile",
    )
}

struct Library {
    rlib: PathBuf,
    metadata: PathBuf,
}

fn library(
    root: &Path,
    evidence: &Path,
    arm: &str,
    crate_name: &str,
    source: &str,
    receipt: &mut File,
) -> Result<Library> {
    let graph = parse(&fs::read_to_string(
        evidence.join(format!("{arm}-units.json")),
    )?)?;
    let src = root.join(source).canonicalize()?;
    let units: Vec<_> = array(field(&graph, "/units")?)?
        .iter()
        .filter(|unit| {
            unit.pointer("/target/src_path").and_then(Value::as_str) == src.to_str()
                && unit.get("mode").and_then(Value::as_str) == Some("build")
        })
        .collect();
    require(
        units.len() == 1,
        format!("expected one build unit for {arm}/{crate_name}"),
    )?;
    profile(field(units[0], "/profile")?)?;
    require(
        array(field(units[0], "/features")?)?.is_empty(),
        "semantic features selected",
    )?;
    let messages: Vec<_> = fs::read_to_string(evidence.join(format!("{arm}-cargo.json")))?
        .lines()
        .map(parse)
        .collect::<Result<_>>()?;
    require(
        messages
            .last()
            .and_then(|m| m.get("success"))
            .and_then(Value::as_bool)
            == Some(true),
        "Cargo did not finish successfully",
    )?;
    let artifact = artifact(&messages, units[0], &src, root, crate_name, source)?;
    let files: Vec<_> = array(field(artifact, "/filenames")?)?
        .iter()
        .filter_map(Value::as_str)
        .filter(|path| Path::new(path).extension().is_some_and(|ext| ext == "rlib"))
        .collect();
    require(files.len() == 1, "expected one exact rlib")?;
    let path = PathBuf::from(files[0]).canonicalize()?;
    require(
        path.starts_with(evidence.join(format!("{arm}-target")).canonicalize()?),
        "artifact outside private target",
    )?;
    let hash = record(&path, receipt)?;
    let alias = evidence.join(format!("{arm}-target/release/lib{crate_name}.rlib"));
    require(
        record(&alias, receipt)? == hash,
        "rlib alias disagrees with Cargo artifact",
    )?;
    let metadata: Vec<_> = array(field(artifact, "/filenames")?)?
        .iter()
        .filter_map(Value::as_str)
        .filter(|path| {
            Path::new(path)
                .extension()
                .is_some_and(|ext| ext == "rmeta")
        })
        .collect();
    require(metadata.len() == 1, "expected one exact metadata artifact")?;
    let metadata = PathBuf::from(metadata[0]).canonicalize()?;
    require(
        metadata.starts_with(evidence.join(format!("{arm}-target")).canonicalize()?),
        "metadata outside private target",
    )?;
    record(&metadata, receipt)?;
    Ok(Library {
        rlib: path,
        metadata,
    })
}

fn artifact<'a>(
    messages: &'a [Value],
    unit: &Value,
    src: &Path,
    root: &Path,
    crate_name: &str,
    source: &str,
) -> Result<&'a Value> {
    let artifacts: Vec<_> = messages
        .iter()
        .filter(|m| {
            m.get("reason").and_then(Value::as_str) == Some("compiler-artifact")
                && m.get("manifest_path").and_then(Value::as_str)
                    == root
                        .join(source)
                        .parent()
                        .and_then(Path::parent)
                        .map(|p| p.join("Cargo.toml"))
                        .as_ref()
                        .and_then(|p| p.to_str())
                && m.pointer("/target/name").and_then(Value::as_str) == Some(crate_name)
        })
        .collect();
    require(
        artifacts.len() == 1,
        format!("expected one artifact for {crate_name}"),
    )?;
    let artifact = artifacts[0];
    require(
        string(artifact, "/package_id")? == string(unit, "/pkg_id")?
            && string(artifact, "/target/src_path")?
                == src.to_str().ok_or("non-UTF-8 source path")?,
        "artifact source disagrees with the selected unit",
    )?;
    require(
        array(field(artifact, "/features")?)?.is_empty(),
        "artifact has semantic features",
    )?;
    require(
        string(artifact, "/profile/opt_level")? == "3"
            && field(artifact, "/profile/test")?.as_bool() == Some(false)
            && field(artifact, "/profile/debuginfo")?.as_u64() == Some(0)
            && field(artifact, "/profile/debug_assertions")?.as_bool() == Some(false)
            && field(artifact, "/profile/overflow_checks")?.as_bool() == Some(false)
            && array(field(artifact, "/target/kind")?)?.len() == 1
            && array(field(artifact, "/target/kind")?)?
                .iter()
                .all(|kind| matches!(kind.as_str(), Some("lib" | "rlib"))),
        "wrong Cargo artifact profile",
    )?;
    Ok(artifact)
}

fn validate_result(store: &Path, log: &Path, case: &str, rows: u64) -> Result<Estimates> {
    let expected = store.join(format!("native_freeze/{case}/{rows}/new/estimates.json"));
    let mut pending = vec![store.to_path_buf()];
    let mut estimates = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else if entry.file_name() == "estimates.json" {
                estimates.push(entry.path());
            }
        }
    }
    require(
        estimates == [expected.clone()],
        "missing, extra or wrong selected benchmark",
    )?;
    let result = Estimates::from_json(&fs::read_to_string(expected)?)?;
    require(
        result.samples.len() == 10
            && result.iterations_per_sample > 0
            && result
                .samples
                .iter()
                .all(|sample| sample.is_finite() && *sample > 0.0)
            && result.throughput == Some(Throughput::Elements(rows)),
        "partial, nonpositive or wrong-sized benchmark results",
    )?;
    let text = fs::read_to_string(log)?;
    require(
        text.matches("1 benchmark measured; 0 failed").count() == 1,
        "benchmark did not measure exactly one case",
    )?;
    Ok(result)
}

fn host(directory: &Path) -> Result<()> {
    fs::create_dir(directory)?;
    for name in [
        "cpuinfo",
        "meminfo",
        "loadavg",
        "stat",
        "self/status",
        "self/cgroup",
    ] {
        fs::copy(
            format!("/proc/{name}"),
            directory.join(name.replace('/', "-")),
        )?;
    }
    // These unified-cgroup counters expose throttling; their absence is recorded.
    for name in ["cpu.stat", "cpu.max", "cpuset.cpus.effective", "memory.max"] {
        let path = Path::new("/sys/fs/cgroup").join(name);
        match fs::read(&path) {
            Ok(data) => fs::write(directory.join(name), data)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::write(directory.join(name), "not present\n")?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    run(&mut Command::new("lscpu"), &directory.join("lscpu.log"))
}

fn collect(evidence: &Path, checkout: &Path, candidate: &str) -> Result<()> {
    require(
        candidate.len() == 40
            && candidate
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "candidate must be a full lowercase commit SHA",
    )?;
    require(
        std::env::var("RUSTFLAGS")? == "-D warnings -C target-cpu=native",
        "unexpected Rust flags",
    )?;
    require(
        std::env::var_os("CARGO_ENCODED_RUSTFLAGS").is_none(),
        "encoded Rust flags are not admitted",
    )?;
    let mut receipt = File::create(evidence.join("blake3-identities.txt"))?;
    for name in [
        "original-source.tar.gz",
        "candidate-source.tar.gz",
        "freeze_runtime_evidence.rs",
        "freeze_original_api.patch",
        "benchmarks.yaml",
        "rustc.txt",
        "cargo.txt",
        "native-cfg.txt",
        "safe-build-environment.txt",
    ] {
        record(&evidence.join(name), &mut receipt)?;
    }
    record(&std::env::current_exe()?, &mut receipt)?;
    let mut libraries = Vec::new();
    for (arm, sha) in ARMS.into_iter().zip([ORIGINAL, candidate]) {
        let root = checkout.join(format!(".worktrees/freeze-runtime-{arm}"));
        run(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&root),
            &evidence.join(format!("{arm}-head.log")),
        )?;
        require(
            fs::read_to_string(evidence.join(format!("{arm}-head.log")))?.trim() == sha,
            "source revision mismatch",
        )?;
        run(
            Command::new("git")
                .args(["status", "--porcelain", "--untracked-files=all"])
                .current_dir(&root),
            &evidence.join(format!("{arm}-clean.log")),
        )?;
        require(
            fs::read_to_string(evidence.join(format!("{arm}-clean.log")))?.is_empty(),
            "source tree is dirty",
        )?;
        record(&root.join("Cargo.lock"), &mut receipt)?;
        record(&root.join("Cargo.toml"), &mut receipt)?;
        libraries.push(library(
            &root,
            evidence,
            arm,
            "purrdf_core",
            "crates/rdf-core/src/lib.rs",
            &mut receipt,
        )?);
    }
    let current = checkout.join(".worktrees/freeze-runtime-candidate");
    let harness = library(
        &current,
        evidence,
        "candidate",
        "purrdf_testkit",
        "crates/testkit/src/lib.rs",
        &mut receipt,
    )?;
    for (name, source) in [
        ("purrdf_lex", "crates/lex/src/lib.rs"),
        ("purrdf_hash", "crates/hash/src/lib.rs"),
    ] {
        library(&current, evidence, "candidate", name, source, &mut receipt)?;
    }
    let mut identities = Vec::new();
    // Preserve the complete transitive library identities, not only the direct rlibs.
    for arm in ARMS {
        for entry in fs::read_dir(evidence.join(format!("{arm}-target/release/deps")))? {
            let path = entry?.path();
            if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("rlib" | "rmeta" | "so")
            ) {
                identities.push((path.clone(), record(&path, &mut receipt)?));
            }
        }
    }
    for (index, arm) in ARMS.iter().enumerate() {
        let export = evidence.join(format!("{arm}-harness"));
        fs::create_dir_all(export.join("support"))?;
        for source in ["freeze.rs", "support/freeze_fixture.rs"] {
            fs::copy(
                current.join("crates/rdf-core/benches").join(source),
                export.join(source),
            )?;
        }
        if *arm == "original" {
            let patch = checkout.join("crates/rdf-core/examples/support/freeze_original_api.patch");
            record(&patch, &mut receipt)?;
            // --fuzz=0 prevents accepting an adapter against changed source context.
            run(
                Command::new("patch")
                    .args(["--batch", "--fuzz=0", "-p1", "-i"])
                    .arg(patch)
                    .current_dir(&export),
                &evidence.join("adapter.log"),
            )?;
        }
        for source in ["freeze.rs", "support/freeze_fixture.rs"] {
            record(&export.join(source), &mut receipt)?;
        }
        let executable = evidence.join(format!("{arm}-freeze"));
        run(
            Command::new("rustc")
                .args([
                    "--edition=2024",
                    "-D",
                    "warnings",
                    "-C",
                    "opt-level=3",
                    "-C",
                    "lto=fat",
                    "-C",
                    "codegen-units=1",
                    "-C",
                    "debuginfo=0",
                    "-C",
                    "strip=none",
                    "-C",
                    "target-cpu=native",
                ])
                .arg(export.join("freeze.rs"))
                .arg("-L")
                .arg(format!(
                    "dependency={}",
                    evidence
                        .join(format!("{arm}-target/release/deps"))
                        .display()
                ))
                .arg("-L")
                .arg(format!(
                    "dependency={}",
                    evidence.join("candidate-target/release/deps").display()
                ))
                .arg("--extern")
                .arg(format!("purrdf_core={}", libraries[index].rlib.display()))
                .arg("--extern")
                .arg(format!(
                    "purrdf_core={}",
                    libraries[index].metadata.display()
                ))
                .arg("--extern")
                .arg(format!("purrdf_testkit={}", harness.rlib.display()))
                .arg("--extern")
                .arg(format!("purrdf_testkit={}", harness.metadata.display()))
                .arg("-o")
                .arg(&executable),
            &evidence.join(format!("{arm}-link.log")),
        )?;
        identities.push((executable.clone(), record(&executable, &mut receipt)?));
    }
    let mut summary = File::create(evidence.join("measurements.tsv"))?;
    writeln!(summary, "case\trows\tarm\tmedian_ns\tci_low_ns\tci_high_ns")?;
    for (index, (case, rows)) in CASES.into_iter().enumerate() {
        let order = if index % 2 == 0 {
            ARMS
        } else {
            [ARMS[1], ARMS[0]]
        };
        for arm in order {
            let name = format!("{index}-{case}-{arm}");
            let store = evidence.join(&name);
            fs::create_dir(&store)?;
            host(&evidence.join(format!("{name}-before")))?;
            let log = evidence.join(format!("{name}.log"));
            let outcome = run(
                Command::new("/usr/bin/time")
                    .arg("-v")
                    .arg("-o")
                    .arg(evidence.join(format!("{name}-resources.txt")))
                    .arg(evidence.join(format!("{arm}-freeze")))
                    .args([
                        "--bench",
                        "--exact",
                        &format!("native_freeze/{case}/{rows}"),
                    ])
                    .env("PURRDF_FREEZE_BENCH_ROWS", rows.to_string())
                    .env("PURRDF_BENCH_HOME", &store),
                &log,
            );
            host(&evidence.join(format!("{name}-after")))?;
            outcome?;
            let result = validate_result(&store, &log, case, rows)?;
            writeln!(
                summary,
                "{case}\t{rows}\t{arm}\t{}\t{}\t{}",
                result.median, result.ci_low, result.ci_high
            )?;
            summary.flush()?;
        }
    }
    for (path, hash) in identities {
        require(
            record(&path, &mut receipt)? == hash,
            format!("artifact changed: {}", path.display()),
        )?;
    }
    fs::write(
        evidence.join("collection-complete.txt"),
        "Eight full-size ten-sample invocations collected. Latency acceptance requires review.\n",
    )?;
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    require(
        arguments.len() == 3,
        "usage: freeze_runtime_evidence EVIDENCE CHECKOUT CANDIDATE_SHA",
    )?;
    collect(
        Path::new(&arguments[0]),
        Path::new(&arguments[1]),
        arguments[2].to_str().ok_or("non-UTF-8 candidate")?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_only_the_effective_bench_contract() -> Result<()> {
        let valid = r#"{"name":"bench","opt_level":"3","lto":"fat","codegen_units":1,"debuginfo":0,"debug_assertions":false,"overflow_checks":false,"incremental":false,"panic":"unwind","strip":{"resolved":"None"}}"#;
        profile(&parse(valid)?)?;
        for changed in [
            valid.replace("\"fat\"", "\"thin\""),
            valid.replace("\"codegen_units\":1", "\"codegen_units\":2"),
            valid.replace("\"opt_level\":\"3\"", "\"opt_level\":\"0\""),
        ] {
            assert!(profile(&parse(&changed)?).is_err());
        }
        assert!(parse(r#"{"features":[],"features":["changed"]}"#).is_err());
        Ok(())
    }

    #[test]
    fn refuses_missing_partial_wrong_sized_and_zero_selected_measurements() -> Result<()> {
        let executable = std::env::current_exe()?;
        let directory =
            purrdf_testkit::TempDir::new_in(executable.parent().ok_or("no executable directory")?)?;
        let store = directory.path().join("store");
        fs::create_dir(&store)?;
        let log = directory.path().join("run.log");
        fs::write(&log, "1 benchmark measured; 0 failed\n")?;
        let case = "freeze_default";
        let rows = 10_300_000;
        assert!(validate_result(&store, &log, case, rows).is_err());
        let path = store.join(format!("native_freeze/{case}/{rows}/new"));
        fs::create_dir_all(&path)?;
        let mut estimates =
            Estimates::from_samples(vec![10.0; 10], 1, Some(Throughput::Elements(rows)), 1);
        let file = path.join("estimates.json");
        fs::write(&file, estimates.to_json()?)?;
        validate_result(&store, &log, case, rows)?;
        fs::write(&log, "0 benchmarks measured; 0 failed\n")?;
        assert!(validate_result(&store, &log, case, rows).is_err());
        fs::write(&log, "1 benchmark measured; 0 failed\n")?;
        estimates.samples.pop();
        fs::write(&file, estimates.to_json()?)?;
        assert!(validate_result(&store, &log, case, rows).is_err());
        estimates.samples.push(10.0);
        estimates.throughput = Some(Throughput::Elements(rows - 1));
        fs::write(&file, estimates.to_json()?)?;
        assert!(validate_result(&store, &log, case, rows).is_err());
        Ok(())
    }
}
