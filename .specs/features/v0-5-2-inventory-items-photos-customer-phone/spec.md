# OpetS v0.5.2 Specification

## Problem Statement

The inventory currently classifies records only as parts or services. Teams
cannot catalog a general-purpose item without misclassifying it, and cannot
recognize inventory records visually in the listing. Customer creation also
requires a phone number but this intent must remain explicit while the other
contact fields stay optional.

## Goals

- [ ] Let staff create a generic **Novo item** record and choose whether it
  controls stock or behaves as an unlimited catalog entry.
- [ ] Let staff add one optional photo to every inventory record and identify
  it by thumbnail in the paginated inventory listing.
- [ ] Keep customer phone required while email and address remain optional.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Multiple photos, galleries, or item attachments | v0.5.2 needs one recognition photo per inventory record. |
| Photo editing, cropping, or remote image URLs | The release only selects, validates, stores, replaces, and removes a photo. |
| Changing existing part or service semantics | Existing part stock tracking and service catalog behavior remain unchanged. |
| Making customer phone optional | Product decision: phone remains the only required contact field. |
| New pricing, tax, barcode, supplier, or inventory-import behavior | These are separate product features. |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Generic item stock mode | **Novo item** presents a clear choice between **Controlar estoque** and **Não controlar estoque**. | The user explicitly requested both flows; stock-controlled items follow part mechanics, while catalog-only items follow service availability. | y |
| Generic item placement | Generic items appear in their own paginated **Itens diversos** listing. | Keeps parts, services, and uncategorized inventory visibly distinct. | y |
| Photo storage | Store one validated PNG, JPEG, or WebP data URL in the encrypted SQLCipher database, limited to 1 MiB of source bytes. | The existing Settings logo uses a data URL; one bounded photo avoids a new file lifecycle while remaining covered by encrypted database backups and LAN payloads. | n |
| Photo listing treatment | Show a 48 px thumbnail and an accessible neutral placeholder when no photo exists. | Enables visual identification without expanding list rows or loading a separate file for each row. | n |
| Customer contact storage | Keep existing empty-string storage and normalize blank email/address to empty strings. | The schema already supports empty values; no customer-table migration is needed. | y |

**Open questions:** none — photo storage and visual dimensions are logged
assumptions and await feature-spec approval.

---

## User Stories

### P1: Create a generic inventory item

**User Story**: As a technician, I want to add a generic item that is neither
a part nor a service so that I can catalog everything I sell or use without
misclassifying it.

**Why P1**: Classification determines stock accounting and availability in a
service order.

**Acceptance Criteria**:

1. WHEN a user opens inventory creation, THEN the system SHALL offer **Nova Peça**, **Novo Serviço**, and **Novo item** actions. <!-- event-driven -->
2. WHEN a user selects **Novo item**, THEN the system SHALL require a choice between **Controlar estoque** and **Não controlar estoque** before saving. <!-- event-driven -->
3. WHEN a user saves a stock-controlled generic item, THEN the system SHALL persist type `item` with stock tracking enabled and expose quantity, minimum-stock, restock, removal, history, and low-stock behavior equivalent to a part. <!-- event-driven -->
4. WHEN a user saves a catalog-only generic item, THEN the system SHALL persist type `item` with stock tracking disabled and keep it available without quantity, stock movement, or low-stock controls equivalent to a service. <!-- event-driven -->
5. WHEN a generic item is added to, removed from, edited on, canceled from, or reactivated on a service order, THEN the system SHALL deduct or restore stock only when that item has stock tracking enabled. <!-- event-driven -->
6. The system SHALL preserve existing part and service behavior and SHALL label generic-item rows, service-order lines, reports, PDFs, and global-search results as **Item**. <!-- ubiquitous -->

**Independent Test**: Create one item in each mode, perform the service-order
stock lifecycle, and assert stock changes only for the stock-controlled item.

### P1: Identify inventory by photo

**User Story**: As a technician, I want to attach a photo to an inventory
record so that I can recognize it in the listing.

**Why P1**: Generic inventory and visually similar parts are faster to select
when a listing provides an image cue.

**Acceptance Criteria**:

