import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Search } from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { dataCommand } from "@/lib/data-client";
import type { ChecklistTemplate, Customer, InventoryItem, Page, ServiceOrder } from "@/lib/types";

type ResultGroup = { label: string; path: string; items: { id: string; title: string; detail: string }[] };

export function GlobalSearch() {
  const [open, setOpen] = useState(false);
  const [term, setTerm] = useState("");
  const [groups, setGroups] = useState<ResultGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setOpen((current) => !current);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    const query = term.trim();
    if (query.length < 2) { setGroups([]); return; }
    const timer = window.setTimeout(() => {
      setLoading(true);
      void Promise.all([
        dataCommand<Page<Customer>>("get_customers_page", { limit: 5, offset: 0, search: query }),
        dataCommand<Page<ServiceOrder>>("get_service_orders_page", { limit: 5, offset: 0, search: query }),
        dataCommand<Page<InventoryItem>>("get_inventory_items_page", { limit: 5, offset: 0, search: query }),
        dataCommand<Page<ChecklistTemplate>>("get_checklist_templates_page", { limit: 5, offset: 0, search: query }),
      ]).then(([customers, orders, inventory, templates]) => setGroups([
        { label: "Clientes", path: "/customers", items: customers.items.map((item) => ({ id: item.id, title: item.name, detail: item.phone || item.email })) },
        { label: "Ordens de serviço", path: "/os", items: orders.items.map((item) => ({ id: item.id, title: item.displayId, detail: `${item.customerName ?? "Cliente"} · ${item.status}` })) },
        { label: "Estoque", path: "/inventory", items: inventory.items.map((item) => ({ id: item.id, title: item.name, detail: item.type === "part" ? "Peça" : "Serviço" })) },
        { label: "Modelos", path: "/templates", items: templates.items.map((item) => ({ id: item.id, title: item.title, detail: "Modelo de checklist" })) },
      ].filter((group) => group.items.length > 0))).catch(() => setGroups([])).finally(() => setLoading(false));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [term]);

  const choose = (path: string) => { setOpen(false); navigate(path); };
  return <>
    <Button variant="outline" size="sm" className="hidden md:flex" onClick={() => setOpen(true)}><Search />Buscar <kbd className="ml-2 text-xs text-muted-foreground">Ctrl K</kbd></Button>
    <Dialog open={open} onOpenChange={setOpen}><DialogContent className="max-w-xl"><DialogHeader><DialogTitle>Busca global</DialogTitle></DialogHeader><Input autoFocus value={term} onChange={(event) => setTerm(event.target.value)} placeholder="Buscar clientes, OS, estoque ou modelos..." />
      {term.trim().length < 2 ? <p className="text-sm text-muted-foreground">Digite pelo menos 2 caracteres.</p> : loading ? <p className="text-sm text-muted-foreground">Buscando...</p> : groups.length === 0 ? <p className="text-sm text-muted-foreground">Nenhum resultado encontrado.</p> : <div className="max-h-80 space-y-3 overflow-y-auto">{groups.map((group) => <section key={group.label}><h3 className="text-xs font-semibold uppercase text-muted-foreground">{group.label}</h3>{group.items.map((item) => <button key={item.id} type="button" className="mt-1 w-full rounded-md px-2 py-2 text-left hover:bg-muted" onClick={() => choose(group.path)}><p className="text-sm font-medium">{item.title}</p><p className="text-xs text-muted-foreground">{item.detail}</p></button>)}</section>)}</div>}
    </DialogContent></Dialog>
  </>;
}
