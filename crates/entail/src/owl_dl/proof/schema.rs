// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The v4 extension, only for a prepared clash. Other proof bytes remain v3.

use super::{
    DlProofError, Ground, NodeRef, ProofGround, ProofRole, Reader, SchemaClashEvidence,
    encode_ground, encode_role, malformed,
};
use crate::owl_dl::bounds::{
    QualifierProof, RoleStep, SchemaBoundEvidence, SchemaRule, SchemaStep, SchemaUpperBound,
};
use crate::owl_dl::concept::Role;
use crate::owl_dl::graph::{GeneratedRoot, NominalId};
use crate::owl_dl::support::Application;

pub(super) fn encode_schema_clash(out: &mut Vec<u8>, evidence: &SchemaClashEvidence) {
    let bounds = &evidence.bounds;
    out.extend_from_slice(&bounds.class.to_le_bytes());
    out.extend_from_slice(&bounds.lower.to_le_bytes());
    match bounds.upper {
        SchemaUpperBound::Restriction(id) => {
            out.push(0);
            out.extend_from_slice(&id.to_le_bytes());
        }
        SchemaUpperBound::DataExtent(count) => {
            out.push(1);
            out.extend_from_slice(&count.to_le_bytes());
        }
    }
    encode_steps(out, &bounds.class_steps);
    match &bounds.qualifier {
        QualifierProof::Identity => out.push(0),
        QualifierProof::Universal => out.push(1),
        QualifierProof::Object(steps) => {
            out.push(2);
            encode_steps(out, steps);
        }
        QualifierProof::Data => out.push(3),
    }
    out.extend_from_slice(&(bounds.roles.len() as u64).to_le_bytes());
    for step in &bounds.roles {
        encode_role(out, ProofRole::of(step.from));
        encode_role(out, ProofRole::of(step.to));
    }
    encode_frame(out, &evidence.frame);
    out.extend_from_slice(&(evidence.support.len() as u64).to_le_bytes());
    for application in &evidence.support {
        out.extend_from_slice(&(application.clause as u64).to_le_bytes());
        encode_frame(out, &application.frame);
        out.push(u8::from(application.branch.is_some()));
        if let Some((branch, ordinal)) = application.branch {
            out.extend_from_slice(&(branch as u64).to_le_bytes());
            out.extend_from_slice(&(ordinal as u64).to_le_bytes());
        }
        out.extend_from_slice(&(application.head.len() as u64).to_le_bytes());
        for atom in &application.head {
            let mapped = atom.map(&mut |&node| {
                NodeRef::Anonymous(u32::try_from(node).expect("native graph identities fit u32"))
            });
            encode_ground(out, &ProofGround::of(&mapped));
        }
    }
}

fn encode_frame(out: &mut Vec<u8>, frame: &[usize]) {
    out.extend_from_slice(&(frame.len() as u64).to_le_bytes());
    for &node in frame {
        out.extend_from_slice(&(node as u64).to_le_bytes());
    }
}

fn encode_steps(out: &mut Vec<u8>, steps: &[SchemaStep]) {
    out.extend_from_slice(&(steps.len() as u64).to_le_bytes());
    for step in steps {
        out.extend_from_slice(&step.concept.to_le_bytes());
        match step.rule {
            SchemaRule::Assume => out.push(0),
            SchemaRule::Top => out.push(1),
            SchemaRule::Conjunct(parent) => {
                out.push(2);
                out.extend_from_slice(&parent.to_le_bytes());
            }
            SchemaRule::Conjunction => out.push(3),
            SchemaRule::Inclusion(index) => {
                out.push(4);
                out.extend_from_slice(&(index as u64).to_le_bytes());
            }
        }
    }
}

fn decode_steps(reader: &mut Reader<'_>) -> Result<Vec<SchemaStep>, DlProofError> {
    let mut steps = Vec::new();
    for _ in 0..reader.length()? {
        let concept = reader.u32()?;
        let rule = match reader.byte()? {
            0 => SchemaRule::Assume,
            1 => SchemaRule::Top,
            2 => SchemaRule::Conjunct(reader.u32()?),
            3 => SchemaRule::Conjunction,
            4 => SchemaRule::Inclusion(reader.length()?),
            _ => return Err(malformed("unknown schema derivation rule")),
        };
        steps.push(SchemaStep { concept, rule });
    }
    Ok(steps)
}

