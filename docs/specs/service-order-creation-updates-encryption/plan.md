# Implementation Plan: Service-Order Inline Creation, Update Notice, and Local Data Encryption

## Phase overview

| Phase | Objective | Depends on |
| --- | --- | --- |
| 1. Inline creation | Create records from service-order autocomplete drawers | - |
| 2. Update notice | Warn at launch and publish usable update feeds | - |
| 3. Encryption foundation | Add versioned keys and encrypted storage | - |
| 4. Migration and backups | Safely migrate and restore encrypted data | Phase 3 |
| 5. Verification | Cover UI, release, migration, and recovery paths | Phases 1-4 |

## Phase 1: Inline creation

**Objective:** Create and immediately use records without leaving service-order creation.

- [ ] **T1.1: Extract the controlled inventory creation sheet**
  - Description: Move the existing inventory form and creation behavior into a shared controlled sheet. Accept `open`, `onOpenChange`, a fixed initial type, and an `onCreated` callback.
  - Files/modules affected: `src/views/Inventory.tsx`, new shared sheet component.
  - Criterion of completion: The Inventory page preserves current behavior and the service-order form can open the same sheet.
  - Depends on: -

- [ ] **T1.2: Extract the controlled checklist-template creation sheet**
  - Description: Move the existing checklist-template form and creation behavior into a shared controlled sheet.
  - Files/modules affected: `src/views/Templates.tsx`, new shared sheet component.
  - Criterion of completion: The Templates page preserves current behavior and the service-order form can open the same sheet.
  - Depends on: -

- [ ] **T1.3: Add inline autocomplete actions**
  - Description: Add `Novo checklist` to `SearchableSelect` and `Nova peca`/`Novo servico` to `ServiceOrderItemsEditor` while their menus are open.
  - Files/modules affected: `src/components/shared/SearchableSelect.tsx`, `src/components/shared/ServiceOrderItemsEditor.tsx`.
  - Criterion of completion: Actions open the relevant sheet without navigation or loss of the service-order draft.
  - Depends on: T1.1, T1.2

- [ ] **T1.4: Connect service-order creation and cache updates**
  - Description: Apply the newly created checklist or inventory item to the draft immediately, then invalidate/refetch all affected query keys.
  - Files/modules affected: `src/views/ServiceOrderCreate.tsx`, inventory and template mutation locations.
  - Criterion of completion: New records appear immediately and remain available after subsequent interactions.
  - Depends on: T1.3

## Phase 2: Update notice

**Objective:** Show one non-blocking launch warning only when an update exists.

- [ ] **T2.1: Add one-time startup update check**
  - Description: Call the existing updater plugin directly from application startup. Guard against duplicate React Strict Mode execution and suppress user-facing failures.
  - Files/modules affected: `src/App.tsx`.
  - Criterion of completion: One updater check runs on each packaged application launch.
  - Depends on: -

- [ ] **T2.2: Add availability-only Sonner alert**
  - Description: Display a Portuguese informational alert directing the user to Settings > Updates only when an update is available.
  - Files/modules affected: `src/App.tsx`, `src/lib/errors.ts` only if a shared helper is necessary.
  - Criterion of completion: There is no modal, download, installation, or alert when no update exists.
  - Depends on: T2.1

- [ ] **T2.3: Publish updater manifests during releases**
  - Description: Generate and publish version-correct updater feed entries after signed artifacts exist.
  - Files/modules affected: `.github/workflows/release.yml`, updater feed assets, release documentation.
  - Criterion of completion: A newly published release is discoverable through the configured updater endpoint.
  - Depends on: -

## Phase 3: Encryption foundation

**Objective:** Introduce stable, versioned encrypted application storage.

- [ ] **T3.1: Configure versioned release-key injection**
  - Description: Inject `v1` application-key material from GitHub Actions secrets for release builds and define development/test key configuration without committing production material.
  - Files/modules affected: release workflow, Rust build configuration, development configuration.
  - Criterion of completion: Release keys are absent from source control and runtime logs, and compatible key versions are explicit.
  - Depends on: -

- [ ] **T3.2: Adopt SQLCipher database connections**
  - Description: Replace ordinary bundled SQLite with SQLCipher and centralize key application before every schema or query operation.
  - Files/modules affected: `src-tauri/Cargo.toml`, `src-tauri/src/database.rs`, database test helpers.
  - Criterion of completion: Newly created databases cannot be opened by ordinary SQLite and all application database paths work with the keyed connection.
  - Depends on: T3.1

- [ ] **T3.3: Encrypt attachments**
  - Description: Store every attachment in a versioned authenticated envelope and decrypt only for preview or user-requested export.
  - Files/modules affected: `src-tauri/src/attachment_service.rs`, attachment commands.
  - Criterion of completion: Stored attachment bytes are unreadable outside the application and tampering is rejected.
  - Depends on: T3.1

