# Versioned Migrations and Performance Indexes Specification

## Problem Statement

The application currently evolves its SQLite schema through one cumulative,
ad-hoc routine. It has no durable schema version ledger, so the exact upgrade
path is hard to prove or recover. At the same time, the financial report's
returning-customer query and paginated checklist-template item lookup lack the
indexes needed for the measured 10,000-record workload.

## Goals

- [ ] Introduce a transactional, numbered migration ledger for all schema
  changes after the v0.4.0 baseline.
- [ ] Preserve restoration of older valid OpetS backups by adapting them to the
  v0.4.0 baseline before numbered migrations run.
- [ ] Add the reporting and template-item indexes through a numbered migration.
- [ ] Preserve financial-report results while making the returning-customer
  lookup indexable and recording fresh performance measurements.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Schema redesign or data-model changes unrelated to the two indexes | This initiative establishes migration infrastructure and fixes measured bottlenecks only. |
| New report metrics or UI changes | The financial report contract must remain unchanged. |
| Filesystem-changing migrations | They need a separate staged journal design and are not required for these database-only changes. |
| Direct upgrade fixtures older than v0.4.0 | Older data remains supported through backup import, not as a direct application-database upgrade guarantee. |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- |
| Migration ledger | Use `PRAGMA user_version`; version changes occur in the same `BEGIN IMMEDIATE` transaction as each migration. | SQLite-native, atomic, and requires no extra metadata table. | y |
| Baseline | Assign the current v0.4.0 schema version 1. | It creates a finite, testable starting point for future migrations. | y |
| Older backups | Validate and adapt every older backup format currently accepted by the restore path to version 1, then run numbered migrations. | Backup import must not regress when direct upgrade support starts at v0.4.0. | y |
| Recovery backup | Before the first pending on-disk migration, create and validate a retained encrypted recovery backup beside the database. In-memory test databases do not create a file backup. | An upgrade must be recoverable before it mutates user data. | y |
| Recovery failure | Abort before changing `user_version` or schema when the recovery backup cannot be created or validated. | A migration without recovery evidence is unsafe. | y |
| Disk-full simulation | Use a deterministic test-only migration failpoint, then verify transaction rollback and unchanged version. | Real disk exhaustion is not deterministic in unit tests. | y |
| Remaining dimensions | Authentication, rate limits, expiry, and external-service fallback are N/A. Ordering, retry, persistence, and failure behavior are covered below. | This is a local database migration feature. | y |

**Open questions:** none - all resolved or logged above.

---

## User Stories

### P1: Safely upgrade a v0.4.0 database

**User Story**: As an OpetS operator, I want database upgrades to be numbered,
atomic, and recoverable so that an application update does not silently damage
my records.

**Why P1**: Schema migration failure is a data-integrity risk.

**Acceptance Criteria**:

1. WHEN a fresh database is initialized THEN the system SHALL create the v0.4.0 baseline schema and set `PRAGMA user_version` to `1`.
2. WHEN a version-1 database starts with pending migrations THEN the system SHALL create and validate one encrypted recovery backup before its first schema mutation.
3. WHEN a pending database-only migration runs THEN the system SHALL execute its schema changes and `user_version` update inside one `BEGIN IMMEDIATE` transaction.
4. IF a numbered migration fails THEN the system SHALL roll back its schema changes, retain its prior `user_version`, and leave the recovery backup available.
5. WHEN all pending migrations finish THEN the system SHALL pass `PRAGMA integrity_check` and `PRAGMA foreign_key_check` before continuing startup.
6. WHEN no migration is pending THEN the system SHALL not create an additional recovery backup or modify the schema version.

**Independent Test**: Start from a version-1 encrypted database, inject a
failing migration, and verify the schema/version are unchanged; repeat without
the failpoint and verify the target version plus integrity checks.

---

### P1: Preserve historical backup import

**User Story**: As an OpetS operator, I want an older valid backup to restore
into the current application so that upgrades do not make my archived data
unusable.

**Why P1**: Backup compatibility is separate from the v0.4.0 direct-upgrade
support window and must remain intact.

**Acceptance Criteria**:

