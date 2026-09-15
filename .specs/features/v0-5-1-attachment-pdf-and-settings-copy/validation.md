# OpetS v0.5.1 Validation

**Date**: 2026-09-14
**Tier**: standard - UI behavior, no auth or data-path change
**Spec**: `.specs/features/v0-5-1-attachment-pdf-and-settings-copy/spec.md`
**Diff range**: `7a85e1a..4c6ad87`
**Verifier**: independent verifier (author != verifier)

---

## Iteration 1 Initial Verdict: FAIL ❌

The PDF and Settings behaviors are implemented and their focused tests pass. The feature cannot close because PDFVIEW-06 requires unchanged image-preview behavior, but its behavior-level mutant survived the new attachment suite. The Settings visual-contract test also does not assert the required ghost variant or copy icon.

## Task Completion

| Task | Status | Notes |
| --- | --- | --- |
| PDF attachment preview | ✅ Done | Implemented in `ServiceOrderDetailSheet.tsx`. |
| Settings copy controls | ✅ Done | One local reusable control is used for all three Host values. |
| Release metadata | ✅ Done | Version is `0.5.1` in package, Cargo, and Tauri metadata. |

## Spec-Anchored Acceptance Criteria

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| --- | --- | --- | --- |
| PDFVIEW-01: PDF attachment shows the PDF control | Exact visible label `Visualizar PDF` for `application/pdf` | `src/components/shared/ServiceOrderDetailSheet.test.tsx:95-97` - `findByRole(... name: "Visualizar PDF")` | ✅ PASS |
| PDFVIEW-02: activating it reads through `read_service_order_attachment` and renders its returned data URL inline | Existing data command with attachment ID; returned URL is iframe `src` | `src/components/shared/ServiceOrderDetailSheet.test.tsx:102-106` - iframe `src` equals returned URL and command arguments equal `{ id }`; `:155-165` asserts LAN remote operation and iframe URL | ✅ PASS |
| PDFVIEW-03: pending request shows exact loading text and no stale document | Exact `Carregando PDF...` before resolution | `src/components/shared/ServiceOrderDetailSheet.test.tsx:100` - `getByText("Carregando PDF...")`; mutation at `src/components/shared/ServiceOrderDetailSheet.tsx:301` was killed | ✅ PASS |
| PDFVIEW-04: failed request shows exact error and preserves download | Exact error and enabled `Baixar` | `src/components/shared/ServiceOrderDetailSheet.test.tsx:127-128` - error text and enabled download button | ✅ PASS |
| PDFVIEW-05: visible PDF control hides preview and uses exact collapse label | `Ocultar visualização` removes the iframe | `src/components/shared/ServiceOrderDetailSheet.test.tsx:108-109` - click exact button name, then iframe absent | ✅ PASS |
| PDFVIEW-06: image behavior remains unchanged and local/LAN paths use existing encrypted reader | Image preview must remain intact; LAN uses the existing data client operation | LAN path: `src/components/shared/ServiceOrderDetailSheet.test.tsx:155-165` asserts `lan_remote_command` operation. Image behavior: no assertion; changing image `src` at `src/components/shared/ServiceOrderDetailSheet.tsx:313` to a wrong PDF URL left all four attachment tests green. | ❌ GAP |
| SETCOPY-01: all three Host controls share compact ghost styling, copy icon, and visible label | Same ghost/small control with icon and `Copiar` | `src/views/Settings.test.tsx:326-329` asserts common compact classes and visible label. It does not assert `variant="ghost"` or the icon. | ❌ GAP |
| SETCOPY-02: each control copies its displayed exact value | Address, pairing code, fingerprint passed verbatim to `copyToClipboard` | `src/views/Settings.test.tsx:335-339` - ordered exact utility calls | ✅ PASS |
| SETCOPY-03: successful copy displays the corresponding Portuguese toast | Exact messages for Endereço, Código, and Impressão digital | `src/views/Settings.test.tsx:340-342` - ordered exact `toast.success` messages | ✅ PASS |
| SETCOPY-04: corresponding accessible names are exposed | Three exact accessible names | `src/views/Settings.test.tsx:316-324` - three `getByRole` lookups by exact name | ✅ PASS |

