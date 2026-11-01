# Bulk Data Import Validation

## Validation: PASS

Date: 2026-10-20. Verification tier: full (data integrity, business rules, LAN contract, migrations).
Implementation range: `main..04f0d23` on `feat/bulk-data-import`.
Independent roles: `import_coverage` reviewed requirements/code/tests without running or changing implementation; `import_sensor` ran isolated behavioral mutations. Coverage initially found one IMP-07 gap; an incremental independent re-review accepted its correction and retained the other fourteen results.

## Acceptance evidence

| Requirement | Spec-defined outcome and evidence | Result |
| --- | --- | --- |
| IMP-01 | Local file parsing exposes indexed columns and physical source rows without business writes. `src-tauri/src/import_file.rs:351` asserts exact headers, cell values and source lines; `src-tauri/src/tauri_ipc_tests.rs:604` exercises registered parsing commands. | PASS |
| IMP-02 | UTF-8/BOM, comma/semicolon/tab and one selected XLSX sheet. `src-tauri/src/import_file.rs:366` asserts delimiters and encoding errors; `src-tauri/src/import_file.rs:463` asserts the selected sheet's values; `src/components/shared/import/ImportWizard.test.tsx:347` checks worksheet selection. | PASS |
| IMP-03 | Indexed source columns, fixed values, ignored unsupported fields and unambiguous bilingual suggestions. `src/lib/data-import.test.ts:6` asserts ambiguous name mapping remains unselected; `src/lib/data-import.test.ts:25` asserts complete mapped values and provenance. | PASS |
| IMP-04 | File/expanded ZIP/row/column/cell/request bounds and malformed input rejection. `src-tauri/src/import_file.rs:392` asserts 5,000 rows accepted and overflow rejected; `src-tauri/src/import_file.rs:483` tests XLSX expansion/corruption/column bounds; `src-tauri/src/import_file.rs:507` tests malformed quotes; `src-tauri/src/lan_api.rs:1799` asserts oversized import payload rejection. | PASS |
| IMP-05 | Editing, source numbers, errors-only filter and reversible exclusions persist across pages. `src/components/shared/import/ImportWizard.test.tsx:163` checks corrections/exclusion/restoration; `src/components/shared/import/ImportWizard.test.tsx:218` checks off-page state and only 80 rendered inputs for a 5,000-row client source. | PASS |
| IMP-06 | Rust authoritatively rejects invalid included fields, email conflicts, dates, quantities, stock settings and prices. `src-tauri/src/import_service.rs:595` asserts field-specific client errors; `src-tauri/src/import_service.rs:625` asserts all duplicate/deleted-employee conflicts; `src-tauri/src/import_service.rs:662` asserts stock/bounds errors. | PASS |
| IMP-07 | Checked integer-cent prices and DD/MM/YYYY, ISO, numeric Excel and typed ISO date-only cells. `src-tauri/src/import_service.rs:563` asserts exact cents/format rejection; `src-tauri/src/import_service.rs:625` asserts normalized joining date; `src-tauri/src/import_file.rs:463` asserts numeric styled Excel date; `src-tauri/src/import_file.rs:527` asserts typed ISO date, actual XLSX numeric provenance and zero-loss warning. | PASS |
| IMP-08 | Discard requires confirmation before replacing source/mapping or closing a draft. `src/components/shared/import/ImportWizard.test.tsx:248` checks cancel preserves edits and confirmed discard closes; `src/components/shared/import/ImportWizard.test.tsx:347` checks worksheet/file replacement confirmation. Visible browser check also verifies Escape restores the initiating control's focus. | PASS |
| IMP-09 | Immediate transaction revalidates, creates fresh IDs/timestamps and stores receipt with records. `src-tauri/src/import_service.rs:705` asserts two distinct IDs for intentionally duplicated records; `src-tauri/src/import_service.rs:883` asserts 5,000 unique created IDs and rows. Transaction and model construction inspected at `src-tauri/src/import_service.rs:439` and `src-tauri/src/import_service.rs:472`. | PASS |
| IMP-10 | A failed row or insertion leaves no batch records/receipt. `src-tauri/src/import_service.rs:744` injects failure on the second insertion and asserts zero customers and receipts; the same test checks a new employee conflict detected at commit. | PASS |
| IMP-11 | Positive initial tracked stock gets an entrada/bulk_import movement, null OS, imported unit cost and matching initial average cost. `src-tauri/src/import_service.rs:787` asserts quantity, cost, average cost, sale price, supplier, movement type/reason/cost and null reference; replay leaves one movement. | PASS |
| IMP-12 | Identical batch replay returns saved result; changed payload fails; concurrency creates once. `src-tauri/src/import_service.rs:705` asserts saved result equality and conflict rejection; `src-tauri/src/import_service.rs:849` checks employee replay precedes its own uniqueness conflict; `src-tauri/src/import_service.rs:911` checks concurrent results and one persisted customer. | PASS |
| IMP-13 | Client reads its own file and sends mapped rows only to authenticated Host, with no local fallback. `src/components/shared/import/ImportWizard.test.tsx:322` asserts local file invoke and remote mutation without file path; `src/components/shared/import/ImportWizard.test.tsx:473` checks unavailable validation blocks save; `src-tauri/src/lan_api.rs:1767` asserts unauthorized commit rejected and `src-tauri/src/lan_api.rs:1793` rejects remote file commands. | PASS |
| IMP-14 | Completion counts and affected query-prefix refresh. `src/components/shared/import/ImportWizard.test.tsx:159` checks client invalidation; `src/components/shared/import/ImportWizard.test.tsx:210` checks created/excluded counts; `src/components/shared/import/ImportWizard.test.tsx:393` exercises employee/inventory payloads and inventory summary invalidation. | PASS |
| IMP-15 | Uncertain response keeps frozen batch/payload; in-flight repeats cannot submit twice. `src/components/shared/import/ImportWizard.test.tsx:266` asserts identical requests and disabled edit/cancel controls after transport error; `src/components/shared/import/ImportWizard.test.tsx:502` asserts one submission while pending. | PASS |

