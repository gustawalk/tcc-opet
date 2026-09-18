import { createContext, ReactNode, useContext, useState } from "react";
import { ChecklistTemplateDetailSheet } from "@/components/shared/ChecklistTemplateDetailSheet";
import type { ChecklistTemplate } from "@/lib/types";

const ChecklistTemplateDrawerContext = createContext<{
  openChecklistTemplate: (template: ChecklistTemplate) => void;
} | null>(null);

export function ChecklistTemplateDrawerProvider({ children }: { children: ReactNode }) {
  const [template, setTemplate] = useState<ChecklistTemplate | null>(null);

  return (
    <ChecklistTemplateDrawerContext.Provider value={{ openChecklistTemplate: setTemplate }}>
      {children}
      <ChecklistTemplateDetailSheet
        template={template}
        open={template !== null}
        onClose={() => setTemplate(null)}
      />
    </ChecklistTemplateDrawerContext.Provider>
  );
}

export function useChecklistTemplateDrawer() {
  const context = useContext(ChecklistTemplateDrawerContext);
  if (!context) {
    throw new Error("useChecklistTemplateDrawer must be used within ChecklistTemplateDrawerProvider");
  }
  return context;
}
