// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{
    error::Error,
    fs,
    io::{Error as IoError, ErrorKind},
    path::Path,
};

fn invalid(message: &str) -> IoError {
    IoError::new(ErrorKind::InvalidData, message)
}

enum Fence {
    Other,
    Rust { body_start: usize },
}

/// Read the one Rust program from the one Rust section without rewriting its bytes.
/// Recognizes the exact, top-level triple-backtick fences used by the READMEs.
fn sample(readme: &str) -> Result<&str, &'static str> {
    let mut offset = 0;
    let mut seen_heading = false;
    let mut in_section = false;
    let mut fence = None;
    let mut sample = None;

    for raw_line in readme.split_inclusive('\n') {
        let start = offset;
        offset += raw_line.len();
        let line = raw_line
            .strip_suffix("\r\n")
            .or_else(|| raw_line.strip_suffix('\n'))
            .unwrap_or(raw_line);
        match fence {
            Some(Fence::Rust { body_start }) => {
                if line == "```" {
                    let body = &readme[body_start..start];
                    if body.trim().is_empty() {
                        return Err("empty Rust quickstart fence");
                    }
                    sample = Some(body);
                    fence = None;
                } else if line.starts_with("```") {
                    return Err("Rust quickstart must close with an exact triple-backtick line");
                }
            }
            Some(Fence::Other) => {
                if line == "```" {
                    fence = None;
                }
            }
            None if line == "### Rust" => {
                if seen_heading {
                    return Err("expected exactly one Rust quickstart heading");
                }
                seen_heading = true;
                in_section = true;
            }
            None if matches!(line.split_once(' '), Some(("#" | "##" | "###", _))) => {
                in_section = false;
            }
            None if in_section && line == "```rust" => {
                if sample.is_some() {
                    return Err("expected exactly one Rust quickstart fence");
                }
                fence = Some(Fence::Rust { body_start: offset });
            }
            None if line.starts_with("```") => fence = Some(Fence::Other),
            None => {}
        }
    }

    if matches!(fence, Some(Fence::Rust { .. })) {
        return Err("unclosed Rust quickstart fence");
    }
    if !seen_heading {
        return Err("missing Rust quickstart heading");
    }
    sample.ok_or("missing exact Rust quickstart fence")
}

fn version(tag: &str) -> Result<&str, IoError> {
    let version = tag
        .strip_prefix("rust-v")
        .ok_or_else(|| invalid("expected a Rust release tag"))?;
    let mut components = version.split('.');
    for _ in 0..3 {
        let component = components
            .next()
            .ok_or_else(|| invalid("release tag is not a stable version"))?;
        if component.parse::<u64>().is_err()
            || !component.bytes().all(|byte| byte.is_ascii_digit())
            || (component.len() > 1 && component.starts_with('0'))
        {
            return Err(invalid("release tag is not a stable version"));
        }
    }
    if components.next().is_some() {
        return Err(invalid("release tag is not a stable version"));
    }
    Ok(version)
}

fn read_sample(readme: &Path) -> Result<String, IoError> {
    let with_path = |error: &str| format!("{}: {error}", readme.display());
    let source = fs::read_to_string(readme)
        .map_err(|error| IoError::new(error.kind(), with_path(&error.to_string())))?;
    sample(&source)
        .map(str::to_owned)
        .map_err(|error| invalid(&with_path(error)))
}

fn manifest(version: &str) -> String {
    format!(
        "[workspace]\n\n[package]\nname = \"purrdf-published-quickstart-smoke\"\n\
         version = \"0.0.0\"\nedition = \"2024\"\npublish = false\nautobins = false\n\n\
         [dependencies]\npurrdf = \"={version}\"\n\n\
         [[bin]]\nname = \"readme-en\"\npath = \"src/readme-en.rs\"\n\n\
         [[bin]]\nname = \"readme-zh\"\npath = \"src/readme-zh.rs\"\n\n\
         [profile.dev]\nopt-level = 3\ndebug = 0\n"
    )
}

/// Check Cargo's freshly generated lock receipt, whose package fields are
/// emitted as individual quoted lines. Never accept a second facade package.
fn verify_lock(lock: &str, tag: &str, checksum: &str) -> Result<(), IoError> {
    let version = version(tag)?;
    if checksum.len() != 64
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("expected a lowercase registry SHA-256"));
    }
    let mut matches = lock
        .split("\n[[package]]\n")
        .skip(1)
        .filter(|block| block.lines().any(|line| line == r#"name = "purrdf""#));
    let Some(package) = matches.next() else {
        return Err(invalid("Cargo.lock is missing the purrdf package"));
    };
    if matches.next().is_some() {
        return Err(invalid("Cargo.lock contains more than one purrdf package"));
    }
    for (key, expected) in [
        ("version", version),
        (
            "source",
            "registry+https://github.com/rust-lang/crates.io-index",
        ),
        ("checksum", checksum),
    ] {
        let prefix = format!("{key} = ");
        let mut values = package
            .lines()
            .filter_map(|line| line.strip_prefix(&prefix));
        let actual = values
            .next()
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'));
        if actual != Some(expected) || values.next().is_some() {
            return Err(invalid(&format!(
                "Cargo.lock purrdf {key} differs from the verified registry package"
            )));
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let repo = args
        .next()
        .ok_or_else(|| invalid("give a source directory or --verify-lock"))?;
    if repo == "--verify-lock" {
        let lock = args.next().ok_or_else(|| invalid("give Cargo.lock path"))?;
        let tag = args.next().ok_or_else(|| invalid("give release tag"))?;
        let checksum = args
            .next()
            .ok_or_else(|| invalid("give registry checksum"))?;
        if args.next().is_some() {
            return Err(invalid("unexpected positional argument").into());
        }
        verify_lock(
            &fs::read_to_string(lock)?,
            tag.to_str()
                .ok_or_else(|| invalid("non-UTF-8 release tag"))?,
            checksum
                .to_str()
                .ok_or_else(|| invalid("non-UTF-8 registry checksum"))?,
        )?;
        return Ok(());
    }
    let output = args
        .next()
        .ok_or_else(|| invalid("give new consumer directory"))?;
    let tag = args.next().ok_or_else(|| invalid("give release tag"))?;
    if args.next().is_some() {
        return Err(invalid("unexpected positional argument").into());
    }
    let tag = tag
        .to_str()
        .ok_or_else(|| invalid("non-UTF-8 release tag"))?;
    let version = version(tag)?;
    let root = Path::new(&repo);
    let english = read_sample(&root.join("README.md"))?;
    let chinese = read_sample(&root.join("README_zh.md"))?;

    let output = Path::new(&output);
    fs::create_dir(output)?;
    let src = output.join("src");
    fs::create_dir(&src)?;
    fs::write(src.join("readme-en.rs"), english)?;
    fs::write(src.join("readme-zh.rs"), chinese)?;
    fs::write(output.join("Cargo.toml"), manifest(version))?;
    Ok(())
}
