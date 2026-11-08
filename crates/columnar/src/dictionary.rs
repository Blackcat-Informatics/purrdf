// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Flat term records shared by projection and reconstruction.

use core::cmp::Ordering;
use std::collections::BTreeMap;

use purrdf_core::{BlankScope, RdfTextDirection, ShallowTermKey, TermRef, TermValue};

#[cfg(test)]
use purrdf_core::{Nested, TermBox};

use crate::error::ProjectionReason;

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

impl purrdf_hash::error::StructuredError for TermRecord {
    purrdf_hash::error_visit! {
        Iri(iri) => "rdf-term-iri" { "iri" => Iri(iri) },
        Blank { label, scope } => "rdf-term-blank" {
            "label" => Text(label), "scope" => Unsigned(u128::from(scope.ordinal())),
        },
        Literal { lexical, datatype, language, direction } => "rdf-term-literal-reference" {
            "lexical_form" => Text(lexical), "datatype" => Unsigned(*datatype as u128),
            "language" => Value(language.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent,
                purrdf_hash::error::ErrorValue::Text)),
            "direction" => Value(direction.map_or(purrdf_hash::error::ErrorValue::Absent,
                |value| purrdf_hash::error::ErrorValue::Code(value.as_str()))),
        },
        Triple { s, p, o } => "rdf-term-triple-reference" {
            "subject" => Unsigned(*s as u128), "predicate" => Unsigned(*p as u128),
            "object" => Unsigned(*o as u128),
        },
    }
}

/// An exact rejected term graph retained without expanding shared children.
///
/// Identifiers address this owner's flat records. Their order is internal and
/// does not identify source ordinals. All referenced records remain available
/// after the source is dropped, including original scalar values and scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionTermGraph {
    records: Vec<TermRecord>,
    root: usize,
}

impl ProjectionTermGraph {
    /// The rejected operand's identifier within this graph.
    #[must_use]
    pub const fn root(&self) -> usize {
        self.root
    }

    /// The number of retained distinct records, including resolved dependencies.
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.records.len()
    }

    /// Borrow one original record, or `None` if the identifier is out of range.
    #[must_use]
    pub fn resolve(&self, id: usize) -> Option<TermRef<'_, usize>> {
        self.records.get(id).map(|record| match record {
            TermRecord::Iri(iri) => TermRef::Iri(iri),
            TermRecord::Blank { label, scope } => TermRef::Blank {
                label,
                scope: *scope,
            },
            TermRecord::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => TermRef::Literal {
                lexical,
                datatype: *datatype,
                language: language.as_deref(),
                direction: *direction,
            },
            TermRecord::Triple { s, p, o } => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
        })
    }
}

impl purrdf_hash::error::StructuredError for ProjectionTermGraph {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::{ErrorValue, visit_list};
        visitor.variant("rdf-term-graph");
        visitor.value(Some("root"), ErrorValue::Unsigned(self.root as u128));
        visit_list(
            visitor,
            Some("records"),
            self.records.iter().map(|record| ErrorValue::Record(record)),
        );
    }
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

    /// Retain the rejected operand without ever unfolding triple dependencies.
    pub(crate) fn datatype_refusal(&self, root: usize) -> ProjectionReason {
        let actual = match &self.records[root] {
            TermRecord::Triple { .. } => {
                let mut local = BTreeMap::new();
                let mut records = Vec::new();
                let mut pending = vec![(root, false)];
                while let Some((id, expanded)) = pending.pop() {
                    if local.contains_key(&id) {
                        continue;
                    }
                    if !expanded {
                        pending.push((id, true));
                        let (children, count) = self.records[id].dependencies();
                        pending.extend(children[..count].iter().rev().map(|&id| (id, false)));
                        continue;
                    }
                    // Dictionary dependencies are acyclic and visited child-first.
                    let record = self.records[id].remap(|child| local[&child]);
                    local.insert(id, records.len());
                    records.push(record);
                }
                return ProjectionReason::DatatypeGraphNotIri {
                    actual: ProjectionTermGraph {
                        records,
                        root: local[&root],
                    },
                };
            }
            TermRecord::Blank { label, scope } => TermValue::Blank {
                label: label.clone(),
                scope: *scope,
            },
            TermRecord::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let TermRecord::Iri(datatype) = &self.records[*datatype] else {
                    unreachable!("dictionary literal datatype already validated")
                };
                TermValue::Literal {
                    lexical_form: lexical.clone(),
                    datatype: datatype.clone(),
                    language: language.clone(),
                    direction: *direction,
                }
            }
            TermRecord::Iri(_) => unreachable!("IRI datatypes are accepted"),
        };
        ProjectionReason::DatatypeNotIri { actual }
    }

    /// Only the small recursive reference tests unfold native values.
    #[cfg(test)]
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
