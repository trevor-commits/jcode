# Session Search Memory Repair Design

## Goal

Keep Jcode's shared server memory bounded when several `session_search` calls target a large transcript collection at the same time, while preserving search results and ordinary tool parallelism.

## Observed failure

Three batched searches can concurrently miss the same in-memory index cache. Each call then reads and tokenizes the same multi-gigabyte transcript store, and each attempts to rename the same fixed `.tmp` file. On macOS this drove the server above 4 GB, produced two cache-save failures, and left the system allocator holding a large high-water footprint after the searches completed. The runtime memory log could not explain the incident because non-Linux snapshots returned empty values.

## Considered approaches

1. **Resource-specific coordination (selected).** Single-flight each index path and put one process-wide permit around the expensive blocking session search. This directly prevents duplicate work while leaving unrelated batch tools parallel.
2. **Make all batch calls sequential.** This would stop the incident but would slow unrelated safe tools and change a broad public behavior.
3. **Stream and redesign every transcript parser.** This could lower the cost of one cold search, but it is a much larger change and does not by itself prevent duplicate index rebuilds or cache-write races.

## Design

### Per-index single-flight

Maintain a small process-local map from normalized index path to a mutex. `build_or_update` obtains the path's mutex before checking the cache or disk. A waiter rechecks the in-memory cache after it acquires the mutex and reuses the completed index rather than rebuilding it. Different source indexes may still build independently.

### Search concurrency gate

Use one asynchronous semaphore permit around the blocking portion of `SessionSearchTool::execute`. Batched or cross-client searches wait without blocking a Tokio worker. Query validation remains outside the gate, and unrelated tools retain the batch tool's existing parallelism.

### Collision-proof cache writes

Write the serialized index through a uniquely named temporary file in the destination directory, then atomically persist it over the target. Temporary files are cleaned up on failure. This protects independent processes as well as threads inside one server.

### macOS telemetry

On macOS, collect resident and virtual memory plus peak resident memory from native Mach/rusage APIs. Unsupported fields remain absent, and snapshot failures are logged rather than changing server behavior.

## Error handling

- A poisoned coordination mutex becomes a normal error rather than silently starting duplicate work.
- Index persistence stays best-effort as today: search results remain available even if saving the cache fails.
- Failure to read macOS memory data yields missing metrics, never a server failure.

## Verification

- A concurrent index-build test proves the transcript reader runs only once per stale slot and both callers receive the same cached index.
- A concurrent-save test proves independent writers do not collide on one temporary filename and the resulting index remains loadable.
- A semaphore test proves at most one expensive search body is active.
- macOS tests prove a live snapshot reports non-zero resident/virtual values and a parser/helper test covers unit conversion where applicable.
- Existing focused tests, formatting, clippy/guardrails, a release build, and an isolated multi-search memory benchmark must pass before installation.

## Scope boundary and rollback

This repair does not change result scoring, transcript formats, default search coverage, or general batch concurrency. The current v0.65.0 binary remains in its immutable version directory; activation changes only the `current` channel symlink/binary and can be rolled back to the existing version.
