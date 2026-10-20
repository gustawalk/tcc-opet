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

- **Status**: pending
**Depends on**: T4
**Tests**: mapping, edit/exclude, confirmation, frozen retries, cache invalidation
**Gate**: yarn typecheck; yarn lint; yarn test; yarn build
- **Done when**: Spec-defined outcomes are asserted and the task gate passes.

### T6: Complete user guide and independent verification

- **Status**: pending
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
