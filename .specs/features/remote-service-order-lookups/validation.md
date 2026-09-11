# Remote Service-Order Lookups Validation

## Iteration 1 — historical FAIL (at `d4971f1`)

**Tier:** standard (UI lookup behavior and form wiring).  Independent verifier
pass over `5132921..d4971f1`; the verifier did not author the implementation.

## Structural and build gates

| Check | Result | Evidence |
| --- | --- | --- |
| Spec gate | PASS | `validate_spec.py .specs/features/remote-service-order-lookups`: 0 errors, 0 warnings |
| Task gate | PASS | `validate_tasks.py .specs/features/remote-service-order-lookups`: 0 errors, 0 warnings |
| Diff hygiene | PASS | `git diff --check 5132921..d4971f1`: no output |
| Build gate | PASS | `yarn typecheck && yarn lint && yarn test --pool=forks --maxWorkers=4 --minWorkers=1`: 23 files / 100 tests passed, 0 failed |
| Test integrity | PASS | Feature diff adds 25 selector-test lines and 238 view-test lines; no test deletion or weakened assertion found in the range |

The build gate was run on Node `v26.8.1`, Yarn `1.22.22`.

## Spec-anchored acceptance criteria

| Requirement | Spec-defined outcome | Evidence | Result |
| --- | --- | --- | --- |
| RSL-01 | Mount never invokes `get_customers`. | `src/views/ServiceOrderCreate.test.tsx:89`–`115` asserts no full request; `src/views/ServiceOrderCreate.tsx:74`–`79` only defines the paged command. | PASS |
| RSL-02 | Closed/no-search customer lookup never invokes the paged command. | `src/views/ServiceOrderCreate.tsx:137`–`142` gates query on open state; `src/views/ServiceOrderCreate.test.tsx:94`–`95` asserts no paged call on mount. | PASS |
| RSL-03 | Focus/open requests `{ limit: 20, offset: 0, search: "" }`. | `src/views/ServiceOrderCreate.tsx:74`–`79`, `431`–`436`; exact assertion at `src/views/ServiceOrderCreate.test.tsx:97`–`104`. | PASS |
| RSL-04 | Unchanged customer text requests its search only after 300 ms. | `src/views/ServiceOrderCreate.tsx:65`–`66`, `129`–`142`; test at `src/views/ServiceOrderCreate.test.tsx:106`–`113` only eventually observes the request. Mutation M1 survived. | FAIL |
| RSL-05 | Selection retains id and populates existing contact fields. | Implementation sets customer/id and contact fields at `src/views/ServiceOrderCreate.tsx:185`–`196`; test at `src/views/ServiceOrderCreate.test.tsx:117`–`146` asserts only the later new-customer path, not populated phone/email/address or selected id. | FAIL (no outcome assertion for contact population/id) |
| RSL-06 | Editing a selected name clears its id before save. | `src/views/ServiceOrderCreate.tsx:456`–`464`; `src/views/ServiceOrderCreate.test.tsx:117`–`145` asserts `customerAction.type === "new"` and edited name. | PASS |
| RSL-07 | Customer failure is visible and does not block valid new-customer submission. | Error UI `src/views/ServiceOrderCreate.tsx:477`–`480`; exact error and save assertion `src/views/ServiceOrderCreate.test.tsx:148`–`183`. | PASS |
| RSL-08 | Mount never invokes `get_users`. | `src/views/ServiceOrderCreate.test.tsx:186`–`240` asserts no full user request; paged-only command at `src/views/ServiceOrderCreate.tsx:80`–`85`. | PASS |
| RSL-09 | Closed/no-search employee selector does not invoke the paged command. | `src/views/ServiceOrderCreate.tsx:143`–`148`, `578`–`601`; mount assertion `src/views/ServiceOrderCreate.test.tsx:212`–`213`. | PASS |
| RSL-10 | Opening requests `{ limit: 20, offset: 0, search: "" }`. | Exact assertion `src/views/ServiceOrderCreate.test.tsx:215`–`222`; lifecycle callback assertion `src/components/shared/SearchableSelect.test.tsx:76`–`99`. | PASS |
| RSL-11 | Unchanged employee text requests its search only after 300 ms. | `src/views/ServiceOrderCreate.tsx:65`–`66`, `133`–`148`; test at `src/views/ServiceOrderCreate.test.tsx:228`–`235` only eventually observes the request. M1 also removes this delay without a test failure. | FAIL |
| RSL-12 | Selected employee id and label survive result replacement. | `src/views/ServiceOrderCreate.tsx:581`–`587`, `631`–`635`; result-replacement label assertion `src/views/ServiceOrderCreate.test.tsx:223`–`239`. Mutation M3 was killed. | PASS |
| RSL-13 | Employee failure displays error and retains existing selection. | Error UI `src/views/ServiceOrderCreate.tsx:589`–`592`; test `src/views/ServiceOrderCreate.test.tsx:242`–`267` asserts error only, with no preselected technician or retained-label assertion. | FAIL (retention has no test evidence) |
| RSL-14 | Successful inline creation selects the employee without full list request. | Selection code `src/views/ServiceOrderCreate.tsx:817`–`823`; test `src/views/ServiceOrderCreate.test.tsx:269`–`292` asserts label and no full request. | PASS |
| RSL-15 | Cancel/fail preserves draft and selected technician. | Test `src/views/ServiceOrderCreate.test.tsx:294`–`313` asserts retained technician after cancellation, but does not assert any draft field and has no failed-creation case. | FAIL |

