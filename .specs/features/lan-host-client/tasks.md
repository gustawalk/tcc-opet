# LAN Host/Client Mode Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `exban-spec-driven` skill: activate it by name and follow its Execute flow and Critical Rules. Do not search for skill files by filesystem path. The skill is the source of truth for the full flow: per-task cycle, task status updates, atomic commits, Verifier, discrimination sensor, and final validation.

If the skill cannot be activated, STOP and tell the user - do not proceed without it.

---

**Design**: `.specs/features/lan-host-client/design.md`
**Status**: In progress

---

## Test Coverage Matrix

> Generated from codebase, project guidelines, and spec - confirm before Execute. Guidelines found: user-provided AGENTS instructions, `package.json`, `src-tauri/Cargo.toml`, `.github/workflows/ci.yml`, existing Rust tests in `src-tauri/src/*tests.rs`, and existing Vitest tests in `src/**/*.test.ts(x)`.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| --- | --- | --- | --- | --- |
| Rust storage mode/config | unit | All mode transitions, config validation, path/token persistence, and client-mode no-production-db behavior from ACs | `src-tauri/src/database.rs` module tests or focused module tests | `cd src-tauri && cargo test <test_name> -- --test-threads=1` |
| Rust LAN API/auth/idempotency/TLS | integration | Happy, invalid, unauthorized, revoked, disconnected, timeout-safe, duplicate-key, conflict, plaintext rejection, certificate pinning, and exact-build mismatch cases | `src-tauri/src/tauri_ipc_tests.rs`, new API tests in `src-tauri/src/lan_api.rs` or `src-tauri/src/backend_e2e_tests.rs` | `cd src-tauri && cargo test lan_ -- --test-threads=1` |
| Rust command facade/business workflows | integration | Same outcomes as existing IPC commands for service orders, inventory, reports, PDFs, and attachments; every listed edge case has coverage | `src-tauri/src/tauri_ipc_tests.rs`, `src-tauri/src/backend_e2e_tests.rs` | `cd src-tauri && cargo test e2e_tests tauri_ipc_tests -- --test-threads=1` |
| Frontend data client | unit | Local adapter preserves current invoke args; remote adapter maps success, auth failure, host unreachable, read/write blocking, and validation errors | `src/lib/*.test.ts` | `yarn test src/lib/<file>.test.ts` |
| Frontend Settings/client status UI | unit/component | Mode switching, host status, pairing errors, revoked/disconnected state, remote backup download, and host-only destructive storage controls | `src/views/Settings.test.tsx` | `yarn test src/views/Settings.test.tsx` |
| Documentation/planning | none | Build gate only; no runtime behavior | `.specs/features/lan-host-client/*.md`, docs if added | build gate only |

## Gate Check Commands

> Generated from codebase - confirm before Execute.

| Gate Level | When to Use | Command |
| --- | --- | --- |
| Targeted | Discrimination sensor - one mutation, one focused run | Rust: `cd src-tauri && cargo test <test_name> -- --test-threads=1` / Frontend: `yarn test <path/to/file.test.tsx>` |
| Quick | After tasks with focused unit tests only | Rust: `cd src-tauri && cargo test <module_or_test> -- --test-threads=1` / Frontend: `yarn test <path/to/file.test.tsx>` |
| Full | After tasks with API/integration behavior | `cd src-tauri && cargo test e2e_tests tauri_ipc_tests -- --test-threads=1` and `yarn test` |
| Build | After phase completion or config/entity-only tasks | `yarn lint && yarn typecheck && yarn test && cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib` |

---

## Execution Plan

Phases are ordered and run sequentially. Tasks within a phase execute in order.

### Phase 1: Runtime Mode Foundation

```text
T1 -> T2 -> T3 -> T4
```

### Phase 2: Host API Foundation

```text
T4 -> T5 -> T6 -> T7 -> T8
```

### Phase 3: API Operation Coverage

```text
T8 -> T9 -> T10 -> T11 -> T12
```

