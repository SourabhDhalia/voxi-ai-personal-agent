//! Bounded parallel execution for fan-out work such as parallel sub-agents.
//!
//! Runs many async workers concurrently with a hard cap on how many are in
//! flight at once, so a fan-out can never overwhelm shared resources (LLM
//! backends, the tool executor) — the "default serial, explicit parallel"
//! discipline borrowed from modern multi-agent runtimes.

use futures_util::stream::StreamExt;
use std::future::Future;

/// Run `worker` over every item with at most `max_concurrent` futures in flight,
/// collecting all results. `max_concurrent` is floored at 1. Results come back
/// in completion order, so each worker should tag its output with the item's
/// index if ordering matters.
pub async fn run_parallel_bounded<T, F, Fut, R>(
    items: Vec<T>,
    max_concurrent: usize,
    worker: F,
) -> Vec<R>
where
    F: Fn(usize, T) -> Fut,
    Fut: Future<Output = R>,
{
    let cap = max_concurrent.max(1);
    futures_util::stream::iter(items.into_iter().enumerate())
        .map(|(index, item)| worker(index, item))
        .buffer_unordered(cap)
        .collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block<F: Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(f)
    }

    #[test]
    fn runs_all_items_with_cap() {
        let out = block(run_parallel_bounded(
            vec![10, 20, 30, 40, 50],
            2,
            |i, v| async move { (i, v * 2) },
        ));
        assert_eq!(out.len(), 5);
        let sum: i32 = out.iter().map(|(_, v)| *v).sum();
        assert_eq!(sum, (10 + 20 + 30 + 40 + 50) * 2);
    }

    #[test]
    fn empty_input_yields_empty() {
        let out: Vec<i32> =
            block(run_parallel_bounded(Vec::<i32>::new(), 4, |_, v| async move { v }));
        assert!(out.is_empty());
    }

    #[test]
    fn zero_cap_is_floored_to_one() {
        let out = block(run_parallel_bounded(
            vec![1, 2, 3],
            0,
            |_, v| async move { v },
        ));
        assert_eq!(out.len(), 3);
    }
}
