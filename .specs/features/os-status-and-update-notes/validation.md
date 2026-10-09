# OS Status and Update Notes Validation

## Validation: PASS

**Tier:** standard, UI behavior using existing backend status transitions.
**Spec:** `.specs/features/os-status-and-update-notes/spec.md`
**Diff range:** `5fa5d6d..0f90e9d`.

## Outcome evidence

| Requirement | Evidence | Result |
| --- | --- | --- |
| OSSTATUS-01 | `src/components/shared/ServiceOrderDetailSheet.tsx:90` lists allowed destinations; `src/components/shared/ServiceOrderDetailSheet.test.tsx:232` checks the available actions. | PASS |
| OSSTATUS-02 | `src/components/shared/ServiceOrderDetailSheet.tsx:464` uses the status command and invalidates order, timeline, list, dashboard, report, and inventory queries; `src/components/shared/ServiceOrderDetailSheet.test.tsx:232` checks the resulting action. | PASS |
| OSSTATUS-03 | `src/components/shared/ServiceOrderDetailSheet.tsx:928` keeps cancellation behind the stock restoration confirmation; `src/components/shared/ServiceOrderDetailSheet.test.tsx:262` checks that no mutation occurs before confirmation. | PASS |
| OSSTATUS-04 | `src/components/shared/ServiceOrderDetailSheet.tsx:487` reports mutation errors; `src/components/shared/ServiceOrderDetailSheet.test.tsx:290` checks the persisted status remains available. | PASS |
| OSSTATUS-05 | `src/components/shared/ServiceOrderEditorSheet.tsx:293` saves data with the current status and has no status action. | PASS |
| UPDATENOTES-01 | `src/components/shared/ReleaseNotesMarkdown.tsx:4` renders Markdown; `src/views/Settings.test.tsx:418` and `src/App.test.tsx:39` verify both dialogs. | PASS |
| UPDATENOTES-02 | `src/components/shared/ReleaseNotesMarkdown.test.tsx:17` confirms HTML is not executed and unsafe link URLs are removed. | PASS |
| UPDATENOTES-03 | `src/components/shared/ReleaseNotesMarkdown.tsx:6` caps and scrolls the notes; both dialog containers also cap their height. | PASS |

## Gates

- `yarn typecheck`: PASS.
- `yarn lint`: PASS.
- `yarn build`: PASS; Vite reports its existing large-chunk advisory.
- `yarn test`: PASS, 128 tests across 29 files.
- `python3 /home/walk/.codex/skills/exban/exban-spec-driven/scripts/validate_spec.py .specs/features/os-status-and-update-notes/spec.md`: PASS.

## Discrimination sensor

In a disposable `/tmp` clone, removing `Em Manutenção` from the allowed actions for `Orçamento` made the focused status test fail because that button was absent. The mutation was killed. The clone was removed and the repository files were untouched.

The changed files are frontend and package files; Rust checks are not applicable. The Markdown parser is lazy loaded so update-note formatting is fetched only when needed.
