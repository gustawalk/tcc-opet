# Global search

Click the header's **Buscar...** input to type directly and open results below
the field. **Ctrl+K** (or **Cmd+K**) opens a separate centered search dialog.
The header input is centered within the main header, and its dropdown is
centered beneath it with a viewport-limited width.
Opening the dialog dismisses the header dropdown; their query text is separate.

Both surfaces offer **Todos**, **Clientes**, **Ordens de serviço**, **Estoque**,
and **Modelos** categories. With no query, they show recent records and quick
actions (quick actions appear in **Todos**). Searches start at two characters,
after a 300 ms pause, and fetch up to five records per category from the backend.
The displayed suggestions are not the complete result set.

After a search completes, each category tab shows its full backend result
count, including zero matches. **Todos** adds the four category totals and
does not count quick actions or recent records. Counts are hidden before a
search and on errors; one shared loading indicator appears in the results area
during searches, without spinners in the tabs.
The category row fills the panel with fixed column proportions and reserved
count space, preventing tabs from shifting when counts appear or disappear.
On narrow screens the row scrolls horizontally instead of compressing labels.
When a category has more matches than the preview shows, its section displays
**Exibindo 5 de 12 resultados** (using the actual preview size and total).

Use **↑/↓** to select a suggestion, **Enter** to open it, and **Esc** to dismiss
search. Use the category tabs to narrow results. Clicking outside also closes
search. Selecting a record opens its existing detail view; quick actions retain
their existing creation and settings behavior.

Loading, no-results, and connection-error messages appear inside the panel.
If a search fails, change the query to try again. Older responses cannot replace
results for a newer query.