## Gates

All commands ran from the repository root using the explicit Rust manifest when applicable.

| Command | Result |
| --- | --- |
| `yarn typecheck` | PASS |
| `yarn lint` | PASS |
| `yarn test` | 143 passed, 31 files, zero failed/skipped |
| `yarn build` | PASS; existing bundle-size and Browserslist-age notices remain |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1` | 241 passed, zero failed, 4 existing ignored tests |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib import_file -- --test-threads=1` | 7 passed after the final date correction |

The ignored Rust tests are three opt-in performance benchmarks and the existing real-PDF test requiring a local Chromium binary. None covers changed PDF/performance behavior. LAN/full Rust checks ran with approved loopback access because the filesystem sandbox also denies local sockets. The initial suite exposed an actual receipt migration replay defect on legacy SQLCipher export; this was corrected and the entire suite passed. A legacy backup assertion was advanced from schema version 5 to 6; its data/integrity/foreign-key assertions remain unchanged. No tests were removed or disabled.

Before feature: 128 frontend tests and 223 discovered Rust tests. After feature: 143 frontend tests and 245 discovered Rust tests (241 executed, 4 pre-existing ignored).

## Discrimination sensor

Independent sensor used an isolated scratch crate, restored each source between probes and verified the real tree remained clean before and after. Eight valid probes compiled and failed their specified business assertion; no survivor. One earlier rollback fault-injection attempt failed compilation and was excluded, then corrected and rerun as a valid probe.

Commands below ran in the scratch crate with the prefix `cargo test --offline --target-dir <repository>/src-tauri/target --lib` and suffix `-- --test-threads=1`. Only cached build artifacts were shared. Scratch sources were discarded.

| Mutation | Changed implementation | Exact test filter | Observed assertion | Result |
| --- | --- | --- | --- | --- |
| Corrupt replay count | `src-tauri/src/import_service.rs:456` | `import_service_commit_creates_duplicates_replays_and_rejects_changed_payload` | Replay count 0 vs 2 | Killed |
| Disable payload conflict rejection | `src-tauri/src/import_service.rs:448` | `import_service_commit_creates_duplicates_replays_and_rejects_changed_payload` | Expected error received successful replay | Killed |
| Commit earlier inserts after later failure | `src-tauri/src/import_service.rs:478` | `import_service_commit_rolls_back_insert_failure_and_revalidates_conflicts` | Persisted customers 1 vs 0 | Killed |
| Suppress initial stock movement | `src-tauri/src/import_service.rs:513` | `import_service_commit_stock_has_exact_costs_and_audit_movement` | Movements 0 vs 1 | Killed |
| Use sale price as movement cost | `src-tauri/src/import_service.rs:516` | `import_service_commit_stock_has_exact_costs_and_audit_movement` | Unit cost 2050 vs 1234 | Killed |
| Multiply integer currency by 10 | `src-tauri/src/import_service.rs:187` | `import_service_money_is_exact_and_locale_specific` | Cents 12396 vs 123456 | Killed |
| Ignore deleted employees | `src-tauri/src/import_service.rs:222` | `import_service_employee_conflicts_include_deleted_and_all_duplicate_rows` | Invalid rows 3 vs 4 | Killed |
| Clear receipt hashes on migration | `src-tauri/src/database.rs:651` | `bulk_import_receipts_survive_schema_version_loss_during_encryption` | Preserved hash empty vs hash | Killed |

Budget: 8/8 for data integrity. These selected high-risk branches were probed once each; this is not exhaustive mutation coverage of parsing, frontend or LAN code. Those outcomes have focused regression, IPC/HTTPS and browser evidence above.

## Independent correction and re-review