### Phase 4: Frontend Client Integration

```text
T12 -> T13 -> T14 -> T15 -> T16
```

### Phase 5: Hardening and Documentation

```text
T16 -> T17 -> T18
```

---

## Task Breakdown

### T1: Define Runtime Mode Configuration

**What**: Add persisted Local/Host/Client runtime mode configuration with validation and defaults.
**Where**: `src-tauri/src/database.rs`
**Depends on**: None
**Reuses**: Existing `get_database_path`, app data directory, serde config, and database module test style.
**Requirement**: LANSRV-01, LANSRV-02

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [x] `StorageModeConfig` supports local, host, and client settings.
- [x] Client settings persist the pinned host certificate fingerprint and never persist a host private key.
- [x] Invalid client URLs and invalid ports are rejected before save.
- [x] Existing local mode behavior remains the default.
- [x] Focused Rust tests cover defaults, round-trip persistence, invalid URL, invalid port, and backward-compatible local startup.
- [x] Gate check passes: `cd src-tauri && cargo test storage_mode -- --test-threads=1`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(lan): add runtime mode configuration`

### T2: Split Client-Only Startup From Local Storage Startup

**What**: Make startup choose between local/host database initialization and client-only configuration initialization.
**Where**: `src-tauri/src/database.rs`
**Depends on**: T1
**Reuses**: Existing `init_db`, `initialize_storage_at`, `get_db`, `database_path`, and storage lock tests.
**Requirement**: LANSRV-01, LANSRV-02

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [x] Local and Host modes still acquire the storage lock and initialize the SQLCipher database.
- [x] Client mode does not open, migrate, seed, reset, or lock the production SQLite database.
- [x] Client mode exposes enough app-data configuration for Settings and client adapter startup.
- [x] Focused Rust tests prove client mode avoids production DB open and local mode remains unchanged.
- [x] Gate check passes: `cd src-tauri && cargo test storage_mode -- --test-threads=1`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(lan): separate client startup from local storage`

### T3: Add Settings Commands for LAN Mode Configuration

**What**: Add Tauri commands to read/update runtime mode config and report host/client status.
**Where**: `src-tauri/src/commands/settings_commands.rs`
**Depends on**: T2
**Reuses**: Existing settings command serialization and IPC test patterns.
**Requirement**: LANSRV-01, LANSRV-02, LANSRV-04

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [x] Commands expose camelCase config for Local, Host, and Client modes.
- [x] Client mode rejects local storage maintenance command attempts with host-only errors.
- [x] IPC tests cover config round-trip and host-only rejection shape.
- [x] Gate check passes: `cd src-tauri && cargo test tauri_ipc_tests -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): expose runtime mode settings commands`

### T4: Add Database Schema for LAN Devices and Idempotency

**What**: Add migrations and repositories for paired devices and idempotency records.
**Where**: `src-tauri/src/database.rs`
**Depends on**: T3
**Reuses**: Existing migration style, timestamp conventions, and repository tests.
**Requirement**: LANSRV-02, LANSRV-03, LANSRV-05

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [x] `lan_devices` stores device name, token fingerprint, created time, last seen, and revoked time.
- [x] `lan_idempotency_records` stores key, route, body hash, status, response JSON, and timestamps.
- [x] Migrations are idempotent and included in reset/backup validation table expectations.
- [x] Rust tests cover schema creation, duplicate idempotency key lookup, body mismatch detection, and device revocation state.
- [x] Gate check passes: `cd src-tauri && cargo test lan_ -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): add device and idempotency schema`

### T5: Implement LAN Authentication Service

