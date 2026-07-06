// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end example: an agent-memory log persisted as a GTS container
//! (append, fold, and read back RDF 1.2 quads plus binary blobs).

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use purrdf_gts::examples::agent_memory::{
    Memory, RecallOptions, RevisionOptions, StoreOptions, ToolCallOptions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (path, clear_existing) = match std::env::args_os().nth(1) {
        Some(path) => (PathBuf::from(path), false),
        None => {
            let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
            (
                std::env::temp_dir().join(format!(
                    "purrdf-gts-agent-memory-{}-{millis}.gts",
                    std::process::id()
                )),
                true,
            )
        }
    };
    if clear_existing {
        let _ = std::fs::remove_file(&path);
    }
    let mem = Memory::new(&path);

    let claim = mem.store(
        "Synthetic rover records battery telemetry in UTC",
        StoreOptions {
            source: Some("synthetic bench run 001"),
            confidence: Some(0.8),
            according_to: Some("example-agent"),
        },
    )?;
    let replacement = mem.store(
        "Synthetic rover records battery and thermal telemetry in UTC",
        StoreOptions {
            source: Some("synthetic bench run 002"),
            confidence: Some(0.9),
            according_to: Some("example-agent"),
        },
    )?;
    mem.record_tool_call(
        "urn:purrdf:tool:synthetic-search",
        ToolCallOptions {
            arguments: Some("{\"query\":\"battery telemetry\"}"),
            result: Some("matched one synthetic claim"),
            invocation: Some("urn:purrdf:invocation:demo"),
            generated: &[replacement.id.as_str()],
        },
    )?;
    mem.revise(
        &claim.id,
        RevisionOptions {
            reason: Some("synthetic correction"),
            superseded_by: Some(&replacement.id),
        },
    )?;

    let current = mem.recall(RecallOptions {
        query: "battery telemetry",
        min_confidence: Some(0.5),
        ..RecallOptions::default()
    })?;
    println!("wrote: {}", mem.path().display());
    println!(
        "current claims: {:?}",
        current.iter().map(|claim| &claim.text).collect::<Vec<_>>()
    );
    println!("tool calls: {}", mem.tool_calls()?.len());
    println!("verify diagnostics: {:?}", mem.verify()?);
    Ok(())
}
