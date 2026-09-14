# Predicted Service-Order Finish Date Specification

## Problem Statement

The service order records its actual closure timestamp but provides no agreed operational forecast. Technicians and front-desk staff cannot communicate when an open repair is expected to be ready or quickly identify work whose promised date has passed.

## Goals

- [ ] Let staff record an optional, calendar-date prediction for an open service order at creation or later during its work.
- [ ] Make the prediction visible wherever an open order is managed, clearly distinguish it from the actual closure date, and make overdue work actionable.
- [ ] Preserve the prediction and its change history without changing financial calculations, stock behavior, or the status lifecycle.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Automatic duration estimates, SLA calculation, or holiday calendars | The application has no duration/SLA policy to calculate a reliable forecast. |
| Notifications by WhatsApp, email, or push | This feature creates operational visibility only; delivery channels require separate consent and integration design. |
| Staff authentication or role permissions | The existing `users` directory identifies the technical responsible person but does not authenticate the current operator. |
| Dashboard/reporting KPIs and customer self-service tracking | These are separate product surfaces and would expand the scope beyond managing an individual OS. |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Precision | Store a date only, in `YYYY-MM-DD`, called `predictedFinishDate`; never a timestamp. | Repair promises are normally communicated as a day; a time would create timezone and SLA semantics the product does not define. | y |
| Requirement at creation | The field is optional for every status. | A diagnosis or part availability may be unknown when the OS is opened. | y |
| Valid date | A new or changed date must be the order's local calendar opening date or later. Existing legacy/null values remain valid. | Prevents a nonsensical forecast before the order existed, while allowing same-day completion. | y |
| Editable states | It can be set, changed, or cleared while status is `Orçamento`, `Em Manutenção`, or `Aguardando Peça`; it becomes read-only when `Finalizada` or `Cancelada`. | A final/cancelled OS is historical evidence, not active planning. | y |
| Overdue definition | An open OS is overdue when it has a prediction earlier than today's local calendar date; due today is not overdue. | Clear, familiar operational behavior without time-of-day ambiguity. | y |
| Responsibility | The existing responsible-technician selection remains independent. No UI claims that only that technician can edit because the app has no authenticated operator. | Enforcing an identity rule only in the UI would be misleading and bypassable through IPC/LAN. | y |
| Audit | Every create, change, or clearing action appends an order event with old/new values; the current data model has no actor identity, so events do not attribute a person. | Deadline changes affect customer commitments and need an auditable timeline. | y |
| Printed OS | If defined, show `Previsão de conclusão: DD/MM/AAAA` in the generated PDF; omit the row when absent and never label it as a guarantee. | The same promise must be visible to staff and customer-facing documentation. | y |

**Open questions:** none - all product decisions are confirmed.

---

## User Stories

### P1: Record and revise a repair forecast ⭐ MVP

**User Story**: As a staff member managing a service order, I want to choose an expected completion date so that the responsible technician and customer have a concrete expectation.

**Why P1**: Without persistence and safe editing, the feature cannot provide a reliable promise.

**Acceptance Criteria**:

1. WHEN a staff member creates an OS THEN the system SHALL offer an optional `Previsão de conclusão` calendar field beside the existing responsible-technician selection. <!-- event-driven -->
2. WHEN an open OS is saved with a valid prediction THEN the system SHALL persist and return its `predictedFinishDate` as an ISO local date (`YYYY-MM-DD`). <!-- event-driven -->
3. WHEN an open OS has no prediction THEN the system SHALL preserve `predictedFinishDate` as `null` and SHALL not block creation or editing. <!-- event-driven -->
4. IF a submitted prediction is malformed, impossible, or earlier than the OS opening date THEN the system SHALL reject the save without changing the persisted order and SHALL present a Portuguese validation message. <!-- unwanted-behavior -->
5. WHILE an OS is `Orçamento`, `Em Manutenção`, or `Aguardando Peça`, the system SHALL allow its prediction to be set, changed, or cleared from the edit sheet. <!-- state-driven -->
6. WHILE an OS is `Finalizada` or `Cancelada`, the system SHALL display its last prediction read-only and SHALL reject direct mutation requests for it. <!-- state-driven -->
7. WHEN a prediction is created, changed, or cleared THEN the system SHALL append one service-order event containing the prior and resulting date values. <!-- event-driven -->

