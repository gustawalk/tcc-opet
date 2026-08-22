# LAN Host/Client Mode Validation

**Date**: 2026-08-21
**Spec**: `.specs/features/lan-host-client/spec.md`
**Diff range**: `main..experimental/lan-host-client`
**Verifier**: fresh-eyes verification pass (independent sub-agent unavailable in this environment)

---

## Validation: LAN Host/Client Mode - PASS

## Task Completion

| Tasks | Status | Notes |
| --- | --- | --- |
| T1-T18 | Done | All task criteria and gates completed; LAN discovery remains the explicitly deferred LANSRV-06 scope. |

## Spec-Anchored Acceptance Criteria

| Criterion | Spec-defined outcome | `file:line` + assertion | Result |
| --- | --- | --- | --- |
| LANSRV-01 host startup/storage/status | Host starts after local storage, reports endpoint, and bind failure preserves local use | `src-tauri/src/lan_api.rs:1099` - `assert!(crate::database::get_db().is_ok())`; `src/views/Settings.test.tsx:271` - host URL/status/code assertions | PASS |
| LANSRV-01 private offline TLS identity | Generated identity persists and private key is mode 0600 | `src-tauri/src/lan_api.rs:1072` - identity equality; `src-tauri/src/lan_api.rs:1095` - `assert_eq!(mode, 0o600)` | PASS |
| LANSRV-02 pairing and local credential persistence | Valid code stores client URL/token/pin; invalid/expired code fails | `src-tauri/src/lan_auth.rs:187` - fingerprint persisted, raw token absent; `src-tauri/src/lan_auth.rs:218` and `:238` - exact errors | PASS |
| LANSRV-02 no local production DB in Client | Client startup creates no DB, metadata, attachment directory, or lock | `src-tauri/src/database.rs:1263` - storage/DB/path absence assertions | PASS |
| LANSRV-02 URL, pin, version and readiness | HTTPS required; changed pin and build mismatch rejected; remote errors remain visible | `src-tauri/src/database.rs:1222`; `src-tauri/src/lan_api.rs:1075`; `src-tauri/src/lan_auth.rs:247`; `src/lib/data-client.test.ts:68` | PASS |
| LANSRV-03 product workflow parity | HTTPS contract performs authenticated catalog/order/dashboard/report operations | `src-tauri/src/lan_api.rs:1151` - full host API workflow with precise response assertions | PASS |
| LANSRV-03 client routing | Client reads/writes use remote transport and stable idempotency keys | `src/lib/data-client.test.ts:48` and `:61` - exact invoke payload assertions | PASS |
| LANSRV-03 retry safety | Same key/body replays original result without re-executing operation | `src-tauri/src/lan_idempotency.rs:140` - duplicate closure panics if called; `:145` - replay equals original | PASS |
| LANSRV-03 concurrent duplicate safety | In-progress duplicate is rejected and first operation completes once | `src-tauri/src/lan_idempotency.rs:214` and `:224` - duplicate non-execution and exact error | PASS |
| LANSRV-03 disconnected fail-closed behavior | Connection errors propagate and UI states reads/writes are blocked | `src/lib/data-client.test.ts:78`; `src/views/Settings.test.tsx:298` | PASS |
| LANSRV-04 remote backup | Host-created encrypted package downloads to client-selected path | `src/views/Settings.test.tsx:305` - exact destination/passphrase invocation | PASS |
| LANSRV-04 destructive host ownership | Client hides import/reset and explains host-only ownership | `src/views/Settings.test.tsx:299` - explanation plus absent controls | PASS |
| LANSRV-05 device records and revocation | Raw token is absent, last-seen updates, revoked token fails exactly | `src-tauri/src/lan_auth.rs:198`; `src-tauri/src/lan_auth.rs:277` | PASS |
| LANSRV-05 host management UI | Host lists device and invokes confirmed revocation | `src/views/Settings.test.tsx:279` - exact `revoke_lan_device` call | PASS |
| LANSRV-07 sharding boundary | API exposes product commands and guide documents central/shardable blockers | `src-tauri/src/lan_api.rs:716` - unknown/destructive operations rejected; `docs/guides/lan-host-client.md:98` - sharding boundary | PASS |