1. WHEN a user creates or edits a part, service, or generic item, THEN the system SHALL allow selecting, replacing, or removing one optional PNG, JPEG, or WebP photo. <!-- event-driven -->
2. IF a selected photo is not a valid supported image or exceeds 1 MiB, THEN the system SHALL reject it with a Portuguese error and SHALL retain the previously saved photo. <!-- unwanted-behavior -->
3. WHEN a valid photo is saved, THEN the system SHALL persist its data URL with the inventory record through the existing encrypted database and LAN command path. <!-- event-driven -->
4. WHERE an inventory record has a photo, THEN the paginated inventory listing SHALL render a 48 px thumbnail with the item name as its alternative text. <!-- optional-feature -->
5. WHERE an inventory record has no photo, THEN the paginated inventory listing SHALL render an accessible neutral placeholder without reserving a separate image request. <!-- optional-feature -->
6. WHEN a photo is replaced or removed, THEN the system SHALL update the local and LAN listings after the existing inventory query keys are invalidated. <!-- event-driven -->

**Independent Test**: Save valid and invalid photos for each inventory type,
then assert the page payload and thumbnail/placeholder behavior in local and
LAN-client modes.

### P1: Require only customer phone

**User Story**: As a technician, I want customer registration to require only
a phone number so that I can create customers without collecting unnecessary
contact data.

**Why P1**: Phone is the team’s required contact channel; email and address
are supplemental.

**Acceptance Criteria**:

1. WHEN a user creates or edits a customer, THEN the system SHALL require a name and a phone number containing at least 10 digits. <!-- event-driven -->
2. WHEN email or address is blank, THEN the system SHALL save the customer successfully and display no invalid-contact error. <!-- event-driven -->
3. IF a supplied email is malformed or a supplied address contains fewer than 5 characters, THEN the system SHALL block saving and show the existing Portuguese field error. <!-- unwanted-behavior -->
4. The system SHALL preserve the same customer-contact validation and empty-value behavior for the Customers page, service-order quick creation, local IPC, and LAN client requests. <!-- ubiquitous -->

**Independent Test**: Create and update a customer with only name and phone
through each entry point, then assert optional-field validation only applies to
non-empty values.

## Edge Cases

- IF a generic item changes stock mode after it has service-order history,
  THEN the system SHALL preserve historical line item types and only apply the
  newly selected mode to future stock operations.
- IF a page changes after an inventory item’s photo or type changes, THEN the
  system SHALL clamp pagination and show the updated record without stale
  thumbnails.
- IF an older database has no photo or stock-mode column, THEN the system
  SHALL migrate it transactionally with generic items absent and existing
  parts/services retaining their current behavior.

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| ITEM-01 | P1: Generic inventory item | Specify | Pending |
| ITEM-02 | P1: Generic inventory item | Specify | Pending |
| ITEM-03 | P1: Generic inventory item | Specify | Pending |
| ITEM-04 | P1: Generic inventory item | Specify | Pending |
| ITEM-05 | P1: Generic inventory item | Specify | Pending |
| ITEM-06 | P1: Generic inventory item | Specify | Pending |
| PHOTO-01 | P1: Identify inventory by photo | Specify | Pending |
| PHOTO-02 | P1: Identify inventory by photo | Specify | Pending |
| PHOTO-03 | P1: Identify inventory by photo | Specify | Pending |
| PHOTO-04 | P1: Identify inventory by photo | Specify | Pending |
| PHOTO-05 | P1: Identify inventory by photo | Specify | Pending |
| PHOTO-06 | P1: Identify inventory by photo | Specify | Pending |
| CUST-01 | P1: Require only customer phone | Specify | Pending |
| CUST-02 | P1: Require only customer phone | Specify | Pending |
| CUST-03 | P1: Require only customer phone | Specify | Pending |
| CUST-04 | P1: Require only customer phone | Specify | Pending |

**Coverage:** 16 total, 0 mapped to implementation, 16 pending.

## Success Criteria

- [ ] A user can create, find, price, and use both stock-controlled and
  catalog-only generic items without changing part or service behavior.
- [ ] One optional inventory photo is safely persisted and visibly represented
  in every applicable paginated listing.
- [ ] Customer creation still rejects absent/invalid phones while accepting
  blank email and address consistently in local and LAN flows.
