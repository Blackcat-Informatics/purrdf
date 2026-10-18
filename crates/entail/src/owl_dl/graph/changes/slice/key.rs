// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Link only selected equalities to their original checked membership branches.

use super::{Error, Kb, Support, poll};
use crate::owl_dl::{
    input_source::{Input, Source},
    key_proofs::{Records, Verification},
    key_values::Common,
    proof::{DlProof, capture::Capture},
};

pub(crate) struct Member<'a> {
    pub(crate) individual: u32,
    pub(crate) class: u32,
    pub(crate) proof: &'a DlProof,
    pub(crate) capture: &'a Capture,
}

impl std::fmt::Debug for Member<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Member")
            .field("individual", &self.individual)
            .field("class", &self.class)
            .field("proof", &self.proof)
            .field("capture", &self.capture)
            .finish()
    }
}

impl Member<'_> {
    pub(crate) fn supports(&self) -> impl Iterator<Item = &Support> {
        self.capture.clashes.iter().map(|clash| {
            clash
                .support
                .as_ref()
                .expect("selected membership has complete original support")
        })
    }
}

pub(crate) struct Derivation<'a> {
    pub(crate) equality: usize,
    pub(crate) key: usize,
    pub(crate) left: u32,
    pub(crate) right: u32,
    pub(crate) values: &'a [Common],
    pub(crate) members: [Member<'a>; 2],
}

impl std::fmt::Debug for Derivation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Key")
            .field("equality", &self.equality)
            .field("key", &self.key)
            .field("left", &self.left)
            .field("right", &self.right)
            .field("values", &self.values)
            .field("members", &self.members)
            .finish()
    }
}

impl Derivation<'_> {
    pub(super) fn inputs(&self) -> impl Iterator<Item = Input> + '_ {
        std::iter::once(Input::Key(self.key)).chain(
            self.values
                .iter()
                .flat_map(|value| [Input::Role(value.left), Input::Role(value.right)]),
        )
    }
}

pub(super) fn select(kb: &Kb, equality: usize) -> Result<Derivation<'_>, Error> {
    poll(kb)?;
    let Some(Source::KeyInference { key, left, right }) = kb.inputs.get(Input::Equality(equality))
    else {
        return Err(Error::MissingKey(equality));
    };
    let Records::Ready(evidence) = &kb.key_proofs else {
        return Err(Error::MissingKey(equality));
    };
    if equality < evidence.phase.equalities || kb.same_as.get(equality) != Some(&(left, right)) {
        return Err(Error::MissingKey(equality));
    }
    let (class, properties) = kb.keys.get(key).ok_or(Error::MissingKey(equality))?;
    let reads = kb
        .inputs
        .key_values
        .as_ref()
        .ok_or(Error::MissingKey(equality))?;
    let values = reads.get(equality).ok_or(Error::MissingKey(equality))?;
    if reads.base != evidence.phase.equalities || values.len() != properties.len() {
        return Err(Error::MissingKey(equality));
    }
    for (value, property) in values.iter().zip(properties) {
        poll(kb)?;
        if value.property != *property
            || value.left >= evidence.phase.roles
            || value.right >= evidence.phase.roles
            || kb.abox_roles.get(value.left) != Some(&(left, *property, value.value))
            || kb.abox_roles.get(value.right) != Some(&(right, *property, value.value))
        {
            return Err(Error::MissingKey(equality));
        }
    }
    let member = |individual| {
        let missing = || Error::MissingMembership { key, individual };
        let member = evidence.member(key, individual).ok_or_else(missing)?;
        if member.class != *class || member.individual != individual {
            return Err(missing());
        }
        let Verification::Closed(report, capture) = &member.verification else {
            return Err(missing());
        };
        if !report.is_closed()
            || report.clashes() + report.checked_merges() != capture.clashes.len()
            || capture.clashes.is_empty()
            || capture.clashes.iter().any(|clash| clash.support.is_err())
        {
            return Err(missing());
        }
        Ok(Member {
            individual,
            class: *class,
            proof: &member.proof,
            capture,
        })
    };
    Ok(Derivation {
        equality,
        key,
        left,
        right,
        values,
        members: [member(left)?, member(right)?],
    })
}
