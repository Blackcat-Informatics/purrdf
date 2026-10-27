// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared quotient and strict reduction of a decided subsumption relation.

use purrdf_core::TermValue;

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Relations {
    pub(super) subsumptions: Vec<(TermValue, TermValue)>,
    pub(super) equivalences: Vec<(TermValue, TermValue)>,
    pub(super) direct: Vec<(TermValue, TermValue)>,
}

impl Relations {
    /// Representatives use the supplied order; emitted pairs use canonical term order.
    /// A stopped reduction is usable only under an incomplete certificate.
    pub(super) fn derive(
        n: usize,
        name: impl Fn(usize) -> TermValue,
        holds: impl Fn(usize, usize) -> bool,
        mut poll: impl FnMut() -> bool,
    ) -> Self {
        let mut out = Self::default();
        'derive: {
            for i in 0..n {
                for j in i + 1..n {
                    if !poll() {
                        break 'derive;
                    }
                    out.record_pair(name(i), name(j), holds(i, j), holds(j, i));
                }
            }
            out.reduce(n, &name, &holds, &mut poll);
        }
        out.sort();
        out
    }

    pub(super) fn record_pair(
        &mut self,
        left: TermValue,
        right: TermValue,
        forward: bool,
        backward: bool,
    ) {
        if forward {
            self.subsumptions.push((left.clone(), right.clone()));
        }
        if backward {
            self.subsumptions.push((right.clone(), left.clone()));
        }
        if forward && backward {
            self.equivalences.push((left, right));
        }
    }

    pub(super) fn reduce(
        &mut self,
        n: usize,
        name: &impl Fn(usize) -> TermValue,
        holds: &impl Fn(usize, usize) -> bool,
        poll: &mut impl FnMut() -> bool,
    ) {
        'derive: {
            let mut canonical = Vec::new();
            for i in 0..n {
                let mut representative = true;
                for j in 0..i {
                    if !poll() {
                        break 'derive;
                    }
                    if holds(i, j) && holds(j, i) {
                        representative = false;
                        break;
                    }
                }
                if representative {
                    canonical.push(i);
                }
            }
            let strict = |a, b| holds(a, b) && !holds(b, a);
            for &i in &canonical {
                for &j in &canonical {
                    if !poll() {
                        break 'derive;
                    }
                    if !strict(i, j) {
                        continue;
                    }
                    let mut interposed = false;
                    for &k in &canonical {
                        if !poll() {
                            break 'derive;
                        }
                        if strict(i, k) && strict(k, j) {
                            interposed = true;
                            break;
                        }
                    }
                    if !interposed {
                        self.direct.push((name(i), name(j)));
                    }
                }
            }
        }
    }

    pub(super) fn sort(&mut self) {
        self.subsumptions.sort();
        self.equivalences.sort();
        self.direct.sort();
    }
}
