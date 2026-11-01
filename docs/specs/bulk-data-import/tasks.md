# Bulk import tasks

## Execution Plan

Sequential tasks T1 through T6, one local commit per verified deliverable.

## Task Breakdown

### T1: Add bounded CSV/XLSX parsing and native commands

- **Status**: complete
**Depends on**: none
**Tests**: parser fixtures and limits
**Gate**: cargo test --lib import_file -- --test-threads=1
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

### T2: Add authoritative normalization and row validation

- **Status**: complete
**Depends on**: T1
**Tests**: required fields, locale money, dates, uniqueness and limits
**Gate**: cargo test --lib import_service -- --test-threads=1
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

### T3: Add receipt migration and transactional persistence

- **Status**: complete
**Depends on**: T2
**Tests**: rollback, stock movement, replay and 5,000 rows
**Gate**: cargo test --lib import -- --test-threads=1
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

### T4: Expose validation/commit through LAN

- **Status**: complete
**Depends on**: T3
**Tests**: authenticated Host, replay, unavailable Host and file-command rejection
**Gate**: cargo test --lib lan_api -- --test-threads=1
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

### T5: Add shared editable wizard and page entry points

- **Status**: complete
**Depends on**: T4
**Tests**: mapping, edit/exclude, confirmation, frozen retries, cache invalidation
**Gate**: yarn typecheck; yarn lint; yarn test; yarn build
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

### T6: Complete user guide and independent verification

- **Status**: complete
**Depends on**: T5
**Tests**: spec coverage and isolated mutations
**Gate**: full
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

## Test Coverage Matrix
| Requirements | Tests |
| --- | --- |
| IMP-01,02,04 | import_file fixtures and bounds |
| IMP-06,07 | import_service normalization/validation |
| IMP-09,10,11,12 | transactional persistence and migration tests |
| IMP-13,15 | LAN and frozen retry tests |
| IMP-03,05,08,14 | wizard behavior tests |

## Gate Check Commands
Frontend: yarn typecheck; yarn lint; yarn test; yarn build.
Backend: cargo fmt --all -- --check; cargo clippy --all-targets --all-features -- -D warnings; cargo test --lib -- --test-threads=1 (cwd src-tauri).
Verification: full (data integrity and migrations). Native interactive UAT requires a desktop session.

## T1 evidence
Parser gate: 5 passed. Evidence: import_file.rs tests assert exact cell contents, row numbers, dates and bounds. No assertions weakened or skipped.

## T2 evidence
Validation gate: 4 passed. Exact cents, normalized dates, required fields and all email conflict rows asserted in import_service.rs. No tests weakened or skipped.

## T3 evidence
Import gate: 17 passed. Exact record/movement/cost assertions, injected insertion rollback, employee replay, concurrent identical requests and 5,000-row persistence pass. Fresh/version-5 migration and failed upgrade version preservation pass.

## T4 evidence
LAN gate: 7 passed with loopback access; data-client gate: 11 passed. Authenticated import, receipt replay, local-file rejection and route-specific payload limits asserted. Sandbox socket restrictions required running LAN tests with approved escalation.

## T5 evidence
Frontend gates: 143 tests passed, typecheck/lint/build passed. Wizard tests cover all three entities, 5,000-row pagination, edits, filter, exclude/restore, discard confirmation, conflicts, pending/double submission, Client routing and frozen retry. Mapping tests assert actual mapped values and numeric provenance.

## Verification findings resolved before independent review
- Legacy SQLCipher export loses user_version: receipt migration now tolerates the preserved valid table; version-loss and failure rollback fixtures pass. The legacy backup contract now expects version 6.
- CSV now rejects misplaced/unclosed quotes and XLSX checks formula metadata including derived shared formulas.
- Native IPC test proves indexed rows, numeric provenance, camelCase validation/commit and replay against the registered command surface.
- Discard confirmation reuses the same dialog to avoid competing focus traps. Host preview validation has an explicit retry action. Parts/services show enforced stock control read-only.
- Required gates passed: 143 frontend tests; 240 Rust tests with 3 existing performance benchmarks and 1 existing real-PDF test ignored; typecheck, lint, build, fmt and clippy.
- Visible Chromium harness passed all three entities for mapping, paginated edits, keyboard confirmation/cancel focus, commit, light/dark themes and 900x700/1440x1000 layouts. Native dialogs/IPC were mocked for browser checks; Rust IPC/HTTPS tests exercised actual handlers.

## Independent verification correction
Coverage verifier identified IMP-07: typed ISO date-only XLSX cells were rejected by datetime conversion. A new fixture failed against the old conversion, then passed after switching to as_date(). It also asserts real XLSX numeric provenance and leading-zero warnings. Independent coverage re-review accepts all 15 requirements. Eight isolated data-integrity mutations were killed; no sensor survivors.

## T6 evidence
Full gates after the independent correction: 143 frontend tests; 241 Rust tests passed, 4 pre-existing ignored; typecheck, lint, build, fmt and clippy pass. All 15 requirements accepted by independent coverage re-review; 8/8 valid isolated mutations killed. Browser checks pass for three entities, both themes, compact/desktop layouts and keyboard discard focus. Validation report contains evidence and limitations; manual native desktop UAT was not performed.

### T7: Refine file selection, sample preview and shared dropdowns

- **Status**: complete
**Depends on**: T6
**Tests**: initial single action, limits only on file step, file summary, active stage, app dropdown selection/Escape/focus, worksheet replacement, existing mapping/review/retry regressions, pagination callbacks.
**Gate**: yarn typecheck; yarn lint; yarn test; yarn build; visible Chromium light/dark/compact checks.
- **Done when**: IMP-16 through IMP-19 pass, existing frontend regressions pass, and the reviewed screens match the requested app styling.

Verification: standard (UI behavior and controls; no backend/data path changes). Scope includes the shared pagination selector because it is rendered by the importer. Assertions for pagination values and callbacks remain intact; test interactions now use the app dropdown rather than native selectOptions.

## T7 evidence
Frontend gates pass: typecheck, lint, 145 tests and production build. The new initial-state test asserts a single action, active stage and contextual limits; the dropdown test asserts custom trigger, selected column, Escape dismissal and focus restoration. Existing worksheet and page-size callback tests retain their value assertions using app Select interactions. Visible Chromium passes all three entity flows in light/dark/compact layouts; a separate eight-column check measures horizontal overflow rather than compressed cells. Two isolated UI mutations (wrong active stage and wrong mapped column index) fail exact assertions; real-tree porcelain is unchanged by the sensor. Validation details are appended to validation.md.
