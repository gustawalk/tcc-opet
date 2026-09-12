# Versioned Migrations and Performance Indexes Tasks

**Design**: `.specs/features/versioned-migrations-performance/design.md`
**Status**: Draft

## Test Coverage Matrix

> Generated from `AGENTS.md`, existing inline Rust tests, and the specification.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| --- | --- | --- | --- | --- |
| Migration runner and schema | unit/integration | Every migration AC, failure rollback, version boundaries, fresh and upgraded schema paths | inline `#[cfg(test)]` module in `src-tauri/src/database.rs` | `cd src-tauri && cargo test --lib database::tests -- --test-threads=1` |
| Backup restore | integration | Legacy fixture, rejected schema, staged validation before activation | inline tests in `src-tauri/src/backup_service.rs` | `cd src-tauri && cargo test --lib backup_service::tests -- --test-threads=1` |
| Repository queries | unit/integration | Report semantic equivalence plus `EXPLAIN QUERY PLAN`; single and multi-template lookup plan | inline tests in affected repository modules | `cd src-tauri && cargo test --lib repositories:: -- --test-threads=1` |
| Benchmark and documentation | ignored benchmark/manual evidence | Named measurements, command, dataset, and recorded output | `src-tauri/src/performance_benchmarks.rs`, `docs/performance/optimization-benchmark.md` | `cd src-tauri && cargo test --release performance_benchmarks -- --ignored --nocapture --test-threads=1` |

## Gate Check Commands

| Gate Level | When to Use | Command |
| --- | --- | --- |
| Targeted | Discrimination sensor, one mutation | `cd src-tauri && cargo test --lib database::tests -- --test-threads=1` |
| Quick | Repository or migration task | `cd src-tauri && cargo test --lib -- --test-threads=1` |
| Full | Backup or migration integration task | `cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib -- --test-threads=1` |
| Build | Final backend handoff | `yarn typecheck && yarn lint && yarn test --pool=forks --maxWorkers=4 --minWorkers=1 && yarn build && cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib -- --test-threads=1` |

## Execution Plan

### Phase 1: Migration foundation

```
T1 → T2 → T3
```

### Phase 2: Measured query improvements

```
T4 → T5 → T6
```

### Phase 3: Recovery and compatibility evidence

```
T7 → T8
```

### Phase 4: Benchmark evidence

```
T9
```

## Task Breakdown

### T1: Define the version ledger and v0.4.0 baseline

**Status**: Complete

**What**: Refactor schema startup so a fresh database reaches version 1 and an
unsupported future version fails before mutation.
**Where**: `src-tauri/src/database.rs`
**Depends on**: None
**Reuses**: `run_schema_migrations`, inline migration tests
**Requirement**: VMP-01
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Fresh schema initialization sets `user_version = 1`.
- [ ] Version above the supported target returns an error without schema change.
- [ ] Inline tests cover fresh baseline and future-version rejection.

**Tests**: unit/integration
**Gate**: quick
**Commit**: `feat(migrations): establish versioned schema baseline`

### T2: Adapt accepted legacy schemas to the baseline

**Status**: Complete

**What**: Isolate the existing version-zero compatibility transforms and mark
successful legacy adaptation as version 1.
**Where**: `src-tauri/src/database.rs`
**Depends on**: T1
**Reuses**: `add_column_if_missing`, legacy user and money migration fixtures
**Requirement**: VMP-07, VMP-09
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Existing accepted legacy schema fixtures upgrade to version 1.
- [ ] Re-running the adapter does not change data or version.
- [ ] Inline tests preserve legacy business records and integrity checks.

**Tests**: unit/integration
**Gate**: full
**Commit**: `feat(migrations): adapt legacy schemas to baseline`

### T3: Add version-2 performance indexes

**Status**: Complete

**What**: Implement numbered migration version 2 with both required indexes in
one `IMMEDIATE` transaction and update the version only on commit.
**Where**: `src-tauri/src/database.rs`
**Depends on**: T2
**Reuses**: existing index creation and migration-idempotency tests
**Requirement**: VMP-03, VMP-10, VMP-11
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Upgrading a version-1 database creates both named indexes and reaches version 2.
- [ ] Fresh databases reach version 2 through the same migration sequence.
- [ ] Retry tests show no duplicate index or version change.

**Tests**: unit/integration
**Gate**: full
**Commit**: `feat(migrations): add performance index migration`

### T4: Make returning-customer lookup indexable

**Status**: Complete

**What**: Change the prior-order predicate to compare `created_date` while
preserving report semantics and testing the composite-index query plan.
**Where**: `src-tauri/src/repositories/financial_report_repo.rs`
**Depends on**: T3
**Reuses**: existing financial report fixture tests
**Requirement**: VMP-12, VMP-13, VMP-14
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Existing returning/new/deleted/out-of-period fixture results remain unchanged.
- [ ] Query-plan test proves the prior-order lookup uses `idx_service_orders_customer_created`.
- [ ] No report IPC or serialized contract changes.

