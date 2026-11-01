# Global Search Benchmark

**Executed:** 13 September 2026  
**Build:** Rust release profile; temporary SQLCipher database and throwaway key only.

## Reproduction

```bash
OPETS_DATA_KEY_V1=<temporary-64-character-hex-key> \
  cargo test --release global_search_benchmark \
  -- --ignored --nocapture --test-threads=1
```

The ignored benchmark is defined in `src-tauri/src/performance_benchmarks.rs`.
It seeds 10,000 customers, 10,000 inventory items, 10,000 checklist templates,
10,000 service orders, and 30,000 template items. It never opens a user
database.

## Workload

The benchmark uses the production global-search term `"00001"`. It runs the
same four paginated repository paths called by the modal:

- customers;
- service orders;
- inventory;
- checklist templates.

Each path uses a limit of five results, executes its production count query,
and serializes the resulting `Page<T>` payload. The term returned matches in
all four groups: 1,111 customers, 1,111 service orders, one inventory item,
and one checklist template.

The combined measurement represents the database and Rust serialization work
for the four requests. It excludes the fixed 300 ms frontend debounce,
WebView rendering, and IPC transport overhead. The application has one shared
SQLCipher connection, so the four database operations contend for that
connection even though the frontend starts them together.

## Results

| Measurement | Median | p95 | Min | Max |
| --- | ---: | ---: | ---: | ---: |
| Combined global search | 51.53 ms | 55.28 ms | 49.59 ms | 55.28 ms |

## Interpretation

At 10,000 records per primary entity, the combined database work remains below
the 100 ms p95 target used for paginated lists, and this measurement excludes
IPC and rendering. The current SQL `LIKE '%term%'` approach is acceptable for
the present dataset but should be remeasured after meaningful data growth or
if users report delayed search results. FTS evaluation remains conditional on
that evidence.

## Search responsiveness review — 1 November 2026

The release benchmark was rerun successfully with the same dataset, query,
five-record limit, three warmups, and 15 measured iterations:

| Measurement | Median | p95 | Min | Max |
| --- | ---: | ---: | ---: | ---: |
| Combined global search (rerun) | 49.91 ms | 55.67 ms | 47.59 ms | 55.67 ms |

This is close to the historical result, with no obvious database-work
regression in this workload. It excludes IPC, the 300 ms debounce, and
rendering, and does not establish end-to-end responsiveness on the user's
database. The benchmark uses a temporary database, not user data.

- `src/components/shared/GlobalSearch.tsx` retains the 300 ms debounce and
  starts four paginated requests with a five-record limit. Category changes
  filter those results locally, without another request. A regression test in
  `GlobalSearch.test.tsx` checks the batch size, payload limits, and tab reuse.
- Tab badges use the existing `Page.total` values; displaying counts adds no
  database queries. One shared results-area loading indicator replaces five
  animated tab indicators. This is a UI simplification, not a measured speedup.
- The four page commands in `src-tauri/src/commands/{customer,service_order,
  inventory,checklist}_commands.rs` are synchronous `#[command]` functions.
  [Tauri documents](https://v2.tauri.app/develop/calling-rust/#async-commands)
  that synchronous commands execute on the main thread unless explicitly
  dispatched asynchronously. Database work or connection contention can
  therefore stall the native event loop. This is a code-level responsiveness
  risk, not proof of the cause of the reported stutter.
- The 300 ms pause is intentional search latency, not a frame-time measurement.
  Neither the backend benchmark nor component tests measure WebKitGTK frame
  times, input-to-paint latency, or Hyprland composition. Attributing the
  reported stutter to the compositor requires an interactive rendering trace
  and a comparable run outside Hyprland; no compositor settings were changed.