1. WHEN a backup without a schema version is accepted by the existing restore validation THEN the system SHALL adapt it to baseline version `1` before applying numbered migrations.
2. IF a backup does not satisfy the existing OpetS schema validation THEN the system SHALL reject it without activating it or changing active storage.
3. WHEN a legacy backup completes restore THEN the system SHALL retain its business records, pass integrity and foreign-key checks, and report the current schema version.

**Independent Test**: Restore a fixture that represents an accepted legacy
backup, then verify its records, current version, and validation checks.

---

### P1: Index measured report and template queries

**User Story**: As an OpetS operator, I want financial reports and paginated
checklist templates to remain responsive as records grow.

**Why P1**: These are the remaining measured database bottlenecks.

**Acceptance Criteria**:

1. WHEN migration version `2` is applied THEN the system SHALL create `idx_service_orders_customer_created` on `service_orders(customer_id, deleted_at, created_date)`.
2. WHEN migration version `2` is applied THEN the system SHALL create `idx_template_items_template` on `template_items(template_id)`.
3. WHEN the financial report counts returning customers THEN the system SHALL compare `previous.created_date` with the requested start date without applying a function to `previous.created_at`.
4. WHEN a report has returning, new, deleted, and out-of-period customer orders THEN the system SHALL preserve the existing returning-customer result.
5. WHEN the returning-customer query plan is inspected on indexed data THEN the system SHALL use `idx_service_orders_customer_created` for the prior-order lookup.
6. WHEN checklist template items are loaded by one or more template IDs THEN the system SHALL use `idx_template_items_template`.

**Independent Test**: Apply migration version 2 to a version-1 fixture, assert
both indexes and query plans, and compare the report result to the existing
expected fixture values.

---

### P2: Preserve measurable performance evidence

**User Story**: As a maintainer, I want reproducible benchmark evidence so that
future performance work is based on measurements rather than assumptions.

**Why P2**: The roadmap's completion criteria require a fresh report and
template benchmark.

**Acceptance Criteria**:

1. WHEN the ignored release benchmark runs against 10,000 primary records THEN the system SHALL print complete-financial-report and paginated-template measurements.
2. WHEN the measurements are captured THEN the performance document SHALL record the command, dataset, post-change results, and index-plan evidence.

**Independent Test**: Run the documented ignored release benchmark and compare
its printed labels with the performance document.

## Edge Cases

- IF a database claims a schema version greater than the application supports
  THEN the system SHALL stop before applying any migration.
- IF a migration is retried after a prior successful run THEN the system SHALL
  make no duplicate schema changes and retain the current version.
- WHEN a restore uses staging storage THEN the system SHALL migrate and validate
  the staged database before active storage is replaced.

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| VMP-01 | P1: Safe v0.4 upgrade | Execute (T1) | Verified |
| VMP-02 | P1: Safe v0.4 upgrade | Design | Pending |
| VMP-03 | P1: Safe v0.4 upgrade | Execute (T3) | Verified |
| VMP-04 | P1: Safe v0.4 upgrade | Execute (T6) | Verified |
| VMP-05 | P1: Safe v0.4 upgrade | Execute (T6) | Verified |
| VMP-06 | P1: Safe v0.4 upgrade | Execute (T6) | Verified |
| VMP-07 | P1: Historical backup import | Execute (T2) | Verified |
| VMP-08 | P1: Historical backup import | Design | Pending |
| VMP-09 | P1: Historical backup import | Execute (T2) | Verified |
| VMP-10 | P1: Query indexes | Execute (T3) | Verified |
| VMP-11 | P1: Query indexes | Execute (T3) | Verified |
| VMP-12 | P1: Query indexes | Execute (T4) | Verified |
| VMP-13 | P1: Query indexes | Execute (T4) | Verified |
| VMP-14 | P1: Query indexes | Execute (T4) | Verified |
| VMP-15 | P1: Query indexes | Execute (T5) | Verified |
| VMP-16 | P2: Performance evidence | Design | Pending |
| VMP-17 | P2: Performance evidence | Design | Pending |

**Coverage:** 17 total, 0 mapped to tasks, 17 unmapped pending design.

## Success Criteria

- [ ] All direct upgrades from the v0.4.0 baseline are versioned, atomic, and recoverable.
- [ ] Older currently accepted backups restore through a tested compatibility path.
- [ ] Financial and template indexes exist on fresh and upgraded databases.
- [ ] Report results are unchanged and benchmark evidence is updated.