**Status**: All implemented-scope acceptance criteria have evidence. LANSRV-06 discovery is deferred and was not implemented.

## Discrimination Sensor

| Mutation | File:line | Description | Command | Killed? |
| --- | --- | --- | --- | --- |
| 1 | `src-tauri/src/lan_auth.rs:71` | Accept mismatched builds and reject equal builds | `cargo test lan_auth_rejects_different_client_build -- --test-threads=1` | Killed |
| 2 | `src-tauri/src/lan_auth.rs:149` | Invert revoked-device check | `cargo test lan_auth_rejects_revoked_token -- --test-threads=1` | Killed |
| 3 | `src-tauri/src/lan_api.rs:1026` | Invert certificate fingerprint comparison | `cargo test lan_api_tls_identity_persists_and_changed_certificate_is_rejected -- --test-threads=1` | Killed |
| 4 | `src-tauri/src/lan_idempotency.rs:37` | Treat first reservation as duplicate | `cargo test lan_idempotency_replays_completed_response_without_repeating_operation -- --test-threads=1` | Killed |
| 5 | `src-tauri/src/database.rs:281` | Initialize production storage in Client mode | `cargo test storage_mode_client_startup_does_not_touch_production_storage -- --test-threads=1` | Killed |

**Tier / budget**: full, auth and data integrity - 5 of 8 allowed.
**Result**: 5/5 killed. Disposable worktree removed; real-tree status matched the baseline.

## Code Quality

| Principle | Status |
| --- | --- |
| Product-operation API only; no arbitrary SQL transport | PASS |
| Client never receives host filesystem paths or private key material | PASS |
| Existing command facade and React Query patterns reused | PASS |
| No automatic writable local fallback | PASS |
| Tests map to requirements and assert precise outcomes | PASS |
| Project guidelines followed: user-provided `AGENTS.md` instructions and CI commands | PASS |

## Edge Cases

- [x] Host exit/unreachable blocks client reads and writes.
- [x] Concurrent writes retain SQLite transaction behavior; duplicate requests are idempotent.
- [x] Attachment failure cleanup uses the existing attachment transaction workflow.
- [x] Host timestamps remain authoritative.
- [x] Request payload limits reject oversized bodies before dispatch.
- [x] Exact build mismatch rejects pairing and health.
- [x] Plaintext and changed certificates are rejected before authenticated operations.

## Gate Check

- **Frontend**: `yarn lint && yarn typecheck && yarn test` - 22 files, 86 tests passed, 0 failed.
- **Backend**: `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --lib -- --test-threads=1` - 205 passed, 0 failed, 2 ignored stress tests.
- **Build**: `yarn build` passed; largest production chunk remained below 500 KB.
- **Before feature**: 187 backend and 68 frontend tests.
- **Delta**: +18 backend and +18 frontend tests; no existing tests removed or weakened.

## Requirement Traceability Update

| Requirement | Result |
| --- | --- |
| LANSRV-01 | Verified |
| LANSRV-02 | Verified |
| LANSRV-03 | Verified |
| LANSRV-04 | Verified |
| LANSRV-05 | Verified |
| LANSRV-06 | Deferred / out of this implementation |
| LANSRV-07 | Verified |

## Summary

**Overall**: Ready

**Spec-anchored check**: all implemented-scope criteria matched expected outcomes.
**Sensor**: 5/5 mutations killed.
**Gate**: 291 automated tests passed; 2 intentional stress tests ignored.
**Issues found**: none remaining. Interactive desktop UAT on two physical LAN computers remains the release-candidate exercise described in the guide.