**What**: Implement pairing-code issuance, token creation, token fingerprint validation, last-seen updates, and revocation checks.
**Where**: `src-tauri/src/lan_auth.rs`
**Depends on**: T4
**Reuses**: Existing encryption metadata authentication patterns and error mapping style.
**Requirement**: LANSRV-02, LANSRV-05

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [x] Pairing codes expire after a configured TTL.
- [x] Successful pairing stores only token fingerprints.
- [x] Authenticated requests update last-seen time.
- [x] Revoked devices are rejected.
- [x] Tests cover valid pairing, expired code, invalid code, token auth, revoked token, and no raw-token persistence.
- [x] Gate check passes: `cd src-tauri && cargo test lan_auth -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): add device pairing authentication`

### T6: Implement Idempotency Service

**What**: Implement request deduplication for mutating LAN API calls.
**Where**: `src-tauri/src/lan_idempotency.rs`
**Depends on**: T5
**Reuses**: Existing SQLCipher connection and AppError patterns.
**Requirement**: LANSRV-03

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [x] First mutating request reserves the idempotency key for a route and body hash.
- [x] Completed duplicate request returns the original response.
- [x] Same key with different body is rejected.
- [x] Failed in-progress requests do not permanently poison the key.
- [x] Tests cover same-body replay, body conflict, cleanup on failure, and concurrent duplicate attempts.
- [x] Gate check passes: `cd src-tauri && cargo test lan_idempotency -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): add idempotent request handling`

### T7: Add Host LAN API Runtime

**What**: Add the pinned-TLS server/client runtime that binds the configured host address and exposes health, pairing, and authenticated route scaffolding.
**Where**: `src-tauri/src/lan_api.rs`
**Depends on**: T6
**Reuses**: Existing AppError serialization, app startup pattern in `src-tauri/src/lib.rs`, and Rust integration tests.
**Requirement**: LANSRV-01, LANSRV-02

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Host mode starts the LAN API only after storage initialization succeeds.
- [ ] Host mode generates and privately persists a self-signed TLS identity without internet or manual certificate setup.
- [ ] Plaintext requests and clients presenting a non-matching pinned certificate fingerprint are rejected before credentials or payloads are sent.
- [ ] Port bind failure leaves local app usable and status reports the bind error.
- [ ] Health endpoint returns app version, API version, mode, server time, and database readiness.
- [ ] Pairing endpoint exchanges valid code for token.
- [ ] Health and pairing reject clients whose exact application build version differs from the host.
- [ ] Auth middleware rejects missing, invalid, and revoked tokens.
- [ ] Tests cover startup success, bind failure, TLS identity persistence, plaintext rejection, changed-certificate rejection, exact-build mismatch, health, pairing, and auth rejection.
- [ ] Gate check passes: `cd src-tauri && cargo test lan_api -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): start authenticated host api`

### T8: Extract Shared Command Facade

