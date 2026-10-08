// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed-work native counters, separate from the elapsed-time bench sampler.
//!
//! Each collected instruction sample starts a fresh `perf stat` process with one
//! inherited user-mode event, disabled until the workload acknowledges its query
//! window. `--no-scale` preserves the raw count. The exact verbose enabled/running
//! tuple must match, as must the CSV count; a rounded coverage percentage alone
//! cannot establish coverage. The child owns fixture, warm-up and result guards.
//! Count reports reuse the bench statistics, codec and atomic record store with
//! explicit units, fixed work, worker count, fixture and retention boundary.

pub use super::estimates::{CountContext, CountReport, CountUnit};

/// The command acknowledged by native perf's FIFO control protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterCommand {
    /// Begin the measured region.
    Enable,
    /// End the measured region before validation and destruction.
    Disable,
}

/// One unscaled sample with exact and displayed coverage metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSample {
    /// Retired user-mode instructions summed over inherited threads.
    pub instructions: u64,
    /// Exact kernel enabled duration metadata; not a workload latency estimate.
    pub enabled: u64,
    /// Exact kernel running duration metadata, required to equal enabled.
    pub running: u64,
    /// Perf's rounded percentage, preserved alongside the exact tuple.
    pub displayed_running_percent: String,
}

impl InstructionSample {
    /// Refuse absent, partial, multiplexed, scaled or inconsistent counter output.
    pub fn from_perf(csv: &str, verbose: &str) -> Result<Self, String> {
        let records: Vec<_> = csv
            .lines()
            .filter(|line| {
                line.split(',')
                    .nth(2)
                    .is_some_and(|event| event.trim() == "instructions:u")
            })
            .collect();
        if records.len() != 1 {
            return Err(format!(
                "expected one instructions:u CSV record, got {}",
                records.len()
            ));
        }
        // Perf's selected event has no quoted or delimiter-bearing fields. This
        // is its fixed event record, not a general CSV reader.
        let fields: Vec<_> = records[0].split(',').map(str::trim).collect();
        if fields.len() < 5 || fields[2] != "instructions:u" || !fields[1].is_empty() {
            return Err(format!("unexpected instructions record: {}", records[0]));
        }
        let instructions = fields[0]
            .parse::<u64>()
            .map_err(|error| format!("unavailable instruction count: {error}"))?;
        let csv_running = fields[3]
            .parse::<u64>()
            .map_err(|error| format!("invalid running metadata: {error}"))?;
        let tuples: Vec<_> = verbose
            .lines()
            .chain(csv.lines())
            .filter_map(|line| line.strip_prefix("instructions:u:"))
            .collect();
        if tuples.len() != 1 {
            return Err(format!(
                "expected one exact instructions tuple, got {}",
                tuples.len()
            ));
        }
        let tuple: Vec<_> = tuples[0]
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<_, _>>()
            .map_err(|error| format!("invalid exact counter tuple: {error}"))?;
        if tuple.len() != 3 || tuple[0] != instructions || tuple[2] != csv_running {
            return Err("exact counter tuple disagrees with CSV count/running metadata".to_owned());
        }
        if instructions == 0 || tuple[1] == 0 || tuple[1] != tuple[2] || fields[4] != "100.00" {
            return Err(format!(
                "incomplete instruction coverage: count={instructions}, enabled={}, running={}, displayed={} percent",
                tuple[1], tuple[2], fields[4]
            ));
        }
        Ok(Self {
            instructions,
            enabled: tuple[1],
            running: tuple[2],
            displayed_running_percent: fields[4].to_owned(),
        })
    }
}

/// Native Linux perf control, shared by every first-party counter workload.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct CounterControl {
    commands: std::fs::File,
    acknowledgements: std::fs::File,
}

