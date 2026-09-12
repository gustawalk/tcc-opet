# Versioned Migrations and Performance Index Benchmark

**Executed:** 12 September 2026  
**Build:** Rust release profile; temporary SQLCipher database and throwaway key only.

## Reproduction

```bash
OPETS_DATA_KEY_V1=<temporary-64-character-hex-key> \
  cargo test --release migration_index_benchmark \
  -- --ignored --nocapture --test-threads=1
```

The ignored benchmark is defined in `src-tauri/src/performance_benchmarks.rs`.
It seeds 10,000 records in each primary entity, including 10,000 service
orders and 30,000 template items. It does not access a user database.

## Results

| Measurement | Median | p95 | Notes |
| --- | ---: | ---: | --- |
| Complete financial report | 872.5 ms | 872.5 ms | One measured complete report for the 10,000-order dataset. |
| Paginated template page | 1.17 ms | 2.76 ms | 20-item page; 3 warmups and 15 samples. |

## Index-plan evidence

```text
returning customer:
SEARCH previous USING COVERING INDEX idx_service_orders_customer_created
  (customer_id=? AND deleted_at=? AND created_date<?)

template items:
SEARCH template_items USING INDEX idx_template_items_template (template_id=?)
```

The report retains the existing financial-report contract while its
returning-customer lookup uses `created_date`, making the composite index
usable. The template-item lookup likewise uses its dedicated index.

## Interpretation

These are environment-specific measurements, not universal latency promises.
They replace the previously open report and template-index performance concern
with reproducible post-change evidence. The broader historical benchmark is
kept in `optimization-benchmark.md`.