**What**: Extract reusable product operation functions so Tauri commands and LAN routes call the same business logic.
**Where**: `src-tauri/src/commands/mod.rs`
**Depends on**: T7
**Reuses**: Existing command modules and `*_with_conn` repository functions.
**Requirement**: LANSRV-03, LANSRV-07

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Existing Tauri commands delegate to shared facade functions without changing IPC payloads.
- [ ] No business SQL is duplicated for LAN API routes.
- [ ] Existing IPC contract test remains green.
- [ ] Gate check passes: `cd src-tauri && cargo test tauri_ipc_tests -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `refactor(lan): share command handlers with api`

### T9: Add LAN Routes for Customers, Users, Inventory, and Checklist Templates

**What**: Expose authenticated LAN API routes for non-service-order CRUD/list workflows.
**Where**: `src-tauri/src/lan_api.rs`
**Depends on**: T8
**Reuses**: Shared command facade, page contracts, and repository validation.
**Requirement**: LANSRV-03

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Routes cover paginated customers, users, inventory items, inventory summary, inventory stock mutations, and checklist templates.
- [ ] Routes preserve camelCase request and response contracts.
- [ ] Mutating routes require idempotency keys.
- [ ] Tests cover happy path, validation error, unauthorized request, and idempotent replay for at least one mutation per resource group.
- [ ] Gate check passes: `cd src-tauri && cargo test lan_api -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): expose catalog api routes`

### T10: Add LAN Routes for Service Orders and Attachments

**What**: Expose authenticated LAN API routes for service order lifecycle, parts, checklist, events, attachments, PDFs, and previews.
**Where**: `src-tauri/src/lan_api.rs`
**Depends on**: T9
**Reuses**: `create_full_service_order_with_conn`, attachment service, PDF service, and existing backend E2E scenarios.
**Requirement**: LANSRV-03

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Client can create a full service order with parts and checklist through the host API.
- [ ] Client can upload, read, export, and delete service order attachments through the host API.
- [ ] Client can transition status and update parts through the host API.
- [ ] Client can request service order PDF preview/save behavior equivalent to local mode.
- [ ] Tests cover service order creation with stock decrement, attachment rollback on failure, status transition, idempotent retry, and unauthorized write.
- [ ] Gate check passes: `cd src-tauri && cargo test lan_api -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): expose service order api routes`

### T11: Add LAN Routes for Dashboard, Reports, and Remote Backup Rules

**What**: Expose read routes for dashboard/reports, allow client-requested host-created backup downloads, and enforce host-only behavior for destructive storage maintenance.
**Where**: `src-tauri/src/lan_api.rs`
**Depends on**: T10
**Reuses**: Dashboard repository, financial report repository, backup service, settings commands, and storage guard.
**Requirement**: LANSRV-03, LANSRV-04

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Dashboard and financial report routes return equivalent data to local commands.
- [ ] Host-created backup export can be requested by an authenticated client and streamed back without exposing database files directly.
- [ ] Restore, import, reset, and authoritative backup configuration routes reject client-origin calls where required by spec.
- [ ] Host-side backup/export/restore still uses existing exclusive storage guard.
- [ ] Tests cover dashboard/report equivalence, client remote backup export, and host-only destructive operation rejection.
- [ ] Gate check passes: `cd src-tauri && cargo test lan_api -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `feat(lan): expose reporting and backup api rules`

### T12: Add End-to-End LAN API Contract Test

**What**: Add a host API integration test that mirrors the current IPC contract workflow.
**Where**: `src-tauri/src/tauri_ipc_tests.rs`
**Depends on**: T11
**Reuses**: Existing `core_commands_preserve_the_frontend_ipc_contract` scenario.
**Requirement**: LANSRV-03, LANSRV-04

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Test pairs a client, creates/restocks inventory, creates a full service order, finalizes it, reads dashboard, reads financial report, and verifies totals.
- [ ] Test proves repeated idempotency key does not duplicate the order.
- [ ] Test proves unauthenticated and revoked-token requests fail.
- [ ] Test proves host shutdown or unreachable host blocks client shared-data reads and writes.
- [ ] Test proves plaintext transport, a changed host certificate, and a different app build are rejected before business operations run.
- [ ] Gate check passes: `cd src-tauri && cargo test tauri_ipc_tests -- --test-threads=1`

**Tests**: integration
**Gate**: full

**Commit**: `test(lan): cover host api contract workflow`

### T13: Add Frontend Data Client Abstraction