**Status**: ❌ Gaps present. 8/10 criteria match the specified outcome; 0 spec-precision gaps.

## Discrimination Sensor

Real-tree baseline and post-cleanup `git status --porcelain=v1` were empty. All mutations ran only in the disposable worktree `/tmp/opets-v051-sensor`, created at `4c6ad87` and removed after testing.

| Mutation | File:line | Description | Command | Killed? |
| --- | --- | --- | --- | --- |
| 1 | `src/components/shared/ServiceOrderDetailSheet.tsx:233` | Changed PDF MIME recognition from `application/pdf` to `application/pdfx` | `yarn test src/components/shared/ServiceOrderDetailSheet.test.tsx` | ✅ Killed, 4 failures: PDF control absent |
| 2 | `src/components/shared/ServiceOrderDetailSheet.tsx:313` | Replaced existing image preview `src` with wrong PDF data URL | `yarn test src/components/shared/ServiceOrderDetailSheet.test.tsx` | ❌ Survived, 4/4 tests passed |
| 3 | `src/components/shared/ServiceOrderDetailSheet.tsx:301` | Changed pending text to `Carregando anexo...` | `yarn test src/components/shared/ServiceOrderDetailSheet.test.tsx` | ✅ Killed, loading-state assertion failed |

**Tier / budget**: standard - 3 used of 3 allowed.
**Spec-constrained branches probed**: 3 of 3 selected branches. The image-preservation branch survived and is a required fix.
**Sensor outcome**: 2/3 killed - FAIL ❌

## Interactive UAT Results

| # | Test | Result | Details |
| --- | --- | --- | --- |
| 1 | PDF open, render, close, error, and Settings copy controls | ⏭️ Skip | No user interaction was conducted during independent validation. |

## Code Quality

| Principle | Status |
| --- | --- |
| Minimum code | ✅ |
| Surgical changes | ✅ |
| No scope creep | ✅ |
| Matches patterns | ✅ |
| Spec-anchored outcome check | ❌ PDFVIEW-06 and SETCOPY-01 have the coverage gaps above. |
| Per-layer Coverage Expectation met | ❌ Image-regression behavior is not covered. |
| Every test maps to a spec requirement | ✅ Focused tests map to PDF and Settings criteria. |
| Documented guidelines followed: `AGENTS.md` and Exban coding principles | ✅ |

## Edge Cases

- [x] PDF failure retains download: asserted at `src/components/shared/ServiceOrderDetailSheet.test.tsx:127-128`.
- [x] Collapsing a successfully loaded PDF does not fetch again immediately: `src/components/shared/ServiceOrderDetailSheet.tsx:235-241` uses the same query key with a 60-second PDF stale time; the test also verifies collapse at `:108-109`.
- [ ] Non-image/non-PDF metadata has no inline preview: implemented by `src/components/shared/ServiceOrderDetailSheet.tsx:232-234,286`, but not independently tested.
- [ ] Clipboard denial produces no success toast: implemented by `src/views/Settings.tsx:102-104`, but not independently tested.

## Gate Check

- **Focused verifier command**: `yarn test src/components/shared/ServiceOrderDetailSheet.test.tsx src/views/Settings.test.tsx`
- **Gate outcome**: 22 passed, 0 failed, 0 skipped.
- **Supplied feature gate evidence**: frontend 111 passed; Rust 215 passed, 4 ignored.
- **Test files before feature**: 23.
- **Test files after feature**: 24.
- **Delta**: +1 test file. No test deletion or weakened assertion was found in the commit range.
- **Failures**: none in the real-tree focused run.

## Fix Plans

### Fix 1: Cover preservation of image preview behavior

- **Root cause**: The new attachment suite creates only a PDF attachment. It never opens an image attachment or asserts the image source.
- **Fix task**: Add an image attachment test that opens the existing image preview and asserts the returned data URL is the image `src`; keep the PDF tests unchanged.
- **Verify**: Repeat mutation 2 in a disposable worktree. The narrow attachment test must fail.
- **Priority**: Major.

