// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Host I/O callbacks over Rust-owned admission and canonical encoding.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::Arc;

use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use super::{PyGtsBlobExportReport, limits_from};

struct Stager {
    begin: Py<PyAny>,
    write: Arc<Py<PyAny>>,
    read: Arc<Py<PyAny>>,
    discard: Arc<Py<PyAny>>,
    failure: Option<PyErr>,
}

impl Stager {
    fn from_host(host: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            begin: host.getattr("begin")?.unbind(),
            write: Arc::new(host.getattr("write")?.unbind()),
            read: Arc::new(host.getattr("read")?.unbind()),
            discard: Arc::new(host.getattr("discard")?.unbind()),
            failure: None,
        })
    }
    fn failed(&mut self, error: PyErr) -> purrdf::RdfDiagnostic {
        if self.failure.is_none() {
            self.failure = Some(error);
        }
        purrdf::RdfDiagnostic::error("gts-staging-io", "caller blob staging failed")
    }
}

impl purrdf::GtsBlobStager for Stager {
    fn scratch(&mut self) -> io::Result<Box<dyn purrdf_lex::cbor::ReplayStorage>> {
        let handle = self
            .begin()
            .map_err(|cause| purrdf_lex::error::io_error(io::ErrorKind::Other, cause))?;
        Ok(Box::new(Scratch {
            reader: PayloadReader {
                read: Arc::clone(&self.read),
                handle,
                offset: 0,
            },
            write: Arc::clone(&self.write),
            discard: Arc::clone(&self.discard),
            length: 0,
        }))
    }
    fn begin(&mut self) -> Result<u64, purrdf::RdfDiagnostic> {
        Python::attach(|py| self.begin.call0(py)?.extract(py)).map_err(|error| self.failed(error))
    }
    fn discard(&mut self, handle: u64) -> Result<(), purrdf::RdfDiagnostic> {
        Python::attach(|py| self.discard.call1(py, (handle,)).map(drop))
            .map_err(|error| self.failed(error))
    }
    fn write(&mut self, handle: u64, bytes: &[u8]) -> Result<(), purrdf::RdfDiagnostic> {
        Python::attach(|py| {
            self.write
                .call1(py, (handle, PyBytes::new(py, bytes)))
                .map(drop)
        })
        .map_err(|error| self.failed(error))
    }
}

fn prepare(
    data: &[u8],
    limits: purrdf::GtsBlobLimits,
    mut stager: Stager,
) -> PyResult<purrdf::GtsStagedColumnarSource> {
    let result = purrdf::prepare_gts_columnar_to(data, limits, &mut stager);
    if let Some(error) = stager.failure {
        return Err(error);
    }
    result.map_err(|error| PyValueError::new_err(error.to_string()))
}

struct PayloadSource {
    payload: purrdf::GtsStagedPayload,
    read: Arc<Py<PyAny>>,
}

impl purrdf_columnar::BlobSource for PayloadSource {
    fn digest(&self) -> purrdf::ContentDigest {
        self.payload.sha256
    }
    fn len(&self) -> usize {
        self.payload.len
    }
    fn open(&self) -> io::Result<Box<dyn Read + '_>> {
        Ok(Box::new(PayloadReader {
            read: Arc::clone(&self.read),
            handle: self.payload.handle,
            offset: 0,
        }))
    }
}

struct PayloadReader {
    read: Arc<Py<PyAny>>,
    handle: u64,
    offset: usize,
}

struct Scratch {
    reader: PayloadReader,
    write: Arc<Py<PyAny>>,
    discard: Arc<Py<PyAny>>,
    length: usize,
}
impl purrdf_lex::cbor::ReplayStorage for Scratch {
    fn finish(&mut self) -> io::Result<()> {
        Python::attach(|py| self.discard.call1(py, (self.reader.handle,)).map(drop))
            .map_err(io::Error::other)
    }
}
impl Read for Scratch {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.reader.read(output)
    }
}
impl Write for Scratch {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.reader.offset != self.length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "scratch writes must append",
            ));
        }
        Python::attach(|py| {
            self.write
                .call1(py, (self.reader.handle, PyBytes::new(py, bytes)))
                .map(drop)
        })
        .map_err(io::Error::other)?;
        self.length = self.length.checked_add(bytes.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "scratch length exceeds address space",
            )
        })?;
        self.reader.offset = self.length;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        // The host callback accepts and flushes each complete chunk.
        Ok(())
    }
}
impl Seek for Scratch {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(position) => i128::from(position),
            SeekFrom::Current(delta) => self.reader.offset as i128 + i128::from(delta),
            SeekFrom::End(delta) => self.length as i128 + i128::from(delta),
        };
        self.reader.offset = usize::try_from(target).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "scratch seek is outside address space",
            )
        })?;
        Ok(self.reader.offset as u64)
    }
}

