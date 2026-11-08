// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Flat term records shared by projection and reconstruction.

use core::cmp::Ordering;
use std::collections::BTreeMap;

use purrdf_core::{BlankScope, Nested, RdfTextDirection, ShallowTermKey, TermBox, TermValue};

/// Dependency IDs name records, never recursively owned child values.
/// Derived ordering is only an exact identity-map key, not RDF term order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TermRecord {
    Iri(String),
    Literal {
        datatype: usize,
        language: Option<String>,
        lexical: String,
        direction: Option<RdfTextDirection>,
    },
    Blank {
        label: String,
        scope: BlankScope,
    },
    Triple {
        s: usize,
        p: usize,
        o: usize,
    },
}

impl TermRecord {
    /// Original dependency visitation order, including literal datatypes.
    pub(crate) const fn dependencies(&self) -> ([usize; 3], usize) {
        match *self {
            Self::Literal { datatype, .. } => ([datatype, 0, 0], 1),
            Self::Triple { s, p, o } => ([s, p, o], 3),
            Self::Iri(_) | Self::Blank { .. } => ([0; 3], 0),
        }
    }

    pub(crate) fn remap(&self, mut id: impl FnMut(usize) -> usize) -> Self {
        match self {
            Self::Iri(iri) => Self::Iri(iri.clone()),
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => Self::Literal {
                lexical: lexical.clone(),
                datatype: id(*datatype),
                language: language.clone(),
                direction: *direction,
            },
            Self::Blank { label, scope } => Self::Blank {
                label: label.clone(),
                scope: *scope,
            },
            Self::Triple { s, p, o } => Self::Triple {
                s: id(*s),
                p: id(*p),
                o: id(*o),
            },
        }
    }
}

/// Exact bottom-up identity memo. Children must already have canonical IDs.
/// Both owners retain only distinct scalar records and their reference edges.
#[derive(Debug, Default)]
pub(crate) struct TermDictionary {
    pub(crate) records: Vec<TermRecord>,
    identities: BTreeMap<TermRecord, usize>,
    #[cfg(test)]
    pub(crate) comparison_steps: core::cell::Cell<usize>,
}

impl TermDictionary {
    pub(crate) fn intern(&mut self, record: TermRecord) -> usize {
        if let Some(&id) = self.identities.get(&record) {
            return id;
        }
        let id = self.records.len();
        self.identities.insert(record.clone(), id);
        self.records.push(record);
        id
    }

    fn shallow(&self, id: usize) -> ShallowTermKey<'_> {
        match &self.records[id] {
            TermRecord::Iri(iri) => ShallowTermKey::Iri(iri),
            TermRecord::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let TermRecord::Iri(datatype) = &self.records[*datatype] else {
                    unreachable!("literal datatype validated before dictionary insertion")
                };
                ShallowTermKey::Literal {
                    datatype,
                    language: language.as_deref(),
                    lexical,
                    direction: *direction,
                }
            }
            TermRecord::Blank { label, scope } => ShallowTermKey::Blank {
                label,
                scope: *scope,
            },
            TermRecord::Triple { .. } => ShallowTermKey::Triple {},
        }
    }

    /// Native total order over exact memoized identities. Equal children are
    /// skipped by identity, so a shared diamond is never expanded. Unequal
    /// triples descend only through their first unequal component: its order
    /// decides the parent, and no return stack or expanded tree is needed.
    pub(crate) fn compare(&self, mut a: usize, mut b: usize) -> Ordering {
        loop {
            #[cfg(test)]
            self.comparison_steps.set(self.comparison_steps.get() + 1);
            if a == b {
                return Ordering::Equal;
            }
            let order = self.shallow(a).cmp(&self.shallow(b));
            if !order.is_eq() {
                return order;
            }
            let (
                TermRecord::Triple {
                    s: sa,
                    p: pa,
                    o: oa,
                },
                TermRecord::Triple {
                    s: sb,
                    p: pb,
                    o: ob,
                },
            ) = (&self.records[a], &self.records[b])
            else {
                unreachable!("equal scalar records have one canonical identity")
            };
            (a, b) = if sa != sb {
                (*sa, *sb)
            } else if pa != pb {
                (*pa, *pb)
            } else {
                (*oa, *ob)
            };
        }
    }

    pub(crate) fn ordered(&self) -> Vec<usize> {
        let mut order: Vec<_> = (0..self.records.len()).collect();
        order.sort_unstable_by(|&a, &b| self.compare(a, b));
        order
    }

    /// Only malformed-input diagnostics require an owned native operand.
    /// Successful projection and reconstruction keep every child as an ID.
    pub(crate) fn diagnostic_value(&self, root: usize) -> TermValue {
        let mut dictionary = self;
        let result = purrdf_core::try_fold_nested(
            root,
            &mut dictionary,
            |dictionary, id| {
                Ok::<_, core::convert::Infallible>(match &dictionary.records[id] {
                    TermRecord::Iri(iri) => Nested::Leaf(TermValue::Iri(iri.clone())),
                    TermRecord::Blank { label, scope } => Nested::Leaf(TermValue::Blank {
                        label: label.clone(),
                        scope: *scope,
                    }),
                    TermRecord::Literal {
                        lexical,
                        datatype,
                        language,
                        direction,
                    } => {
                        let TermRecord::Iri(datatype) = &dictionary.records[*datatype] else {
                            unreachable!("validated dictionary datatype")
                        };
                        Nested::Leaf(TermValue::Literal {
                            lexical_form: lexical.clone(),
                            datatype: datatype.clone(),
                            language: language.clone(),
                            direction: *direction,
                        })
                    }
                    TermRecord::Triple { s, p, o } => Nested::Triple(*s, *p, *o),
                })
            },
            |_, _, s, p, o| {
                Ok(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                })
            },
        );
        match result {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }
}
