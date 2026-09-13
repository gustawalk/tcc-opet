import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ClipboardList, Package, Search, Users, Wrench, type LucideIcon } from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useSidebar } from "@/components/ui/sidebar";
import { useServiceOrderDrawer } from "@/components/shared/ServiceOrderDrawerProvider";
import { dataCommand } from "@/lib/data-client";
import type { ChecklistTemplate, Customer, InventoryItem, Page, ServiceOrder } from "@/lib/types";
import { getThemePreference, setThemePreference, THEME_OPTIONS, type Theme } from "@/lib/theme";
import { getFontScalePreference, setFontScalePreference, FONT_SCALE_OPTIONS, type FontScale } from "@/lib/font-scale";

type SearchKind = "customer" | "inventory" | "order" | "template";
type ResultGroup = { label: string; path: string; kind: SearchKind; icon: LucideIcon; items: { id: string; title: string; detail: string }[] };
type SearchHistoryItem = { id: string; title: string; detail: string; path: string; kind: SearchKind };
const HISTORY_KEY = "opets-global-search-history";

export function GlobalSearch() {
  const [open, setOpen] = useState(false);
  const [term, setTerm] = useState("");
  const [groups, setGroups] = useState<ResultGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedOption, setSelectedOption] = useState(-1);
  const [theme, setTheme] = useState<Theme>(getThemePreference);
  const [fontScale, setFontScale] = useState<FontScale>(getFontScalePreference);
  const [history, setHistory] = useState<SearchHistoryItem[]>(() => {
    try {
      const parsed: unknown = JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
      return Array.isArray(parsed) ? parsed.filter((item): item is SearchHistoryItem =>
        typeof item === "object" && item !== null && ["customer", "inventory", "order", "template"].includes((item as SearchHistoryItem).kind),
      ) : [];
    } catch { return []; }
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
    setSelectedOption(-1);
  }, [open, term, groups]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp" && event.key !== "Enter") return;
      const options = Array.from(document.querySelectorAll<HTMLButtonElement>("[data-global-search-option]"));
      if (options.length === 0) return;
      if (event.key === "Enter") {
        if (selectedOption >= 0) { event.preventDefault(); options[selectedOption]?.click(); }
        return;
      }
      event.preventDefault();
      const next = event.key === "ArrowDown"
        ? (selectedOption + 1) % options.length
        : (selectedOption - 1 + options.length) % options.length;
      setSelectedOption(next);
      options[next]?.focus();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, selectedOption]);

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
      ]).then(([customers, orders, inventory, templates]) => setGroups(([
        { label: "Clientes", path: "/customers", kind: "customer", icon: Users, items: customers.items.map((item) => ({ id: item.id, title: item.name, detail: item.phone || item.email })) },
        { label: "Ordens de serviço", path: "/os", kind: "order", icon: Wrench, items: orders.items.map((item) => ({ id: item.id, title: item.displayId, detail: `${item.customerName ?? "Cliente"} · ${item.status}` })) },
        { label: "Estoque", path: "/inventory", kind: "inventory", icon: Package, items: inventory.items.map((item) => ({ id: item.id, title: item.name, detail: item.type === "part" ? "Peça" : "Serviço" })) },
        { label: "Modelos", path: "/templates", kind: "template", icon: ClipboardList, items: templates.items.map((item) => ({ id: item.id, title: item.title, detail: "Modelo de checklist" })) },
      ].filter((group) => group.items.length > 0)) as ResultGroup[])).catch(() => setGroups([])).finally(() => setLoading(false));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [term]);

  const choose = (path: string, item?: { id: string; title: string; detail: string }, kind: SearchKind = "customer") => {
    if (item) {
      const next = [{ ...item, path, kind }, ...history.filter((entry) => entry.id !== item.id || entry.kind !== kind)].slice(0, 3);
      setHistory(next); localStorage.setItem(HISTORY_KEY, JSON.stringify(next));
    }
    handleOpenChange(false);
    if (kind === "order" && item) openServiceOrder(item.id); else navigate(path);
  };
  const quickAction = (action: () => void) => { handleOpenChange(false); action(); };
  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) { setTerm(""); setGroups([]); }
  };
  const cycleTheme = () => {
    const themes: Theme[] = ["light", "dark", "system"];
    const next = themes[(themes.indexOf(theme) + 1) % themes.length];
    setThemePreference(next); setTheme(next);
  };
  const cycleFontScale = () => {
    const scales: FontScale[] = ["sm", "md", "lg"];
    const next = scales[(scales.indexOf(fontScale) + 1) % scales.length];
    setFontScalePreference(next); setFontScale(next);
  };
  const icons: Record<SearchKind, LucideIcon> = { customer: Users, inventory: Package, order: Wrench, template: ClipboardList };
  const quickActions = [
    { label: "Nova ordem de serviço", run: () => quickAction(() => navigate("/os/new")) },
    { label: "Alternar menu lateral", run: () => quickAction(toggleSidebar) },
    { label: `Alterar tema — ${THEME_OPTIONS.find((option) => option.value === theme)?.label}`, run: () => quickAction(cycleTheme) },
    { label: `Alterar tamanho da fonte — ${FONT_SCALE_OPTIONS.find((option) => option.value === fontScale)?.label}`, run: () => quickAction(cycleFontScale) },
  ].filter((action) => !term.trim() || action.label.toLocaleLowerCase().includes(term.trim().toLocaleLowerCase()));
  return <>
    <Button variant="outline" size="sm" className="hidden w-64 justify-start md:flex" onClick={() => setOpen(true)}><Search />Buscar <kbd className="ml-auto text-xs text-muted-foreground">Ctrl K</kbd></Button>
    <Dialog open={open} onOpenChange={handleOpenChange}><DialogContent className="top-1/2 max-w-xl -translate-y-1/2"><DialogHeader><DialogTitle>Busca global</DialogTitle></DialogHeader><Input autoFocus value={term} onChange={(event) => setTerm(event.target.value)} placeholder="Buscar clientes, OS, estoque ou modelos..." />
      <div className="max-h-80 space-y-4 overflow-y-auto">{term.trim().length < 2 && history.length > 0 && <section><h3 className="text-xs font-semibold uppercase text-muted-foreground">Itens recentes</h3>{history.map((item) => { const Icon = icons[item.kind]; return <button data-global-search-option key={`${item.kind}-${item.id}`} className="mt-1 flex w-full items-center gap-2 rounded-md px-2 py-2 text-left outline-none hover:bg-muted focus:bg-muted focus:ring-1 focus:ring-inset focus:ring-ring" onClick={() => choose(item.path, item, item.kind)}><Icon className="h-4 w-4 shrink-0 text-muted-foreground" /><span><p className="text-sm font-medium">{item.title}</p><p className="text-xs text-muted-foreground">{item.detail}</p></span></button>; })}</section>}{quickActions.length > 0 && <section><h3 className="text-xs font-semibold uppercase text-muted-foreground">Ações rápidas</h3>{quickActions.map((action) => <button data-global-search-option key={action.label} className="mt-1 w-full rounded-md px-2 py-2 text-left text-sm outline-none hover:bg-muted focus:bg-muted focus:ring-1 focus:ring-inset focus:ring-ring" onClick={action.run}>{action.label}</button>)}</section>}{term.trim().length < 2 ? null : loading ? <p className="text-sm text-muted-foreground">Buscando...</p> : groups.length === 0 && quickActions.length === 0 ? <p className="text-sm text-muted-foreground">Nenhum resultado encontrado.</p> : groups.map((group) => { const Icon = group.icon; return <section key={group.label}><h3 className="flex items-center gap-1 text-xs font-semibold uppercase text-muted-foreground"><Icon className="h-3.5 w-3.5" />{group.label}</h3>{group.items.map((item) => <button data-global-search-option key={item.id} type="button" className="mt-1 flex w-full items-center gap-2 rounded-md px-2 py-2 text-left outline-none hover:bg-muted focus:bg-muted focus:ring-1 focus:ring-inset focus:ring-ring" onClick={() => choose(group.path, item, group.kind)}><Icon className="h-4 w-4 shrink-0 text-muted-foreground" /><span><p className="text-sm font-medium">{item.title}</p><p className="text-xs text-muted-foreground">{item.detail}</p></span></button>)}</section>; })}</div>
    </DialogContent></Dialog>
  </>;
}
