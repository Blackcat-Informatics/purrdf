// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Content identities over the algebra's typed structural token stream.
//! Integers have fixed little-endian widths; machine-sized lengths use u64.
//! The versioned layout includes the full query head, prologue and tree, rather
//! than surface spelling or a process-local plan address.

use crate::{Query, walk::NodeRef};
use core::hash::{Hash, Hasher};
use purrdf_hash::{Domain, blake3, hex::Digest32};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

const QUERY: Domain = Domain::new(b"purrdf-sparql-algebra/query-identity/v1");
const NODE: Domain = Domain::new(b"purrdf-sparql-algebra/node-identity/v1");
const UPDATE: Domain = Domain::new(b"purrdf-sparql-algebra/update-identity/v1");

struct StructuralHash(blake3::Streaming<1024>);

macro_rules! integer_writes {
    ($($method:ident: $ty:ty),* $(,)?) => {$ (
        fn $method(&mut self, value: $ty) {
            self.0.update(&value.to_le_bytes());
        }
    )*};
}

impl Hasher for StructuralHash {
    fn finish(&self) -> u64 {
        let digest = self.0.finalize();
        // A 32-byte digest always contains this fixed-width prefix.
        u64::from_le_bytes(*digest.as_bytes().first_chunk::<8>().expect("digest prefix"))
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
    integer_writes!(write_u8: u8, write_u16: u16, write_u32: u32,
        write_u64: u64, write_u128: u128, write_i8: i8, write_i16: i16,
        write_i32: i32, write_i64: i64, write_i128: i128);
    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
    fn write_isize(&mut self, value: isize) {
        self.write_i64(value as i64);
    }
}

/// Identify an expression, pattern or term through the shared structural walk.
///
/// # Errors
/// Returns original-account admission, layout or allocation refusal.
pub fn node_with_memory<S: Admission + ?Sized>(
    node: NodeRef<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<Digest32, StorageError> {
    let mut state = StructuralHash(blake3::Streaming::new());
    state.write(NODE.as_bytes());
    crate::traits::node_hash_with_memory(node, &mut state, memory)?;
    Ok(Digest32::new(*state.0.finalize().as_bytes()))
}

/// Identify the complete query, including template, targets and dataset clause.
///
/// # Errors
/// Returns original-account admission, layout or allocation refusal.
pub fn query_with_memory<S: Admission + ?Sized>(
    query: &Query,
    memory: &mut Memory<'_, S>,
) -> Result<Digest32, StorageError> {
    let mut state = StructuralHash(blake3::Streaming::new());
    state.write(QUERY.as_bytes());
    match query {
        Query::Select { .. } => state.write_u8(0),
        Query::Construct { template, .. } => {
            state.write_u8(1);
            quads(template, &mut state, memory)?;
        }
        Query::Describe { targets, .. } => {
            state.write_u8(2);
            targets.hash(&mut state);
        }
        Query::Ask { .. } => state.write_u8(3),
    }
    query.dataset().hash(&mut state);
    query.base_iri().hash(&mut state);
    query
        .version()
        .map(crate::SparqlVersion::raw)
        .hash(&mut state);
    crate::traits::node_hash_with_memory(NodeRef::Pattern(query.pattern()), &mut state, memory)?;
    Ok(Digest32::new(*state.0.finalize().as_bytes()))
}

fn quads<S: Admission + ?Sized>(
    quads: &[crate::QuadPattern],
    state: &mut StructuralHash,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    quads.len().hash(state);
    for quad in quads {
        crate::traits::node_hash_with_memory(NodeRef::Triple(&quad.triple), state, memory)?;
        quad.graph.hash(state);
    }
    Ok(())
}

/// Identify the full ordered UPDATE program, including templates and dataset clauses.
///
/// # Errors
/// Returns original-account admission, layout or allocation refusal.
pub fn update_with_memory<S: Admission + ?Sized>(
    update: &crate::Update,
    memory: &mut Memory<'_, S>,
) -> Result<Digest32, StorageError> {
    use crate::GraphUpdateOperation;
    let mut state = StructuralHash(blake3::Streaming::new());
    state.write(UPDATE.as_bytes());
    update.base_iri.hash(&mut state);
    update
        .version()
        .map(crate::SparqlVersion::raw)
        .hash(&mut state);
    update.operations.len().hash(&mut state);
    for operation in &update.operations {
        match operation {
            GraphUpdateOperation::InsertData { data } => {
                state.write_u8(0);
                quads(data, &mut state, memory)?;
            }
            GraphUpdateOperation::DeleteData { data } => {
                state.write_u8(1);
                quads(data, &mut state, memory)?;
            }
            GraphUpdateOperation::DeleteInsert {
                delete,
                insert,
                with,
                using,
                pattern,
            } => {
                state.write_u8(2);
                quads(delete, &mut state, memory)?;
                quads(insert, &mut state, memory)?;
                with.hash(&mut state);
                using.hash(&mut state);
                crate::traits::node_hash_with_memory(
                    NodeRef::Pattern(pattern),
                    &mut state,
                    memory,
                )?;
            }
            GraphUpdateOperation::Load {
                silent,
                source,
                destination,
            } => {
                state.write_u8(3);
                silent.hash(&mut state);
                source.hash(&mut state);
                destination.hash(&mut state);
            }
            GraphUpdateOperation::Clear { silent, target } => {
                state.write_u8(4);
                silent.hash(&mut state);
                target.hash(&mut state);
            }
            GraphUpdateOperation::Drop { silent, target } => {
                state.write_u8(5);
                silent.hash(&mut state);
                target.hash(&mut state);
            }
            GraphUpdateOperation::Create { silent, graph } => {
                state.write_u8(6);
                silent.hash(&mut state);
                graph.hash(&mut state);
            }
            GraphUpdateOperation::Add {
                silent,
                source,
                destination,
            } => {
                state.write_u8(7);
                silent.hash(&mut state);
                source.hash(&mut state);
                destination.hash(&mut state);
            }
            GraphUpdateOperation::Move {
                silent,
                source,
                destination,
            } => {
                state.write_u8(8);
                silent.hash(&mut state);
                source.hash(&mut state);
                destination.hash(&mut state);
            }
            GraphUpdateOperation::Copy {
                silent,
                source,
                destination,
            } => {
                state.write_u8(9);
                silent.hash(&mut state);
                source.hash(&mut state);
                destination.hash(&mut state);
            }
        }
    }
    Ok(Digest32::new(*state.0.finalize().as_bytes()))
}