impl Read for PayloadReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = Python::attach(|py| -> PyResult<usize> {
            let result = self
                .read
                .call1(py, (self.handle, self.offset, output.len()))?;
            let bytes = result.bind(py).cast::<PyBytes>()?;
            let bytes = bytes.as_bytes();
            if bytes.len() > output.len() {
                return Err(PyOSError::new_err(
                    "blob reader returned more bytes than requested",
                ));
            }
            output[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        })
        .map_err(io::Error::other)?;
        self.offset = self.offset.checked_add(count).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "blob read offset exceeds address space",
            )
        })?;
        Ok(count)
    }
}

/// Stream native files through host callbacks; payload staging stays provisional.
#[pyfunction(name = "_gts_columnar_stream_to")]
#[pyo3(signature = (data, *, blob_limits, staging, emit))]
fn gts_columnar_stream_to(
    py: Python<'_>,
    data: &[u8],
    blob_limits: &Bound<'_, PyAny>,
    staging: &Bound<'_, PyAny>,
    emit: Py<PyAny>,
) -> PyResult<PyGtsBlobExportReport> {
    let limits = limits_from(blob_limits)?;
    let stager = Stager::from_host(staging)?;
    let read = Arc::clone(&stager.read);
    py.detach(move || {
        let source = prepare(data, limits, stager)?;
        let blobs: Vec<_> = source
            .blobs
            .into_iter()
            .map(|blob| PayloadSource {
                payload: blob.bytes,
                read: Arc::clone(&read),
            })
            .collect();
        let written = purrdf_columnar::write_with_blob_sources_to(
            &*source.dataset,
            &blobs,
            |table, bytes| {
                Python::attach(|py| {
                    emit.call1(py, (table.file_name(), PyBytes::new(py, bytes)))
                        .map(drop)
                })
            },
        )
        .map_err(|error| match error {
            purrdf_columnar::ColumnarSinkError::Sink { cause, .. } => cause,
            error => PyValueError::new_err(error.to_string()),
        })?;
        let mut report = source.report;
        report
            .append_writer_losses(written.losses)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(PyGtsBlobExportReport::from_native(report))
    })
}

type StagedRows<'py> = (
    Bound<'py, PyDict>,
    Vec<(String, usize, u64)>,
    PyGtsBlobExportReport,
);

/// Canonical RDF rows and replayable blob handles, without resident payload rows.
#[pyfunction(name = "_gts_columnar_staged_from_bytes")]
#[pyo3(signature = (data, *, blob_limits, staging))]
fn gts_columnar_staged_from_bytes<'py>(
    py: Python<'py>,
    data: &[u8],
    blob_limits: &Bound<'_, PyAny>,
    staging: &Bound<'_, PyAny>,
) -> PyResult<StagedRows<'py>> {
    let limits = limits_from(blob_limits)?;
    let stager = Stager::from_host(staging)?;
    let (projection, blobs, report) = py.detach(move || {
        let source = prepare(data, limits, stager)?;
        let projection = purrdf_columnar::project(&*source.dataset, &purrdf::ContentStore::new())
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        let blobs = source
            .blobs
            .into_iter()
            .map(|blob| {
                (
                    blob.bytes.sha256.to_hex(),
                    blob.bytes.len,
                    blob.bytes.handle,
                )
            })
            .collect::<Vec<_>>();
        Ok::<_, PyErr>((projection, blobs, source.report))
    })?;
    Ok((
        crate::py_gts_view::columnar_rows_dict(py, &projection)?,
        blobs,
        PyGtsBlobExportReport::from_native(report),
    ))
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(gts_columnar_stream_to, module)?)?;
    module.add_function(wrap_pyfunction!(gts_columnar_staged_from_bytes, module)?)?;
    Ok(())
}
