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
