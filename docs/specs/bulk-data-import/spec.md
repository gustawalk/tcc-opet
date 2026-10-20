# Bulk Data Import Specification

## Problem Statement
Users migrating from other systems need to bring clients, employees, and inventory into OpetS without retyping records. Source files have different headers and may contain invalid values or unsupported fields.

## Goals
Import CSV/XLSX with explicit field mapping and editable review, then atomically create up to 5,000 records.

## Out of Scope
Updates/merges, custom fields, saved mappings, persisted drafts, images, source identifiers, historical stock movements, and multi-sheet imports are excluded.

## Assumptions & Open Questions
| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Existing records | Always create; retain database uniqueness | User decision | yes |
| Invalid rows | Edit or explicitly exclude before saving | User decision | yes |
| Field coverage | Existing fields only | User decision | yes |
| Mapping lifetime | Current wizard session | User decision | yes |
| Batch size | 5,000 nonempty data rows | User decision | yes |
| Format defaults | UTF-8 CSV; Brazilian numbers; first nonempty row is header | Desktop Portuguese product | plan approved |
| Bounds | 10 MiB file, 100 columns, 4,096 characters/cell, 8 MiB mapped request, 64 MiB expanded XLSX | Bound local/LAN resource use | plan approved |

**Open questions:** none.

## User Stories
### P1: Select and map a source file
**Acceptance Criteria**:
1. WHEN a user selects CSV or XLSX THEN the system SHALL expose indexed headers, samples, and source row numbers without writing business data. (IMP-01)
2. WHEN a CSV is selected THEN the system SHALL support UTF-8/BOM and comma, semicolon, or tab delimiters; XLSX SHALL allow one worksheet to be selected. (IMP-02)
3. WHEN mapping fields THEN the system SHALL offer source columns, fixed values, and ignored optional fields, with unambiguous English/Portuguese name suggestions and no custom field storage. (IMP-03)
4. IF a file exceeds a configured bound or is malformed THEN the system SHALL reject it with a Portuguese error. (IMP-04)

### P1: Correct the import
**Acceptance Criteria**:
1. WHEN previewing mapped rows THEN the system SHALL provide paginated editing, source row numbers, errors-only filtering, and exclusion/restoration without losing off-page edits. (IMP-05)
2. IF an included row violates existing form requirements, employee email uniqueness, date, integer, stock, or price rules THEN the system SHALL return a field-specific error from Rust and prevent saving. (IMP-06)
3. WHEN Brazilian or international number format is chosen THEN the system SHALL parse prices exactly to integer cents and dates from DD/MM/YYYY, ISO, or Excel date cells. (IMP-07)
4. WHEN changing a draft source or mapping or closing before submission THEN the system SHALL request confirmation before discarding review edits. (IMP-08)

### P1: Save a batch safely
**Acceptance Criteria**:
1. WHEN included rows are confirmed THEN the system SHALL revalidate and create every included record and a durable receipt in one transaction, with fresh IDs and timestamps. (IMP-09)
2. IF validation, uniqueness, or insertion fails THEN the system SHALL persist no records or receipt from that batch. (IMP-10)
3. WHEN an inventory record has positive initial tracked stock THEN the system SHALL create an entrada movement with reason bulk_import, imported unit cost, and no service-order reference, and initialize average cost to cost price. (IMP-11)
4. WHEN an identical batch ID and payload are retried THEN the system SHALL return the saved result without additional records; changed content under that ID SHALL be rejected. (IMP-12)
5. WHILE operating as a LAN Client the system SHALL parse its selected file locally and validate/save only against the authenticated Host without local database fallback. (IMP-13)
6. WHEN an import succeeds THEN the system SHALL show created/excluded counts and invalidate affected paginated lists and inventory summary queries. (IMP-14)
7. WHILE a save response is uncertain the system SHALL retain a frozen payload and batch ID for safe retry without permitting a second submission with changed data. (IMP-15)

## Edge Cases
Blank rows are ignored; duplicate headers use indexed identities. Numeric Excel identifiers receive a leading-zero warning. Optional values remain empty/absent. Non-tracked inventory cannot have positive quantities. Employee uniqueness includes deleted records. Included row set must not be empty. Source formulas are never evaluated; formula cells require correction or exclusion.

## Requirement Traceability
| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| IMP-01 | P1 | Execute | Verified |
| IMP-02 | P1 | Execute | Verified |
| IMP-03 | P1 | Execute | Verified |
| IMP-04 | P1 | Execute | Verified |
| IMP-05 | P1 | Execute | Verified |
| IMP-06 | P1 | Execute | Verified |
| IMP-07 | P1 | Execute | Verified |
| IMP-08 | P1 | Execute | Verified |
| IMP-09 | P1 | Execute | Verified |
| IMP-10 | P1 | Execute | Verified |
| IMP-11 | P1 | Execute | Verified |
| IMP-12 | P1 | Execute | Verified |
| IMP-13 | P1 | Execute | Verified |
| IMP-14 | P1 | Execute | Verified |
| IMP-15 | P1 | Execute | Verified |

## Success Criteria
A 5,000-row batch can be reviewed in bounded pages and saved atomically; duplicate requests do not duplicate writes; required frontend and backend gates pass.
