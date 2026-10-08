// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reusable Linux release-CLI qualification collector. Arguments: production
//! binary, separate allocation-observer binary, new evidence directory. Every
//! child uses unchanged rule defaults. Successful measurements are accepted only
//! after exact facts and complete authored premise blocks have been validated.

#[cfg(target_os = "linux")]
mod native {
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::fmt::Write as _;
    use std::fs::{self, File};
    use std::os::unix::process::ExitStatusExt as _;
    use std::path::Path;
    use std::process::{Command, ExitStatus, Stdio};
    use std::time::Instant;

    const NS: &str = "https://example.invalid/k#";

    #[derive(Clone, Copy)]
    struct Allocation {
        calls: u64,
        requested: u64,
        retained: i64,
        peak: i64,
    }

    fn allocation_receipt(text: &str) -> Result<Allocation, Box<dyn Error>> {
        let mut fields = std::collections::BTreeMap::new();
        if !text.ends_with('\n') {
            return Err("unterminated allocation receipt".into());
        }
        for line in text.lines() {
            let (key, value) = line
                .split_once('\t')
                .ok_or("allocation receipt requires exactly two columns")?;
            if value.contains('\t') || fields.insert(key, value).is_some() {
                return Err("duplicate or malformed allocation metric".into());
            }
        }
        if fields.len() != 4 {
            return Err("allocation receipt requires four unique metrics".into());
        }
        let mut take = |name| {
            fields
                .remove(name)
                .ok_or("allocation receipt metric missing")
        };
        let result = Allocation {
            calls: take("allocation_calls")?.parse()?,
            requested: take("requested_bytes")?.parse()?,
            retained: take("retained_bytes")?.parse()?,
            peak: take("peak_working_bytes")?.parse()?,
        };
        if result.calls == 0 || result.requested == 0 || result.peak <= 0 {
            return Err("allocation receipt has no valid measured CLI window".into());
        }
        Ok(result)
    }

    fn iri(name: &str) -> String {
        format!("<{NS}{name}>")
    }
    fn triple(s: &str, p: &str, o: &str) -> String {
        format!("{s} {p} {o} .")
    }

    fn fixture(n: usize, two: bool) -> String {
        let mut out = format!("@prefix k: <{NS}> .\n");
        for i in 0..n {
            writeln!(
                out,
                "k:v{i} a k:Vault ; k:target k:v{i}a, k:v{i}b, k:v{i}c ."
            )
            .unwrap();
            writeln!(
                out,
                "k:v{i}a a k:Store .\nk:v{i}b a k:{} .\nk:v{i}c a k:Spool .",
                if two { "Spool" } else { "Store" }
            )
            .unwrap();
        }
        out
    }

    fn rule(kind: &str) -> String {
        let body = match kind {
            "single" => "?v rdf:type k:Vault ; k:target ?s . ?s rdf:type k:Spool .",
            "notype" => "?v k:target ?s . ?s rdf:type k:Spool .",
            "pair" => {
                "?v rdf:type k:Vault ; k:target ?s1, ?s2 . ?s1 rdf:type k:Spool . ?s2 rdf:type k:Spool . FILTER(?s1 != ?s2)"
            }
            _ => panic!("unknown fixture rule"),
        };
        format!(
            "PREFIX k: <{NS}>\nPREFIX rdf: <{}>\nRULE {{ {} }} WHERE {{ {body} }}\n",
            purrdf_iri::vocab::rdf::NS,
            if kind == "pair" {
                "?v k:hasTwoSpools true"
            } else {
                "?v k:hasSpool ?s"
            }
        )
    }

