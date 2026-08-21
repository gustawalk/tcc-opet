# LAN Host/Client Mode Specification

## Problem Statement

The stable application stores all business data in one local SQLCipher SQLite database and protects it with a single process lock. Teams that use multiple computers on the same internal network need to work against one shared operational dataset without relying on SMB/NFS shared database files, because SQLite file locking over network shares is fragile and platform-dependent. This feature introduces a host/client LAN mode where one host process owns the database and other desktop clients access it through authenticated product APIs.

## Goals

- [ ] Let 2-5 computers on the same LAN create, update, and read the same operational data through one host-owned database.
- [ ] Preserve the existing single-file SQLite transaction semantics on the host for service orders, inventory updates, attachments, reports, backup, and restore.
- [ ] Prevent client machines from directly opening or mutating the production database file.
- [ ] Provide clear setup, connection status, and failure feedback for non-technical shop users.
- [ ] Keep SQLite sharding out of the MVP while leaving storage boundaries explicit enough to revisit later.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
| --- | --- |
| Transparent SQL proxy or PostgreSQL wire compatibility | This would require a distributed query engine and does not match the current command/repository architecture. |
| SQLite row/file sharding | Sharding introduces cross-shard transaction, inventory, reporting, and backup problems before the LAN access problem is solved. |
| Multi-host replication or automatic failover | Requires consensus, leader election, and conflict handling; not needed for the first internal LAN version. |
| Internet/WAN access | Security, TLS, NAT traversal, and support requirements are materially different from a trusted internal LAN. |
| Role-based permissions redesign | The first version authenticates devices to the host; user authorization remains the existing app behavior unless separately redesigned. |
| Mobile/web client | The feature targets the existing Tauri desktop app. |

---

## Assumptions & Open Questions

Every ambiguity is resolved or recorded here - nothing is left silently unclear.

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| LAN topology | One desktop machine is configured as Host; other desktop machines are Clients. | This fits the current single SQLite file and avoids distributed database behavior. | n |
| Server transport | HTTPS JSON API bound to a configurable LAN address and port, using a host-generated certificate pinned by clients during pairing. | The feature must work offline without requiring a public certificate authority while still encrypting credentials and business payloads on the LAN. | y |
| Authentication | Pairing code creates per-device bearer tokens stored locally by the client and revocable by the host. | Same-LAN is not enough protection for customer/business data; per-device tokens are a practical MVP boundary. | n |
| API shape | API endpoints mirror existing Tauri command contracts using camelCase JSON. | The frontend and IPC tests already depend on camelCase contracts. | n |
| Write retry handling | Mutating client requests carry idempotency keys. | Network timeouts should not duplicate service orders, inventory movements, or attachments. | n |
| Backup, restore, and reset | Clients may request/export/download host-created encrypted backups, including scheduled remote backup downloads; only the host may restore/import or reset shared data. | Small deployments with up to five users can tolerate client-triggered backup load, but only the host can safely snapshot the database and attachments. Restore/reset replace authoritative storage and must stay host-owned. | n |
| Attachments | Clients upload/download attachments through the host API; the host stores files in its current attachment directory. | Attachment metadata and file storage must stay consistent with host transactions. | n |
| Discovery | MVP supports manual URL entry plus a host screen that shows URL and pairing code; mDNS is P2. | Manual entry is enough to validate the architecture and avoids adding discovery complexity to the first slice. | n |
| Offline behavior | Clients fail closed when the host is unreachable; no offline read/write access to shared business views and no automatic fallback to local writable storage. | Offline reads could show stale data and offline writes require conflict resolution; users need a clear source of truth. | n |
| Transport encryption | Every LAN request uses TLS. The host generates and stores a private certificate, pairing returns its fingerprint through the user-verified pairing flow, and clients reject certificate changes until explicitly re-paired. | The user requires all host/client transactions to remain encrypted without internet access or manual certificate setup. | y |
| Build compatibility | Pairing and normal health checks require the exact same application build version on host and client. | Mixed builds can disagree on operation contracts and database behavior; rejecting them is safer than partial compatibility. | y |

**Open questions:** none - all resolved or logged above.

---

## User Stories

### P1: Configure Host Mode - MVP

**User Story**: As the shop owner, I want one computer to act as the LAN host so that all workstations use the same operational database.

**Why P1**: Without a host, there is no safe shared database entry point.

**Acceptance Criteria**:

