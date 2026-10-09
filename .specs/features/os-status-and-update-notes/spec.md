# OS Status and Update Notes Specification

## Problem Statement

Changing an order's status currently requires opening its detail and then its edit sheet. The update dialogs display Markdown release notes as raw text.

## Goals

- [x] Change an OS status directly from its detail view.
- [x] Render Markdown release notes in both update dialogs.
- [x] Offer a development-only preview of the update screens without installing an update.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Kanban board | Separate product decision. |
| New status transitions | Existing backend rules remain authoritative. |

## Assumptions & Open Questions

| Decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Status interaction | Show all five statuses in a fixed layout, mark the current status, and disable current or invalid destinations. | Keeps the status options stable while respecting backend transition rules. | y |
| Cancellation | Retain the existing stock restoration confirmation. | Cancellation can mutate inventory. | n |
| Update notes | Format both preinstall and postinstall dialogs. | Both display the same Markdown body. | n |
| Update preview | Show a manually advanced mock flow only in development builds. | Staff can inspect every stage without downloads, installation, or restart. | y |

**Open questions:** none; the defaults follow the existing flows.

## User Stories

### P1: Change status in the OS view

**User Story:** As a staff member, I want to change status from the OS view so that I do not need to open the edit sheet.

**Acceptance Criteria:**

1. WHEN an OS detail view opens THEN the system SHALL show all five status buttons, highlight and disable the current status, and disable backend-disallowed destinations.
2. WHEN a non-cancellation status is selected THEN the system SHALL call `transition_service_order_status` with `restoreStock: false` and refresh the order, timeline, list, and affected summaries.
3. WHEN cancellation is selected THEN the system SHALL request confirmation before calling `transition_service_order_status` with `restoreStock: true`.
4. IF a status change fails THEN the system SHALL show a Portuguese error and retain the persisted status.
5. WHEN the OS edit sheet opens THEN the system SHALL present only data-edit controls, with no status action.

**Independent Test:** Open an OS, change to Em Manutenção, cancel another OS after confirmation, and verify the edit sheet has no status control.

### P1: Read formatted update notes

**User Story:** As a user updating OpetS, I want release notes to appear as readable headings and lists.

**Acceptance Criteria:**

1. WHEN update notes contain Markdown headings, lists, emphasis, or links THEN both update dialogs SHALL render their structure.
2. IF update notes contain raw HTML or an unsafe URL THEN the system SHALL not render executable HTML or an unsafe link destination.
3. WHEN update notes exceed the dialog height THEN the system SHALL make the notes scrollable within the dialog.

**Independent Test:** Open the available update dialog with formatted notes, then open the completed update dialog after restart with the same body.

### P1: Preview an update in development

**User Story:** As a developer or reviewer, I want to inspect each update screen without performing an installation.

**Acceptance Criteria:**

1. WHILE running a development build, the system SHALL show a `Pré-visualizar atualização` action in Settings.
2. WHEN the preview starts THEN the system SHALL show an example update offer with formatted release notes and a clear preview notice.
3. WHEN the preview advances THEN the system SHALL show download progress, installation progress, and the completed-update dialog in order.
4. WHILE the preview is active, the system SHALL not call update checking, downloading, installation, or restart APIs and SHALL not write pending update notes to local storage.
5. WHEN the preview closes THEN the system SHALL return to Settings without an update dialog or progress overlay.
6. WHILE running a production build, the system SHALL not show the preview action.

**Independent Test:** In development Settings, start the preview, advance through every stage, close it, and verify updater and restart mocks are unused.

## Edge Cases

- IF an OS is Cancelada and stock cannot be consumed for reactivation THEN the backend SHALL reject the transition without changing the order; the UI SHALL show its error.

## Requirement Traceability

| Requirement ID | Story | Status |
| --- | --- | --- |
| OSSTATUS-01 | Change status in the OS view | Verified |
| OSSTATUS-02 | Change status in the OS view | Verified |
| OSSTATUS-03 | Change status in the OS view | Verified |
| OSSTATUS-04 | Change status in the OS view | Verified |
| OSSTATUS-05 | Change status in the OS view | Verified |
| UPDATENOTES-01 | Read formatted update notes | Verified |
| UPDATENOTES-02 | Read formatted update notes | Verified |
| UPDATENOTES-03 | Read formatted update notes | Verified |
| UPDATEPREVIEW-01 | Preview an update in development | Verified |
| UPDATEPREVIEW-02 | Preview an update in development | Verified |
| UPDATEPREVIEW-03 | Preview an update in development | Verified |
| UPDATEPREVIEW-04 | Preview an update in development | Verified |
| UPDATEPREVIEW-05 | Preview an update in development | Verified |
| UPDATEPREVIEW-06 | Preview an update in development | Verified |

**Coverage:** 14 requirements; focused UI tests and existing backend transition tests cover the behavior.

## Success Criteria

- [x] A valid status change takes one action after opening the OS.
- [x] Update notes appear as formatted Markdown in both dialogs.
- [x] Development builds can preview update screens without installation side effects.
