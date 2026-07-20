// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic adaptive scheduling for independent SHACL focus nodes.
//!
//! Work is split with rayon's indexed `par_chunks`, evaluated in source order
//! inside each chunk, and reduced strictly in chunk order. Output and hard-error
//! selection therefore match the serial traversal regardless of worker timing.

/// Focus sets at or below this stay serial. The value is benchmark-tuned against
/// both inexpensive Core constraints and SHACL-SPARQL focus evaluation.
pub(crate) const PARALLEL_MIN_FOCUS_NODES: usize = 1_024;

/// Avoid handing workers fragments dominated by scope setup and result staging.
const PARALLEL_MIN_CHUNK_ITEMS: usize = 64;

fn chunk_size_for(len: usize) -> usize {
    #[cfg(test)]
    if let Some(forced) = FORCE_CHUNK_SIZE.with(std::cell::Cell::get) {
        return forced.max(1);
    }
    let threads = rayon::current_num_threads().max(1);
    (len / (threads * 4).max(1)).max(PARALLEL_MIN_CHUNK_ITEMS)
}

#[cfg(test)]
std::thread_local! {
    static FORCE_PARALLEL: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    static FORCE_CHUNK_SIZE: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Force the scheduler branch for a deterministic equivalence test.
#[cfg(test)]
#[must_use]
pub(crate) fn force_parallel_for_test(force: bool) -> ForceParallelGuard {
    let previous = FORCE_PARALLEL.with(|cell| cell.replace(Some(force)));
    ForceParallelGuard { previous }
}

/// Restores the production scheduler decision when a test scope ends.
#[cfg(test)]
pub(crate) struct ForceParallelGuard {
    previous: Option<bool>,
}

#[cfg(test)]
impl Drop for ForceParallelGuard {
    fn drop(&mut self) {
        FORCE_PARALLEL.with(|cell| cell.set(self.previous));
    }
}

fn should_parallelize(work_items: usize) -> bool {
    #[cfg(test)]
    if let Some(forced) = FORCE_PARALLEL.with(std::cell::Cell::get) {
        return forced;
    }
    work_items > PARALLEL_MIN_FOCUS_NODES && rayon::current_num_threads() > 1
}

/// Evaluate independent items with one worker-local state per chunk.
///
/// The first error by source chunk (and, within it, source item) wins. Successful
/// outputs concatenate in exact source order.
pub(crate) fn try_map_chunks<T, S, R, E>(
    items: &[T],
    init: impl Fn() -> S + Sync,
    push: impl Fn(&mut S, &mut Vec<R>, &T) -> Result<(), E> + Sync,
) -> Result<Vec<R>, E>
where
    T: Sync,
    R: Send,
    E: Send,
{
    if !should_parallelize(items.len()) {
        let mut state = init();
        let mut out = Vec::new();
        for item in items {
            push(&mut state, &mut out, item)?;
        }
        return Ok(out);
    }

    use rayon::prelude::*;

    let chunk_size = chunk_size_for(items.len());
    let per_chunk: Vec<Result<Vec<R>, E>> = items
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut state = init();
            let mut out = Vec::new();
            for item in chunk {
                push(&mut state, &mut out, item)?;
            }
            Ok(out)
        })
        .collect();

    let mut out = Vec::with_capacity(
        per_chunk
            .iter()
            .map(|result| result.as_ref().map_or(0, Vec::len))
            .sum(),
    );
    for result in per_chunk {
        out.extend(result?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OverrideGuard {
        parallel: Option<bool>,
        chunk_size: Option<usize>,
    }

    impl OverrideGuard {
        fn new(parallel: bool, chunk_size: usize) -> Self {
            let previous_parallel = FORCE_PARALLEL.with(|cell| cell.replace(Some(parallel)));
            let previous_chunk = FORCE_CHUNK_SIZE.with(|cell| cell.replace(Some(chunk_size)));
            Self {
                parallel: previous_parallel,
                chunk_size: previous_chunk,
            }
        }
    }

    impl Drop for OverrideGuard {
        fn drop(&mut self) {
            FORCE_PARALLEL.with(|cell| cell.set(self.parallel));
            FORCE_CHUNK_SIZE.with(|cell| cell.set(self.chunk_size));
        }
    }

    fn run(items: &[usize], parallel: bool) -> Result<Vec<usize>, usize> {
        let _guard = OverrideGuard::new(parallel, 3);
        try_map_chunks(
            items,
            || (),
            |(), out, item| {
                if *item == 7 || *item == 11 {
                    Err(*item)
                } else {
                    out.extend([*item, item * 2]);
                    Ok(())
                }
            },
        )
    }

    #[test]
    fn parallel_output_matches_serial_source_order() {
        let items: Vec<usize> = (0..7).collect();
        assert_eq!(run(&items, false), run(&items, true));
    }

    #[test]
    fn earliest_source_error_wins() {
        let items: Vec<usize> = (0..16).collect();
        assert_eq!(run(&items, false), Err(7));
        assert_eq!(run(&items, true), Err(7));
    }

    #[test]
    fn threshold_boundary_is_strict() {
        assert!(!should_parallelize(PARALLEL_MIN_FOCUS_NODES));
    }
}