1. WHEN a user enables Host mode THEN the system SHALL start a LAN API server after the local database initializes successfully.
2. WHILE Host mode is active the system SHALL keep using the existing local SQLCipher database and attachment directory as the only writable storage.
3. WHEN Host mode starts THEN the system SHALL display the host URL, port, connection status, and pairing code in Settings.
4. IF the host port is unavailable THEN the system SHALL keep the app usable locally and show a clear error that the LAN server did not start.
5. IF the local database fails to initialize THEN the system SHALL not start the LAN server.
6. WHEN Host mode is enabled for the first time THEN the system SHALL generate and privately store a TLS certificate without requiring internet access or manual certificate installation.

**Independent Test**: Configure Host mode, restart the app, confirm the local database works and the Settings page shows an active LAN endpoint.

### P1: Pair Client Device - MVP

**User Story**: As a workstation user, I want to connect this app to the host computer so that I can use the shared data without opening a local production database.

**Why P1**: Client machines must authenticate to the host and avoid local writes.

**Acceptance Criteria**:

1. WHEN a user selects Client mode and enters a host URL plus valid pairing code THEN the system SHALL store the server URL and a per-device token locally.
2. WHILE Client mode is active the system SHALL route business data operations through the host API instead of opening the production SQLite database locally.
3. IF the host URL is invalid THEN the system SHALL reject the configuration before saving it.
4. IF the pairing code is invalid or expired THEN the system SHALL keep the client unpaired and show a clear authentication error.
5. WHEN a paired client starts THEN the system SHALL verify host reachability before showing shared-data views as ready.
6. WHEN a client pairs THEN the system SHALL pin the host certificate fingerprint and reject later connections whose certificate does not match until the device is explicitly re-paired.
7. IF the host and client application build versions differ THEN the system SHALL reject pairing and shared-data access with a clear version-mismatch error.

**Independent Test**: Pair a client with a running host, restart the client, and confirm shared-data screens load from the host.

### P1: Use Core Business Workflows Over LAN - MVP

**User Story**: As a technician or attendant, I want customers, inventory, service orders, dashboard, reports, PDFs, and attachments to work from client machines so that daily operations can happen from multiple workstations.

**Why P1**: The LAN feature is only useful if the core operational workflow works end to end.

**Acceptance Criteria**:

1. WHEN a client creates a full service order THEN the host SHALL create customer updates, service order, service order events, checklist rows, inventory decrements, inventory movements, attachment metadata, and attachment files with the same atomicity as local mode.
2. WHEN a client updates service order status or item quantity THEN the host SHALL apply the existing business rules and return the same success or error shape as local mode.
3. WHEN a client reads paginated lists, dashboard data, financial reports, PDFs, or attachments THEN the host SHALL return data equivalent to local Tauri commands for the same database state.
4. IF a client retries a mutating request with the same idempotency key THEN the host SHALL return the original successful result without applying the mutation twice.
5. IF a client loses connection during a read or write THEN the client SHALL show a recoverable connection error without claiming that the operation succeeded.

**Independent Test**: Use a client to create and finalize an order with one part and one attachment, then verify the host dashboard/report totals and attachment readback.

### P1: Protect Destructive Storage Operations and Allow Remote Backup - MVP

**User Story**: As the shop owner, I want clients to be able to export encrypted backups from the host but not restore, import, or reset shared data so that backup copies can be distributed without risking the authoritative database.

**Why P1**: Backup creation is safe when the host owns the snapshot; restore/reset are destructive and must remain host-only.

**Acceptance Criteria**:

1. WHILE Client mode is active the system SHALL allow manual encrypted backup export by asking the host to create the backup package and streaming the result to the client-selected destination.
2. WHILE Client mode is active the system SHALL allow automatic backup only as a scheduled remote backup download where the host creates the backup and the client stores the resulting package.
3. WHILE Client mode is active the system SHALL hide or disable reset, restore, import, and backup-destination mutation controls that would replace or mutate authoritative host storage.
4. WHEN a client opens Settings THEN the system SHALL explain that restore, import, reset, and authoritative backup configuration are available only on the host computer.
5. WHEN the host creates, exports, or restores a backup THEN the system SHALL use the existing exclusive storage guard and close shared database connection before replacing files.
6. IF a client calls a host-only destructive storage operation through the client API THEN the system SHALL reject it with a host-only error.

