# Predicted Service-Order Finish Date Context

**Gathered:** 2026-09-13
**Spec:** `.specs/features/predicted-service-order-finish-date/spec.md`
**Status:** Ready for design

---

## Feature Boundary

Add an optional predicted completion date to service orders, safely persist and audit it, expose it throughout active OS management, and include it in the service-order PDF. It is not a delivery-notification, SLA, authentication, dashboard, reporting, or legal-document project.

---

## Implementation Decisions

### Deadline semantics and lifecycle

- The value is a date-only local business value (`YYYY-MM-DD`), not a timestamp.
- It is optional at creation and may be set, changed, or cleared only for `Orçamento`, `Em Manutenção`, and `Aguardando Peça`.
- A changed value must not precede the OS opening date; invalid requests fail atomically with a Portuguese error.
- Finalised and cancelled OSs retain the last value read-only. The value never changes status, closure time, financial totals, or stock.
- An active OS becomes overdue only after its date has passed; due-today is distinct from overdue.

### Operator and audit behavior

- The responsible technician remains the existing selectable employee; the system has no authenticated current operator, so it will not present unenforceable technician-only permissions.
- Each create/change/clear action writes one immutable timeline event with previous and next values. It cannot name an actor until authenticated identity exists.

### PDF inclusion and content audit

- The PDF will show `Previsão de conclusão: DD/MM/AAAA` in the existing `Aparelho` panel, immediately after `Abertura` and before the conditional actual `Encerramento` row.
- The row is omitted when no forecast exists. Its label intentionally says *previsão*, not *garantia*.
- Current PDF coverage is already appropriate for the available OS data: company identity, customer contacts, OS number/status, equipment/serial, responsible technician, opening/closing dates, description, checklist, chargeable items, totals, signature space, and generation timestamp.
- Potential document gaps—customer authorization/acceptance terms, warranty terms, technician signature, tax fields, and a separate diagnosis field—are deferred. The application does not currently model a legally approved text, tax regime, or distinct diagnostic record, so inventing any of them would create unreliable documentation.

### List and search behavior

- The paginated OS list displays the forecast and an overdue/due-today/unscheduled state.
- Forecast start/end filters follow the existing date-filter pattern, reset pagination, and remain backend-paginated.

## Agent's Discretion

- Choose the smallest accessible visual treatment consistent with the existing Badge and DatePicker components.
- Add an index only if the date-filter query plan or benchmark demonstrates that the existing order index is inadequate; do not pre-optimise a nullable filter.

### Declined / Undiscussed Gray Areas → Assumptions

None. The user approved the defined scope and explicitly required PDF inclusion.
