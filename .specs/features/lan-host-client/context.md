# LAN Host/Client Mode Context

**Gathered:** 2026-08-21
**Spec:** `.specs/features/lan-host-client/spec.md`
**Status:** Ready for design

---

## Feature Boundary

This feature delivers LAN host/client mode for the existing Tauri desktop app. One host computer owns the current local SQLCipher SQLite database and attachment directory. Client computers authenticate to the host and use product APIs for shared business data. The feature does not implement SQLite shared-folder mode, sharding, replication, failover, WAN access, or arbitrary SQL routing.

---

## Implementation Decisions

### Host and Client Modes

- Host mode keeps the current local database initialization path and starts an internal LAN API server after storage is ready.
- Client mode stores server URL and token locally and routes business data through the LAN API.
- Client mode allows remote backup download from the host but blocks local restore, import, reset, and destructive storage maintenance for shared data.

### Pairing and Authentication

- Pairing uses a short-lived host-generated code.
- Successful pairing creates a per-device token.
- The host stores token fingerprints, never raw tokens.
- Pairing and health checks enforce the exact same application build version.
- Host mode generates a private TLS certificate locally; clients pin its fingerprint during pairing and require explicit re-pairing if it changes.

### Operation Contracts

- API contracts mirror existing Tauri command input/output shapes using camelCase JSON.
- Mutating requests require idempotency keys.
- The host exposes product operations only; it never exposes arbitrary SQL.

### Failure Behavior

- Clients fail closed when disconnected, block shared-data reads and writes, and do not queue offline operations.
- Host remains usable locally if the LAN server fails to bind.
- Storage initialization failure prevents host API startup.

### Agent's Discretion

- Exact HTTPS framework is open to implementation research; the server and certificate-pinning client run in the Rust backend so the app can trust only the paired self-signed host certificate.
- Exact settings layout can follow the existing Settings page density and card patterns.

### Declined / Undiscussed Gray Areas -> Assumptions

- Discovery defaults to manual URL entry in MVP; mDNS is P2.
- Backup creation can be requested by clients as a remote host-created download; restore, import, reset, and authoritative backup configuration remain host-only.
- Sharding remains documentation-only and unavailable in settings.

---

## Specific References

- Stable branch is `main` at tag `v0.3.2`.
- Current storage path uses `DATABASE_PATH` or app data fallback in `src-tauri/src/database.rs`.
- Current commands already expose product-operation boundaries in `src-tauri/src/commands`.

---

## Deferred Ideas

- mDNS/Bonjour host discovery.
- Multi-host replication or standby restore.
- Customer-sharded SQLite cluster with central inventory and report aggregation.