**Spec-anchored result:** 10/15 criteria have exact outcome evidence; 5 are gaps.

## Edge cases

| Edge case | Evidence | Result |
| --- | --- | --- |
| Late response from older query cannot replace active-query results | Query keys include debounced search at `src/views/ServiceOrderCreate.tsx:137`–`148`, but no controlled out-of-order-response test exists. | FAIL (no behavioral evidence) |
| Selected customer/employee absent from result page retains label | Employee is asserted at `src/views/ServiceOrderCreate.test.tsx:228`–`238`; customer input preserves selected name in code at `src/views/ServiceOrderCreate.tsx:185`–`190`, but lacks a page-replacement test. | FAIL (customer coverage missing) |
| Leaving customer input/result panel closes panel without clearing typed new-customer data | Close implementation `src/views/ServiceOrderCreate.tsx:437`–`446`; `src/views/ServiceOrderCreate.test.tsx:73`–`87` checks closure, not typed-value retention. | FAIL (retention missing) |

## Discrimination sensor

Scratch: detached worktree at `/tmp/rsl-lookup-sensor` from `d4971f1`, with
the real-tree porcelain captured before setup. The worktree was removed after
the runs; real-tree porcelain exactly matched that baseline before this report
was written.

| ID | Changed diff line / fault | Targeted command | Result |
| --- | --- | --- | --- |
| M1 | `src/views/ServiceOrderCreate.tsx:66`: `LOOKUP_DEBOUNCE_MS = 300` → `0` | `yarn test src/views/ServiceOrderCreate.test.tsx --pool=forks --maxWorkers=4 --minWorkers=1` | **SURVIVED**: 10/10 passed. The tests do not prove 300 ms. |
| M2 | `src/views/ServiceOrderCreate.tsx:140`: customer `enabled: isCustomerLookupOpen` → `enabled: false` | same | KILLED: 4/10 failed, including expected paged lookup/error flows. |
| M3 | `src/views/ServiceOrderCreate.tsx:587`: selected label → `undefined` | same | KILLED: 2/10 failed, including retained selection and inline-created label. |

Sensor result: **3 injected, 2 killed, 1 survived**. This alone requires a
FAIL at standard tier.

## Code-quality scope check

| Check | Result |
| --- | --- |
| Only feature-required files changed | PASS |
| Existing React Query / selector patterns retained | PASS |
| No unrelated refactor or unnecessary abstraction | PASS |
| Tests are spec-mapped and non-shallow | FAIL — the acceptance/edge-case gaps above remain |

## Ranked gaps

1. Add deterministic fake-timer tests that prove neither lookup request fires before 300 ms and exactly the active query fires at 300 ms (RSL-04, RSL-11; M1 survived).
2. Add controlled deferred-response coverage for latest-query-wins, then assert customer selected id/contact population and label retention across a replaced result page (RSL-05 and two edge cases).
3. Start with a selected technician, reject a subsequent lookup, and assert its label/id remains; cover inline-creation failure/cancel with typed draft fields unchanged (RSL-13, RSL-15).
4. Assert customer typed text remains after focus leaves the panel (edge case).

`validate_state.py` was intentionally not passing in iteration 1. The
following incremental iteration re-checks every prior gap against `24390c4`.

## Validation: Remote Service-Order Lookups - PASS

## Iteration 2 — incremental re-verification (`d4971f1..24390c4`)

**Verdict:** PASS. All five former acceptance gaps and all three listed edge
cases now have implementation plus outcome evidence. The original range
reviewed is `5132921..24390c4`.

