# Remote Service-Order Lookups Tasks

## Execution Protocol

Implement these tasks with the `exban-spec-driven` skill. Execute one task at a
time, run its gate before marking it complete, and make one Conventional Commit
that contains the code, tests, and task/spec status updates.

**Design**: Inline in the task definitions; the feature reuses existing
paginated commands and the existing `SearchableSelect` component.
**Status**: Approved

## Test Coverage Matrix

> Generated from `AGENTS.md`, `README.md`, `package.json`, existing colocated
> Vitest tests, and the feature specification. The repository requires focused
> regression coverage plus typecheck, lint, and frontend tests for cross-file
> frontend changes.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| --- | --- | --- | --- | --- |
| Shared React component | unit/component | Open/close callback, remote-search interaction, retained label, loading/error behavior | `src/components/shared/*.test.tsx` | `yarn test src/components/shared/SearchableSelect.test.tsx` |
| Service-order creation view | integration/component | Every P1 acceptance criterion and listed UI edge case, including request arguments and submitted identity | `src/views/ServiceOrderCreate.test.tsx` | `yarn test src/views/ServiceOrderCreate.test.tsx` |
| Type and lint configuration | none | Build gate only | - | `yarn typecheck && yarn lint` |

## Gate Check Commands

| Gate Level | When to Use | Command |
| --- | --- | --- |
| Targeted | One task or one sensor mutation | `yarn test <test-file>` |
| Quick | After component or view task | `yarn typecheck && yarn lint && yarn test <test-file>` |
| Full | After the final frontend task | `yarn typecheck && yarn lint && yarn test` |
| Build | Before handoff | `yarn typecheck && yarn lint && yarn test && yarn build` |

## Execution Plan

### Phase 1: Remote-select lifecycle

```
T1
```

### Phase 2: Service-order creation lookups

```
T1 → T2 → T3
```

## Task Breakdown

### T1: Expose remote lookup lifecycle from the shared selector

**What**: Add an open-state callback to `SearchableSelect` so callers can
enable a remote query only while the selector is open, without changing local
filtering, keyboard navigation, selection, loading, or error behavior.

**Where**: `src/components/shared/SearchableSelect.tsx`
**Test file**: `src/components/shared/SearchableSelect.test.tsx`
**Depends on**: None
**Reuses**: `SearchableSelect:onSearchChange`, `SearchableSelect:selectedLabel`
**Requirement**: RSL-02, RSL-09, RSL-10, RSL-12

**Tools**:

- MCP: NONE
- Skills: `exban-spec-driven`, `codenavi`

**Done when**:

- [ ] The callback reports opening and closing exactly once per state change.
- [ ] Existing remote-search, loading, keyboard, and retained-label tests stay green.
- [ ] A focused component test asserts the callback lifecycle.
- [ ] Gate passes: `yarn typecheck && yarn lint && yarn test src/components/shared/SearchableSelect.test.tsx`.

**Tests**: unit/component
**Gate**: quick
**Commit**: `feat(lookup): expose remote selector lifecycle`
**Status**: Complete

### T2: Move customer reuse to a bounded remote lookup

**What**: Replace the full customer query in `ServiceOrderCreate` with an
open/focus-gated, 300 ms debounced `get_customers_page` query while preserving
customer reuse, the new-customer path, contact-field population, and selected
customer identity handling.

**Where**: `src/views/ServiceOrderCreate.tsx`
**Test file**: `src/views/ServiceOrderCreate.test.tsx`
**Depends on**: T1
**Reuses**: `ServiceOrders` paginated lookup pattern, `useDebounce`,
`get_customers_page`
**Requirement**: RSL-01, RSL-02, RSL-03, RSL-04, RSL-05, RSL-06, RSL-07

**Tools**:

- MCP: NONE
- Skills: `exban-spec-driven`, `codenavi`

**Done when**:

- [ ] Mounting does not call `get_customers` or `get_customers_page`.
- [ ] Focusing the customer lookup requests `{ limit: 20, offset: 0 }`.
- [ ] Typing sends the search only after 300 ms and never calls `get_customers`.
- [ ] Selecting fills contact fields; editing the selected name clears its identity.
- [ ] Customer lookup failure remains local and does not block valid new-customer submission.
- [ ] Focused view tests assert request arguments and the new-customer request shape.
- [ ] Gate passes: `yarn typecheck && yarn lint && yarn test src/views/ServiceOrderCreate.test.tsx`.

**Tests**: integration/component
**Gate**: quick
**Commit**: `feat(service-orders): load customers on demand`

### T3: Move technician selection to a bounded remote lookup

**What**: Replace the full employee query in `ServiceOrderCreate` with an
open-gated, 300 ms debounced `get_users_page` query, retaining the selected
employee label and identifier across result changes, failures, and inline
employee creation.

**Where**: `src/views/ServiceOrderCreate.tsx`
**Test file**: `src/views/ServiceOrderCreate.test.tsx`
**Depends on**: T2
**Reuses**: `SearchableSelect:onOpenChange`, `SearchableSelect:selectedLabel`,
`get_users_page`, `EmployeeCreateSheet:onCreated`
**Requirement**: RSL-08, RSL-09, RSL-10, RSL-11, RSL-12, RSL-13, RSL-14, RSL-15

**Tools**:

- MCP: NONE
- Skills: `exban-spec-driven`, `codenavi`

**Done when**:

- [ ] Mounting does not call `get_users` or `get_users_page`.
- [ ] Opening and searching the employee selector use the bounded paginated command after the debounce.
- [ ] The selected technician remains visible and is submitted after a later search omits it.
- [ ] Lookup failure retains an existing technician and displays a local error.
- [ ] Inline employee creation selects the new employee without a full-list request.
- [ ] Focused view tests cover remote requests, retained selection, failure, and inline creation.
- [ ] Full gate passes: `yarn typecheck && yarn lint && yarn test`.

**Tests**: integration/component
**Gate**: full
**Commit**: `feat(service-orders): load technicians on demand`

## Phase Execution Map

```
Phase 1: T1
Phase 2: T1 → T2 → T3
```

## Task Granularity Check

| Task | Scope | Status |
| --- | --- | --- |
| T1 | Shared selector lifecycle plus colocated component test | ✅ Granular |
| T2 | Customer lookup behavior in one view plus colocated test | ✅ Granular |
| T3 | Technician lookup behavior in one view plus colocated test | ✅ Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| --- | --- | --- | --- |
| T1 | None | No incoming arrow | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T2 | T2 → T3 | ✅ Match |

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| --- | --- | --- | --- | --- |
| T1 | Shared React component | unit/component | unit/component | ✅ OK |
| T2 | Service-order creation view | integration/component | integration/component | ✅ OK |
| T3 | Service-order creation view | integration/component | integration/component | ✅ OK |