**What**: Add typed local and remote adapters for product operations used by the UI, with remote requests delegated to a Rust pinned-TLS transport command.
**Where**: `src/lib/data-client.ts`
**Depends on**: T12
**Reuses**: `src/lib/types.ts`, current `invoke` argument shapes, and React Query call sites.
**Requirement**: LANSRV-02, LANSRV-03

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Local adapter calls Tauri commands with unchanged command names and args.
- [ ] Remote adapter invokes the Rust pinned-TLS transport with bearer token, pinned fingerprint, exact app build, idempotency keys for mutations, and typed error mapping.
- [ ] Unit tests cover local invoke args, remote success, unauthorized, host unreachable read/write blocking, certificate mismatch, version mismatch, validation error, and idempotency header.
- [ ] Gate check passes: `yarn test src/lib/data-client.test.ts`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(lan): add frontend data client`

### T14: Migrate Core Views to Data Client

**What**: Route customers, users, inventory, service orders, dashboard, reports, and shared sheets through the data client.
**Where**: `src/App.tsx`
**Depends on**: T13
**Reuses**: Existing React Query keys, pagination pattern, and view tests.
**Requirement**: LANSRV-03

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Core views no longer call `invoke` directly for shared business data.
- [ ] Existing query keys and invalidation behavior remain stable.
- [ ] Frontend tests cover representative local-mode and client-mode calls.
- [ ] Gate check passes: `yarn test`

**Tests**: unit
**Gate**: full

**Commit**: `refactor(lan): route core views through data client`

### T15: Add Settings UI for Local, Host, and Client Modes

**What**: Add Settings controls for runtime mode, host URL/port/status, pairing code, client pairing, and connection status.
**Where**: `src/views/Settings.tsx`
**Depends on**: T14
**Reuses**: Existing Settings section patterns, toast handling, and Testing Library tests.
**Requirement**: LANSRV-01, LANSRV-02, LANSRV-04

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] User can choose Local, Host, or Client mode.
- [ ] Host mode shows URL, port, status, and pairing code.
- [ ] Client mode validates URL, accepts pairing code, shows connection status, and stores pairing result.
- [ ] Settings explains that traffic is encrypted and shows the paired host certificate fingerprint without exposing private key material.
- [ ] Client mode allows manual remote backup download and optional scheduled remote backup download through the host.
- [ ] Client mode hides or disables reset, restore, import, and authoritative backup configuration controls with host-only explanation.
- [ ] Tests cover host setup, invalid URL, invalid pairing, successful pairing, encrypted-status display, certificate mismatch, version mismatch, disconnected state, remote backup download, and host-only destructive storage messaging.
- [ ] Gate check passes: `yarn test src/views/Settings.test.tsx`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(lan): add host client settings ui`

### T16: Add Device Management UI

**What**: Add host-side paired device list, last-seen display, and revoke action.
**Where**: `src/views/Settings.tsx`
**Depends on**: T15
**Reuses**: Existing table/button/dialog patterns and Settings tests.
**Requirement**: LANSRV-05

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Host user can see paired devices with name, created time, last seen, and revoked state.
- [ ] Host user can revoke a device after confirmation.
- [ ] Revoked clients show disconnected or revoked status on next call.
- [ ] Tests cover rendering, revoke confirmation, API call, and revoked client state.
- [ ] Gate check passes: `yarn test src/views/Settings.test.tsx`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(lan): add paired device management`

### T17: Document Host/Client Operations and Sharding Boundary

**What**: Add user and maintainer documentation for LAN setup, host-only operations, security assumptions, and deferred sharding constraints.
**Where**: `docs/guides/lan-host-client.md`
**Depends on**: T16
**Reuses**: Existing guide style in `docs/guides`.
**Requirement**: LANSRV-04, LANSRV-07

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] Guide explains Local, Host, and Client modes.
- [ ] Guide explains pairing, device revocation, client remote backup download, restore/reset ownership, and disconnected read/write blocking.
- [ ] Maintainer section lists central tables, future shard candidates, and current sharding blockers.
- [ ] Build gate passes.

**Tests**: none
**Gate**: build

**Commit**: `docs(lan): document host client operations`

### T18: Run Full Hardening Gate and Update Release Notes

**What**: Run full project gates, fix regressions, and add release-note documentation for the LAN feature.
**Where**: `docs/releases/v0.4.0.md`
**Depends on**: T17
**Reuses**: Existing release note style and CI command ladder.
**Requirement**: LANSRV-01, LANSRV-02, LANSRV-03, LANSRV-04, LANSRV-05, LANSRV-07

**Tools**:

- MCP: NONE
- Skill: exban-spec-driven

**Done when**:

- [ ] `yarn lint` passes.
- [ ] `yarn typecheck` passes.
- [ ] `yarn test` passes.
- [ ] `cd src-tauri && cargo fmt --all -- --check` passes.
- [ ] `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cd src-tauri && cargo test --lib` passes.
- [ ] Release notes include host/client mode, security limitations, and deferred sharding note.

**Tests**: none
**Gate**: build

**Commit**: `docs(release): note lan host client mode`

---

## Phase Execution Map

```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5

