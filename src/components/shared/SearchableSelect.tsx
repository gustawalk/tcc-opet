import { useState, useRef, useEffect, useMemo, useId } from "react";
import { Search, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface SearchableSelectProps<T> {
  options: T[];
  value: string | null;
  onSelect: (item: T) => void;
  placeholder?: string;
  searchPlaceholder?: string;
  maxOptions?: number;
  getKey: (item: T) => string;
  getLabel: (item: T) => string;
  getSubtitle?: (item: T) => string | undefined;
  className?: string;
}

const OPTION_HEIGHT = 36;

export function SearchableSelect<T>({
  options,
  value,
  onSelect,
  placeholder = "Selecione...",
  searchPlaceholder = "Buscar...",
  maxOptions = 5,
  getKey,
  getLabel,
  getSubtitle,
  className,
}: SearchableSelectProps<T>) {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listboxId = useId();

  const filteredOptions = useMemo(() => {
    if (!search.trim()) return options;
    const term = search.toLowerCase();
    return options.filter((item) =>
      getLabel(item).toLowerCase().includes(term),
    );
  }, [options, search, getLabel]);

  const maxHeight = maxOptions * OPTION_HEIGHT;

  const selectedLabel = useMemo(() => {
    if (!value) return null;
    const selected = options.find((item) => getKey(item) === value);
    return selected ? getLabel(selected) : null;
  }, [options, value, getKey, getLabel]);

  const close = (restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) requestAnimationFrame(() => triggerRef.current?.focus());
  };

  const open = (direction?: "first" | "last") => {
    setSearch("");
    const selectedIndex = filteredOptions.findIndex(
      (item) => getKey(item) === value,
    );
    setActiveIndex(
      direction === "last"
        ? filteredOptions.length - 1
        : direction === "first"
          ? 0
          : Math.max(selectedIndex, 0),
    );
    setIsOpen(true);
  };

  useEffect(() => {
    if (isOpen) {
      requestAnimationFrame(() => searchInputRef.current?.focus());
    }
  }, [isOpen]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      )
        close();
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleSelect = (item: T) => {
    onSelect(item);
    close(true);
  };

  const moveActiveOption = (step: number) => {
    if (!filteredOptions.length) return;
    setActiveIndex((index) => {
      const current = index < 0 ? (step > 0 ? -1 : 0) : index;
      return Math.max(0, Math.min(filteredOptions.length - 1, current + step));
    });
  };

  const handleListKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        moveActiveOption(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        moveActiveOption(-1);
        break;
      case "Enter":
        event.preventDefault();
        if (activeIndex >= 0 && filteredOptions[activeIndex])
          handleSelect(filteredOptions[activeIndex]);
        break;
      case "Escape":
        event.preventDefault();
        close(true);
        break;
      case "Tab":
        close();
        break;
    }
  };

  return (
    <div ref={containerRef} className={cn("relative", className)}>
      <Button
        ref={triggerRef}
        type="button"
        variant="outline"
        className="w-full justify-between text-left font-normal"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={listboxId}
        onClick={() => (isOpen ? close() : open())}
        onKeyDown={(event) => {
          if (
            event.key === "Enter" ||
            event.key === " " ||
            event.key === "ArrowDown" ||
            event.key === "ArrowUp"
          ) {
            event.preventDefault();
            if (!isOpen) open(event.key === "ArrowUp" ? "last" : "first");
          }
        }}
      >
        <span className="truncate">{selectedLabel || placeholder}</span>
        <ChevronRight
          className={cn(
            "h-4 w-4 ml-2 opacity-50 transition-transform",
            isOpen && "rotate-90",
          )}
        />
      </Button>

      {isOpen && (
        <Card className="absolute z-50 w-full mt-1 shadow-lg border-primary/20 overflow-hidden">
          <div className="p-1 border-b">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <Input
                ref={searchInputRef}
                role="combobox"
                aria-autocomplete="list"
                aria-controls={listboxId}
                aria-expanded="true"
                aria-activedescendant={
                  activeIndex >= 0
                    ? `${listboxId}-option-${activeIndex}`
                    : undefined
                }
                placeholder={searchPlaceholder}
                className="h-8 pl-8 text-sm"
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setActiveIndex(0);
                }}
                onKeyDown={handleListKeyDown}
              />
            </div>
          </div>
          <div
            id={listboxId}
            role="listbox"
            aria-label={placeholder}
            className="overflow-y-auto"
            style={{ maxHeight }}
          >
            {filteredOptions.length > 0 ? (
              filteredOptions.map((item, index) => (
                <div
                  key={getKey(item)}
                  id={`${listboxId}-option-${index}`}
                  role="option"
                  aria-selected={getKey(item) === value}
                  className={cn(
                    "flex items-center justify-between p-2 rounded-sm cursor-pointer transition-colors",
                    index === activeIndex && "bg-accent",
                    getKey(item) === value &&
                      index !== activeIndex &&
                      "bg-accent/50",
                  )}
                  onMouseEnter={() => setActiveIndex(index)}
                  onClick={() => handleSelect(item)}
                >
                  <div className="flex flex-col">
                    <span className="text-sm">{getLabel(item)}</span>
                    {getSubtitle?.(item) && (
                      <span className="text-xs text-muted-foreground">
                        {getSubtitle(item)}
                      </span>
                    )}
                  </div>
                  <ChevronRight className="h-3 w-3 text-muted-foreground" />
                </div>
              ))
            ) : (
              <div className="p-4 text-center text-xs text-muted-foreground italic">
                Nenhum resultado encontrado.
              </div>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
