# OS Status and Update Notes Specification

## Problem Statement

Changing an order's status currently requires opening its detail and then its edit sheet. The update dialogs display Markdown release notes as raw text.

## Goals

- [x] Change an OS status directly from its detail view.
- [x] Render Markdown release notes in both update dialogs.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Kanban board | Separate product decision. |
| New status transitions | Existing backend rules remain authoritative. |

## Assumptions & Open Questions

| Decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Status interaction | Show valid destination buttons in the detail sheet. | Matches the existing button interaction with one fewer navigation step. | n |
| Cancellation | Retain the existing stock restoration confirmation. | Cancellation can mutate inventory. | n |
| Update notes | Format both preinstall and postinstall dialogs. | Both display the same Markdown body. | n |

**Open questions:** none; the defaults follow the existing flows.

## User Stories

### P1: Change status in the OS view

**User Story:** As a staff member, I want to change status from the OS view so that I do not need to open the edit sheet.

**Acceptance Criteria:**

1. WHEN an OS detail view opens THEN the system SHALL show buttons for backend-allowed next statuses and SHALL omit the current status as an action.
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

**Coverage:** 8 requirements; focused UI tests and existing backend transition tests cover the behavior.

## Success Criteria

- [x] A valid status change takes one action after opening the OS.
- [x] Update notes appear as formatted Markdown in both dialogs.