**Independent Test**: Create an OS with a date, edit it to another date, clear it, and verify the returned OS and timeline after each save; attempt an invalid and a finalised-order edit and verify nothing changes.

### P1: Operate from a trustworthy deadline view ⭐ MVP

**User Story**: As a front-desk employee or technician, I want to see due and overdue orders at a glance so that I can prioritize communication and work.

**Why P1**: A saved date is not useful if it is hidden during daily management.

**Acceptance Criteria**:

1. WHEN an OS has a prediction THEN the system SHALL show `Previsão de conclusão: DD/MM/AAAA` in its detail sheet, editor, and generated PDF; WHEN it has no prediction, the PDF SHALL omit that row. <!-- event-driven -->
2. WHEN the OS list returns an open order predicted before the current local date THEN the system SHALL show an `Atrasada` visual indicator and its predicted date without changing its status. <!-- event-driven -->
3. WHEN the OS list returns an open order predicted for the current local date THEN the system SHALL show its predicted date as due today without an overdue indicator. <!-- event-driven -->
4. WHEN the OS list returns an open order with no prediction THEN the system SHALL show `Sem previsão` rather than an empty or invented date. <!-- event-driven -->
5. WHEN an order is finalised THEN the system SHALL retain its prediction for history and SHALL continue to show the separate actual closure date. <!-- event-driven -->

**Independent Test**: Load list, detail, editor, and PDF data for an overdue, due-today, future, unscheduled, and finalised OS; verify each displayed state and that statuses are unchanged.

### P2: Find work by its forecast

**User Story**: As a service manager, I want to filter the paginated OS list by forecast date so that I can plan the queue without downloading every order.

**Why P2**: Queue planning is a strong operational follow-up once the forecast is visible, but P1 already delivers the primary commitment workflow.

**Acceptance Criteria**:

1. WHEN a user applies predicted-date start or end filters THEN the system SHALL request and return only matching records through the existing paginated OS query. <!-- event-driven -->
2. WHEN either predicted-date filter changes THEN the system SHALL reset to page one and include the filters in the query key. <!-- event-driven -->
3. IF the end date precedes the start date THEN the system SHALL prevent the request and SHALL display a Portuguese validation message. <!-- unwanted-behavior -->

**Independent Test**: Filter a multi-page fixture by each boundary and both boundaries, then change a filter on page two and verify the query requests page one.

## Edge Cases

- IF a clean installation or an upgraded database is opened THEN the system SHALL expose the nullable column without changing existing orders' values or breaking startup.
- IF the LAN client retries a failed mutation with its idempotency key THEN the system SHALL apply the prediction change and audit event at most once.
- IF today's date changes while the application is open THEN the next list fetch/render SHALL calculate overdue state from the current local date rather than a date cached at application startup.
- IF a legacy command writes an OS without a prediction THEN the system SHALL persist `null` safely.

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| PFD-01 | P1: Record and revise a repair forecast | Design | Pending |
| PFD-02 | P1: Record and revise a repair forecast | Design | Pending |
| PFD-03 | P1: Record and revise a repair forecast | Design | Pending |
| PFD-04 | P1: Operate from a trustworthy deadline view | Design | Pending |
| PFD-05 | P1: Operate from a trustworthy deadline view | Design | Pending |
| PFD-06 | P2: Find work by its forecast | Design | Pending |
| PFD-07 | P2: Find work by its forecast | Design | Pending |
| PFD-08 | P1: Record and revise a repair forecast | Design | Pending |

**Coverage:** 8 total, 0 mapped to tasks, 8 unmapped pending design.

## Success Criteria

- [ ] A staff member can save or revise an optional predicted finish date for an active OS without altering its stock, total, or status.
- [ ] An overdue active OS is distinguishable from due-today, future, and unscheduled work in the management list.
- [ ] Upgraded and clean databases preserve existing OS records and support the new nullable value.
- [ ] The predicted date and actual closure date cannot be confused in the application or generated PDF.