### Fix 2: Assert the full Settings copy-button contract

- **Root cause**: The Settings test verifies common utility classes and label only. It does not verify ghost variant styling or copy icon presence.
- **Fix task**: Assert the standard ghost-button contract and icon for each control, using accessible or DOM-level evidence consistent with existing component tests.
- **Verify**: Mutate the button variant or remove the icon in a disposable worktree; the narrow Settings test must fail.
- **Priority**: Minor.

## Requirement Traceability Update

| Requirement | Previous Status | New Status |
| --- | --- | --- |
| PDFVIEW-01 to PDFVIEW-05 | Verified | ✅ Verified |
| PDFVIEW-06 | Verified | ❌ Needs Fix |
| SETCOPY-01 | Verified | ❌ Needs Fix |
| SETCOPY-02 to SETCOPY-04 | Verified | ✅ Verified |

## Summary

**Overall**: ❌ Not Ready.

**Spec-anchored check**: 8/10 ACs match precise specified outcomes.
**Sensor**: 2/3 mutations killed; the image-preview regression mutant survived.
**Gate**: 22 focused tests passed. Supplied full gate evidence reports 111 frontend tests passed and 215 Rust tests passed with 4 ignored.

**What works**: PDF detection, local and LAN data-command use, loading/error/collapse states, download retention, exact Settings values/toasts/accessible names, and v0.5.1 metadata.

**Issues found**: The feature lacks a regression test proving existing image preview behavior remains unchanged. The Settings visual contract lacks assertions for two explicit requirements.

**Next steps**: Implement Fix 1 and Fix 2, then re-run incremental standard verification.

## Iteration 2

**Diff reviewed**: `4c6ad87..0c43f84` (`fix(attachments): render PDFs with PDF.js`)

### Incremental Acceptance-Criteria Ledger

| Prior gap / changed behavior | Spec-defined outcome | `file:line` + assertion | Result |
| --- | --- | --- | --- |
| PDFVIEW-06: existing image preview remains unchanged | Image preview reads the attachment data URL and renders that exact URL as the image source | `src/components/shared/ServiceOrderDetailSheet.test.tsx:153-175` - image attachment opens through its existing control and image `src` equals `data:image/png;base64,iVBORw0KGgo=` | ✅ PASS |
| SETCOPY-01: shared compact ghost control, copy icon, visible label | Each Host copy button has the standard ghost styling, `Copiar` label, and copy icon | `src/views/Settings.test.tsx:326-338` - all three buttons assert compact plus ghost hover classes, `Copiar`, and `svg.lucide-copy` | ✅ PASS |
| PDFVIEW-02, revised renderer implementation: returned PDF data is displayed inline on a canvas | Data URL becomes PDF.js bytes, a canvas page is rendered, and page navigation works | `src/components/shared/PdfAttachmentPreview.test.tsx:51-62` - asserts `Uint8Array` input, page 1 render call, page count, page 2 navigation, visible second canvas, and final-page disabled state | ✅ PASS |

The earlier passing PDF and Settings rows are carried from iteration 1. The canvas renderer is rendered in isolation with mocked PDF.js; no desktop-WebView interactive UAT occurred.

### Incremental Discrimination Sensor

Real-tree baseline and post-cleanup `git status --porcelain=v1` both contained only the pre-existing untracked `validation.md`. Mutations ran in the disposable worktree `.sensor-v051`, which was removed after testing. Its temporary Vitest config only permitted the scratch test runner to resolve the existing workspace dependencies; no product files were changed.

