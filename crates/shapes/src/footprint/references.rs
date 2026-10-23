// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compose the relative reads observed by the one lowering walk. Paths form an
//! arena DAG, so sharing a definition never unfolds its path expressions.

use purrdf_core::{FastMap, SmallVec};

use super::{
    Broadcast, Endpoint, Footprint, FootprintWalk, OPAQUE_CROSS_FOCUS, OPAQUE_UNROOTED, Trigger,
    compose,
};
use crate::PreparationRefusal;
use crate::plan::LoweredPath;
use crate::shapes::Path;
use crate::term::NamedNode;

type ReadKey = (Option<NamedNode>, Endpoint);

#[derive(Default)]
struct Summary {
    reads: FastMap<usize, Option<usize>>,
    broadcasts: Vec<Broadcast>,
    opaque: Option<&'static str>,
}

#[derive(Default)]
struct Paths(Vec<LoweredPath>);

impl Paths {
    fn finish(mut self, triggers: &mut [Trigger]) -> Vec<LoweredPath> {
        let mut incoming = SmallVec::<[u8; 16]>::new();
        incoming.resize(self.0.len(), 0);
        Self::references(&mut self.0, triggers, |reference| {
            if let LoweredPath::Reference(index) = reference {
                incoming[*index] = incoming[*index].saturating_add(1);
            }
        });
        Self::references(&mut self.0, triggers, |reference| {
            if let LoweredPath::Reference(index) = reference
                && incoming[*index] > 1
            {
                *reference = LoweredPath::SharedReference(*index);
            }
        });
        self.0
    }

    fn references(
        paths: &mut [LoweredPath],
        triggers: &mut [Trigger],
        mut visit: impl FnMut(&mut LoweredPath),
    ) {
        for path in paths.iter_mut().chain(
            triggers
                .iter_mut()
                .filter_map(|trigger| trigger.reversed.as_mut()),
        ) {
            let mut pending = purrdf_lex::walk::WorkList::<_, 16>::default();
            pending.push(path);
            while let Some(path) = pending.pop() {
                visit(path);
                match path {
                    LoweredPath::Sequence(parts) | LoweredPath::Alternative(parts) => {
                        pending.extend(parts.iter_mut());
                    }
                    LoweredPath::InvertedComposite(inner)
                    | LoweredPath::ZeroOrMore(inner)
                    | LoweredPath::OneOrMore(inner)
                    | LoweredPath::ZeroOrOne(inner) => pending.push(inner),
                    LoweredPath::Reference(_)
                    | LoweredPath::SharedReference(_)
                    | LoweredPath::Predicate(_)
                    | LoweredPath::InversePredicate(_) => {}
                }
            }
        }
    }

    fn push(&mut self, path: LoweredPath) -> usize {
        let index = self.0.len();
        self.0.push(path);
        index
    }

    fn sequence(&mut self, left: Option<usize>, right: Option<usize>) -> Option<usize> {
        match (left, right) {
            (None, path) | (path, None) => path,
            (Some(left), Some(right)) => Some(self.push(LoweredPath::Sequence(Box::new([
                LoweredPath::Reference(left),
                LoweredPath::Reference(right),
            ])))),
        }
    }

    fn union(&mut self, left: Option<usize>, right: Option<usize>) -> Option<usize> {
        if left == right {
            return left;
        }
        match (left, right) {
            (None, Some(path)) | (Some(path), None) => Some(self.push(LoweredPath::ZeroOrOne(
                Box::new(LoweredPath::Reference(path)),
            ))),
            (Some(left), Some(right)) => Some(self.push(LoweredPath::Alternative(Box::new([
                LoweredPath::Reference(left),
                LoweredPath::Reference(right),
            ])))),
            (None, None) => None,
        }
    }

    fn add(&mut self, summary: &mut Summary, key: usize, path: Option<usize>) {
        if let Some(previous) = summary.reads.get_mut(&key) {
            *previous = self.union(*previous, path);
        } else {
            summary.reads.insert(key, path);
        }
    }
}