Phase 1:  T1 -> T2 -> T3 -> T4
Phase 2:  T4 -> T5 -> T6 -> T7 -> T8
Phase 3:  T8 -> T9 -> T10 -> T11 -> T12
Phase 4:  T12 -> T13 -> T14 -> T15 -> T16
Phase 5:  T16 -> T17 -> T18
```

## Task Granularity Check

| Task | Scope | Status |
| --- | --- | --- |
| T1 | One config model/default/validation unit | Granular |
| T2 | One startup-mode separation | Granular |
| T3 | One settings command surface | Granular |
| T4 | One schema/repository foundation | Granular |
| T5 | One authentication service | Granular |
| T6 | One idempotency service | Granular |
| T7 | One host API runtime scaffold | Granular |
| T8 | One shared command facade refactor | Granular |
| T9 | One catalog route group | Granular |
| T10 | One service-order route group | Granular |
| T11 | One reporting/storage route group | Granular |
| T12 | One end-to-end API contract test | Granular |
| T13 | One frontend data client | Granular |
| T14 | One UI data access migration | Cohesive multi-view refactor |
| T15 | One Settings mode UI slice | Granular |
| T16 | One Settings device management slice | Granular |
| T17 | One guide document | Granular |
| T18 | One hardening/release-note closeout | Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| --- | --- | --- | --- |
| T1 | None | None | Match |
| T2 | T1 | T1 -> T2 | Match |
| T3 | T2 | T2 -> T3 | Match |
| T4 | T3 | T3 -> T4 | Match |
| T5 | T4 | T4 -> T5 | Match |
| T6 | T5 | T5 -> T6 | Match |
| T7 | T6 | T6 -> T7 | Match |
| T8 | T7 | T7 -> T8 | Match |
| T9 | T8 | T8 -> T9 | Match |
| T10 | T9 | T9 -> T10 | Match |
| T11 | T10 | T10 -> T11 | Match |
| T12 | T11 | T11 -> T12 | Match |
| T13 | T12 | T12 -> T13 | Match |
| T14 | T13 | T13 -> T14 | Match |
| T15 | T14 | T14 -> T15 | Match |
| T16 | T15 | T15 -> T16 | Match |
| T17 | T16 | T16 -> T17 | Match |
| T18 | T17 | T17 -> T18 | Match |

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| --- | --- | --- | --- | --- |
| T1 | Rust storage mode/config | unit | unit | OK |
| T2 | Rust storage mode/config | unit | unit | OK |
| T3 | Rust command/settings integration | integration | integration | OK |
| T4 | Rust schema/repository | integration | integration | OK |
| T5 | Rust LAN auth | integration | integration | OK |
| T6 | Rust idempotency | integration | integration | OK |
| T7 | Rust LAN API runtime | integration | integration | OK |
| T8 | Rust command facade/business workflows | integration | integration | OK |
| T9 | Rust LAN API routes | integration | integration | OK |
| T10 | Rust LAN API routes and attachments | integration | integration | OK |
| T11 | Rust reporting/storage API | integration | integration | OK |
| T12 | Rust API contract test | integration | integration | OK |
| T13 | Frontend data client | unit | unit | OK |
| T14 | Frontend data access migration | unit | unit | OK |
| T15 | Frontend Settings UI | unit | unit | OK |
| T16 | Frontend Settings UI | unit | unit | OK |
| T17 | Documentation | none | none | OK |
| T18 | Documentation/hardening | none | none | OK |

## Execution Note

This plan has 18 tasks. During Execute, the skill should offer batch sub-agents before implementation because the task count exceeds one task-budgeted batch.