| Prior gap / constrained behavior | New evidence | Result |
| --- | --- | --- |
| Customer debounce is exactly 300 ms (RSL-04) | `src/views/ServiceOrderCreate.test.tsx:126`–`148` uses fake timers, asserts no request at 299 ms and the exact paged request at 300 ms; delay wiring is `src/views/ServiceOrderCreate.tsx:65`–`66`, `129`–`142`. | PASS |
| Customer selection fills contacts and retains the existing identity (RSL-05) | `src/views/ServiceOrderCreate.test.tsx:150`–`168` asserts formatted phone, email, address, visible selected name, and `{ type: "existing", id: "customer-1", update: null }`; implementation is `src/views/ServiceOrderCreate.tsx:185`–`196`. | PASS |
| Blur closes without clearing new-customer text (edge case) | `src/views/ServiceOrderCreate.test.tsx:171`–`181` asserts retained text and hidden panel; close path only formats the current text at `src/views/ServiceOrderCreate.tsx:437`–`446`. | PASS |
| Employee debounce is exactly 300 ms (RSL-11) | `src/views/ServiceOrderCreate.test.tsx:308`–`329` uses the same 299/300-ms assertion; wiring is `src/views/ServiceOrderCreate.tsx:65`–`66`, `133`–`148`. | PASS |
| A late response cannot replace the active query (edge case) | Deferred older response and newer-option assertions are `src/views/ServiceOrderCreate.test.tsx:331`–`367`; both lookup query keys include debounced text at `src/views/ServiceOrderCreate.tsx:137`–`148`. | PASS |
| Selected employee survives failed lookup (RSL-13) | `src/views/ServiceOrderCreate.test.tsx:369`–`405` starts selected, rejects lookup, and asserts error plus selected label; retained label is supplied at `src/views/ServiceOrderCreate.tsx:581`–`592`. | PASS |
| Inline cancel/failure preserves technician and draft (RSL-15) | `src/views/ServiceOrderCreate.test.tsx:426`–`467` asserts the selected label and `Equipamento` value after cancellation and rejected creation; creation selection path is `src/views/ServiceOrderCreate.tsx:817`–`823`. | PASS |
| Selected labels outside a current page (edge case) | Employee result replacement is asserted at `src/views/ServiceOrderCreate.test.tsx:291`–`305`; customer selected-name retention is asserted at `src/views/ServiceOrderCreate.test.tsx:155`–`161` and is independent of `customersQuery.data` at `src/views/ServiceOrderCreate.tsx:185`–`190`. | PASS |

All previously passed criteria remain unchanged and carry forward from iteration
1: RSL-01–03, RSL-06–10, RSL-12, and RSL-14. Their cited source/test files did
not change except the additive view-test coverage above.

### Gates

| Check | Result |
| --- | --- |
| `validate_spec.py .specs/features/remote-service-order-lookups` | PASS — 0 errors, 0 warnings |
| `validate_tasks.py .specs/features/remote-service-order-lookups` | PASS — 0 errors, 0 warnings |
| `yarn typecheck && yarn lint && yarn test --pool=forks --maxWorkers=4 --minWorkers=1` | PASS — 23 files / 105 tests, 0 failed |
| `git diff --check 5132921..24390c4` | PASS — no whitespace errors |

### Incremental discrimination sensor

Disposable detached worktree `/tmp/rsl-lookup-reverify` at `24390c4`; real-tree
porcelain was captured before and exactly matched after removal.

| Mutation | Command | Result |
| --- | --- | --- |
| M1 requested literal: `LOOKUP_DEBOUNCE_MS` `300 → 0` | `yarn test src/views/ServiceOrderCreate.test.tsx --pool=forks --maxWorkers=4 --minWorkers=1` | Equivalent, not a behavioral survivor: `src/hooks/use-debounce.ts:7`–`9` evaluates `delay || 300`, so zero still produces 300 ms; 15/15 passed. |
| M1b effective debounce fault: `300 → 1` | same | KILLED: customer and employee 299-ms assertions failed (`src/views/ServiceOrderCreate.test.tsx:139`, `320`); 13/15 passed, 2 failed. |

Sensor conclusion: the literal requested M1 cannot alter behavior because of the
existing hook fallback; the semantically equivalent effective-delay mutation was
killed. No surviving behavior-level mutation remains.

### Quality

The corrective commit only extends focused view-test coverage; implementation
scope and surrounding React Query/select patterns remain unchanged. Every
feature acceptance criterion and listed edge case now has an outcome assertion
or direct state evidence above. No lessons were distilled: iteration 2 has no
grounded failure.