/// Resolve reference edges, including cycles, without revisiting any AST body.
pub(crate) fn resolve(
    main: &mut FootprintWalk,
    definitions: &mut [FootprintWalk],
    mut lower_reverse: impl FnMut(&Path) -> Result<LoweredPath, PreparationRefusal>,
) -> Result<Footprint, PreparationRefusal> {
    let mut main = std::mem::take(main);
    // The common value-only catalog has no indirect data reads. In particular,
    // don't allocate a graph merely to prove that unused definitions read nothing.
    if definitions
        .iter()
        .all(|node| node.triggers.is_empty() && node.broadcasts.is_empty() && node.opaque.is_none())
    {
        let pending = std::mem::take(&mut main.triggers);
        let mut triggers = Vec::with_capacity(pending.len());
        for trigger in pending {
            triggers.push(Trigger {
                predicate: trigger.predicate,
                endpoint: trigger.endpoint,
                reversed: trigger.chain.as_ref().map(&mut lower_reverse).transpose()?,
            });
        }
        return Ok(main.finish(triggers));
    }

    let main_id = definitions.len();
    let mut nodes: SmallVec<[_; 4]> = definitions.iter_mut().map(std::mem::take).collect();
    nodes.push(main);
    for node in &nodes {
        assert!(
            node.references.iter().all(|edge| edge.target < main_id),
            "a footprint reference names a missing definition"
        );
    }
    // Demand discovery normally numbers a parent before its children. Strictly
    // increasing edges prove this graph acyclic without graph scratch storage.
    // A shared back edge or any cycle takes the general SCC implementation.
    let forward = nodes[..main_id]
        .iter()
        .enumerate()
        .all(|(source, node)| node.references.iter().all(|edge| edge.target > source));
    let components: SmallVec<[usize; 8]> = if forward {
        (0..main_id).rev().chain(std::iter::once(main_id)).collect()
    } else {
        let adjacency: Vec<Vec<usize>> = nodes
            .iter()
            .map(|node| node.references.iter().map(|edge| edge.target).collect())
            .collect();
        purrdf_core::graph::scc_component_index(&adjacency)
            .into_iter()
            .collect()
    };
    let count = components
        .iter()
        .copied()
        .max()
        .map_or(0, |index| index + 1);
    let mut members: SmallVec<[SmallVec<[usize; 1]>; 4]> = SmallVec::new();
    members.resize_with(count, SmallVec::new);
    for (node, &component) in components.iter().enumerate() {
        members[component].push(node);
    }
    let mut summaries: SmallVec<[Summary; 4]> = (0..count).map(|_| Summary::default()).collect();
    let mut paths = Paths::default();
    let mut keys: FastMap<ReadKey, usize> = FastMap::default();

    // Component ids are reverse topological: every outgoing component is ready.
    for (component, members) in members.into_iter().enumerate() {
        let mut summary = Summary::default();
        let mut outgoing: SmallVec<[_; 2]> = SmallVec::new();
        let mut closure = None;
        let mut unknown_cycle = false;
        for node in members {
            let node = &mut nodes[node];
            if let Some(reason) = node.opaque {
                summary.opaque.get_or_insert(reason);
            }
            summary.broadcasts.append(&mut node.broadcasts);
            for trigger in node.triggers.drain(..) {
                let key = (trigger.predicate, trigger.endpoint);
                let next = keys.len();
                let key = *keys.entry(key).or_insert(next);
                let path = trigger
                    .chain
                    .as_ref()
                    .map(&mut lower_reverse)
                    .transpose()?
                    .map(|path| paths.push(path));
                paths.add(&mut summary, key, path);
            }
            for edge in std::mem::take(&mut node.references) {
                let known = edge.chain.is_some();
                let chain = edge.chain.and_then(compose);
                let path = chain
                    .as_ref()
                    .map(&mut lower_reverse)
                    .transpose()?
                    .map(|path| paths.push(path));
                let target = components[edge.target];
                if target == component {
                    unknown_cycle |= !known;
                    if path.is_some() {
                        closure = Some(match closure {
                            None => path,
                            Some(previous) => paths.union(previous, path),
                        });
                    }
                } else {
                    outgoing.push((target, path, known, edge.root));
                }
            }
        }
        for (target, prefix, known, root) in outgoing {
            let child = &summaries[target];
            if let Some(reason) = child.opaque {
                summary.opaque.get_or_insert(reason);
            }
            if !known && !child.reads.is_empty() {
                summary.opaque.get_or_insert(OPAQUE_UNROOTED);
            }
            for (&key, &path) in &child.reads {
                let path = paths.sequence(path, prefix);
                paths.add(&mut summary, key, path);
            }
            for broadcast in &child.broadcasts {
                if let Some(shape) = root {
                    if !summary.broadcasts.iter().any(|existing| {
                        existing.shape == shape && existing.predicate == broadcast.predicate
                    }) {
                        summary.broadcasts.push(Broadcast {
                            shape,
                            predicate: broadcast.predicate.clone(),
                        });
                    }
                } else {
                    summary.opaque.get_or_insert(OPAQUE_CROSS_FOCUS);
                }
            }
        }
        if unknown_cycle && !summary.reads.is_empty() {
            summary.opaque.get_or_insert(OPAQUE_UNROOTED);
        }
        if let Some(Some(closure)) = closure {
            // Every internal edge is admitted under a star. This is a bounded,
            // sound over-approximation of all routes through the component.
            let closure = paths.push(LoweredPath::ZeroOrMore(Box::new(LoweredPath::Reference(
                closure,
            ))));
            for path in summary.reads.values_mut() {
                *path = paths.sequence(*path, Some(closure));
            }
        }
        summaries[component] = summary;
    }
    let summary = std::mem::take(&mut summaries[components[main_id]]);
    let mut names: Vec<Option<ReadKey>> = (0..keys.len()).map(|_| None).collect();
    for (key, index) in keys {
        names[index] = Some(key);
    }
    let mut triggers: Vec<_> = summary
        .reads
        .into_iter()
        .map(|(key, path)| {
            let (predicate, endpoint) = names[key]
                .take()
                .expect("each footprint read has its original key");
            Trigger {
                predicate,
                endpoint,
                reversed: path.map(LoweredPath::Reference),
            }
        })
        .collect();
    let paths = paths.finish(&mut triggers);
    Ok(Footprint {
        triggers,
        broadcasts: summary.broadcasts,
        opaque: summary.opaque,
        paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::footprint::PendingTrigger;

    fn predicate(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("http://example.org/{local}"))
    }

    fn read() -> FootprintWalk {
        let mut walk = FootprintWalk::definition();
        walk.triggers.push(PendingTrigger {
            predicate: Some(predicate("read")),
            endpoint: Endpoint::Subject,
            chain: None,
        });
        walk
    }

    fn edge(walk: &mut FootprintWalk, target: usize, step: &str) {
        walk.chain = Some(vec![Path::Predicate(predicate(step))]);
        walk.reference(target, false);
    }

    fn resolved(main: &mut FootprintWalk, definitions: &mut [FootprintWalk]) -> Footprint {
        resolve(main, definitions, |path| {
            let Path::Predicate(iri) = path else {
                panic!("the fixture edges are single predicate steps");
            };
            let slot = if *iri == predicate("p") {
                0
            } else {
                assert_eq!(*iri, predicate("q"));
                1
            };
            Ok(LoweredPath::InversePredicate(slot))
        })
        .unwrap()
    }

    fn expanded(footprint: &Footprint, data: &str, focus: &str) -> Vec<String> {
        let data = crate::text_ingest::parse_turtle_to_dataset(data, None).unwrap();
        let lowering = crate::plan::lower_standalone_path(&Path::Sequence(vec![
            Path::Predicate(predicate("p")),
            Path::Predicate(predicate("q")),
        ]))
        .unwrap();
        let binding = lowering.bind(&data);
        assert_eq!(footprint.triggers.len(), 1);
        assert!(footprint.opaque.is_none());
        let path = footprint.triggers[0].reversed.as_ref().unwrap();
        let ids = crate::path::eval_planned_ids_with_paths(
            &data,
            data.term_id_by_iri(focus).unwrap(),
            path,
            &binding,
            &footprint.paths,
        )
        .unwrap();
        let mut values: Vec<_> = ids
            .into_iter()
            .map(|id| crate::term::term_id_to_native(&data, id).to_string())
            .collect();
        values.sort();
        values
    }

    #[test]
    fn shared_reading_diamonds_keep_linear_paths_and_bounded_evaluation() {
        for depth in [1, 8, 32] {
            let mut definitions = vec![read()];
            for target in 0..depth {
                let mut walk = FootprintWalk::definition();
                edge(&mut walk, target, "p");
                edge(&mut walk, target, "q");
                definitions.push(walk);
            }
            let mut main = FootprintWalk::definition();
            main.reference(depth, false);
            let footprint = resolved(&mut main, &mut definitions);
            assert!(footprint.paths.len() <= 5 * depth, "{depth}: {footprint:?}");
            assert_eq!(
                expanded(
                    &footprint,
                    "<http://example.org/a> <http://example.org/p> <http://example.org/a>;
                 <http://example.org/q> <http://example.org/a>.
                 <http://example.org/outside> <http://example.org/read> 1.",
                    "http://example.org/a"
                ),
                ["<http://example.org/a>"]
            );
        }
    }

    #[test]
    fn cyclic_read_routes_include_every_reachable_predecessor() {
        let mut first = FootprintWalk::definition();
        edge(&mut first, 1, "p");
        let mut second = read();
        edge(&mut second, 0, "q");
        let mut main = FootprintWalk::definition();
        main.reference(0, false);
        let footprint = resolved(&mut main, &mut [first, second]);
        assert!(footprint.paths.len() <= 5);
        assert_eq!(
            expanded(
                &footprint,
                "<http://example.org/a> <http://example.org/p> <http://example.org/b>.
             <http://example.org/b> <http://example.org/q> <http://example.org/c>.
             <http://example.org/c> <http://example.org/read> 1.
             <http://example.org/outside> <http://example.org/unrelated> <http://example.org/c>.",
                "http://example.org/c"
            ),
            [
                "<http://example.org/a>",
                "<http://example.org/b>",
                "<http://example.org/c>"
            ]
        );
    }
}
