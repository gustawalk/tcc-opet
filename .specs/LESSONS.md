# LESSONS - auto-maintained by scripts/lessons.py

> Machine-owned. Do NOT hand-edit. Changes are overwritten on the next `lessons.py` write.
> Canonical state lives in `.specs/lessons.json`. Edit lessons only via the script.
> promote_threshold=2 distinct features · window_days=45 · quarantine_threshold=2
> per-signal promotion overrides: gate_fail=1, surviving_mutant=1 (machine-observed signals promote on first sighting)

## Confirmed (load these at Specify/Design)

Corroborated across multiple features. Safe to apply as guidance.

### L-002 - Make additive schema migrations replay-safe for accepted SQLCipher export paths that preserve tables but lose user_version, and test preservation of both data and receipt metadata.
- signal: `gate_fail` · recurrence: 1 feature(s) · scope: `sqlcipher,migrations,backup` · harmful: 0
- features: bulk-data-import
- evidence: src-tauri/src/database.rs:647; legacy backup encryption regression (sqlcipher,migrations,backup)
- last seen: 2026-10-20T13:06:54Z

## Candidates (under observation - do NOT load as guidance yet)

Seen once or not yet corroborated. Tracked, not trusted.

### L-001 - When importing calendar dates from XLSX, use a date conversion that accepts both numeric Excel dates and typed ISO date-only cells, and exercise both representations in fixtures.
- signal: `ac_gap` · recurrence: 1 feature(s) · scope: `xlsx,dates,imports` · harmful: 0
- features: bulk-data-import
- evidence: src-tauri/src/import_file.rs:285; IMP-07 (xlsx,dates,imports)
- last seen: 2026-10-20T13:06:54Z

## Quarantined (failed when applied - ignore)

A confirmed lesson that recurred alongside failure. Kept for the maintainer to review.

_none_
