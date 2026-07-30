# Specification: Service-Order Inline Creation, Update Notice, and Local Data Encryption

## Context and objective

Creating a service order currently requires leaving the workflow to register a missing checklist template, part, or service. Updates can be checked manually in Settings but are not surfaced on launch. Application data is stored in plaintext SQLite, attachment, and backup files.

This work reduces interruptions while creating service orders, gives users a non-blocking update reminder, and prevents casual inspection of local application data.

## Current scenario

- `src/views/ServiceOrderCreate.tsx` uses `SearchableSelect` for checklist templates and `ServiceOrderItemsEditor` for inventory records.
- The inventory and template creation sheets are embedded in `src/views/Inventory.tsx` and `src/views/Templates.tsx`.
- `src/components/shared/ServiceOrderDrawerProvider.tsx` is the existing decoupled-drawer reference.
- `src/views/Settings.tsx` already checks, downloads, installs, and relaunches updates with `@tauri-apps/plugin-updater`.
- The updater feed is configured, but the release workflow does not generate and publish a new signed feed for each release.
- SQLite, attachments, and `.osbkp` backups are plaintext.

## Description

As a technician, I want to create a checklist, part, or service while creating a service order so that I can complete the registration without leaving the workflow.

As an application user, I want a lightweight warning when an update exists so that I can decide when to install it from Settings.

As an application owner, I want local data protected from casual file inspection while retaining safe migration and recovery.

## Functional requirements

### Inline creation in service orders

- The checklist selector displays a `Novo checklist` action while its dropdown is open.
- The inventory autocomplete displays `Nova peca` and `Novo servico` actions while its dropdown is open.
- These actions open reusable controlled sheets without navigating away from the service-order form.
- Successfully creating a checklist immediately selects it and applies its items to the draft service order.
- Successfully creating a part or service immediately adds it to the draft service order.
- Cancelling or failing creation leaves the service-order draft unchanged.
- Inventory creation refreshes both `['inventory']` and `['inventory-lookup']` query data.
- Checklist creation refreshes `['checklist-templates']` query data.

### Launch-time update notice

- Each packaged application launch performs one updater check against the configured Tauri updater feed.
- If no update exists, the application shows nothing.
- If the check fails, the application shows no user-facing error.
- If an update exists, the application shows only an informational Sonner alert directing the user to Settings > Updates.
- The alert does not open a modal, download an update, or begin installation.
- Settings remains the only location for update details, download, installation, and relaunch.
- The release process publishes a valid signed updater feed for each supported platform.

### Local data encryption

- SQLite database contents use SQLCipher encryption.
- Attachments use authenticated per-file encryption with a unique nonce per file.
- Existing plaintext databases and attachments are migrated through a verified staging process with rollback safety.
- Release builds receive stable, versioned application-key material from GitHub Actions secrets rather than repository source.
- Application key versions remain compatible across updates. A deliberate future key rotation retains old-version support during verified re-encryption.
- Backup export always prompts for a passphrase, which may be empty.
- A backup with an empty passphrase is recoverable using a compatible application key version.
- A backup with a non-empty passphrase requires that passphrase; the embedded application key does not bypass the password.
- Restore decrypts and validates all content before atomically replacing active application data.

### Backup compatibility and key rotation

- Version `0.1.1` recognizes the legacy plaintext `.osbkp` format exported by `0.1.0`.
- Importing a legacy backup displays a warning, restores only into a staging area, validates the data, and writes the activated result in encrypted v1 storage.
- The source legacy backup is never modified during import.
- Encrypted backups include a non-secret versioned header containing the backup format version, application key version, and whether a passphrase is required. Password-protected backups also contain the KDF parameters and salt needed to derive the recovery key.
- A version that rotates the active application key, such as `0.1.2` changing from v1 to v2, retains v1 decryption support and adds v2 as the active write key.
- Importing a v1 backup in a v2 application reads the header, decrypts with v1 after any required passphrase check, validates the staged content, and re-encrypts active data with v2.
- Active storage has authenticated version metadata so the application can select a compatible key before opening SQLCipher data.
- Key rotation is atomic: v1 active data remains intact until v2 output has been completely written and validated.
- An interrupted migration leaves the existing active version usable on the next launch.
- Downgrading after a successful key rotation is unsupported by default. Before switching active data to v2, the rotating version creates a recoverable v1 backup for manual rollback.

## Non-functional requirements

- No release key is committed to source control or written to runtime logs.
- Encryption uses authenticated encryption and random nonces; encoding or obfuscation alone is insufficient.
- Database, attachment, migration, backup, and restore failures must not overwrite active data.
- Attachment preview and export decrypt content only when needed.
- Existing users receive a recoverable encrypted backup before plaintext migration.
- Secure deletion cannot be guaranteed for historical files, SSD blocks, previous plaintext backups, or user-exported files.

## Contracts and interfaces affected

- `src/components/shared/SearchableSelect.tsx`
- `src/components/shared/ServiceOrderItemsEditor.tsx`
- `src/views/ServiceOrderCreate.tsx`
- `src/views/Inventory.tsx`
- `src/views/Templates.tsx`
- `src/App.tsx`
- `src/views/Settings.tsx`
- `.github/workflows/release.yml`
- `src-tauri/Cargo.toml`
- `src-tauri/src/database.rs`
- `src-tauri/src/attachment_service.rs`
- `src-tauri/src/backup_service.rs`

## Test strategy

- Add component tests for inline actions, cancellation, successful automatic selection, and query invalidation.
- Add updater tests or mocks for available, unavailable, and failed checks.
- Verify release workflow output: feed version, URLs, signatures, and platform targets match released artifacts.
- Add Rust tests for encrypted database access, legacy migration, wrong keys, attachment authentication failures, passworded and passwordless backups, restore rollback, and key-version compatibility.
- Verify the compatibility matrix: v0.1.0 legacy export imported by v0.1.1; v1 passwordless and password-protected exports imported by a v2 application; wrong password; corrupt data; and interrupted v1-to-v2 migration.

## Out of scope

- Automatic download or installation at application startup.
- Protection from a user or malware that can execute the installed application.
- Encrypting files exported by a user to a chosen destination.
- Retroactively erasing historical plaintext backups or filesystem remnants.

## Risks and assumptions

- An embedded release key deters casual inspection only. A determined attacker can recover it from a distributed binary.
- Rotating from v1 to v2 does not secure existing copied v1 data because v1 remains available in older distributed binaries. Rotation protects data newly written with v2.
- SQLCipher changes the native build and requires Windows and Linux release verification.
- Linux in-app updater support remains AppImage-focused; DEB installations are normally package-managed.
- Legacy plaintext backup restoration should remain supported with a warning and a re-export path, so existing backups are not abandoned.