**Tests**: unit/integration
**Gate**: quick
**Commit**: `perf(reports): index returning customer lookup`

### T5: Prove indexed template-item lookup

**Status**: Complete

**What**: Add a repository-level plan assertion for loading items by one and
multiple template IDs.
**Where**: `src-tauri/src/repositories/checklist_repo.rs`
**Depends on**: T4
**Reuses**: `get_page_with_conn` and existing checklist fixtures
**Requirement**: VMP-15
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Single and multi-template item retrieval returns the same grouped items.
- [ ] The query plan uses `idx_template_items_template`.

**Tests**: unit/integration
**Gate**: quick
**Commit**: `test(checklists): prove template item index use`

### T6: Verify migration rollback and post-migration validation

**Status**: Complete

**What**: Add deterministic migration failpoint coverage for rollback, version
retention, integrity, and foreign-key validation.
**Where**: `src-tauri/src/database.rs`
**Depends on**: T5
**Reuses**: existing migration tests and `PRAGMA` checks
**Requirement**: VMP-04, VMP-05, VMP-06
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] A test-only failure inside version 2 leaves schema and version at 1.
- [ ] Successful migration validates integrity and foreign keys.
- [ ] A no-op startup creates no new recovery state.

**Tests**: unit/integration
**Gate**: full
**Commit**: `test(migrations): cover rollback and validation`

### T7: Create validated recovery backups for on-disk migration

**Status**: Complete

**What**: Wire migration context into local startup so the first pending
on-disk migration produces a retained, validated encrypted recovery backup.
**Where**: `src-tauri/src/database.rs`
**Depends on**: T6
**Reuses**: `export_backup_with_passphrase`, backup validation, storage paths
**Requirement**: VMP-02, VMP-04
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Pending on-disk migration creates exactly one validated recovery archive before mutation.
- [ ] Backup creation/validation failure aborts without changing the database version.
- [ ] In-memory migration callers remain independent of filesystem backup setup.

**Tests**: unit/integration
**Gate**: full
**Commit**: `feat(migrations): back up before on-disk upgrades`

### T8: Preserve legacy backup restoration through versioned migration

**What**: Exercise staged restore of an accepted legacy backup through baseline
adaptation, current migration, and validation before activation.
**Where**: `src-tauri/src/backup_service.rs`
**Depends on**: T7
**Reuses**: `restore_backup_with_paths`, existing legacy restore tests
**Requirement**: VMP-08, edge case: staged restore
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Legacy backup fixture restores its records to version 2.
- [ ] Invalid backup remains rejected before active storage replacement.
- [ ] Tests prove integrity and foreign-key checks occur before activation.

**Tests**: integration
**Gate**: full
**Commit**: `test(backups): cover versioned legacy restore`

### T9: Measure and document the completed optimization

**What**: Add complete-report and paginated-template measurements, run the
ignored release benchmark, and record reproducible results and plans.
**Where**: `src-tauri/src/performance_benchmarks.rs`
**Depends on**: T8
**Reuses**: existing 10,000-record benchmark and performance report
**Requirement**: VMP-16, VMP-17
**Tools**: MCP: NONE; Skill: NONE
**Done when**:

- [ ] Benchmark prints named report and template measurements plus query-plan evidence.
- [ ] Performance documentation records the executed command, environment, and measured output.
- [ ] No benchmark runs in the default test suite.

**Tests**: ignored benchmark/manual evidence
**Gate**: build
**Commit**: `docs(performance): record migration index benchmarks`

## Phase Execution Map

```
T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9
```

## Diagram-Definition Cross-Check

| Task | Depends on | Diagram shows | Status |
| --- | --- | --- | --- |
| T1 | None | phase start | Match |
| T2 | T1 | T1 → T2 | Match |
| T3 | T2 | T2 → T3 | Match |
| T4 | T3 | T3 → T4 | Match |
| T5 | T4 | T4 → T5 | Match |
| T6 | T5 | T5 → T6 | Match |
| T7 | T6 | T6 → T7 | Match |
| T8 | T7 | T7 → T8 | Match |
| T9 | T8 | T8 → T9 | Match |

## Test Co-location Validation

| Task | Code layer | Matrix requires | Task says | Status |
| --- | --- | --- | --- | --- |
| T1 | Migration runner | unit/integration | unit/integration | OK |
| T2 | Legacy adapter | unit/integration | unit/integration | OK |
| T3 | Index migration | unit/integration | unit/integration | OK |
| T4 | Report repository | unit/integration | unit/integration | OK |
| T5 | Checklist repository | unit/integration | unit/integration | OK |
| T6 | Migration runner | unit/integration | unit/integration | OK |
| T7 | On-disk migration | unit/integration | unit/integration | OK |
| T8 | Backup restore | integration | integration | OK |
| T9 | Benchmark/documentation | ignored benchmark/manual evidence | ignored benchmark/manual evidence | OK |