- [ ] **T3.4: Define encrypted-storage compatibility metadata**
  - Description: Add authenticated, non-secret metadata that identifies the active storage format and application key version before SQLCipher data is opened. Define a compatibility registry that maps supported key versions to decryption material and identifies the current write key.
  - Files/modules affected: `src-tauri/src/database.rs`, encryption configuration module, backup format definitions.
  - Criterion of completion: The application can select v1 or a future v2 key deterministically, rejects unsupported versions without modifying active data, and never logs key material.
  - Depends on: T3.1, T3.2

## Phase 4: Migration and backups

**Objective:** Convert existing data and make backup recovery safe.

- [ ] **T4.1: Migrate plaintext databases and attachments**
  - Description: Create an encrypted recovery backup, convert active plaintext v0.1.0 data through staging, validate it, and atomically activate encrypted v1 storage. Recognize v0.1.0 legacy backup archives on import, warn the user, preserve the source archive, and encrypt the validated imported result before activation.
  - Files/modules affected: `src-tauri/src/database.rs`, `src-tauri/src/attachment_service.rs`.
  - Criterion of completion: A v0.1.0 export imports successfully in v0.1.1, existing data remains usable after migration, and any failed migration or legacy import preserves active data.
  - Depends on: T3.2, T3.3, T3.4

- [ ] **T4.2: Version and encrypt backup archives**
  - Description: Add a non-secret backup header containing backup format version, application key version, passphrase requirement, and KDF parameters. Prompt for an optional passphrase. Use application-key recovery only for empty passphrases; use Argon2id-derived password recovery for non-empty passphrases.
  - Files/modules affected: `src-tauri/src/backup_service.rs`, backup controls in `src/views/Settings.tsx`.
  - Criterion of completion: Backup metadata and payload are encrypted, and non-empty passwords are required for their archives.
  - Depends on: T3.2, T3.3, T3.4

- [ ] **T4.3: Restore encrypted and legacy backups**
  - Description: Read the versioned header before restore, prompt for a passphrase when required, validate all content before activation, support legacy backups with a warning, and re-encrypt restored data using the active key version.
  - Files/modules affected: `src-tauri/src/backup_service.rs`, restore controls in `src/views/Settings.tsx`.
  - Criterion of completion: Wrong passwords, unsupported key versions, and corrupted archives cannot modify active data.
  - Depends on: T4.1, T4.2

- [ ] **T4.4: Implement key-rotation migration**
  - Description: When introducing v2, retain v1 read support, create a recoverable v1 backup, decrypt active v1 data into staging, write and validate v2 data, then atomically activate v2. Retain v1 support for historical backup imports.
  - Files/modules affected: `src-tauri/src/database.rs`, `src-tauri/src/attachment_service.rs`, `src-tauri/src/backup_service.rs`, release-key configuration.
  - Criterion of completion: A v1 application backup imports in v2 and becomes active v2 data; an interrupted rotation leaves v1 active data intact; a v2 application can still read supported v1 backups.
  - Depends on: T4.1, T4.2, T4.3

## Phase 5: Verification

**Objective:** Prove user-facing behavior and recovery safety.

- [ ] **T5.1: Cover frontend workflows**
  - Description: Test inline creation, automatic selection, query invalidation, and update notice scenarios.
  - Files/modules affected: frontend test setup and relevant component tests.
  - Criterion of completion: Expected success, cancellation, unavailable-update, and failed-check behavior passes.
  - Depends on: Phase 1, Phase 2

- [ ] **T5.2: Cover encrypted backend workflows**
  - Description: Test database access, attachment encryption, migration, backup passwords, key versions, and restore rollback. Cover the exact compatibility matrix: v0.1.0 legacy export to v0.1.1 import; v1 passwordless export to v2 import; v1 password-protected export to v2 import; wrong password; corrupt archive or attachment; unsupported key version; and interrupted v1-to-v2 rotation.
  - Files/modules affected: Rust test modules and test helpers.
  - Criterion of completion: Every compatibility case has an automated pass/fail assertion, all failed paths preserve active data, and encryption and recovery tests pass in the supported native build environment.
  - Depends on: Phase 3, Phase 4

- [ ] **T5.3: Verify release artifacts**
  - Description: Verify signed Windows and AppImage artifacts are discoverable from the published updater feed.
  - Files/modules affected: release workflow verification steps and release documentation.
  - Criterion of completion: A clean packaged application detects the corresponding release.
  - Depends on: T2.3

## Checkpoints

1. After Phase 1, validate the drawer experience and automatic service-order selection.
2. After Phase 2, validate a real signed prerelease and updater feed before shipping the launch alert.
3. Before Phase 4 activation, test migration against a copy of representative production data.
4. Before a key-rotation release, test v1 backup import and interrupted migration on a clean v2 installation.
5. Before release, test backup restoration on a clean installation.