fn decode_frame(reader: &mut Reader<'_>) -> Result<Vec<usize>, DlProofError> {
    let mut frame = Vec::new();
    for _ in 0..reader.length()? {
        frame.push(reader.length()?);
    }
    Ok(frame)
}

pub(super) fn decode_schema_clash(
    reader: &mut Reader<'_>,
) -> Result<SchemaClashEvidence, DlProofError> {
    let class = reader.u32()?;
    let lower = reader.u32()?;
    let upper = match reader.byte()? {
        0 => SchemaUpperBound::Restriction(reader.u32()?),
        1 => SchemaUpperBound::DataExtent(reader.u64()?),
        _ => return Err(malformed("unknown schema upper bound")),
    };
    let class_steps = decode_steps(reader)?;
    let qualifier = match reader.byte()? {
        0 => QualifierProof::Identity,
        1 => QualifierProof::Universal,
        2 => QualifierProof::Object(decode_steps(reader)?),
        3 => QualifierProof::Data,
        _ => return Err(malformed("unknown qualifier proof")),
    };
    let mut roles = Vec::new();
    for _ in 0..reader.length()? {
        roles.push(RoleStep {
            from: native_role(reader.role()?),
            to: native_role(reader.role()?),
        });
    }
    let bounds = SchemaBoundEvidence {
        class,
        lower,
        upper,
        class_steps,
        qualifier,
        roles,
    };
    let frame = decode_frame(reader)?;
    let mut support = Vec::new();
    for _ in 0..reader.length()? {
        let clause = reader.length()?;
        let frame = decode_frame(reader)?;
        let branch = if reader.flag()? {
            Some((reader.length()?, reader.length()?))
        } else {
            None
        };
        let mut head = Vec::new();
        for _ in 0..reader.length()? {
            head.push(native_ground(&reader.ground()?)?);
        }
        support.push(Application {
            clause,
            frame,
            branch,
            head,
        });
    }
    Ok(SchemaClashEvidence {
        bounds,
        support,
        frame,
    })
}

fn native_role(role: ProofRole) -> Role {
    if role.inverse {
        Role::Inv(role.property)
    } else {
        Role::Named(role.property)
    }
}

fn native_ground(atom: &ProofGround) -> Result<Ground<usize>, DlProofError> {
    let index = |node: &NodeRef| match node {
        NodeRef::Anonymous(index) => Ok(*index as usize),
        _ => Err(malformed(
            "support frames use native indices, not asserted identities",
        )),
    };
    Ok(match atom {
        ProofGround::Concept { node, concept } => Ground::Concept(index(node)?, *concept),
        ProofGround::SelfLoop { node, role } => Ground::SelfLoop(index(node)?, native_role(*role)),
        ProofGround::AtLeast {
            node,
            n,
            role,
            filler,
        } => Ground::AtLeast(index(node)?, *n, native_role(*role), *filler),
        ProofGround::Equal { left, right } => Ground::Equal(index(left)?, index(right)?),
        ProofGround::EqualIndividual { node, individual } => {
            Ground::EqualIndividual(index(node)?, *individual)
        }
        ProofGround::EqualReserved { node, root } => {
            Ground::EqualReserved(index(node)?, native_reserved(root)?)
        }
    })
}

fn native_reserved(root: &super::ReservedRef) -> Result<GeneratedRoot, DlProofError> {
    let origin = match &root.origin {
        NodeRef::Individual(id) => NominalId::Named(*id),
        NodeRef::Reserved(parent) => NominalId::Generated(Box::new(native_reserved(parent)?)),
        NodeRef::Anonymous(_) => return Err(malformed("a reserved root needs a nominal source")),
    };
    Ok(GeneratedRoot {
        origin,
        role: native_role(root.role),
        filler: root.filler,
        index: root.index,
    })
}
