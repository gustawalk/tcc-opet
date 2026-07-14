import { useState, useCallback } from "react";

export type SortDirection = "asc" | "desc";
export type SortConfig = {
  column: string | null;
  direction: SortDirection | null;
};

export function useSort() {
  const [sortConfig, setSortConfig] = useState<SortConfig>({
    column: null,
    direction: null,
  });

  const cycleSort = useCallback((column: string) => {
    setSortConfig((prev) => {
      if (prev.column !== column) {
        return { column, direction: "desc" };
      }
      if (prev.direction === "desc") {
        return { column, direction: "asc" };
      }
      return { column: null, direction: null };
    });
  }, []);

  const resetSort = useCallback(() => {
    setSortConfig({ column: null, direction: null });
  }, []);

  return { sortConfig, cycleSort, resetSort };
}
