# OpetS v0.5.1 Specification

## Problem Statement

Service-order attachments already accept encrypted PDF files, but the detail
sheet only previews image attachments. Staff must export a PDF before reading
it. The three copy actions in LAN Settings also have subtly different sizing,
spacing, accessible labels, and success feedback.

## Goals

- [ ] Let a user read an attached PDF from the service-order detail sheet
  without first exporting it.
- [ ] Give all Settings copy actions one visual treatment, accessible naming,
  and consistent success feedback.
- [ ] Release these contained UI improvements as v0.5.1 without a schema,
  attachment-format, or LAN API change.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Editing, annotating, or printing attached PDFs | This release only adds viewing; export remains the existing way to use the file elsewhere. |
| A new PDF rendering dependency or server-side conversion | The attachment reader already provides a verified data URL and the desktop WebView renders the document. |
| Changing which attachment formats are accepted, their 10 MB limit, encryption, storage, or backups | Those are established storage/security contracts. |
| Standardizing copy controls outside Settings | The request is limited to the Settings page. |
| New settings, database migrations, or LAN endpoints | Neither behavior needs persisted state or a new remote contract. |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| PDF placement | Expand the PDF in the same attachment card used for image preview, rather than opening a new dialog. | It follows the requested existing image-viewer interaction and keeps the change patch-sized. | y |
| PDF rendering | Use the browser/WebView's built-in PDF viewer in an iframe sourced from the existing authenticated data URL; permit that frame source in the Tauri CSP. | No plaintext file or additional backend/API is necessary; the same decrypt-and-validate path is used in local and LAN-client modes. | y |
| Unsupported WebView behavior | Keep the existing **Baixar** action available; do not introduce a PDF.js fallback in v0.5.1. | A renderer fallback would add a dependency and a separate rendering surface beyond the requested parity feature. | y |
| Settings-copy standard | Extract one local reusable Settings copy button with ghost/small styling, copy icon, visible **Copiar** label, specific accessible name, and action-specific success toast. | This removes the three current implementation differences while retaining Portuguese UI language and distinct feedback. | y |

**Open questions:** none - all resolved or logged above.

---

## User Stories

### P1: View an attached PDF

**User Story**: As a service technician, I want to view a PDF attached to a
service order so that I can read its contents without exporting it first.

**Why P1**: PDF is already a supported attachment type, and viewing it is the
missing counterpart to the existing image preview.

**Acceptance Criteria**:

1. WHEN a service-order detail sheet shows an attachment whose `mimeType` is `application/pdf`, THEN the system SHALL show a **Visualizar PDF** control in that attachment card. <!-- event-driven -->
2. WHEN the user activates **Visualizar PDF**, THEN the system SHALL request that attachment through the existing `read_service_order_attachment` data command and render the returned PDF data URL inline in the card. <!-- event-driven -->
3. WHILE the PDF request is pending, THEN the system SHALL show **Carregando PDF...** and SHALL not render a stale document. <!-- state-driven -->
4. IF the PDF request fails, THEN the system SHALL show **Não foi possível carregar o PDF.** and SHALL keep the existing **Baixar** action available. <!-- unwanted-behavior -->
5. WHEN the user activates the PDF control while the PDF is visible, THEN the system SHALL hide the PDF and label the control **Ocultar visualização**. <!-- event-driven -->
6. The system SHALL keep image preview behavior unchanged and SHALL continue to use the existing encrypted attachment read path for both local and LAN-client access. <!-- ubiquitous -->

**Independent Test**: Render the detail sheet with a PDF attachment, activate
the preview, assert the data command and embedded document source, then assert
loading, error, and hide states independently.

### P1: Consistent Settings copy controls

**User Story**: As an administrator configuring LAN access, I want copy
buttons in Settings to behave and look the same so that copying connection
information is predictable.

**Why P1**: Pairing requires accurately copying the host address, pairing code,
and certificate fingerprint; inconsistent controls increase friction in this
security-sensitive flow.

**Acceptance Criteria**:

1. WHEN the Host Settings section displays a copyable host address, pairing code, or certificate fingerprint, THEN the system SHALL render each action with the same compact ghost-button styling, copy icon, and visible **Copiar** label. <!-- event-driven -->
2. WHEN a user activates one of those controls, THEN the system SHALL call the shared `copyToClipboard` utility with exactly that control's displayed value. <!-- event-driven -->
3. WHEN copying succeeds, THEN the system SHALL show one Portuguese success toast that identifies the copied value as **Endereço**, **Código**, or **Impressão digital**, respectively. <!-- event-driven -->
4. The system SHALL expose the accessible names **Copiar endereço do servidor**, **Copiar código de pareamento**, and **Copiar impressão digital** for the corresponding controls. <!-- ubiquitous -->

**Independent Test**: Render Host Settings and verify all three controls share
the standard button contract, copy their exact values, and produce their
respective feedback.

## Edge Cases

- IF attachment metadata has a non-image, non-PDF MIME type, THEN the system
  SHALL not offer an inline preview and SHALL retain export/delete behavior.
- IF clipboard permission is unavailable, THEN the system SHALL not show a
  success toast.
- WHEN a PDF viewer is collapsed after a successful load, THEN the system SHALL
  not refetch the same attachment until it is requested again after its cached
  data becomes stale or the component is remounted.

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| PDFVIEW-01 | P1: View an attached PDF | Implement | Verified |
| PDFVIEW-02 | P1: View an attached PDF | Implement | Verified |
| PDFVIEW-03 | P1: View an attached PDF | Implement | Verified |
| PDFVIEW-04 | P1: View an attached PDF | Implement | Verified |
| PDFVIEW-05 | P1: View an attached PDF | Implement | Verified |
| PDFVIEW-06 | P1: View an attached PDF | Implement | Verified |
| SETCOPY-01 | P1: Consistent Settings copy controls | Implement | Pending |
| SETCOPY-02 | P1: Consistent Settings copy controls | Implement | Pending |
| SETCOPY-03 | P1: Consistent Settings copy controls | Implement | Pending |
| SETCOPY-04 | P1: Consistent Settings copy controls | Implement | Pending |

**Coverage:** 10 total, 10 mapped to implementation, 0 unmapped.

## Success Criteria

- [ ] A PDF attachment can be opened and closed inline in the service-order
  detail sheet in local and LAN-client modes, with a clear loading and failure
  state.
- [ ] The three Host Settings copy controls have one UI contract and copy the
  correct values with distinguishable Portuguese success feedback.
- [ ] v0.5.1 version metadata and release notes describe only these delivered
  improvements.
