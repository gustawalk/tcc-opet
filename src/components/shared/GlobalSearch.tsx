import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Search } from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useSidebar } from "@/components/ui/sidebar";
import { useServiceOrderDrawer } from "@/components/shared/ServiceOrderDrawerProvider";
import { dataCommand } from "@/lib/data-client";
import type { ChecklistTemplate, Customer, InventoryItem, Page, ServiceOrder } from "@/lib/types";
import { getThemePreference, setThemePreference, type Theme } from "@/lib/theme";
import { getFontScalePreference, setFontScalePreference, type FontScale } from "@/lib/font-scale";

type ResultGroup = { label: string; path: string; items: { id: string; title: string; detail: string }[] };
type SearchHistoryItem = { id: string; title: string; detail: string; path: string; kind: "order" | "page" };
const HISTORY_KEY = "opets-global-search-history";

export function GlobalSearch() {
  const [open, setOpen] = useState(false);
  const [term, setTerm] = useState("");
  const [groups, setGroups] = useState<ResultGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [history, setHistory] = useState<SearchHistoryItem[]>(() => {
    try { return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]"); } catch { return []; }
  });
  const navigate = useNavigate();
  const { toggleSidebar } = useSidebar();
  const { openServiceOrder } = useServiceOrderDrawer();

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

  const choose = (path: string, item?: { id: string; title: string; detail: string }, kind: "order" | "page" = "page") => {
    if (item) {
      const next = [{ ...item, path, kind }, ...history.filter((entry) => entry.id !== item.id || entry.kind !== kind)].slice(0, 3);
      setHistory(next); localStorage.setItem(HISTORY_KEY, JSON.stringify(next));
    }
    setOpen(false);
    if (kind === "order" && item) openServiceOrder(item.id); else navigate(path);
  };
  const quickAction = (action: () => void) => { setOpen(false); action(); };
  const cycleTheme = () => {
    const themes: Theme[] = ["light", "dark", "system"];
    setThemePreference(themes[(themes.indexOf(getThemePreference()) + 1) % themes.length]);
  };
  const cycleFontScale = () => {
    const scales: FontScale[] = ["sm", "md", "lg"];
    setFontScalePreference(scales[(scales.indexOf(getFontScalePreference()) + 1) % scales.length]);
  };
  return <>
    <Button variant="outline" size="sm" className="hidden md:flex" onClick={() => setOpen(true)}><Search />Buscar <kbd className="ml-2 text-xs text-muted-foreground">Ctrl K</kbd></Button>
    <Dialog open={open} onOpenChange={setOpen}><DialogContent className="max-w-xl"><DialogHeader><DialogTitle>Busca global</DialogTitle></DialogHeader><Input autoFocus value={term} onChange={(event) => setTerm(event.target.value)} placeholder="Buscar clientes, OS, estoque ou modelos..." />
      {term.trim().length < 2 ? <div className="space-y-4"><section><h3 className="text-xs font-semibold uppercase text-muted-foreground">Ações rápidas</h3><div className="mt-1 grid gap-1"><button className="rounded-md px-2 py-2 text-left text-sm hover:bg-muted" onClick={() => quickAction(() => navigate("/os/new"))}>Nova ordem de serviço</button><button className="rounded-md px-2 py-2 text-left text-sm hover:bg-muted" onClick={() => quickAction(toggleSidebar)}>Alternar menu lateral</button><button className="rounded-md px-2 py-2 text-left text-sm hover:bg-muted" onClick={() => quickAction(cycleTheme)}>Alterar tema</button><button className="rounded-md px-2 py-2 text-left text-sm hover:bg-muted" onClick={() => quickAction(cycleFontScale)}>Alterar tamanho da fonte</button></div></section>{history.length > 0 && <section><h3 className="text-xs font-semibold uppercase text-muted-foreground">Buscas recentes</h3>{history.map((item) => <button key={`${item.kind}-${item.id}`} className="mt-1 w-full rounded-md px-2 py-2 text-left hover:bg-muted" onClick={() => choose(item.path, item, item.kind)}><p className="text-sm font-medium">{item.title}</p><p className="text-xs text-muted-foreground">{item.detail}</p></button>)}</section>}</div> : loading ? <p className="text-sm text-muted-foreground">Buscando...</p> : groups.length === 0 ? <p className="text-sm text-muted-foreground">Nenhum resultado encontrado.</p> : <div className="max-h-80 space-y-3 overflow-y-auto">{groups.map((group) => <section key={group.label}><h3 className="text-xs font-semibold uppercase text-muted-foreground">{group.label}</h3>{group.items.map((item) => <button key={item.id} type="button" className="mt-1 w-full rounded-md px-2 py-2 text-left hover:bg-muted" onClick={() => choose(group.path, item, group.path === "/os" ? "order" : "page")}><p className="text-sm font-medium">{item.title}</p><p className="text-xs text-muted-foreground">{item.detail}</p></button>)}</section>)}</div>}
    </DialogContent></Dialog>
  </>;
}
