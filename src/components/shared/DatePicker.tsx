import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Calendar, ChevronLeft, ChevronRight, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface DatePickerProps {
  value: string;
  onChange: (value: string) => void;
  id?: string;
  placeholder?: string;
  className?: string;
}

const weekDays = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sab"];
const isoDate = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;

function parseIsoDate(value: string) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return null;
  const date = new Date(
    Number(match[1]),
    Number(match[2]) - 1,
    Number(match[3]),
  );
  return isoDate(date) === value ? date : null;
}

function monthStart(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

export function DatePicker({
  value,
  onChange,
  id,
  placeholder = "Selecione uma data",
  className,
}: DatePickerProps) {
  const selectedDate = useMemo(() => parseIsoDate(value), [value]);
  const [isOpen, setIsOpen] = useState(false);
  const [displayMonth, setDisplayMonth] = useState(() =>
    monthStart(selectedDate ?? new Date()),
  );
  const [focusedDate, setFocusedDate] = useState<Date | null>(selectedDate);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const dayRefs = useRef(new Map<string, HTMLButtonElement>());
  const popupId = useId();

  const days = useMemo(() => {
    const firstDay = displayMonth.getDay();
    const totalDays = new Date(
      displayMonth.getFullYear(),
      displayMonth.getMonth() + 1,
      0,
    ).getDate();
    return Array.from({ length: firstDay + totalDays }, (_, index) =>
      index < firstDay
        ? null
        : new Date(
            displayMonth.getFullYear(),
            displayMonth.getMonth(),
            index - firstDay + 1,
          ),
    );
  }, [displayMonth]);

  const close = (restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) requestAnimationFrame(() => triggerRef.current?.focus());
  };

  const open = () => {
    const date = selectedDate ?? new Date();
    setDisplayMonth(monthStart(date));
    setFocusedDate(date);
    setIsOpen(true);
  };

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

  useEffect(() => {
    if (!isOpen || !focusedDate) return;
    requestAnimationFrame(() =>
      dayRefs.current.get(isoDate(focusedDate))?.focus(),
    );
  }, [isOpen, focusedDate, displayMonth]);

  const moveFocus = (amount: number) => {
    const next = new Date(
      (focusedDate ?? selectedDate ?? new Date()).getFullYear(),
      (focusedDate ?? selectedDate ?? new Date()).getMonth(),
      (focusedDate ?? selectedDate ?? new Date()).getDate() + amount,
    );
    setFocusedDate(next);
    setDisplayMonth(monthStart(next));
  };

  const changeMonth = (amount: number) => {
    const nextMonth = new Date(
      displayMonth.getFullYear(),
      displayMonth.getMonth() + amount,
      1,
    );
    const day = Math.min(
      (focusedDate ?? selectedDate ?? new Date()).getDate(),
      new Date(nextMonth.getFullYear(), nextMonth.getMonth() + 1, 0).getDate(),
    );
    const next = new Date(nextMonth.getFullYear(), nextMonth.getMonth(), day);
    setDisplayMonth(nextMonth);
    setFocusedDate(next);
  };

  const selectDate = (date: Date) => {
    onChange(isoDate(date));
    close(true);
  };

  const handleDayKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    switch (event.key) {
      case "ArrowRight":
        event.preventDefault();
        moveFocus(1);
        break;
      case "ArrowLeft":
        event.preventDefault();
        moveFocus(-1);
        break;
      case "ArrowDown":
        event.preventDefault();
        moveFocus(7);
        break;
      case "ArrowUp":
        event.preventDefault();
        moveFocus(-7);
        break;
      case "Home":
        event.preventDefault();
        moveFocus(-(focusedDate ?? new Date()).getDay());
        break;
      case "End":
        event.preventDefault();
        moveFocus(6 - (focusedDate ?? new Date()).getDay());
        break;
      case "PageDown":
        event.preventDefault();
        changeMonth(event.shiftKey ? 12 : 1);
        break;
      case "PageUp":
        event.preventDefault();
        changeMonth(event.shiftKey ? -12 : -1);
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
        id={id}
        type="button"
        variant="outline"
        className="w-full justify-start gap-2 px-3 text-left font-normal"
        aria-haspopup="dialog"
        aria-expanded={isOpen}
        aria-controls={popupId}
        onClick={() => (isOpen ? close() : open())}
        onKeyDown={(event) => {
          if (["Enter", " ", "ArrowDown"].includes(event.key)) {
            event.preventDefault();
            if (!isOpen) open();
          }
        }}
      >
        <Calendar className="h-4 w-4 text-muted-foreground" />
        <span className={cn(!selectedDate && "text-muted-foreground")}>
          {selectedDate
            ? selectedDate.toLocaleDateString("pt-BR")
            : placeholder}
        </span>
      </Button>

      {isOpen && (
        <Card
          id={popupId}
          role="dialog"
          aria-label="Selecionar data"
          className="absolute z-50 mt-1 w-[19rem] p-3 shadow-lg"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              close(true);
            }
            if (event.key === "Tab") close();
          }}
        >
          <div className="mb-3 flex items-center justify-between">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              aria-label="Mês anterior"
              onClick={() => changeMonth(-1)}
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <span className="text-sm font-medium capitalize">
              {displayMonth.toLocaleDateString("pt-BR", {
                month: "long",
                year: "numeric",
              })}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              aria-label="Próximo mês"
              onClick={() => changeMonth(1)}
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
          <div
            role="grid"
            aria-label={`Dias de ${displayMonth.toLocaleDateString("pt-BR", { month: "long", year: "numeric" })}`}
            className="grid grid-cols-7 gap-1"
          >
            {weekDays.map((day) => (
              <span
                key={day}
                className="flex h-8 items-center justify-center text-xs text-muted-foreground"
              >
                {day}
              </span>
            ))}
            {days.map((date, index) =>
              date ? (
                <Button
                  key={isoDate(date)}
                  ref={(element) => {
                    if (element) dayRefs.current.set(isoDate(date), element);
                    else dayRefs.current.delete(isoDate(date));
                  }}
                  type="button"
                  variant={
                    selectedDate && isoDate(date) === isoDate(selectedDate)
                      ? "default"
                      : "ghost"
                  }
                  size="icon"
                  role="gridcell"
                  aria-label={date.toLocaleDateString("pt-BR", {
                    weekday: "long",
                    day: "numeric",
                    month: "long",
                    year: "numeric",
                  })}
                  aria-selected={
                    selectedDate
                      ? isoDate(date) === isoDate(selectedDate)
                      : false
                  }
                  tabIndex={
                    focusedDate && isoDate(date) === isoDate(focusedDate)
                      ? 0
                      : -1
                  }
                  className="h-8 w-8 text-xs"
                  onClick={() => selectDate(date)}
                  onKeyDown={handleDayKeyDown}
                >
                  {date.getDate()}
                </Button>
              ) : (
                <span
                  key={`blank-${index}`}
                  role="presentation"
                  className="h-8 w-8"
                />
              ),
            )}
          </div>
          <div className="mt-3 flex justify-end border-t pt-3">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="gap-1"
              onClick={() => {
                onChange("");
                close(true);
              }}
              disabled={!value}
            >
              <X className="h-3 w-3" /> Limpar
            </Button>
          </div>
        </Card>
      )}
    </div>
  );
}
