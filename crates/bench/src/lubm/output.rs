// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{PROFILE, Spec};
use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

/// Actual department file identity, established by rereading flushed bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReceipt {
    /// Relative payload filename.
    pub name: String,
    /// Nonzero byte length.
    pub bytes: u64,
    /// BLAKE3-256 in lowercase base16.
    pub blake3: String,
}
purrdf_lex::json_record!(FileReceipt as "native department receipt" { "name" => name: required, "bytes" => bytes: required, "blake3" => blake3: required });

/// Complete generation byte identity. Graph acceptance is independently checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Native profile identity.
    pub profile: String,
    /// Seed.
    pub seed: u64,
    /// First absolute university index.
    pub index: u64,
    /// University count.
    pub universities: u64,
    /// Selected ontology identity.
    pub ontology: String,
    /// Selected document base.
    pub document_base: String,
    /// Sorted actual payload identities.
    pub files: Vec<FileReceipt>,
}
purrdf_lex::json_record!(Receipt as "native university receipt" {
    "profile" => profile: required, "seed" => seed: required, "index" => index: required,
    "universities" => universities: required, "ontology" => ontology: required,
    "document_base" => document_base: required, "files" => files: required,
});

/// Generate into a freshly created owned directory, preserving failed-run evidence.
///
/// # Errors
/// Refuses an existing directory, failed payload writes/flushes/reads and receipt writes.
pub fn generate_directory(spec: &Spec, output: &Path) -> io::Result<Receipt> {
    Spec::new(
        spec.seed,
        spec.index,
        spec.universities,
        spec.ontology.clone(),
        spec.document_base.clone(),
    )
    .map_err(io::Error::other)?;
    fs::create_dir(output)?;
    let mut files = Vec::new();
    for index in spec.index..spec.index + spec.universities {
        super::generate::university(spec, index, |department, emit| {
            let name = format!("University{index}_{department}.nt");
            let path = output.join(&name);
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            let mut writer = BufWriter::new(file);
            emit_and_flush(emit, &mut writer)?;
            writer.get_ref().sync_all()?;
            drop(writer);
            let mut reader = fs::File::open(path)?;
            let mut hash = purrdf_hash::blake3::Hasher::new();
            let mut bytes = 0_u64;
            let mut buffer = vec![0_u8; 65536].into_boxed_slice();
            loop {
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                bytes = bytes
                    .checked_add(u64::try_from(read).expect("read length fits u64"))
                    .ok_or_else(|| io::Error::other("file size overflow"))?;
                hash.update(&buffer[..read]);
            }
            if bytes == 0 {
                return Err(io::Error::other("empty generated department"));
            }
            files.push(FileReceipt {
                name,
                bytes,
                blake3: purrdf_hash::hex::Lower(hash.finalize().as_bytes()).to_string(),
            });
            Ok(())
        })?;
    }
    files.sort_by(|left, right| left.name.cmp(&right.name));
    let receipt = Receipt {
        profile: PROFILE.into(),
        seed: spec.seed,
        index: spec.index,
        universities: spec.universities,
        ontology: spec.ontology.clone(),
        document_base: spec.document_base.clone(),
        files,
    };
    let pending = output.join("receipt.pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    file.write_all(&purrdf_lex::json::record::to_vec(&receipt))?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    fs::rename(pending, output.join("receipt.json"))?;
    Ok(receipt)
}

fn emit_and_flush<W: Write>(
    emit: &mut dyn FnMut(&mut W) -> io::Result<()>,
    writer: &mut W,
) -> io::Result<()> {
    emit(writer)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FlushFailure;
    impl Write for FlushFailure {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("injected flush refusal"))
        }
    }
    #[test]
    fn actual_payload_finish_refuses_flush_failure() {
        let spec = Spec::new(
            0,
            0,
            1,
            "http://example.org/schema.owl".into(),
            "http://example.org/data/".into(),
        )
        .unwrap();
        let error = super::super::generate::university(&spec, 0, |_, emit| {
            emit_and_flush(emit, &mut FlushFailure)
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "injected flush refusal");
    }
}
