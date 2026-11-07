// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bookkeeping candidates measured under an actual query-owned capability.
//! Initialization counts cover newly initialized stamp payloads and page clears,
//! excluding ordinary visit marks and replacement generations on existing entries.
//! Allocator measurements also include the native lookup table, its capacity and
//! replacement overlap.

use std::sync::{Arc, Mutex};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::{SegmentedBuildLimits, SegmentedBuilder, SegmentedReadLimits, SparqlRequest};
use purrdf_sparql_eval::{
    AdmittedMap, BindingPattern, EvalError, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity,
    PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, QueryOptions, Volatility,
    WorkspaceCapability, WorkspaceCursor,
};

use super::{PAGE_WORDS, Stamps, Visited};
use crate::HnswError;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const CEILING: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
enum Choice {
    Sparse,
    Pages,
    Dense,
}

enum Candidate {
    Native(Visited),
    Sparse {
        entries: AdmittedMap<usize, u32>,
        generation: u32,
    },
}

fn native_error(error: HnswError) -> EvalError {
    match error {
        HnswError::Workspace(error) => error,
        other => panic!("unexpected bookkeeping refusal: {other}"),
    }
}

impl Candidate {
    fn new(
        choice: Choice,
        nodes: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        match choice {
            Choice::Sparse => Ok(Self::Sparse {
                entries: AdmittedMap::default(),
                generation: 0,
            }),
            Choice::Dense => Visited::dense(nodes, workspace)
                .map(Self::Native)
                .map_err(native_error),
            Choice::Pages => Visited::try_new(nodes, workspace)
                .map(Self::Native)
                .map_err(native_error),
        }
    }

    fn begin(&mut self) {
        match self {
            Self::Native(visited) => visited.begin(),
            Self::Sparse { generation, .. } => *generation += 1,
        }
    }

    fn visit(
        &mut self,
        row: usize,
        workspace: &WorkspaceCapability,
        initialized: &mut usize,
    ) -> Result<bool, EvalError> {
        match self {
            Self::Native(visited) => {
                if let Stamps::Pages(pages) = &visited.stamps
                    && pages
                        .get(&(row / super::PAGE_ROWS))
                        .is_none_or(|page| page.generation != visited.generation)
                {
                    *initialized += size_of::<[u64; PAGE_WORDS]>();
                }
                visited.visit(row).map_err(native_error)
            }
            Self::Sparse {
                entries,
                generation,
            } => {
                let old_len = entries.len();
                let previous = entries.insert_admitted(row, *generation, workspace)?;
                *initialized += (entries.len() - old_len) * size_of::<u32>();
                Ok(previous != Some(*generation))
            }
        }
    }