| Mutation | File:line | Description | Command | Killed? |
| --- | --- | --- | --- | --- |
| 1 | `src/components/shared/PdfAttachmentPreview.tsx:104` | Changed next-page handler from increment to decrement | `yarn vitest run --config .sensor-v051/sensor.config.ts src/components/shared/PdfAttachmentPreview.test.tsx` | ✅ Killed: expected page 2, received page 0 |
| 2 | `src/components/shared/PdfAttachmentPreview.tsx:60-65` | Removed the `page.render(...)` side effect and resolved immediately | same narrow renderer command | ✅ Killed: `renderMock` was never called |
| 3 | `src/components/shared/PdfAttachmentPreview.tsx:68` | Suppressed `setError(true)` after a PDF.js render failure | same narrow renderer command | ❌ Survived: 1/1 test passed |

**Tier / budget**: standard, 3 used of 3 allowed.
**Sensor outcome**: 2/3 killed. The surviving error-state mutation recreates an unreported blank canvas after an invalid/failed PDF.js render.

### Focused Gate

- `yarn test src/components/shared/PdfAttachmentPreview.test.tsx src/components/shared/ServiceOrderDetailSheet.test.tsx src/views/Settings.test.tsx` → 24 passed, 0 failed, 0 skipped.
- `yarn typecheck` → passed.

### Remaining Fix Plan

#### Fix 3: Cover PDF.js renderer failure feedback

- **Root cause**: `PdfAttachmentPreview.test.tsx` covers successful canvas rendering and navigation only. It never rejects `getDocument`, `getPage`, or `render().promise` and therefore cannot prove that the component exposes `Não foi possível renderizar o PDF.`.
- **Fix task**: Add a renderer test that rejects the PDF.js loading or render promise and asserts the exact error message, the absence of a visible stale canvas, and disabled navigation while rendering is complete.
- **Verify**: Repeat mutation 3 in a disposable worktree. The renderer test must fail.
- **Priority**: Major, because the intended replacement directly addresses a blank-PDF symptom.

### Iteration 2 Verdict: FAIL ❌

The prior image and Settings test gaps are closed. PDF.js page rendering and navigation have discriminating coverage, but its failure feedback has no coverage and the sensor proves that a blank renderer can return without the intended error state. Re-verify after Fix 3.

## Iteration 3

**Diff reviewed**: `0c43f84..d0f5764` (`test(attachments): cover PDF render failure`)

### Incremental Acceptance-Criteria Ledger

| Prior blocker | Spec-defined outcome | `file:line` + assertion | Result |
| --- | --- | --- | --- |
| PDF.js renderer failure must not leave a blank visible document | On a PDF.js failure, the user sees clear PDF-rendering feedback and no stale canvas | `src/components/shared/PdfAttachmentPreview.test.tsx:65-79` - rejected PDF.js load produces exact `Não foi possível renderizar o PDF.` and `Página 1 de laudo.pdf` canvas is absent | ✅ PASS |

### Incremental Discrimination Sensor

| Mutation | File:line | Description | Command | Killed? |
| --- | --- | --- | --- | --- |
| 1 | `src/components/shared/PdfAttachmentPreview.tsx:68` | Replaced `setError(true)` with `setError(false)` after a PDF.js failure | `yarn vitest run --config .sensor-v051/sensor.config.ts src/components/shared/PdfAttachmentPreview.test.tsx` | ✅ Killed: exact error text was absent; stale canvas remained |

**Tier / budget**: standard incremental re-verification, 1 required surviving-mutant branch probed.
**Sensor outcome**: 1/1 killed - PASS ✅.

### Focused Gate

- `yarn test src/components/shared/PdfAttachmentPreview.test.tsx` → 2 passed, 0 failed, 0 skipped.
- Earlier iteration gate carried: 24 focused tests passed and `yarn typecheck` passed.

### Isolation

The sensor ran only in disposable `.sensor-v051`. Its worktree was removed. Real-tree `git status --porcelain=v1` before and after sensor work contained only this pre-existing untracked validation report.

## Validation: OpetS v0.5.1 - PASS ✅

All prior gaps are resolved. PDF attachment preview, image-preview preservation, Settings copy-button contract, PDF.js canvas rendering/navigation, and renderer failure feedback have direct assertions. The three historical mutations that matter to the changed renderer path are now killed. Automated interactive UAT remains skipped because no desktop user interaction was conducted.
