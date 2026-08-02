# Session Search Memory Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent concurrent session searches from multiplying transcript-index memory, eliminate cache temp-file collisions, and make macOS memory incidents observable.

**Architecture:** Coordinate index construction per cache path, serialize only the expensive session-search body with an async permit, persist indexes through unique same-directory temporary files, and populate non-Linux memory snapshots with native macOS APIs. Keep scoring, result formatting, transcript formats, and unrelated batch parallelism unchanged.

**Tech Stack:** Rust 2024, Tokio synchronization, tempfile, libc/Mach APIs, Cargo tests and guardrails.

---

### Task 1: Single-flight index construction

**Files:**
- Modify: `crates/jcode-app-core/src/tool/session_search_index.rs`
- Test: `crates/jcode-app-core/src/tool/session_search_index.rs`

- [ ] **Step 1: Write the failing concurrency test**

Add a test that starts two scoped threads behind a barrier, calls `build_or_update` with the same unique index path/spec, counts `read_text` calls, and asserts one read plus `Arc::ptr_eq` for both returned indexes.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p jcode-app-core --lib session_search_index::tests::concurrent_builds_share_one_rebuild -- --exact`

Expected: FAIL because both callers read and tokenize the stale slot and return different `Arc`s.

- [ ] **Step 3: Implement per-path coordination**

Add a `OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>`. Resolve the path lock without holding the map during work, acquire it at the beginning of `build_or_update`, and perform the existing cache check only after acquisition. Convert poisoned locks into `anyhow` errors.

- [ ] **Step 4: Run focused index tests and verify GREEN**

Run: `cargo test -p jcode-app-core --lib session_search_index::tests`

Expected: all index tests PASS, including one read and one shared `Arc` in the new concurrent test.

- [ ] **Step 5: Commit the single-flight fix**

Commit: `fix: single-flight session search index builds`

### Task 2: Collision-proof index persistence

**Files:**
- Modify: `crates/jcode-app-core/src/tool/session_search_index.rs`
- Test: `crates/jcode-app-core/src/tool/session_search_index.rs`

- [ ] **Step 1: Write the failing temporary-path test**

Add a test for a new `create_index_temp_file` helper. Two calls for the same final path must create different temporary paths in the same parent directory.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p jcode-app-core --lib session_search_index::tests::index_writes_use_unique_same_directory_temp_files -- --exact`

Expected: compilation FAIL because `create_index_temp_file` does not exist.

- [ ] **Step 3: Implement atomic unique writes**

Create parent directories, use `tempfile::NamedTempFile::new_in(parent)`, write with `std::io::Write::write_all`, and atomically `persist(path)`. Keep the named temp object alive until persistence so failures clean themselves up.

- [ ] **Step 4: Run focused index tests and verify GREEN**

Run: `cargo test -p jcode-app-core --lib session_search_index::tests`

Expected: all index tests PASS and the persisted index round-trip remains loadable.

- [ ] **Step 5: Commit the persistence fix**

Commit: `fix: isolate session index cache writes`

### Task 3: Bound heavy search concurrency

**Files:**
- Modify: `crates/jcode-app-core/src/tool/session_search.rs`
- Test: `crates/jcode-app-core/src/tool/session_search_tests.rs`

- [ ] **Step 1: Write the failing permit test**

Add an async test that acquires the first session-search permit, starts a second acquisition, proves the second remains pending, releases the first permit, and proves the second then completes.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p jcode-app-core --lib session_search_tests::session_search_permit_serializes_expensive_searches -- --exact`

Expected: compilation FAIL because the permit helper does not exist.

- [ ] **Step 3: Implement the process-wide gate**

Add a process-wide Tokio semaphore with one permit and an async acquisition helper. Acquire the permit after input/path validation and immediately before `spawn_blocking`; retain it until the blocking search completes. Map a closed semaphore to an `anyhow` error.

- [ ] **Step 4: Run focused session-search tests and verify GREEN**

Run: `cargo test -p jcode-app-core --lib session_search_tests`

Expected: all session-search tests PASS; unrelated batch implementation remains unchanged.

- [ ] **Step 5: Commit the concurrency bound**

Commit: `fix: serialize memory-heavy session searches`

### Task 4: Report macOS process memory

**Files:**
- Modify: `crates/jcode-base/src/process_memory.rs`
- Test: `crates/jcode-base/src/process_memory.rs`

- [ ] **Step 1: Write the failing macOS snapshot test**

Under `#[cfg(target_os = "macos")]`, call `snapshot_with_source` and assert resident bytes, peak resident bytes, and virtual bytes are present and non-zero.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p jcode-base process_memory::tests::macos_snapshot_reports_process_sizes -- --exact`

Expected: FAIL because the current non-Linux implementation returns a default snapshot with `None` metrics.

- [ ] **Step 3: Implement native macOS collection**

Call `libc::task_info` with `MACH_TASK_BASIC_INFO` for current resident, resident max, and virtual bytes. Use `getrusage(RUSAGE_SELF)` as a peak fallback, preserve unsupported fields as `None`, include allocator information, record the snapshot, and log native-call failures without failing Jcode.

- [ ] **Step 4: Run focused memory tests and verify GREEN**

Run: `cargo test -p jcode-base process_memory::tests`

Expected: all process-memory tests PASS and the live macOS snapshot has non-zero values.

- [ ] **Step 5: Commit telemetry repair**

Commit: `fix: report process memory on macOS`

### Task 5: Integrated verification and safe activation

**Files:**
- Verify: all task-touched files
- Install target: `~/.jcode/builds/current/jcode`
- Rollback source: `~/.jcode/builds/versions/0.65.0/jcode` or the verified existing v0.65.0 immutable binary

- [ ] **Step 1: Run formatting and focused workspace checks**

Run: `cargo fmt --all -- --check`

Run: `cargo test -p jcode-app-core --lib session_search`

Run: `cargo test -p jcode-base process_memory`

Run: `scripts/check_guardrails.sh`

Expected: every command exits 0 with no new warnings.

- [ ] **Step 2: Build the release binary**

Run the repository's documented local/selfdev release build path without publishing it to the active channel first. Confirm the binary reports the intended source revision.

- [ ] **Step 3: Run an isolated concurrency benchmark**

Use a temporary Jcode home with representative large JSONL fixtures. Invoke three simultaneous searches against the patched binary and sample peak physical footprint. Gate acceptance on one index rebuild, zero cache-save errors, and a peak far below the observed 4.21 GB multiplication.

- [ ] **Step 4: Review the final diff and preserve rollback**

Run: `git diff v0.65.0...HEAD --check`

Run: `git status --short --branch`

Confirm the original immutable v0.65.0 binary still exists and record its exact rollback path before activation.

- [ ] **Step 5: Publish and activate**

Push `codex/session-search-memory-v0.65.0` to `trevor-commits/jcode`. Install the verified build through Jcode's documented selfdev/current-channel path. Restart or reload the shared server only after preserving the active session and verifying rollback.
