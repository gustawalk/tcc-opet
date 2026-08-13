import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from "lucide-react";
import type { RefObject } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface PaginationProps {
  totalItems: number;
  page: number;
  pageSize: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
  pageSizeOptions?: number[];
  scrollTargetRef?: RefObject<HTMLElement | null>;
  className?: string;
}

type PageItem = number | "left-ellipsis" | "right-ellipsis";

function pageWindow(current: number, total: number): PageItem[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1);
  }
  const items: PageItem[] = [1];
  if (current > 4) items.push("left-ellipsis");
  const start = Math.max(2, current - 2);
  const end = Math.min(total - 1, current + 2);
  for (let page = start; page <= end; page += 1) items.push(page);
  if (current < total - 3) items.push("right-ellipsis");
  items.push(total);
  return items;
}

export function Pagination({
  totalItems,
  page,
  pageSize,
  onPageChange,
  onPageSizeChange,
  pageSizeOptions = [10, 20, 50, 100],
  scrollTargetRef,
  className,
}: PaginationProps) {
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize));
  const currentPage = Math.min(Math.max(page, 1), totalPages);
  const firstItem = totalItems === 0 ? 0 : (currentPage - 1) * pageSize + 1;
  const lastItem = Math.min(currentPage * pageSize, totalItems);

  const scrollToTarget = () => {
    scrollTargetRef?.current?.scrollIntoView({
      behavior: "smooth",
      block: "start",
    });
  };

  const goTo = (target: number) => {
    const clamped = Math.min(Math.max(target, 1), totalPages);
    if (clamped !== currentPage) {
      onPageChange(clamped);
      scrollToTarget();
    }
  };

  const changePageSize = (nextPageSize: number) => {
    if (nextPageSize !== pageSize) {
      onPageSizeChange(nextPageSize);
      scrollToTarget();
    }
  };

  return (
    <div className={cn("flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between", className)}>
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <span className="whitespace-nowrap">
          Mostrando {firstItem}–{lastItem} de {totalItems}
        </span>
        <select
          aria-label="Itens por página"
          value={pageSize}
          onChange={(event) => changePageSize(Number(event.target.value))}
          className="h-8 rounded-md border border-input bg-background px-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          {pageSizeOptions.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      </div>

      <div className="flex items-center gap-1">
        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8"
          aria-label="Primeira página"
          disabled={currentPage <= 1}
          onClick={() => goTo(1)}
        >
          <ChevronsLeft className="h-4 w-4" />
        </Button>
        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8"
          aria-label="Página anterior"
          disabled={currentPage <= 1}
          onClick={() => goTo(currentPage - 1)}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>

        {pageWindow(currentPage, totalPages).map((item, index) =>
          typeof item === "number" ? (
            <Button
              key={item}
              variant={item === currentPage ? "default" : "outline"}
              size="icon"
              className="h-8 w-8"
              onClick={() => goTo(item)}
            >
              {item}
            </Button>
          ) : (
            <span
              key={`${item}-${index}`}
              className="flex h-8 w-8 items-center justify-center text-sm text-muted-foreground"
            >
              …
            </span>
          ),
        )}

        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8"
          aria-label="Próxima página"
          disabled={currentPage >= totalPages}
          onClick={() => goTo(currentPage + 1)}
        >
          <ChevronRight className="h-4 w-4" />
        </Button>
        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8"
          aria-label="Última página"
          disabled={currentPage >= totalPages}
          onClick={() => goTo(totalPages)}
        >
          <ChevronsRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
