# Inventory Items, Photos, and Customer Phone Tasks

**Design**: `.specs/features/v0-5-2-inventory-items-photos-customer-phone/design.md`
**Status**: Complete

## Test Coverage Matrix

> Generated from `AGENTS.md`, `package.json`, Rust tests, Vitest tests, and
> the feature spec. Frontend and domain behaviors cover every applicable AC
> and listed edge case.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| --- | --- | --- | --- | --- |
| Schema/model/repository | Rust unit/integration | Fresh and upgraded schema plus all stock-policy branches | `src-tauri/src/**/tests` | `cd src-tauri && cargo test --lib -- --test-threads=1` |
| LAN/IPC facade | Rust integration | Local and LAN payload/result parity, including invalid input | `src-tauri/src/{lan_api,tauri_ipc_tests}.rs` | `cd src-tauri && cargo test --lib -- --test-threads=1` |
| React forms/listing | Vitest component | Each UI AC, page query state, image/placeholder outcomes | `src/**/*.test.tsx` | `yarn test` |
| Type/lint/build | Static | Every changed TS/Rust boundary compiles and lints | project root / `src-tauri` | full build gate |

## Gate Check Commands

| Gate Level | When to Use | Command |
| --- | --- | --- |
| Targeted | Discrimination sensor | `yarn test src/components/shared/InventoryItemSheet.test.tsx` or `cd src-tauri && cargo test --lib inventory` |
| Quick | Focused React/unit task | `yarn test <changed-test-files>` |
| Full | Rust/LAN integration task | `cd src-tauri && cargo test --lib -- --test-threads=1` |
| Build | Phase completion | `yarn typecheck && yarn lint && yarn test && yarn build && cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib -- --test-threads=1` |

## Execution Plan

### Phase 1: Persisted inventory contract

```
T1 → T2 → T3
```

### Phase 2: User workflows

```
T1 → T2 → T3 → T4 → T5 → T6
```

## Task Breakdown

### T1: Migrate generic item and photo fields

**Status**: Complete

**What**: Add the numbered transactional inventory migration.
**Where**: `src-tauri/src/database.rs`
**Depends on**: None
**Reuses**: numbered migration tests
**Requirement**: ITEM-03, PHOTO-03
**Tools**: MCP NONE; Skill codenavi
**Done when**:
- [x] Fresh and upgraded schemas allow `item`, persist `tracks_stock`, and retain null photos.
- [x] Existing parts track stock and services do not after upgrade.
- [x] Migration rollback and future-schema guards remain covered.
**Tests**: Rust unit/integration
**Gate**: Full

### T2: Enforce generic-item stock policy

**Status**: Complete

**What**: Apply stock policy to inventory commands, repository mapping, and service-order lifecycle.
**Where**: `src-tauri/src/commands/inventory_commands.rs`, `src-tauri/src/repositories/{inventory_repo,service_order_repo}.rs`, `src-tauri/src/commands/service_order_commands.rs`
**Depends on**: T1
**Reuses**: existing part movement and order transaction rules
**Requirement**: ITEM-03, ITEM-04, ITEM-05, ITEM-06
**Tools**: MCP NONE; Skill codenavi
**Done when**:
- [x] Stock-controlled items follow restock/removal and service-order deduction/restoration.
- [x] Catalog-only items remain unlimited and reject direct stock movements.
- [x] Existing parts/services retain their semantics.
**Tests**: Rust unit/integration
**Gate**: Full

### T3: Extend local and LAN inventory contracts

**Status**: Complete

**What**: Carry generic item mode and photo data through Tauri and LAN product operations.
**Where**: `src-tauri/src/{lan_api,tauri_ipc_tests}.rs` and command registration/facade files as required
**Depends on**: T2
**Reuses**: existing camelCase IPC and LAN facade tests
**Requirement**: ITEM-06, PHOTO-03, CUST-04
**Tools**: MCP NONE; Skill codenavi
**Done when**:
- [x] Local and LAN create/page/update payloads retain exact type, mode, and photo values.
- [x] Invalid item modes and invalid photo data receive friendly validation errors.
- [x] Customer LAN requests preserve required-phone and optional email/address behavior.
**Tests**: Rust integration
**Gate**: Build

