import { useState } from "react";
import { Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ImportEntity } from "@/lib/data-import";
import { ImportWizard } from "./ImportWizard";

export function ImportDataButton({ entity }: { entity: ImportEntity }) {
  const [open, setOpen] = useState(false);
  return <>
    <Button variant="outline" className="gap-2" onClick={() => setOpen(true)}><Upload className="h-4 w-4" />Importar dados</Button>
    {open && <ImportWizard entity={entity} onClose={() => setOpen(false)} />}
  </>;
}
