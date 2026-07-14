import { ChevronDown, ChevronUp } from "lucide-react";
import { TableHead } from "@/components/ui/table";
import type { SortConfig } from "@/hooks/useSort";

interface SortableHeaderProps {
  column: string;
  label: string;
  sortConfig: SortConfig;
  onSort: (column: string) => void;
  className?: string;
  align?: "left" | "right" | "center";
}

export function SortableHeader({
  column,
  label,
  sortConfig,
  onSort,
  className = "",
  align = "left",
}: SortableHeaderProps) {
  const isActive = sortConfig.column === column;
  const justifyMap = { left: "justify-start", right: "justify-end", center: "justify-center" };
  return (
    <TableHead className={className}>
      <button
        onClick={() => onSort(column)}
        className={`flex items-center gap-1 w-full font-medium ${justifyMap[align]}`}
      >
        {label}
        {isActive && sortConfig.direction === "desc" && <ChevronDown className="h-4 w-4 shrink-0" />}
        {isActive && sortConfig.direction === "asc" && <ChevronUp className="h-4 w-4 shrink-0" />}
      </button>
    </TableHead>
  );
}
