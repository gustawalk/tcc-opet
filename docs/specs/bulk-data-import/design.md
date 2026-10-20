# Bulk import design

The React wizard owns a transient draft. Local Rust parsing commands inspect/read a selected file in a blocking worker without a database lock. Parsing returns indexed headers and string cells, converting typed Excel dates to ISO. Rust csv and calamine handle file syntax; ZIP preflight caps expanded XLSX content. Source rows retain their physical row index.

Mapped rows use a typed entity discriminator, source row number, and whitelisted string fields. Rust performs normalization and business validation for both preview and commit, returning values and field errors. Prices use checked integer decimal arithmetic, never floating point money conversion. Employee uniqueness is queried against all existing employees and checked within the included set.

An IMMEDIATE transaction revalidates, creates records via connection-aware repositories, inserts initial inventory movements, and records the batch ID/hash/result. Replay reads the receipt before uniqueness validation. A numbered schema migration adds receipts for fresh and upgraded databases. Receipts retain only the request hash and resulting IDs/count, not source data.

LAN exposes only validation and commit; file commands never reach the Host dispatcher. Import paths alone receive an 8 MiB body limit. Commit uses its own receipt transaction rather than the generic LAN idempotency wrapper, whose response persistence is separate from the product transaction. Native local commits use the same batch semantics.

The wizard freezes its final request until an explicit success or authoritative validation failure. Unknown transport failures offer retry with that same request. No source data is persisted in browser storage or logs. Query prefix invalidation refreshes lists, summary, and insights.

## Implementation details

XLSX cells are streamed using calamine formula metadata, so sparse declared dimensions do not allocate giant ranges. Numeric column provenance survives mapping to distinguish typed Excel numbers from locale-formatted text. CSV quote structure is checked before decoding to reject unterminated or misplaced quotes.

Inventory currently displays separate lists rather than tabs, so its global import button defaults to part. Users can map or assign another type. Discard confirmation reuses the wizard dialog instead of stacking modal focus traps; cancel restores the triggering control's focus. Preview pages paginate a bounded transient source file, not database entity lists.

The receipt migration accepts an existing valid table because SQLCipher export can erase user_version while preserving the table during legacy restore. It validates the expected columns before advancing the version. Reset drops import receipts along with business data.
