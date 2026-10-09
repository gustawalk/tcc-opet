# OS Status and Update Notes Validation

## Validation: PASS

**Tier:** standard, UI behavior using existing backend status transitions.
**Spec:** `.specs/features/os-status-and-update-notes/spec.md`
**Diff range:** original feature `5fa5d6d..0f90e9d`; follow-up `3d1e1f3..8fd5587`.

## Outcome evidence

| Requirement | Evidence | Result |
| --- | --- | --- |
| OSSTATUS-01 | `src/components/shared/ServiceOrderDetailSheet.tsx:98` fixes all five statuses in one order, and `src/components/shared/ServiceOrderDetailSheet.tsx:607` marks the current one and disables invalid destinations; `src/components/shared/ServiceOrderDetailSheet.test.tsx:251` checks those states. | PASS |
| OSSTATUS-02 | `src/components/shared/ServiceOrderDetailSheet.tsx:464` uses the status command and invalidates order, timeline, list, dashboard, report, and inventory queries; `src/components/shared/ServiceOrderDetailSheet.test.tsx:232` checks the resulting action. | PASS |
| OSSTATUS-03 | `src/components/shared/ServiceOrderDetailSheet.tsx:928` keeps cancellation behind the stock restoration confirmation; `src/components/shared/ServiceOrderDetailSheet.test.tsx:262` checks that no mutation occurs before confirmation. | PASS |
| OSSTATUS-04 | `src/components/shared/ServiceOrderDetailSheet.tsx:487` reports mutation errors; `src/components/shared/ServiceOrderDetailSheet.test.tsx:290` checks the persisted status remains available. | PASS |
| OSSTATUS-05 | `src/components/shared/ServiceOrderEditorSheet.tsx:293` saves data with the current status and has no status action. | PASS |
| UPDATENOTES-01 | `src/components/shared/ReleaseNotesMarkdown.tsx:4` renders Markdown; `src/views/Settings.test.tsx:418` and `src/App.test.tsx:39` verify both dialogs. | PASS |
| UPDATENOTES-02 | `src/components/shared/ReleaseNotesMarkdown.test.tsx:17` confirms HTML is not executed and unsafe link URLs are removed. | PASS |
| UPDATENOTES-03 | `src/components/shared/ReleaseNotesMarkdown.tsx:6` caps and scrolls the notes; both dialog containers also cap their height. | PASS |
| UPDATEPREVIEW-01 | `src/views/Settings.tsx:886` guards the preview action with `import.meta.env.DEV`; the production build has no `Pré-visualizar atualização` string. | PASS |
| UPDATEPREVIEW-02 | `src/views/Settings.tsx:1656` shows the same available-update dialog with a preview notice and example Markdown; `src/views/Settings.test.tsx:435` checks it. | PASS |
| UPDATEPREVIEW-03 | `src/views/Settings.tsx:1718` uses the existing progress presentation for download and install, followed by the completion dialog at `src/views/Settings.tsx:1760`; `src/views/Settings.test.tsx:444` checks the sequence. | PASS |
| UPDATEPREVIEW-04 | `src/views/Settings.tsx:1691` advances local preview state instead of calling `installUpdate`; `src/views/Settings.test.tsx:454` checks updater, relaunch, and local-storage absence. | PASS |
| UPDATEPREVIEW-05 | `src/views/Settings.tsx:1760` closes the preview dialog; `src/views/Settings.test.tsx:459` checks dismissal. | PASS |
| UPDATEPREVIEW-06 | `src/views/Settings.tsx:886` uses the Vite development guard; production build string inspection confirms the control is absent. | PASS |

## Gates

- `yarn typecheck`: PASS.
- `yarn lint`: PASS.
- `yarn build`: PASS; Vite reports its existing large-chunk advisory.
- `yarn test`: PASS, 129 tests across 29 files after the follow-up.
- Production bundle inspection: `Pré-visualizar atualização` is absent from `dist/assets`.
- `python3 /home/walk/.codex/skills/exban/exban-spec-driven/scripts/validate_spec.py .specs/features/os-status-and-update-notes/spec.md`: PASS.

## Discrimination sensor

In a disposable `/tmp` clone, removing `Em Manutenção` from the allowed actions for `Orçamento` made the focused status test fail because that button was absent. The mutation was killed. The clone was removed and the repository files were untouched.

For the follow-up, two mutations in another disposable `/tmp` clone were killed: forcing every status button's `aria-pressed` to `false` failed the current-status test, and replacing the preview action with a real update check failed the safe-preview test. The clone was removed; the repository files were untouched.

The changed files are frontend and package files; Rust checks are not applicable. The Markdown parser is lazy loaded so update-note formatting is fetched only when needed.
