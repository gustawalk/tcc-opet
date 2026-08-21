# Spec-Driven State

## Decisions

| ID | Status | Decision | Rationale | Date |
| --- | --- | --- | --- | --- |
| AD-001 | active | LAN multi-user access uses a host-owned API server before any shared-folder or sharded SQLite architecture. | Stable `main` uses one SQLCipher connection, one database file, one attachment directory, and one process lock. A host API preserves SQLite local-file semantics and avoids network filesystem locking failures. | 2026-08-21 |
| AD-002 | active | The first LAN version exposes product operations, not arbitrary SQL. | Existing Tauri commands already define business operation boundaries; exposing SQL would bypass validation, authorization, idempotency, and domain transactions. | 2026-08-21 |
| AD-003 | active | Sharding is explicitly deferred until host/client mode is stable and measured. | Current service order, inventory, reporting, backup, and display-ID flows require one complete database view. Sharding would introduce distributed ownership and transaction rules before the simpler LAN requirement is solved. | 2026-08-21 |
| AD-004 | active | LAN traffic uses a host-generated TLS certificate pinned by clients during pairing, and host/client builds must match exactly. | The deployment must remain fully offline while encrypting every transaction; certificate pinning avoids public CA and manual trust-store setup, while exact build equality prevents operation-contract drift. | 2026-08-21 |

## Handoff

No active implementation handoff.
