# Performance & Memory Safety Audit

## Objective

Perform a deep audit of the entire Rust codebase looking for:
1. Memory leaks (logical leaks, not UB — Rust prevents UB but not logical leaks)
2. Performance anti-patterns that degrade throughput or latency
3. Unnecessary allocations in hot paths
4. Async runtime misuse (blocking in async context, spawn overhead)
5. Data structure choices that cause cache misses or excessive copying

## Methodology

### Phase 1: Static Analysis

Run these checks and report findings:

```bash
# Performance-related clippy lints
cargo clippy --workspace --all-targets -- \
  -W clippy::perf \
  -W clippy::large_enum_variant \
  -W clippy::large_stack_frames \
  -W clippy::redundant_clone \
  -W clippy::unnecessary_to_owned \
  -W clippy::needless_collect \
  -W clippy::manual_memcpy \
  -W clippy::inefficient_to_string \
  -W clippy::format_collect \
  -W clippy::single_char_pattern \
  -W clippy::bytes_count_to_len \
  -W clippy::vec_init_then_push

# Check for large types on stack
cargo clippy -- -W clippy::large_types_passed_by_value
```

### Phase 2: Memory Leak Patterns

Search the codebase for these known Rust memory leak sources:

1. **Arc/Rc cycles** — Arc<Mutex<Vec<Arc<...>>>> patterns where child references parent
2. **Unbounded channels** — `mpsc::unbounded_channel()` without backpressure (memory grows forever under load)
3. **Unbounded Vec/VecDeque growth** — collections that push without ever clearing/rotating
4. **Forgotten futures** — `tokio::spawn` without joining or cancelling (task lives forever)
5. **Global state accumulation** — HashMaps/Vecs behind Mutex that only insert, never remove
6. **String formatting in loops** — `format!()` in hot loops allocates every iteration
7. **Clone in loops** — `.clone()` on large types inside iteration
8. **Box::leak** or `mem::forget` — explicit leaks (should be documented if intentional)

### Phase 3: Performance Anti-Patterns

Look for these in hot paths (telemetry collection, metrics stream, REST handlers):

| Anti-Pattern | What to look for | Fix |
|---|---|---|
| **Allocation in hot path** | `Vec::new()`, `String::new()`, `format!()` inside loops or per-request handlers | Pre-allocate, reuse buffers |
| **Excessive cloning** | `.clone()` on large structs (payloads, configs) | Use references, `Arc`, or `Cow` |
| **Lock contention** | `Mutex` held across `.await` points | Use `tokio::sync::Mutex` or restructure |
| **Blocking in async** | `std::fs::read`, `std::thread::sleep` in async context | Use `tokio::fs`, `tokio::time::sleep` |
| **Unnecessary boxing** | `Box<dyn Future>` where concrete types work | Use `impl Future` or generics |
| **String operations** | `to_string()`, `String::from()` where `&str` suffices | Pass references |
| **Collect then iterate** | `.collect::<Vec<_>>()` followed by `.iter()` | Chain iterators directly |
| **Large enum variants** | Enum where one variant is 10x larger than others | Box the large variant |
| **Repeated parsing** | Re-parsing config/env vars on every request | Parse once at startup, store result |
| **Synchronous DNS** | `TcpStream::connect("hostname:port")` blocks resolver | Pre-resolve or use async resolver |

### Phase 4: Async Runtime Audit

Specific to our tokio usage:

1. **current_thread vs multi_thread** — Agent uses `current_thread` (correct for single-core). Master uses `multi_thread` (correct). Verify no blocking calls.
2. **Spawn proliferation** — Each `tokio::spawn` has overhead (~300 bytes + scheduling). Are we spawning unnecessarily?
3. **Channel sizing** — `mpsc::channel(32)` — is 32 appropriate? Too small = backpressure stalls. Too large = memory waste.
4. **Timer precision** — `tokio::time::interval` vs `tokio::time::sleep` in loops. Interval drifts less.
5. **Select fairness** — `tokio::select!` is random by default. Critical paths should be prioritized.

### Phase 5: Data Structure Review

For each crate, verify optimal data structures:

| Use Case | Current | Better? |
|---|---|---|
| Metrics history buffer | `VecDeque<MetricsPayload>` | Ring buffer with fixed capacity |
| Agent lookup by ID | Linear search on Vec | `HashMap<String, Agent>` |
| Sparkline history | `Vec<u64>` with `remove(0)` | `VecDeque<u64>` (O(1) pop_front) |
| Config parsing | Parse on every access | Parse once, store structured |
| HTTP response building | `serde_json::json!()` macro | Pre-built response templates for common shapes |

### Phase 6: Benchmarking Targets

After identifying issues, create benchmarks for:

```bash
# Key hot paths to benchmark:
# 1. Agent: collect_snapshot() — 12 parallel collectors → MetricsPayload
# 2. Master: insert_metrics() — write to 11 tables per payload
# 3. Master: get_overview_data() — N+1 query pattern (1 per agent)
# 4. Agent: HTTP response parsing in updater (handwritten parser)
# 5. TUI: render cycle (how fast can we redraw)
```

## Output Format

For each finding, report:

```
## [SEVERITY: HIGH/MEDIUM/LOW] — Title

**File:** path/to/file.rs:line
**Pattern:** What was found
**Impact:** How it affects performance/memory
**Fix:** Specific code change recommended
**Effort:** Lines of code to fix
```

## Known Good Patterns (don't flag these)

- `unwrap_or_default()` on Option — intentional, no panic
- `clone()` on small types (String IDs, enums) — acceptable
- `Arc<RwLock<>>` for shared state — correct for our use case
- Single `unsafe` block (statvfs) — documented, necessary
- `VecDeque` with cap 100 for offline buffer — bounded, correct

## Tools to Suggest

If issues are found, recommend:
- `cargo flamegraph` — CPU profiling
- `cargo bloat` — binary size breakdown
- `DHAT` (via valgrind) — heap profiling
- `tokio-console` — async runtime introspection
- `criterion` — micro-benchmarks
- `tracing` spans — latency measurement per operation