    fn retained_units(&self) -> usize {
        match self {
            Self::Native(Visited {
                stamps: Stamps::Pages(entries),
                ..
            }) => entries.len(),
            Self::Native(Visited {
                stamps: Stamps::Dense { values, .. },
                ..
            }) => values.len(),
            Self::Sparse { entries, .. } => entries.len(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    storage: Measurement,
    initialized_cold: usize,
    initialized_reused: usize,
    retained_units: usize,
}

struct Probe {
    choice: Choice,
    nodes: usize,
    touched: usize,
    modes: [BindingPattern; 1],
    sample: Arc<Mutex<Option<Sample>>>,
}

struct Exhausted;
impl PfCursor for Exhausted {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }

    fn next_admitted(&mut self) -> Result<Option<purrdf_sparql_eval::AdmittedPfRow>, EvalError> {
        Ok(None)
    }
}

impl PropertyFunction for Probe {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }
    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }
    fn rows_per_invocation(&self, _: BindingPattern) -> u64 {
        1
    }
    fn open<'cursor>(
        &self,
        _: &PfArgs<'_>,
        _: Option<u64>,
    ) -> Result<Box<dyn PfCursor + 'cursor>, EvalError>
    where
        Self: 'cursor,
    {
        Err(EvalError::WorkspaceUnpriced(
            "visitation probe requires its native account",
        ))
    }
    fn open_admitted<'cursor>(
        &self,
        _: &PfArgs<'_>,
        _: Option<u64>,
        workspace: WorkspaceCapability,
    ) -> Result<WorkspaceCursor<'cursor>, EvalError>
    where
        Self: 'cursor,
    {
        assert!(workspace.is_bounded());
        let window = CurrentThreadWindow::open();
        let mut candidate = Candidate::new(self.choice, self.nodes, &workspace)?;
        let mut initialized = if matches!(self.choice, Choice::Dense) {
            self.nodes * size_of::<u32>()
        } else {
            0
        };
        candidate.begin();
        for at in 0..self.touched {
            let row = at * (self.nodes / self.touched).max(1);
            assert!(candidate.visit(row, &workspace, &mut initialized)?);
            assert!(!candidate.visit(row, &workspace, &mut initialized)?);
        }
        let cold = initialized;
        candidate.begin();
        for at in 0..self.touched {
            assert!(candidate.visit(
                at * (self.nodes / self.touched).max(1),
                &workspace,
                &mut initialized
            )?);
        }
        let sample = Sample {
            storage: window.close(),
            initialized_cold: cold,
            initialized_reused: initialized - cold,
            retained_units: candidate.retained_units(),
        };
        drop(candidate);
        *self.sample.lock().unwrap() = Some(sample);
        WorkspaceCursor::new(Exhausted, &workspace, "visitation probe cursor")
    }
}

#[test]
fn compare_query_owned_visitation_at_sparse_and_dense_fractions() {
    let image = SegmentedBuilder::new(SegmentedBuildLimits::new(8, 512, 8, 512, 4).unwrap())
        .unwrap()
        .seal()
        .unwrap();
    for nodes in [4096, 1_048_576] {
        for touched in [32, nodes / 16, nodes] {
            for choice in [Choice::Sparse, Choice::Pages, Choice::Dense] {
                let source = image
                    .open_session(SegmentedReadLimits::new(CEILING, 2, 32, 1_000_000, 8))
                    .unwrap();
                let opening = source.evidence();
                let sample = Arc::new(Mutex::new(None));
                let mut registry = PropertyFunctionRegistry::new();
                registry.register(
                    "https://example.org/visitation".to_owned(),
                    Arc::new(Probe {
                        choice,
                        nodes,
                        touched,
                        modes: [PfArity::new(1, 1).all_free_mode()],
                        sample: sample.clone(),
                    }),
                );
                let env = ExtensionEnv::over_relations(registry).unwrap();
                let result = NativeSparqlEngine::new().query_fallible_view(&source,
                    SparqlRequest { query: "SELECT ?row WHERE { ?row <https://example.org/visitation> <https://example.org/input> }", base_iri: None, substitutions: &[] },
                    &QueryOptions::new().with_env(&env)).unwrap();
                drop(result);
                let sample = sample
                    .lock()
                    .unwrap()
                    .take()
                    .expect("the query entered the probe");
                let evidence = source.evidence();
                assert!(
                    u64::try_from(sample.storage.peak_working_bytes).unwrap()
                        <= evidence.peak_bytes()
                );
                assert_eq!(
                    evidence.live_bytes()
                        - evidence.unpinned_cache_bytes()
                        - evidence.bookkeeping_bytes(),
                    opening.live_bytes()
                        - opening.unpinned_cache_bytes()
                        - opening.bookkeeping_bytes()
                );
                eprintln!(
                    "visitation={choice:?}, graph={nodes}, visited={touched}, retained_units={}, initialized_stamp_bytes_cold={}, initialized_stamp_bytes_reused={}, actual_capacity_bytes={}, allocations={}, requested_bytes={}, allocator_peak={}, accounted_peak={}",
                    sample.retained_units,
                    sample.initialized_cold,
                    sample.initialized_reused,
                    sample.storage.retained_bytes,
                    sample.storage.allocations,
                    sample.storage.requested_bytes,
                    sample.storage.peak_working_bytes,
                    evidence.peak_bytes()
                );
            }
        }
    }
}