#[cfg(target_os = "linux")]
impl CounterControl {
    /// Open the paired environment-selected control FIFOs. Either both names
    /// exist or neither does; malformed configuration is never an absent counter.
    pub fn from_env() -> std::io::Result<Option<Self>> {
        let command = std::env::var_os("PURRDF_BENCH_PERF_CONTROL");
        let ack = std::env::var_os("PURRDF_BENCH_PERF_ACK");
        match (command, ack) {
            (None, None) => Ok(None),
            (Some(command), Some(ack)) => {
                Self::open(std::path::Path::new(&command), std::path::Path::new(&ack)).map(Some)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "both PURRDF_BENCH_PERF_CONTROL and PURRDF_BENCH_PERF_ACK are required",
            )),
        }
    }

    /// Require an admitted native counter; count mode never falls back to time.
    pub fn required() -> std::io::Result<Self> {
        Self::from_env()?.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "instruction counting requires native perf control and acknowledgement FIFOs",
            )
        })
    }

    /// Open the same protocol at explicit paths, also used by protocol fixtures.
    pub fn open(command: &std::path::Path, ack: &std::path::Path) -> std::io::Result<Self> {
        use std::os::unix::fs::OpenOptionsExt as _;
        // Linux O_NONBLOCK: missing readers fail at open, and an absent
        // acknowledgement is diagnosed under the bounded command deadline.
        const NONBLOCK: i32 = 0x800;
        Ok(Self {
            commands: std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(NONBLOCK)
                .open(command)?,
            acknowledgements: std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(NONBLOCK)
                .open(ack)?,
        })
    }

    /// Send one command and verify its complete acknowledgement before proceeding.
    pub fn command(&mut self, command: CounterCommand) -> std::io::Result<()> {
        use std::io::{Read as _, Write as _};
        let bytes: &[u8] = match command {
            CounterCommand::Enable => b"enable\n",
            CounterCommand::Disable => b"disable\n",
        };
        self.commands.write_all(bytes)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut acknowledgement = [0; 5];
        let mut read = 0;
        while read < acknowledgement.len() {
            match self.acknowledgements.read(&mut acknowledgement[read..]) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "perf acknowledgement ended early",
                    ));
                }
                Ok(amount) => read += amount,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "perf control acknowledgement missing after five seconds",
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(error) => return Err(error),
            }
        }
        if &acknowledgement != b"ack\n\0" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid perf acknowledgement",
            ));
        }
        Ok(())
    }
}

/// Collect one fresh native instruction sample and preserve the raw protocol
/// receipts. `directory` must be new, so an older sample cannot be overwritten.
#[cfg(target_os = "linux")]
pub fn collect_instructions(
    executable: &std::path::Path,
    args: &[std::ffi::OsString],
    directory: &std::path::Path,
) -> Result<InstructionSample, String> {
    use std::process::Command;
    std::fs::create_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let command = directory.join("control.fifo");
    let ack = directory.join("ack.fifo");
    let status = Command::new("mkfifo")
        .arg(&command)
        .arg(&ack)
        .status()
        .map_err(|error| format!("native counter FIFO prerequisite: {error}"))?;
    if !status.success() {
        return Err(format!("native counter FIFO creation failed: {status}"));
    }
    let csv = directory.join("perf.csv");
    let verbose = directory.join("perf.verbose");
    let output = directory.join("workload.stdout");
    let status = Command::new("perf")
        .args([
            "stat",
            "--no-scale",
            "--no-big-num",
            "-x,",
            "-v",
            "--delay=-1",
        ])
        .arg(format!(
            "--control=fifo:{},{}",
            command.display(),
            ack.display()
        ))
        .args(["-e", "instructions:u", "--output"])
        .arg(&csv)
        .arg("--")
        .arg(executable)
        .args(args)
        .env("LC_ALL", "C")
        .env("PURRDF_BENCH_PERF_CONTROL", &command)
        .env("PURRDF_BENCH_PERF_ACK", &ack)
        .stdout(std::fs::File::create(output).map_err(|error| error.to_string())?)
        .stderr(std::fs::File::create(&verbose).map_err(|error| error.to_string())?)
        .status()
        .map_err(|error| format!("native perf prerequisite: {error}"))?;
    if !status.success() {
        return Err(format!(
            "native instruction sample failed: {status}; inspect {}",
            verbose.display()
        ));
    }
    let csv_text = std::fs::read_to_string(&csv).map_err(|error| error.to_string())?;
    let verbose_text = std::fs::read_to_string(&verbose).map_err(|error| error.to_string())?;
    InstructionSample::from_perf(&csv_text, &verbose_text)
}

/// Save explicit count units through the benchmark store's atomic writer.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_report(
    report: &CountReport,
    components: &[&str],
    record: &str,
) -> Result<std::path::PathBuf, String> {
    let store = super::store::Store::resolve(std::env::var(super::store::HOME_VARIABLE).ok(), None);
    store.write_text(
        components,
        record,
        &report.to_json().map_err(|error| error.to_string())?,
    )
}

/// Read a required count record through the same benchmark path home and codec.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_report(components: &[&str], record: &str) -> Result<CountReport, String> {
    let store = super::store::Store::resolve(std::env::var(super::store::HOME_VARIABLE).ok(), None);
    let path = store.record_path(components, record)?;
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    CountReport::from_json(&text).map_err(|error| format!("{}: {error}", path.display()))
}