**Independent Test**: In Client mode, export a backup file created by the host and verify restore/import/reset are unavailable; in Host mode, export and restore still use the existing paths.

### P2: Manage Devices and Observability

**User Story**: As the shop owner, I want to see connected devices and revoke access so that lost or retired computers no longer access business data.

**Why P2**: Device management is important after the MVP proves host/client operation.

**Acceptance Criteria**:

1. WHEN a device pairs successfully THEN the host SHALL store a device record with name, token fingerprint, creation time, and last-seen time.
2. WHEN the host revokes a device THEN the host SHALL reject future API calls authenticated with that device token.
3. WHEN an authenticated client makes an API call THEN the host SHALL update the device last-seen time without storing the raw bearer token.
4. The system SHALL log host API startup, shutdown, pairing, authentication failure, and device revocation events without logging secrets.

**Independent Test**: Pair two clients, revoke one, and confirm only the revoked client loses access.

### P2: Discover Hosts on LAN

**User Story**: As a workstation user, I want the app to find available hosts on the LAN so that setup does not require manually typing IP addresses.

**Why P2**: Manual entry is acceptable for MVP but discovery improves setup reliability.

**Acceptance Criteria**:

1. WHERE LAN discovery is enabled the host SHALL advertise its application name, host name, port, and instance identifier on the local network.
2. WHEN a client scans the LAN THEN the system SHALL list discovered compatible hosts and let the user select one before pairing.
3. IF discovery finds no hosts THEN the system SHALL still allow manual host URL entry.

**Independent Test**: Start a host, open a client setup screen, and select the discovered host without typing the URL.

### P3: Prepare for Future Sharding

**User Story**: As a maintainer, I want storage and API boundaries documented so that future sharding work can be evaluated without rewriting the LAN client experience.

**Why P3**: Sharding is not in scope, but this feature should avoid painting the architecture into a corner.

**Acceptance Criteria**:

1. The system SHALL keep client-facing APIs expressed as product operations rather than SQL statements.
2. The system SHALL document which tables are central, which tables could later be customer-sharded, and which workflows currently block sharding.
3. The system SHALL keep sharding disabled and unavailable in product settings.

**Independent Test**: Review design documentation and confirm no user-facing setting enables sharding.

## Edge Cases

- IF the host process exits while clients are active THEN clients SHALL show disconnected status and prevent shared-data reads and writes until reconnection succeeds.
- IF two clients submit writes concurrently THEN the host SHALL serialize database writes through the existing SQLite transaction behavior and return business-rule errors when constraints fail.
- IF an attachment upload succeeds on disk but the database transaction fails THEN the host SHALL remove or stage cleanup for the orphaned file.
- IF a client clock differs from the host clock THEN host-created timestamps SHALL remain authoritative for persisted business rows.
- IF a request body exceeds the configured attachment or API payload limit THEN the host SHALL reject it before mutating storage.
- IF the host app version differs from the client app version THEN pairing and health checks SHALL reject the connection even when the API major/minor version is otherwise compatible.
- IF any LAN request is attempted without TLS or with a certificate fingerprint different from the pinned host fingerprint THEN the client SHALL reject the connection before sending a bearer token or business payload.

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| LANSRV-01 | P1: Configure Host Mode | Execute (T1) | In progress |
| LANSRV-02 | P1: Pair Client Device | Execute (T1) | In progress |
| LANSRV-03 | P1: Use Core Business Workflows Over LAN | Design | Pending |
| LANSRV-04 | P1: Protect Host-Only Storage Operations | Design | Pending |
| LANSRV-05 | P2: Manage Devices and Observability | Design | Pending |
| LANSRV-06 | P2: Discover Hosts on LAN | Design | Pending |
| LANSRV-07 | P3: Prepare for Future Sharding | Design | Pending |

**Coverage:** 7 total, 7 mapped to tasks, 0 unmapped.

## Success Criteria

- [ ] A host and at least two clients can complete the service-order workflow against one host database.
- [ ] Retried mutating requests do not create duplicate service orders, movements, attachments, or device records.
- [ ] Client mode does not open, read from, or mutate the production SQLite database file directly.
- [ ] Client backup export works as a host-created encrypted backup download while restore/import/reset remain protected by host-only storage guards.
- [ ] Network tests prove LAN credentials and business operations use pinned TLS and reject plaintext, changed certificates, and different application build versions.
- [ ] Full frontend and backend gates pass, including IPC/API contract coverage for host and client paths.
