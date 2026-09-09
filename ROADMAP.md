# OpetS Roadmap

This document is the single source of truth for planned work. It contains only
open work or work that needs an explicit product decision. Release notes,
feature specifications, user guides, and benchmarks remain the source of truth
for delivered behavior.

## How to use this roadmap

- **Now** items are confirmed technical gaps and should be addressed before
  optional product expansion.
- **Next** items are approved improvements whose implementation can be planned
  independently.
- **Later / decision required** items need a product, operational, or budget
  decision before implementation begins.
- An item is removed when its acceptance criteria are met and its relevant
  specification, guide, benchmark, or release note is updated.

## Now

### 1. Financial-report performance

**Problem:** the complete financial report took about 66.61 seconds with
10,000 service orders. The returning-customer metric uses a correlated query
over `date(previous.created_at, 'localtime')`.

**Deliver:**

- Use `previous.created_date` in the returning-customer query.
- Add and migrate a composite index on
  `service_orders(customer_id, deleted_at, created_date)`.
- Benchmark the full report again and record the result.

**Done when:** the query plan uses the new index where appropriate, report
results remain equivalent, and the benchmark documents the new full-report
time.

### 2. Checklist-template item index

**Problem:** page queries retrieve `template_items` by `template_id` without an
index on that column.

**Deliver:** add a schema migration for `template_items(template_id)`, test it,
and rerun the template benchmark.

**Done when:** the index exists on upgraded and clean installations and the
benchmark is updated.

## Next

### 3. Safe, versioned schema migrations

**Deliver:**

- Introduce an explicit migration ledger (`PRAGMA user_version` or a migration
  table) and numbered migrations.
- Run each database-only migration in an `IMMEDIATE` transaction.
- Create and retain a verified recovery backup before the first pending
  migration, then validate integrity and foreign keys after migration.
- Add fixtures for every supported released schema and fault-injection tests
  for interruption, no disk space, and partial migration states.
- Use staging plus an application journal for migrations that combine database
  and filesystem changes.

### 4. PDF generation resilience

**Deliver:**

- Build document data before rendering and release the database connection
  before a browser process starts.
- Run rendering in an isolated blocking worker with a single-job queue.
- Enforce timeouts and clean up failed browser processes and abandoned preview
  data.
- Test browser absence, crash, timeout, and concurrent requests.
- Run a real PDF-generation check in Windows and Linux CI, and capture timing,
  memory, and child-process metrics.

### 5. Checklist-template visual adjustment

**Blocked by:** a screenshot identifying the target border (drawer exterior,
scroll area, or checklist rows).

**Deliver after clarification:** make only the specified border less prominent
inside `ChecklistTemplateSheet`; validate light/dark themes, keyboard focus,
desktop, and mobile without changing the shared sheet component.

### 6. User-facing resilience

**Deliver:** add a global React error boundary with a recovery action; continue
normalizing unexpected errors through the shared user-friendly error utility;
and add smoke coverage that every route renders without console errors.

## Later / product decision required

### 7. Code signing for Windows distribution

Choose a signing provider (Azure Trusted Signing, OV/EV certificate, or
SignPath), then sign the executable, installer, and uninstaller with a trusted
timestamp. Add `Get-AuthenticodeSignature` validation to CI and block releases
with an unexpected publisher or invalid signature. This requires a budget and
signing credentials. The existing updater signature remains separate.

### 8. Encryption-key rotation

When a v2 application key is introduced, retain v1 read support, create a
recoverable v1 backup, migrate through validated staging, atomically activate
v2, and retain v1 backup-import compatibility. Test successful import,
password-protected import, unsupported keys, corruption, and interruption.

### 9. Search at larger scale

Current substring search (`LIKE '%term%'`) and deep `LIMIT/OFFSET` pages are
acceptable for the measured dataset. If growth makes them slow, evaluate FTS
for search and keyset pagination for deep navigation based on a new benchmark.

### 10. Optional product enhancements

These are not committed scope and require a product decision and acceptance
criteria before implementation:

- customer digital signatures stored with finalized service orders;
- global search across entities;
- inventory reorder recommendations and CSV import/export;
- service-order cloning/repetition and payment status tracking;
- customer contact shortcuts and notes.

## Completed work intentionally not tracked here

The following are delivered and should not return to the active backlog unless
a regression is found: encrypted SQLCipher storage and attachments, encrypted
backup/restore and legacy migration, automatic backups, LAN Host/Client mode,
WAL concurrency validation, paginated list pages and service-order filters,
stock movements linked to service orders, item-identity financial aggregation,
updater availability notices, the Windows backup-restore fix, and remote
customer and employee lookups during service-order creation.
