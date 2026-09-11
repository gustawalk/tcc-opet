# Remote Service-Order Lookups Specification

## Problem Statement

`ServiceOrderCreate` loads every customer and employee before the user needs a
lookup. This makes opening the service-order form increasingly expensive as the
database grows, despite paginated search commands and an established remote
lookup pattern already existing elsewhere in the application.

## Goals

- [x] Prevent service-order creation from transferring complete customer or
  employee lists.
- [x] Preserve the existing customer-reuse and technician-selection workflows.
- [x] Make lookup loading and failure states understandable without blocking a
  valid new-customer service order.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Remote checklist-template and inventory lookups | This roadmap item is limited to customer and employee lists. |
| New backend search commands | `get_customers_page` and `get_users_page` already provide the required contract. |
| Numeric pagination inside dropdowns | A debounced, bounded lookup is sufficient for operational selection. |
| Changes to customer, employee, or service-order persistence rules | The feature changes lookup transport and presentation only. |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Initial lookup | Request up to 20 records only after the relevant lookup is opened or focused. | Avoids mount-time list transfer while still offering immediate choices. | y |
| Search delay | Debounce remote lookup text by 300 ms. | Matches existing list-page behavior. | y |
| Customer lookup failure | Show a local lookup error but keep the new-customer form usable. | A network or query failure must not turn a typed new customer into a blocked form. | n |
| Employee lookup failure | Show a local lookup error and retain an already selected employee; the technician remains optional. | Current service-order creation permits no technician. | n |
| Selected record lifetime | Keep the selected customer or employee label and identifier after a new lookup result replaces its source list. | A valid selection must not disappear because search results change. | y |
| Remaining dimensions | Input bounds, idempotency, authorization, concurrency, lifecycle, and observability are N/A for this UI-only use of existing read commands. | The feature adds no persistence, mutation, endpoint, or authorization behavior. | y |

**Open questions:** none - all resolved or logged above.

---

## User Stories

### P1: Reuse a customer without downloading every customer

**User Story**: As a service-order creator, I want to search existing customers
on demand so that I can reuse their contact data without loading the full
customer database.

**Why P1**: Customer reuse is a core part of service-order creation and is the
largest remaining full-list transfer in that form.

**Acceptance Criteria**:

1. WHEN `ServiceOrderCreate` renders THEN the system SHALL not invoke `get_customers`.
2. WHILE the customer lookup is closed and has no active search THEN the system SHALL not invoke `get_customers_page`.
3. WHEN the customer lookup is opened or focused THEN the system SHALL request `get_customers_page` with `limit: 20` and `offset: 0`.
4. WHEN customer lookup text remains unchanged for 300 ms THEN the system SHALL request `get_customers_page` with that text as `search`.
5. WHEN a customer is selected THEN the system SHALL retain that customer's identifier and populate the existing contact fields.
6. WHEN a selected customer's name is edited THEN the system SHALL clear the selected customer identifier before a service order can be saved.
7. IF customer lookup fails THEN the system SHALL display a customer-lookup error without preventing creation of a valid new customer.

**Independent Test**: Open the form, confirm no full customer request occurs,
focus and search for a customer outside the first 20 results, select it, then
edit its name and verify that save uses the new-customer path.

---

### P1: Select a technician through a bounded remote lookup

**User Story**: As a service-order creator, I want to search employees on demand
so that technician selection remains usable with a large employee database.

**Why P1**: The employee selector currently transfers every employee despite an
existing paginated backend command.

**Acceptance Criteria**:

1. WHEN `ServiceOrderCreate` renders THEN the system SHALL not invoke `get_users`.
2. WHILE the employee selector is closed and has no active search THEN the system SHALL not invoke `get_users_page`.
3. WHEN the employee selector opens THEN the system SHALL request `get_users_page` with `limit: 20` and `offset: 0`.
4. WHEN employee lookup text remains unchanged for 300 ms THEN the system SHALL request `get_users_page` with that text as `search`.
5. WHEN an employee is selected THEN the system SHALL retain the employee identifier and label after later lookup results replace the current options.
6. IF employee lookup fails THEN the system SHALL display an employee-lookup error and preserve any existing technician selection.

**Independent Test**: Open the employee selector, search after the debounce,
select an employee, perform a different search, and verify that the chosen
employee remains visible and is submitted with the service order.

---

### P2: Preserve inline employee creation compatibility

**User Story**: As a service-order creator, I want an employee created from the
form to remain selected so that I can continue without reopening the lookup.

**Why P2**: Inline creation is already available and remote lookup changes must
not regress it.

**Acceptance Criteria**:

1. WHEN inline employee creation succeeds THEN the system SHALL select the newly created employee without requiring a full employee-list request.
2. IF inline employee creation is cancelled or fails THEN the system SHALL preserve the current service-order draft and selected technician.

**Independent Test**: Create an employee from the selector, verify it becomes
the selected technician, then repeat with cancellation and verify the draft is
unchanged.

## Edge Cases

- IF a lookup response arrives after a newer search request THEN the system
  SHALL present results for the latest active query.
- WHEN a selected customer or employee is absent from the current result page
  THEN the system SHALL show its retained label rather than an empty selector.
- IF the user leaves the customer input and its result panel THEN the system
  SHALL close the panel without clearing typed new-customer data.

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| RSL-01 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-02 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-03 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-04 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-05 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-06 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-07 | P1: Customer lookup | Execute (T2) | Verified |
| RSL-08 | P1: Employee lookup | Execute (T3) | Verified |
| RSL-09 | P1: Employee lookup | Execute (T3) | Verified |
| RSL-10 | P1: Employee lookup | Execute (T3) | Verified |
| RSL-11 | P1: Employee lookup | Execute (T3) | Verified |
| RSL-12 | P1: Employee lookup | Execute (T3) | Verified |
| RSL-13 | P1: Employee lookup | Execute (T3) | Verified |
| RSL-14 | P2: Inline employee creation | Execute (T3) | Verified |
| RSL-15 | P2: Inline employee creation | Execute (T3) | Verified |

**Coverage:** 15 total, 0 mapped to tasks, 15 unmapped pending design.

## Success Criteria

- [x] No complete customer or employee list is requested by `ServiceOrderCreate`.
- [x] Both lookups request at most 20 records per search and debounce input by 300 ms.
- [x] Customer reuse, new-customer creation, technician selection, and inline
  employee creation remain functional.
- [x] Focus, keyboard, loading, error, and selected-label behavior have focused
  frontend test coverage.
