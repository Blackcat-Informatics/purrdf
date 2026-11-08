// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-owned property-kind constraints, collected by the native parser.

use std::collections::BTreeMap;

use crate::EntailError;
use purrdf_core::TermValue;

pub(crate) const OBJECT: u8 = 1;
pub(crate) const DATA: u8 = 2;
pub(crate) const ANNOTATION: u8 = 4;
pub(crate) const LOGICAL: u8 = OBJECT | DATA;
pub(crate) const ANY: u8 = LOGICAL | ANNOTATION;

/// Each mask is the remaining permitted kinds, not an inferred default.
#[derive(Debug, Default)]
pub(crate) struct PropertyKinds {
    permitted: BTreeMap<u32, u8>,
    equal: Vec<(u32, u32)>,
}

impl PropertyKinds {
    pub(crate) fn constrain(&mut self, property: u32, permitted: u8) {
        *self.permitted.entry(property).or_insert(ANY) &= permitted;
    }

    pub(crate) fn link(&mut self, left: u32, right: u32) {
        self.constrain(left, ANY);
        self.constrain(right, ANY);
        self.equal.push((left, right));
    }

    /// Resolve only this service's inventory; legacy reasoning is unaffected.
    pub(crate) fn objects(
        &self,
        interner: &crate::interner::Interner,
        mut poll: impl FnMut() -> Result<(), EntailError>,
    ) -> Result<Vec<u32>, EntailError> {
        let mut permitted = BTreeMap::new();
        for (&property, &kinds) in &self.permitted {
            poll()?;
            permitted.insert(property, kinds);
        }
        loop {
            let mut changed = false;
            for &(left, right) in &self.equal {
                poll()?;
                let common = permitted[&left] & permitted[&right];
                for property in [left, right] {
                    let previous = permitted.insert(property, common);
                    changed |= previous != Some(common);
                }
            }
            if !changed {
                break;
            }
        }
        let mut objects = Vec::new();
        for (property, kinds) in permitted {
            poll()?;
            let TermValue::Iri(iri) = interner.value(property) else {
                continue;
            };
            match kinds {
                OBJECT => objects.push(property),
                DATA | ANNOTATION => {}
                _ => {
                    return Err(EntailError::PropertyKind {
                        property: iri.clone(),
                        conflict: kinds == 0,
                    });
                }
            }
        }
        objects.sort_by(|&a, &b| interner.value(a).cmp(interner.value(b)));
        Ok(objects)
    }
}
