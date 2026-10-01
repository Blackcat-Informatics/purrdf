// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepare a resident-certified fixture, then reopen it in a separate bounded process.
//! The trusted receipt pin must come from the prepare step, through an independent
//! channel; hashing an unvalidated data file cannot establish truthful pruning.

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use purrdf_core::{
        DatasetView, GraphMatch, QuadIds, SegmentedBuildLimits, SegmentedBuilder, SegmentedError,
        SegmentedProvider, SegmentedReadLimits, SegmentedReceipt, SegmentedReceiptAuthority,
        SegmentedSession, SegmentedSnapshot, TermValue,
    };

    #[derive(Debug)]
    struct FileProvider {
        file: Mutex<File>,
        bytes: u64,
        snapshot: SegmentedSnapshot,
    }
    impl SegmentedProvider for FileProvider {
        fn snapshot(&self) -> SegmentedSnapshot {
            self.snapshot
        }
        fn byte_len(&self) -> u64 {
            self.bytes
        }
        fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
            let mut file = self.file.lock().map_err(|_| SegmentedError::Provider {
                operation: "file mutex",
                host_code: None,
            })?;
            file.seek(SeekFrom::Start(position))
                .map_err(|error| SegmentedError::Provider {
                    operation: "file seek",
                    host_code: error.raw_os_error(),
                })?;
            file.read_exact(output)
                .map_err(|error| SegmentedError::Provider {
                    operation: "file read_exact",
                    host_code: error.raw_os_error(),
                })
        }
    }

    struct ReceiptPin([u8; 32]);
    impl SegmentedReceiptAuthority for ReceiptPin {
        fn authenticate(&self, receipt: &[u8]) -> Result<(), SegmentedError> {
            if purrdf_hash::blake3::hash(receipt).as_bytes() == &self.0 {
                Ok(())
            } else {
                Err(SegmentedError::SnapshotMismatch)
            }
        }
    }

    fn create(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    }

    fn prepare(path: &Path, rows: u32) -> Result<(), Box<dyn std::error::Error>> {
        if !(1000..=1_000_000).contains(&rows) {
            return Err("fixture row count must be between 1000 and 1000000".into());
        }
        let limits = SegmentedBuildLimits::new(2100, 200_000, rows + 1, 8192, 128)?;
        let mut builder = SegmentedBuilder::new(limits);
        let mut values = vec![
            TermValue::iri("http://example.org/noise"),
            TermValue::iri("http://example.org/link"),
            TermValue::iri("http://example.org/other"),
            TermValue::iri("http://example.org/empty"),
        ];
        values.extend((0..1000).map(|n| TermValue::iri(format!("http://example.org/s{n:04}"))));
        values.extend((0..1000).map(|n| TermValue::iri(format!("http://example.org/o{n:04}"))));
        let mut ids = Vec::with_capacity(values.len());
        builder.intern_batch(&values, |_, id| ids.push(id))?;
        for index in 0..rows - 1000 {
            builder.push_quad(QuadIds {
                s: ids[4 + usize::try_from(index / 1000)?],
                p: ids[0],
                o: ids[1004 + usize::try_from(index % 1000)?],
                g: None,
            })?;
        }
        for index in 0..500 {
            for predicate in [ids[1], ids[2]] {
                builder.push_quad(QuadIds {
                    s: ids[4 + index],
                    p: predicate,
                    o: ids[1004 + (index + 17) % 1000],
                    g: None,
                })?;
            }
        }
        builder.declare_named_graph(ids[3])?;
        let image = builder.seal()?;
        let receipt = image.receipt().encode();
        create(path, image.bytes())?;
        create(&path.with_extension("receipt"), &receipt)?;
        println!("rows={rows}");
        println!("encoded_bytes={}", image.receipt().byte_len());
        println!(
            "trusted_receipt_pin={}",
            purrdf_hash::hex::Lower(purrdf_hash::blake3::hash(&receipt).as_bytes())
        );
        Ok(())
    }

    fn read(path: &Path, pin: &str, live_bytes: u64) -> Result<(), Box<dyn std::error::Error>> {
        let pin =
            ReceiptPin(purrdf_hash::hex::decode_32_canonical(pin).ok_or("invalid receipt pin")?);
        // A receipt is fixed-size; never read an arbitrarily sized manifest into a Vec.
        let mut receipt_file = File::open(path.with_extension("receipt"))?;
        if receipt_file.metadata()?.len() != u64::try_from(SegmentedReceipt::ENCODED_BYTES)? {
            return Err("wrong receipt length".into());
        }
        let mut encoded_receipt = [0_u8; SegmentedReceipt::ENCODED_BYTES];
        receipt_file.read_exact(&mut encoded_receipt)?;
        let receipt = SegmentedReceipt::from_authenticated_bytes(&encoded_receipt, &pin)?;
        let file = File::open(path)?;
        let provider = Arc::new(FileProvider {
            bytes: file.metadata()?.len(),
            file: Mutex::new(file),
            snapshot: receipt.snapshot(),
        });
        let window = purrdf_alloc_probe::WholeProcessWindow::open();
        let session = SegmentedSession::open(
            provider,
            &receipt,
            SegmentedReadLimits::new(live_bytes, 512, 2_000_000, 2_000_000_000, 8),
        )?;
        let open = session.evidence();
        let source_rows =
            session.checked_read(|view| view.quads().fold(0_u64, |count, _| count + 1))?;
        let subject = session
            .term_id_by_value(&TermValue::iri("http://example.org/s0042"))?
            .ok_or("missing subject")?;
        let predicate = session
            .term_id_by_value(&TermValue::iri("http://example.org/link"))?
            .ok_or("missing predicate")?;
        let selective_rows = session.checked_read(|view| {
            view.quads_for_pattern(Some(subject), Some(predicate), None, GraphMatch::Default)
                .count()
        })?;
        if selective_rows != 1 {
            return Err("selective fixture answer mismatch".into());
        }
        let mut drain = purrdf_core::sink::Measure::default();
        let evidence = session.export_trig_lines(&mut drain)?;
        let measured = window.close();
        println!("source_rows={source_rows}");
        println!("selective_rows={selective_rows}");
        println!("open_requests={}", open.request_count());
        println!("total_requests={}", evidence.request_count());
        println!("io_bytes={}", evidence.io_bytes());
        println!("evictions={}", evidence.evictions());
        println!("charged_peak_bytes={}", evidence.peak_bytes());
        println!("counted_peak_bytes={}", measured.peak_working_bytes);
        println!("export_bytes={}", drain.bytes());
        if u64::try_from(measured.peak_working_bytes)? > evidence.peak_bytes() {
            return Err("reader allocations escaped the conservative budget".into());
        }
        Ok(())
    }

    pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args = std::env::args().collect::<Vec<_>>();
        match args.get(1).map(String::as_str) {
            Some("prepare") if args.len() == 4 => prepare(Path::new(&args[2]), args[3].parse()?),
            Some("read") if args.len() == 5 => read(Path::new(&args[2]), &args[3], args[4].parse()?),
            _ => Err("usage: segmented_fixture prepare FILE ROWS | read FILE TRUSTED_RECEIPT_PIN LIVE_BYTES".into()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    host::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