The coverage verifier identified a valid typed date-only XLSX cell rejected by `as_datetime()`. A new real XLSX fixture first failed against that implementation: expected `2026-10-20`, received the formula/error sentinel. Switching to `as_date()` fixed it while retaining numeric styled Excel dates. All seven parser tests passed and independent re-review marked IMP-07 PASS, carrying forward the other fourteen requirement results. The final complete Rust suite and clippy also passed after this change. No outstanding correctness gaps remain.

## Browser and desktop limits

Visible Chromium checked all three entity wizards at 1440×1000 and 900×700 in light/dark themes: mapping, edits across pages, discard confirmation, Escape/focus restoration, final commit and no browser errors. A 5,000-row React test verified bounded review rendering; a Rust test persisted 5,000 distinct records and a receipt. Screenshots were inspected for layout. Browser tests mocked native dialog/IPC; registered native commands and authenticated HTTPS transport were exercised separately by Rust tests. A full manual Tauri desktop session with real platform file dialogs was not performed.

## Completion

All 15 requirements are verified. Documentation: `docs/guides/data-import.md`; roadmap removes the delivered CSV-import capability from optional backlog text. Two grounded lessons record date representation and SQLCipher schema-version loss using the skill's bookkeeping tool. Scope remains create-only, existing fields, transient mappings/drafts, one worksheet, and bounded atomic imports.


## UI refinement validation: PASS

Verification: standard (UI behavior, shared controls and compatible dialog dependencies; no Rust/data-path changes). This section supersedes the original report's frontend test count and covers the screenshot-driven refinement after `ab8cb7c`.

| Requirement | Evidence | Result |
| --- | --- | --- |
| IMP-16 | `src/components/shared/import/ImportWizard.test.tsx:127` asserts one initial selection action, absence of premature mapping action, contextual limits, and active file/mapping stages. `src/components/shared/import/ImportProgress.tsx:11` provides semantic navigation/current-step state. | PASS |
| IMP-17 | `src/components/shared/import/ImportWizard.test.tsx:127` asserts filename, read status, counts and preview. `src/components/shared/import/ImportFileStep.tsx:52` renders the summary and a maximum three-row table with minimum column widths. Visible Chromium with eight columns measured table width 1532 px in a 1148 px viewport; horizontal scrolling preserves column readability. | PASS |
| IMP-18 | `src/components/shared/import/ImportWizard.test.tsx:169` asserts a button-based app Select trigger and selected source-column text; `src/components/shared/import/ImportWizard.test.tsx:410` selects an XLSX sheet through app options; `src/components/shared/Pagination.test.tsx:75` asserts the same numeric page-size callback. Worksheet, separator, mapping, number-format and pagination controls use the shared Select. | PASS |
| IMP-19 | `src/components/shared/import/ImportWizard.test.tsx:169` opens with the keyboard, presses Escape, asserts the menu closes, the wizard remains open and trigger focus returns. Visible Chromium repeats pointer opening and Escape dismissal for each entity. | PASS |

Gates: `yarn typecheck`, `yarn lint`, `yarn test` (145 passed, 31 files), and `yarn build` all pass. Existing bundle-size/Browserslist notices remain. Backend commands were not rerun for this frontend-only revision; the original backend validation remains above. The two existing pagination tests changed interaction from native selectOptions to app dropdown clicks; callback, value, scroll and call-count assertions remain intact. The file-error test now asserts no mapping action rather than a disabled action, matching the redesigned single-action empty state.

The installed Dialog and Select originally resolved different focus-scope/dismissable-layer modules, producing competing focus traps. Dialog 1.1.23 and explicit Yarn resolutions unify those internals. Identical stale generated nested copies from the old install were removed after checking version and code equality. An offline frozen-lockfile installation then retained one resolved instance for both primitives. No manual focus workaround or custom dropdown was introduced. A full frontend run covers the shared pagination/dialog surface. Future Radix upgrades must keep the layer versions aligned.

The visible browser harness checked all three entities at 1440×1000 and 900×700, including file summary, stage changes, app menu, editing, pagination, discard focus and commit. Dark screenshots wait for the app's color transitions; open-menu screenshots retain the menu's normal animation. Inspected screenshots confirm the upload card, framed preview, readable columns and app dropdown. Native IPC/dialogs are mocked in this browser harness, as in the original check.

Standard sensor ran in isolated source copies with a symlinked dependency directory and scratch Vite config. Two mutations of the highest-risk changed file, ImportWizard.tsx, compiled and failed exact UI assertions:

| Mutation | Test filter | Expected failure | Result |
| --- | --- | --- | --- |
| Always pass file as the progress stage | introduces file selection | Active stage remained Arquivo instead of Mapeamento | Killed |
| Always map selected columns to index zero | uses the app dropdown | Trigger showed Full name instead of selected Phone number | Killed |

Command for each: `node <repository>/node_modules/vitest/vitest.mjs run --config <scratch>/vite.config.ts --root <scratch> src/components/shared/import/ImportWizard.test.tsx -t '<filter>'`. Budget: 2 of at most 3. Real working-tree porcelain matched before/after; scratch originals were restored. All nineteen requirements are now verified; no outstanding UI refinement gaps remain.
