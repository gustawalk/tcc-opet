# OpetS Agent Guide

## Purpose

OpetS is a desktop-first service-order management application built with Tauri
v2, React, TypeScript, Rust, SQLCipher, and SQLite. The application supports
local storage and LAN Host/Client operation.

This file contains durable working rules for contributors and coding agents.
It must not be used as a session log, release note, benchmark report, or
feature backlog.

## Sources of truth

- `ROADMAP.md` is the single source of truth for open work and its priority.
- `docs/specs/` contains feature specifications and implementation evidence.
- `docs/guides/` contains operational and user-facing procedures.
- `docs/performance/optimization-benchmark.md` contains measured performance
  results and methodology.
- `docs/releases/` contains released behavior. Do not re-add delivered work to
  the active roadmap unless there is a verified regression.

Update the relevant source document when a change modifies its behavior,
contract, benchmark result, or operational procedure.

## Current work order

Follow this order unless the user explicitly changes the priority:

1. Returning-customer query and full financial-report performance.
2. Index `template_items(template_id)` and remeasure template pagination.
3. Safe, versioned schema migrations.
4. PDF generation resilience.
5. Checklist-template visual adjustment after the target is identified.
6. Global error boundary and route smoke coverage.
7. Windows code signing, after a provider, budget, and credentials are chosen.
8. Encryption-key rotation, when a v2 key is introduced.
9. FTS/keyset-pagination evaluation only if a new benchmark demonstrates a
    scaling problem.
10. Optional product enhancements only after product scope and acceptance
    criteria are approved.

Read `ROADMAP.md` before starting any roadmap item; it defines its problem,
deliverables, blockers, and completion criteria.

## Application contracts

- IPC payloads serialize in camelCase.
- `Page<T>` serializes as `{ items, total }`.
- New list pages must use backend pagination and a 300 ms debounced search;
  never fetch an entire entity list merely to render a list page.
- Use query keys that include page, page size, search, and active filters.
  Reset the page when any of those inputs changes, and clamp it after a total
  changes.
- Mutations must invalidate the corresponding paginated query-key prefix.
- `get_inventory_summary` is the source for inventory summary cards; do not
  fetch the full inventory list for those cards.
- Business rules and financial calculations belong in Rust, not only in the
  frontend.
- Critical database mutations must remain transactional. Stock changes must
  produce inventory movements linked to their service order where applicable.
- Preserve SQLCipher encryption, attachment encryption, backup validation, and
  storage-mode boundaries. A LAN Client must not silently fall back to a local
  database when the Host is unavailable.

## Database and performance rules

- Keep shared-connection work short. Load required data before slow work such
  as browser-based PDF rendering, backups, or expensive reports.
- Do not change SQLite journaling, locking, or LAN storage behavior without
  targeted tests and updated evidence. Network filesystems are not a substitute
  for the Host/Client LAN architecture.
- Add schema changes through the established migration path and cover both new
  and upgraded installations.
- Benchmark query or pagination changes when they address a documented
  performance issue. Record reproducible measurements in the performance
  document.

## Implementation expectations

- Match existing React, TypeScript, Rust, and Tauri conventions before adding
  abstractions.
- Use the shared error utilities and accessible UI primitives.
- Preserve keyboard behavior, loading states, and user-friendly Portuguese
  messages in user-facing flows.
- Add focused regression tests for changed behavior. Do not treat historical
  planning documents as evidence that a feature is still missing.

## Required verification

Run the checks relevant to the change before handoff. For cross-stack changes,
run all of the following:

```bash
yarn typecheck
yarn lint
yarn test
yarn build

cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib -- --test-threads=1
```

Run ignored benchmarks or stress tests only when the changed work requires
their evidence. State explicitly when an environment-specific check cannot run.

## Documentation hygiene

- Keep this file concise and durable. Do not append completed-session notes,
  test counts, temporary paths, or release-specific facts.
- Keep `ROADMAP.md` in English and remove or mark completed work when its
  acceptance criteria are met.
- Keep release notes and user guides accurate, but do not use them as a second
  active backlog.
- Before creating a new planning Markdown file, check whether its content
  belongs in `ROADMAP.md`, a feature specification, or a guide instead.
