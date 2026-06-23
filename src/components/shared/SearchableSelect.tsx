import { useState, useRef, useEffect, useMemo } from "react";
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
  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const filteredOptions = useMemo(() => {
    if (!search.trim()) return options;
    const term = search.toLowerCase();
    return options.filter((item) =>
      getLabel(item).toLowerCase().includes(term)
    );
  }, [options, search, getLabel]);

  const maxHeight = maxOptions * OPTION_HEIGHT;

  const selectedLabel = useMemo(() => {
    if (!value) return null;
    const selected = options.find((item) => getKey(item) === value);
    return selected ? getLabel(selected) : null;
  }, [options, value, getKey, getLabel]);

  useEffect(() => {
    if (isOpen) {
      setSearch("");
      setTimeout(() => searchInputRef.current?.focus(), 0);
    }
  }, [isOpen]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") setIsOpen(false);
    };
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, []);

  const handleSelect = (item: T) => {
    onSelect(item);
    setIsOpen(false);
  };

  const optionList = (
    <>
      {filteredOptions.length > 0 ? (
        filteredOptions.map((item) => (
          <div
            key={getKey(item)}
            className={cn(
              "flex items-center justify-between p-2 hover:bg-accent rounded-sm cursor-pointer transition-colors",
              getKey(item) === value && "bg-accent"
            )}
            onClick={() => handleSelect(item)}
          >
            <div className="flex flex-col">
              <span className="text-sm">{getLabel(item)}</span>
              {getSubtitle && getSubtitle(item) && (
                <span className="text-xs text-muted-foreground">{getSubtitle(item)}</span>
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
    </>
  );

  return (
    <div ref={containerRef} className={cn("relative", className)}>
      <Button
        variant="outline"
        className="w-full justify-between text-left font-normal"
        onClick={() => setIsOpen(!isOpen)}
      >
        <span className="truncate">{selectedLabel || placeholder}</span>
        <ChevronRight
          className={cn(
            "h-4 w-4 ml-2 opacity-50 transition-transform",
            isOpen && "rotate-90"
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
                placeholder={searchPlaceholder}
                className="h-8 pl-8 text-sm"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
          </div>
          <div className="overflow-y-auto" style={{ maxHeight }}>
            {optionList}
          </div>
        </Card>
      )}
    </div>
  );
}
