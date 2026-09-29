// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Where estimates files live.
//!
//! The store is a directory: `PURRDF_BENCH_HOME` when set, otherwise
//! `purrdf-bench/` beside the `CARGO_TARGET_TMPDIR` the bench target was compiled
//! with (its parent is the build directory: `target/` in cargo's default layout).
//! A benchmark's records sit under one directory per id component
//! (`<group>[/<function>][/<parameter>]`), each component spelled by
//! [`path_component`], and each record is `<record>/estimates.json`.
//!
//! wasm32-unknown-unknown has no file system: there the store is absent, the
//! options that need it are refused when the command line is parsed, and a
//! measured run says once that it prints its estimates without writing them.

use std::fmt::Write as _;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use super::estimates::Estimates;

/// The environment variable that places the store.
pub const HOME_VARIABLE: &str = "PURRDF_BENCH_HOME";

/// One id component as one path component, injectively: ASCII letters,
/// digits, `-`, `_`, and `.` (except leading) stand for themselves, and every
/// other byte of the UTF-8 text is `%XX` in upper-case hex. `/` inside a
/// component therefore never adds a directory level, and `.`/`..` never name
/// a parent.
pub fn path_component(component: &str) -> String {
    let mut out = String::with_capacity(component.len());
    for (index, byte) in component.bytes().enumerate() {
        let literal = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_')
            || (byte == b'.' && index > 0);
        if literal {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The benchmark store's root, or why there is none.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Store {
    /// The directory records are read from and written to.
    Directory(PathBuf),
    /// No `PURRDF_BENCH_HOME` and no `CARGO_TARGET_TMPDIR` to place it beside.
    Unplaced,
}

#[cfg(not(target_arch = "wasm32"))]
impl Store {
    /// `PURRDF_BENCH_HOME`, else `purrdf-bench/` beside `target_tmpdir` (the
    /// `CARGO_TARGET_TMPDIR` a bench target is compiled with, which is
    /// `<target>/tmp`).
    pub(crate) fn resolve(home: Option<String>, target_tmpdir: Option<&str>) -> Self {
        if let Some(home) = home.filter(|home| !home.is_empty()) {
            return Self::Directory(PathBuf::from(home));
        }
        target_tmpdir
            .and_then(|tmpdir| Path::new(tmpdir).parent())
            .map_or(Self::Unplaced, |target| {
                Self::Directory(target.join("purrdf-bench"))
            })
    }

    fn root(&self) -> Result<&Path, String> {
        match self {
            Self::Directory(root) => Ok(root),
            Self::Unplaced => Err(format!(
                "no benchmark store: set {HOME_VARIABLE}, or build the binary with cargo so its CARGO_TARGET_TMPDIR places the store"
            )),
        }
    }

    /// The path of `record` of the benchmark whose id components are
    /// `components`.
    pub(crate) fn record_path(&self, components: &[&str], record: &str) -> Result<PathBuf, String> {
        let mut path = self.root()?.to_path_buf();
        for component in components {
            path.push(path_component(component));
        }
        path.push(record);
        path.push("estimates.json");
        Ok(path)
    }

    /// Read `record`, or `None` when it does not exist. A record that exists
    /// but cannot be read or parsed is an error, never an absence.
    pub(crate) fn read(
        &self,
        components: &[&str],
        record: &str,
    ) -> Result<Option<Estimates>, String> {
        let path = self.record_path(components, record)?;
        match std::fs::read_to_string(&path) {
            Ok(text) => Estimates::from_json(&text)
                .map(Some)
                .map_err(|error| format!("{}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    /// Write `estimates` as `record`, replacing any previous file whole: the
    /// text goes to a sibling temporary file that is then renamed over it.
    pub(crate) fn write(
        &self,
        components: &[&str],
        record: &str,
        estimates: &Estimates,
    ) -> Result<PathBuf, String> {
        let path = self.record_path(components, record)?;
        let text = estimates.to_json().map_err(|error| error.to_string())?;
        let directory = path
            .parent()
            .ok_or_else(|| format!("{}: no parent directory", path.display()))?;
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let staging = directory.join(format!("estimates.json.{}.tmp", std::process::id()));
        std::fs::write(&staging, text)
            .map_err(|error| format!("{}: {error}", staging.display()))?;
        std::fs::rename(&staging, &path).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(path)
    }
}

#[cfg(test)]
#[allow(dead_code, clippy::all, clippy::pedantic, clippy::nursery)]
mod hex_frozen_vectors {


    use crate::vectors::{VectorFile, decode_str};

    /// Every frozen record: input bytes, lowercase and uppercase renderings.
    fn records() -> Vec<(Vec<u8>, String, String)> {
        let file = VectorFile::parse(include_str!("../../../hash-conformance/tests/vectors/hex_vectors.txt")).expect("frozen hex vectors");
        file.records()
            .iter()
            .map(|r| {
                let len: usize = r.fields[0].parse().expect("length");
                let first: u8 = r.fields[1].parse().expect("first");
                let input = (0..len).map(|i| first.wrapping_add(i as u8)).collect();
                (input, decode_str(r.fields[2]).expect("lower"), decode_str(r.fields[3]).expect("upper"))
            })
            .collect()
    }

    /// Every byte with its any-case, lowercase and uppercase digit values.
    fn digits() -> Vec<(u8, Option<u8>, Option<u8>, Option<u8>)> {
        let file = VectorFile::parse(include_str!("../../../hash-conformance/tests/vectors/hex_digit_vectors.txt")).expect("frozen digit vectors");
        let value = |f: &str| if f == "-" { None } else { Some(f.parse::<u8>().expect("value")) };
        file.records()
            .iter()
            .map(|r| (r.fields[0].parse().expect("byte"), value(r.fields[1]), value(r.fields[2]), value(r.fields[3])))
            .collect()
    }

    /// The frozen two-digit renderings of every byte, indexed by byte: (lower, upper).
    fn pairs() -> Vec<(String, String)> {
        records().into_iter().filter(|(input, _, _)| input.len() == 1).map(|(_, l, u)| (l, u)).collect()
    }


    #[test]
    fn path_component_escapes_replay_the_frozen_vectors() {
        let table = pairs();
        for byte in 0u8..0x80 {
            let text = format!("a{}", char::from(byte));
            let got = super::path_component(&text);
            if got != text {
                assert_eq!(got, format!("a%{}", table[usize::from(byte)].1));
            }
        }
    }

}