### T4: Add item mode and photo editing to the inventory sheet

**Status**: Complete

**What**: Add generic item creation, stock-mode choice, image selection/validation, replacement, and removal.
**Where**: `src/components/shared/InventoryItemSheet.tsx`, `src/components/shared/InventoryItemSheet.test.tsx`, `src/lib/{types,validation}.ts`
**Depends on**: T3
**Reuses**: existing InventoryItemSheet mutations and Zod form errors
**Requirement**: ITEM-01, ITEM-02, PHOTO-01, PHOTO-02
**Tools**: MCP NONE; Skill codenavi
**Done when**:
- [x] Sheet sends explicit item stock mode and photo value for create/update.
- [x] Invalid/oversized image is rejected without replacing a prior image.
- [x] The user can remove a previously selected image.
**Tests**: Vitest component
**Gate**: Quick

### T5: Render item listing, thumbnails, and service-order selection

**Status**: Complete

**What**: Add item pagination/listing thumbnail states and stock-aware service-order selection labels.
**Where**: `src/views/{Inventory,ServiceOrderCreate}.tsx`, `src/components/shared/{ServiceOrderItemsEditor,ServiceOrderEditorSheet,GlobalSearch,InventoryDrawerProvider}.tsx` and co-located tests
**Depends on**: T4
**Reuses**: paginated query keys, existing thumbnail-capable image elements, and item selection controls
**Requirement**: ITEM-01, ITEM-03, ITEM-04, ITEM-05, ITEM-06, PHOTO-04, PHOTO-05, PHOTO-06
**Tools**: MCP NONE; Skill codenavi
**Done when**:
- [x] A third paginated listing renders generic items with image thumbnail or placeholder.
- [x] Stock action controls and order availability follow `tracksStock`.
- [x] Query keys, reset, clamp, and invalidation cover the new listing.
**Tests**: Vitest component
**Gate**: Build

### T6: Preserve phone-only customer requirement

**Status**: Complete

**What**: Align customer validation and all frontend entry points around required phone and optional email/address.
**Where**: `src/lib/validation.ts`, `src/views/Customers.tsx`, `src/views/ServiceOrderCreate.tsx`, related tests
**Depends on**: T5
**Reuses**: `customerSchema`, `newCustomerSchema`, and existing customer form tests
**Requirement**: CUST-01, CUST-02, CUST-03, CUST-04
**Tools**: MCP NONE; Skill codenavi
**Done when**:
- [x] Name/phone remain mandatory with the existing digit requirement.
- [x] Blank email/address save in every customer entry point.
- [x] Non-empty invalid optional values block save with the existing Portuguese errors.
**Tests**: Vitest component
**Gate**: Build

## Dependency Cross-Check

| Diagram edge | Task dependency | Result |
| --- | --- | --- |
| T1 → T2 | T2 depends on T1 | ✅ |
| T2 → T3 | T3 depends on T2 | ✅ |
| T3 → T4 | T4 depends on T3 | ✅ |
| T4 → T5 | T5 depends on T4 | ✅ |
| T5 → T6 | T6 depends on T5 | ✅ |

## Test Co-location Validation

| Task | Required test layer | Tests field | Result |
| --- | --- | --- | --- |
| T1 | Rust schema/model | Rust unit/integration | ✅ |
| T2 | Rust domain/repository | Rust unit/integration | ✅ |
| T3 | Rust LAN/IPC | Rust integration | ✅ |
| T4 | React form | Vitest component | ✅ |
| T5 | React list/selector | Vitest component | ✅ |
| T6 | React validation/forms | Vitest component | ✅ |