    fn expected(n: usize, kind: &str, two: bool) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut facts = BTreeSet::new();
        let mut proofs = BTreeSet::new();
        let rdf_type = format!("<{}>", purrdf_iri::vocab::rdf::TYPE);
        for i in 0..n {
            if kind == "pair" && !two {
                continue;
            }
            let v = iri(&format!("v{i}"));
            let c = iri(&format!("v{i}c"));
            let conclusion = if kind == "pair" {
                triple(
                    &v,
                    &iri("hasTwoSpools"),
                    &format!("\"true\"^^<{}>", purrdf_core::datatype::XSD_BOOLEAN),
                )
            } else {
                triple(&v, &iri("hasSpool"), &c)
            };
            facts.insert(conclusion.clone());
            let mut proof = format!("derived {conclusion}\n  rule _:srl-rule-0\n");
            let premises = if kind == "pair" {
                let b = iri(&format!("v{i}b"));
                vec![
                    triple(&v, &rdf_type, &iri("Vault")),
                    triple(&v, &iri("target"), &b),
                    triple(&v, &iri("target"), &c),
                    triple(&b, &rdf_type, &iri("Spool")),
                    triple(&c, &rdf_type, &iri("Spool")),
                ]
            } else {
                let mut p = Vec::new();
                if kind == "single" {
                    p.push(triple(&v, &rdf_type, &iri("Vault")));
                }
                p.push(triple(&v, &iri("target"), &c));
                p.push(triple(&c, &rdf_type, &iri("Spool")));
                p
            };
            for premise in premises {
                writeln!(proof, "  premise {premise}").unwrap();
            }
            proofs.insert(proof);
        }
        (facts, proofs)
    }

    fn validate(dir: &Path, n: usize, kind: &str, two: bool) -> Result<(), Box<dyn Error>> {
        let (facts, proofs) = expected(n, kind, two);
        let output = fs::read_to_string(dir.join("output.nt"))?;
        let actual_facts: BTreeSet<_> = output.lines().map(str::to_owned).collect();
        if output.lines().count() != facts.len() || actual_facts != facts {
            return Err("inferred fact mismatch or duplicate".into());
        }
        let proof_text = fs::read_to_string(dir.join("proof.txt"))?;
        if (proof_text.is_empty() != proofs.is_empty())
            || (!proof_text.is_empty() && !proof_text.starts_with("derived "))
        {
            return Err("missing proof or unexpected proof prologue".into());
        }
        let actual_proofs: BTreeSet<_> = proof_text
            .split("derived ")
            .skip(1)
            .map(|p| format!("derived {p}"))
            .collect();
        if actual_proofs != proofs || proof_text.matches("derived ").count() != proofs.len() {
            return Err("complete authored proof mismatch".into());
        }
        if fs::read_to_string(dir.join("stderr.txt"))?
            != format!("rules inferred {}\n", facts.len())
        {
            return Err("unexpected CLI diagnostic or inference counter".into());
        }
        if !fs::read(dir.join("stdout.txt"))?.is_empty() {
            return Err("unexpected standard output".into());
        }
        Ok(())
    }

    fn child(
        binary: &Path,
        dir: &Path,
        input: &Path,
        rules: &Path,
        observer: bool,
    ) -> Result<(u128, i64), Box<dyn Error>> {
        let args = vec![
            "rules".to_owned(),
            "--srl".to_owned(),
            rules.display().to_string(),
            format!("--explain={}", dir.join("proof.txt").display()),
            input.display().to_string(),
            dir.join("output.nt").display().to_string(),
        ];
        fs::write(dir.join("argv.txt"), format!("{binary:?}\n{args:#?}\n"))?;
        let mut command = Command::new(binary);
        command
            .args(&args)
            .stdout(Stdio::from(File::create(dir.join("stdout.txt"))?))
            .stderr(Stdio::from(File::create(dir.join("stderr.txt"))?));
        if observer {
            command.env("PURRDF_RULES_ALLOC_RECEIPT", dir.join("allocation.tsv"));
        }
        fs::write(dir.join("load-before.txt"), fs::read("/proc/loadavg")?)?;
        let start = Instant::now();
        let child = command.spawn()?;
        let mut status = 0;
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // SAFETY: wait4 receives writable, correctly aligned status/rusage storage;
        // this process owns this exact child, and reads usage only after success.
        loop {
            let waited = unsafe {
                libc::wait4(
                    i32::try_from(child.id())?,
                    &raw mut status,
                    0,
                    usage.as_mut_ptr(),
                )
            };
            if waited >= 0 {
                break;
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                fs::write(dir.join("status.txt"), format!("wait4 failed: {error}\n"))?;
                return Err(error.into());
            }
        }
        let elapsed = start.elapsed().as_nanos();
        fs::write(dir.join("load-after.txt"), fs::read("/proc/loadavg")?)?;
        // SAFETY: successful wait4 initialized the complete rusage record.
        let rss = unsafe { usage.assume_init() }.ru_maxrss;
        let status = ExitStatus::from_raw(status);
        fs::write(
            dir.join("status.txt"),
            format!("{status}\nelapsed_ns={elapsed}\nchild_peak_rss_kib={rss}\n"),
        )?;
        if !status.success() {
            return Err(format!("{binary:?} failed: {status}").into());
        }
        Ok((elapsed, rss))
    }

    /// Run the Linux collector or its adversarial validator controls.
    pub(super) fn run() -> Result<(), Box<dyn Error>> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.first().is_some_and(|arg| arg == "--self-test") {
            if args.len() != 2 {
                return Err("expected --self-test SCRATCH_ROOT".into());
            }
            return self_test(Path::new(&args[1]));
        }
        if args.len() != 3 {
            return Err(
                "expected PRODUCTION_BINARY ALLOCATION_BINARY NEW_EVIDENCE_DIRECTORY".into(),
            );
        }
        let binary = Path::new(&args[0]).canonicalize()?;
        let observer = Path::new(&args[1]).canonicalize()?;
        let root = Path::new(&args[2]);
        fs::create_dir(root)?;
        for (name, path) in [("production", &binary), ("observer", &observer)] {
            fs::write(
                root.join(format!("{name}.identity")),
                format!(
                    "{}\n{}\n",
                    path.display(),
                    purrdf_hash::hex::encode(
                        purrdf_hash::blake3::hash(&fs::read(path)?).as_bytes()
                    )
                ),
            )?;
        }
        for file in [
            "/proc/loadavg",
            "/proc/meminfo",
            "/proc/cpuinfo",
            "/proc/version",
        ] {
            fs::write(
                root.join(Path::new(file).file_name().ok_or("missing filename")?),
                fs::read(file)?,
            )?;
        }
        for command in ["rustc", "cargo", "uname", "df", "git"] {
            let argument = match command {
                "rustc" => "-Vv",
                "uname" => "-a",
                "df" => "-h",
                "git" => "status",
                _ => "--version",
            };
            let output = Command::new(command).arg(argument).output()?;
            if !output.status.success() {
                return Err(format!("host capture {command} failed").into());
            }
            fs::write(root.join(format!("{command}.txt")), output.stdout)?;
        }
        for (name, arguments) in [
            ("source-head.txt", vec!["rev-parse", "HEAD"]),
            ("source-diff.patch", vec!["diff", "--binary", "HEAD"]),
            (
                "source-untracked.txt",
                vec!["ls-files", "--others", "--exclude-standard", "-z"],
            ),
        ] {
            let output = Command::new("git").args(arguments).output()?;
            if !output.status.success() {
                return Err("Git source capture failed".into());
            }
            fs::write(root.join(name), output.stdout)?;
        }
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "crates/cli/Cargo.toml",
            "crates/cli/examples/rules_campaign.rs",
            "crates/cli/examples/rules_alloc.rs",
            "crates/datalog/tests/factor_allocation.rs",
        ] {
            fs::write(root.join(path.replace('/', "_")), fs::read(path)?)?;
        }
        let mut prior: std::collections::BTreeMap<&str, (u128, i64)> =
            std::collections::BTreeMap::new();
        let mut prior_allocations: std::collections::BTreeMap<&str, Allocation> =
            std::collections::BTreeMap::new();
        let mut rows = String::from(
            "kind\tvaults\ttwo_spools\tobserver\telapsed_ns\tpeak_rss_kib\tallocation_calls\trequested_bytes\tretained_bytes\tpeak_working_bytes\n",
        );
        for (n, two) in [
            (1_000, false),
            (10_000, false),
            (100_000, false),
            (1_000, true),
        ] {
            let input = root.join(format!("d{n}-{two}.ttl"));
            fs::write(&input, fixture(n, two))?;
            for kind in ["single", "pair", "notype"] {
                if two && kind != "pair" {
                    continue;
                }
                let rules = root.join(format!("{kind}.srl"));
                fs::write(&rules, rule(kind))?;
                for (executable, allocation) in [(&binary, false), (&observer, true)] {
                    let dir = root.join(format!("{kind}-{n}-{two}-{allocation}"));
                    fs::create_dir(&dir)?;
                    let (elapsed, rss) = child(executable, &dir, &input, &rules, allocation)?;
                    validate(&dir, n, kind, two)?;
                    let allocation_columns = if allocation {
                        let measured =
                            allocation_receipt(&fs::read_to_string(dir.join("allocation.tsv"))?)?;
                        if !two
                            && let Some(previous) = prior_allocations.insert(kind, measured)
                            && (measured.requested > previous.requested * 20
                                || measured.peak > previous.peak * 20)
                        {
                            return Err(format!("{kind}: tenfold input growth exceeded conservative allocation traffic/peak envelope").into());
                        }
                        format!(
                            "{}\t{}\t{}\t{}",
                            measured.calls, measured.requested, measured.retained, measured.peak
                        )
                    } else {
                        "NA\tNA\tNA\tNA".to_owned()
                    };
                    fs::write(
                        dir.join("validation.txt"),
                        "PASS: all facts, complete authored proofs, inference counter and output channels\n",
                    )?;
                    if !allocation
                        && !two
                        && let Some((previous_time, previous_rss)) =
                            prior.insert(kind, (elapsed, rss))
                        && (elapsed > previous_time * 20 || rss > previous_rss * 20)
                    {
                        return Err(format!("{kind}: tenfold input growth exceeded conservative linear timing/RSS envelope").into());
                    }
                    writeln!(
                        rows,
                        "{kind}\t{n}\t{two}\t{allocation}\t{elapsed}\t{rss}\t{allocation_columns}"
                    )?;
                    fs::write(root.join("measurements.tsv"), &rows)?;
                }
            }
        }
        Ok(())
    }

    fn self_test(root: &Path) -> Result<(), Box<dyn Error>> {
        let temp = purrdf_testkit::TempDir::with_prefix_in("rules-validator", root)?;
        let dir = temp.path();
        let valid = "allocation_calls\t10\nrequested_bytes\t1024\nretained_bytes\t-1\npeak_working_bytes\t512\n";
        let measured = allocation_receipt(valid)?;
        assert_eq!(measured.retained, -1);
        for invalid in [
            String::new(),
            valid.replace("allocation_calls\t10\n", ""),
            format!("{valid}allocation_calls\t10\n"),
            valid.replace("requested_bytes", "unknown"),
            valid.replace("1024", "not-a-number"),
            valid.replace("512", "-1"),
            valid.replace("10\n", "0\n"),
            valid.replace("1024", "0"),
            valid.replace("512", "512\textra"),
            valid.trim_end().to_owned(),
        ] {
            assert!(allocation_receipt(&invalid).is_err());
        }
        for kind in ["single", "notype", "pair"] {
            for two in [false, true] {
                if two && kind != "pair" {
                    continue;
                }
                let (facts, proofs) = expected(2, kind, two);
                let mut output = String::new();
                for line in &facts {
                    writeln!(output, "{line}")?;
                }
                let proof = proofs.into_iter().collect::<String>();
                fs::write(dir.join("output.nt"), &output)?;
                fs::write(dir.join("proof.txt"), &proof)?;
                fs::write(
                    dir.join("stderr.txt"),
                    format!("rules inferred {}\n", facts.len()),
                )?;
                fs::write(dir.join("stdout.txt"), "")?;
                validate(dir, 2, kind, two)?;
                for corrupt in [
                    format!("junk\n{proof}"),
                    format!("{proof}junk\n"),
                    proof.replace("  premise ", "  omitted "),
                    format!("{proof}{proof}"),
                ] {
                    if corrupt == proof {
                        continue;
                    }
                    fs::write(dir.join("proof.txt"), corrupt)?;
                    assert!(validate(dir, 2, kind, two).is_err());
                }
                fs::write(dir.join("proof.txt"), &proof)?;
                fs::write(dir.join("output.nt"), format!("{output}junk\n"))?;
                assert!(validate(dir, 2, kind, two).is_err());
                fs::write(dir.join("output.nt"), &output)?;
                fs::write(dir.join("stdout.txt"), "unexpected")?;
                assert!(validate(dir, 2, kind, two).is_err());
            }
        }
        println!("rules campaign validator adversarial controls PASS");
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        native::run()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("rules_campaign requires Linux wait4 child peak-RSS accounting".into())
    }
}
