import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

const steps = [
  { key: "file", label: "Arquivo" },
  { key: "mapping", label: "Mapeamento" },
  { key: "review", label: "Revisão" },
  { key: "confirm", label: "Confirmação" },
];

export function ImportProgress({ step }: { step: string }) {
  const current =
    step === "done"
      ? steps.length
      : steps.findIndex((item) => item.key === step);
  return (
    <nav aria-label="Etapas da importação" className="px-6 pb-5">
      <ol className="flex items-center">
        {steps.map((item, index) => {
          const completed = index < current;
          const active = index === current;
          return (
            <li
              key={item.key}
              aria-current={active ? "step" : undefined}
              className={cn(
                "flex min-w-0 items-center",
                index < steps.length - 1 && "flex-1",
              )}
            >
              <span
                className={cn(
                  "flex items-center gap-2 text-xs font-medium sm:text-sm",
                  active ? "text-foreground" : "text-muted-foreground",
                )}
              >
                <span
                  className={cn(
                    "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border text-xs",
                    active
                      ? "border-primary bg-primary text-primary-foreground"
                      : completed
                        ? "border-primary/20 bg-primary/10 text-primary"
                        : "border-border bg-muted/50",
                  )}
                >
                  {completed ? (
                    <Check className="h-3.5 w-3.5" aria-hidden="true" />
                  ) : (
                    <span aria-hidden="true">{index + 1}</span>
                  )}
                </span>
                <span className={cn(!active && "sr-only sm:not-sr-only")}>
                  {item.label}
                </span>
                {completed && <span className="sr-only">concluído</span>}
              </span>
              {index < steps.length - 1 && (
                <span
                  aria-hidden="true"
                  className={cn(
                    "mx-2 h-px min-w-2 flex-1 sm:mx-4",
                    completed ? "bg-primary/30" : "bg-border",
                  )}
                />
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
