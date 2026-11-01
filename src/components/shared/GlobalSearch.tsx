import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { useNavigate } from "react-router-dom";
import { ClipboardList, LoaderCircle, Package, Search, Users, Wrench, type LucideIcon } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSidebar } from "@/components/ui/sidebar";
import { useServiceOrderDrawer } from "@/components/shared/ServiceOrderDrawerProvider";
import { useCustomerDrawer } from "@/components/shared/CustomerDrawerProvider";
import { useInventoryDrawer } from "@/components/shared/InventoryDrawerProvider";
import { useChecklistTemplateDrawer } from "@/components/shared/ChecklistTemplateDrawerProvider";
import { useInventoryItemCreate } from "@/components/shared/InventoryItemCreateProvider";
import { dataCommand } from "@/lib/data-client";
import type { ChecklistTemplate, Customer, InventoryItem, Page, ServiceOrder } from "@/lib/types";
import { getThemePreference, setThemePreference, THEME_OPTIONS, type Theme } from "@/lib/theme";
import { getFontScalePreference, setFontScalePreference, FONT_SCALE_OPTIONS, type FontScale } from "@/lib/font-scale";

type SearchKind = "customer" | "inventory" | "order" | "template";
type SearchResultItem = { id: string; title: string; detail: string; status?: string; template?: ChecklistTemplate };
type ResultGroup = { label: string; path: string; kind: SearchKind; icon: LucideIcon; total: number; items: SearchResultItem[] };
type SearchHistoryItem = { id: string; title: string; detail: string; path: string; kind: SearchKind };
const HISTORY_KEY = "opets-global-search-history";

export function GlobalSearch() {
  const [open, setOpen] = useState(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [category, setCategory] = useState<SearchKind | "all">("all");
  const [error, setError] = useState(false);
  const headerRef = useRef<HTMLDivElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  const [headerTerm, setHeaderTerm] = useState("");
  const [modalTerm, setModalTerm] = useState("");
  const term = open ? modalTerm : headerTerm;
  const setTerm = open ? setModalTerm : setHeaderTerm;
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
  const { openCustomerHistory } = useCustomerDrawer();
  const { openInventoryItem } = useInventoryDrawer();
  const { openChecklistTemplate } = useChecklistTemplateDrawer();
  const { openInventoryItemCreate } = useInventoryItemCreate();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setDropdownOpen(false);
        setModalTerm("");
        setGroups([]);
        setOpen((current) => !current);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    setSelectedOption(-1);
  }, [open, dropdownOpen, term, groups, category]);

  useEffect(() => {
    const options = resultsRef.current?.querySelectorAll<HTMLButtonElement>("[data-global-search-option]");
    options?.forEach((option, index) => {
      option.classList.toggle("bg-muted", index === selectedOption);
      option.setAttribute("data-selected", String(index === selectedOption));
    });
  }, [selectedOption, groups, category, open, dropdownOpen]);

  useEffect(() => {
    if (!dropdownOpen) return;
    const outside = (event: PointerEvent) => {
      if (!headerRef.current?.contains(event.target as Node)) setDropdownOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [dropdownOpen]);

  const onSearchKeyDown = (event: ReactKeyboardEvent) => {
      if (event.key === "Escape") { setDropdownOpen(false); return; }
      if ((event.target as HTMLElement).closest('[role="tab"]')) return;
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp" && event.key !== "Enter") return;
      const options = Array.from(resultsRef.current?.querySelectorAll<HTMLButtonElement>("[data-global-search-option]") ?? []);
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
      options[next]?.scrollIntoView?.({ block: "nearest" });
  };

  useEffect(() => {
    const query = term.trim();
    let cancelled = false;
    setGroups([]);
    setError(false);
    if ((!open && !dropdownOpen) || query.length < 2) { setLoading(false); return; }
    setLoading(true);
    const timer = window.setTimeout(() => {
      setLoading(true);
      void Promise.all([
        dataCommand<Page<Customer>>("get_customers_page", { limit: 5, offset: 0, search: query }),
        dataCommand<Page<ServiceOrder>>("get_service_orders_page", { limit: 5, offset: 0, search: query }),
        dataCommand<Page<InventoryItem>>("get_inventory_items_page", { limit: 5, offset: 0, search: query }),
        dataCommand<Page<ChecklistTemplate>>("get_checklist_templates_page", { limit: 5, offset: 0, search: query }),
      ]).then(([customers, orders, inventory, templates]) => { if (!cancelled) setGroups(([
        { label: "Clientes", path: "/customers", kind: "customer", icon: Users, total: customers.total, items: customers.items.map((item) => ({ id: item.id, title: item.name, detail: item.phone || item.email })) },
        { label: "Ordens de serviço", path: "/os", kind: "order", icon: Wrench, total: orders.total, items: orders.items.map((item) => ({ id: item.id, title: item.customerName ?? "Cliente", detail: item.displayId?.trim() ?? "", status: item.status })) },
        { label: "Estoque", path: "/inventory", kind: "inventory", icon: Package, total: inventory.total, items: inventory.items.map((item) => ({ id: item.id, title: item.name, detail: `${item.type === "part" ? "Peça" : item.type === "service" ? "Serviço" : "Item"}${item.tracksStock ? ` · Estoque: ${item.currentQuantity}` : ""}` })) },
        { label: "Modelos", path: "/templates", kind: "template", icon: ClipboardList, total: templates.total, items: templates.items.map((item) => ({ id: item.id, title: item.title, detail: "Modelo de checklist", template: item })) },
      ]) as ResultGroup[]); }).catch(() => { if (!cancelled) setError(true); }).finally(() => { if (!cancelled) setLoading(false); });
    }, 300);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [term, open, dropdownOpen]);

  const choose = (path: string, item?: SearchResultItem, kind: SearchKind = "customer") => {
    if (item) {
      const next = [{ ...item, path, kind }, ...history.filter((entry) => entry.id !== item.id || entry.kind !== kind)].slice(0, 3);
      setHistory(next); localStorage.setItem(HISTORY_KEY, JSON.stringify(next));
    }
    handleOpenChange(false);
    if (kind === "order" && item) openServiceOrder(item.id);
    else if (kind === "customer" && item) openCustomerHistory(item.id);
    else if (kind === "inventory" && item) openInventoryItem(item.id);
    else if (kind === "template" && item?.template) openChecklistTemplate(item.template);
    else navigate(path);
  };
  const quickAction = (action: () => void) => { handleOpenChange(false); action(); };
  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) setDropdownOpen(false);
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
    { label: "Novo item", run: () => quickAction(() => openInventoryItemCreate("item")) },
    { label: "Novo serviço", run: () => quickAction(() => openInventoryItemCreate("service")) },
    { label: "Nova peça", run: () => quickAction(() => openInventoryItemCreate("part")) },
    { label: "Novo cliente", run: () => quickAction(() => navigate("/customers?new=1")) },
    { label: "Novo checklist", run: () => quickAction(() => navigate("/templates?new=1")) },
    { label: "Alternar menu lateral", run: () => quickAction(toggleSidebar) },
    { label: `Alterar tema — ${THEME_OPTIONS.find((option) => option.value === theme)?.label}`, run: () => quickAction(cycleTheme) },
    { label: `Alterar tamanho da fonte — ${FONT_SCALE_OPTIONS.find((option) => option.value === fontScale)?.label}`, run: () => quickAction(cycleFontScale) },
  ].filter((action) => !term.trim() || action.label.toLocaleLowerCase().includes(term.trim().toLocaleLowerCase()));
  const categories: { value: SearchKind | "all"; label: string }[] = [
    { value: "all", label: "Todos" }, { value: "customer", label: "Clientes" },
    { value: "order", label: "Ordens de serviço" }, { value: "inventory", label: "Estoque" },
    { value: "template", label: "Modelos" },
  ];
  const visibleGroups = groups.filter((group) => group.items.length > 0 && (category === "all" || group.kind === category));
  const hasQuery = term.trim().length >= 2;
  const showCounts = hasQuery && !loading && !error && groups.length > 0;
  const content = <Tabs value={category} onValueChange={(value) => setCategory(value as SearchKind | "all")}>
          <div className="overflow-x-auto border-b pb-2">
            <TabsList aria-label="Categorias da busca" className="grid w-full min-w-[38rem] grid-cols-[1fr_1fr_1.8fr_1fr_1fr]">
              {categories.map(({ value, label }) => (
                <TabsTrigger key={value} value={value} className="min-w-0 justify-center gap-1.5 px-2 text-center">
                  {label}{" "}
                     <span
                       aria-hidden={!showCounts}
                       className={`inline-flex h-4 w-7 shrink-0 items-center justify-center rounded-sm border border-border/60 bg-background/60 px-1 text-[10px] font-medium leading-none tabular-nums text-muted-foreground ${showCounts ? "" : "invisible"}`}
                     >
                       {showCounts ? groups.reduce((total, group) => total + (value === "all" || group.kind === value ? group.total : 0), 0) : null}
                    </span>
                </TabsTrigger>
              ))}
            </TabsList>
          </div>
          <TabsContent value={category}>
          <div ref={resultsRef} className="h-[min(24rem,50vh)] space-y-5 overflow-y-auto p-1" aria-busy={loading}>
            {term.trim().length < 2 && history.length > 0 && (
              <section>
                <h3 className="text-xs font-semibold uppercase text-muted-foreground">
                  Itens recentes
                </h3>

                {history.filter((item) => category === "all" || item.kind === category).map((item) => {
                  const Icon = icons[item.kind];

                  return (
                    <button
                      data-global-search-option
                      key={`${item.kind}-${item.id}`}
                      className="mt-1 flex w-full items-center gap-2 rounded-md px-2 py-2 text-left outline-none hover:bg-muted focus:bg-muted focus:ring-1 focus:ring-inset focus:ring-ring"
                      onClick={() => choose(item.path, item, item.kind)}
                    >
                      <Icon className="h-4 w-4 shrink-0 text-muted-foreground" />

                      <span>
                        <p className="text-sm font-medium">{item.title}</p>
                        <p className="text-xs text-muted-foreground">
                          {item.detail}
                        </p>
                      </span>
                    </button>
                  );
                })}
              </section>
            )}

            {category === "all" && quickActions.length > 0 && (
              <section>
                <h3 className="text-xs font-semibold uppercase text-muted-foreground">
                  Ações rápidas
                </h3>

                <div className="grid grid-cols-2 gap-2 pt-2">{quickActions.map((action) => (
                  <button
                    data-global-search-option
                    key={action.label}
                    className="w-full rounded-lg border px-3 py-3 text-left text-sm outline-none hover:bg-muted focus:bg-muted focus:ring-1 focus:ring-inset focus:ring-ring"
                    onClick={action.run}
                  >
                    {action.label}
                  </button>
                ))}</div>
              </section>
            )}

            {term.trim().length < 2 ? null : loading ? (
               <p role="status" className="flex items-center gap-2 text-sm text-muted-foreground">
                 <LoaderCircle aria-hidden="true" className="h-4 w-4 animate-spin motion-reduce:animate-none" />
                 Buscando...
               </p>
            ) : error ? <p role="alert" className="text-sm text-destructive">Não foi possível buscar. Tente novamente.</p> : visibleGroups.length === 0 && (category !== "all" || quickActions.length === 0) ? (
              <p className="text-sm text-muted-foreground">
                Nenhum resultado encontrado.
              </p>
            ) : (
              visibleGroups.map((group) => {
                const Icon = group.icon;

                return (
                  <section key={group.label}>
                    <h3 className="flex items-center gap-1 text-xs font-semibold uppercase text-muted-foreground">
                      <Icon className="h-3.5 w-3.5" />
                      {group.label}
                    </h3>

                    {group.total > group.items.length && (
                      <p className="mt-1 text-xs text-muted-foreground">
                        Exibindo {group.items.length} de {group.total} resultados
                      </p>
                    )}

                    {group.items.map((item) => (
                      <button
                        data-global-search-option
                        key={item.id}
                        type="button"
                        className="mt-1 flex w-full items-center gap-2 rounded-md px-2 py-2 text-left outline-none hover:bg-muted focus:bg-muted focus:ring-1 focus:ring-inset focus:ring-ring"
                        onClick={() => choose(group.path, item, group.kind)}
                      >
                        <Icon className="h-4 w-4 shrink-0 text-muted-foreground" />

                        <span>
                          <p className="text-sm font-medium">{item.title}</p>
                          <span className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
                            {item.detail && <span>{item.detail}</span>}
                            {item.status && <span className="rounded-md border border-border/60 bg-muted/60 px-1.5 py-0.5 text-[11px] leading-tight">{item.status}</span>}
                          </span>
                        </span>
                      </button>
                    ))}
                  </section>
                );
              })
            )}
          </div>
          </TabsContent>
          <div className="border-t pt-3 text-xs text-muted-foreground">↑ ↓ navegar · Enter abrir · Esc fechar</div>
  </Tabs>;
  return (
    <>
      <div ref={headerRef} className="relative w-full" onKeyDown={onSearchKeyDown}>
        <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
        <Input
          aria-label="Busca global no cabeçalho"
          placeholder="Buscar..."
          className="pl-9 pr-16"
          value={headerTerm}
          onClick={() => { setOpen(false); setDropdownOpen(true); }}
          onChange={(event) => { setHeaderTerm(event.target.value); setDropdownOpen(true); }}
        />
        <kbd className="pointer-events-none absolute right-3 top-3 text-[10px] text-muted-foreground">Ctrl K</kbd>
        {dropdownOpen && !open && (
          <div
            role="region"
            aria-label="Resultados da busca"
            className="absolute left-1/2 top-full z-50 mt-2 w-[min(42rem,calc(100vw-2rem))] -translate-x-1/2 space-y-3 rounded-xl border bg-popover p-4 text-popover-foreground shadow-xl"
          >
            {content}
          </div>
        )}
      </div>
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <DialogContent className="max-w-2xl" onKeyDown={onSearchKeyDown}>
          <DialogHeader>
            <DialogTitle>Busca global</DialogTitle>
            <DialogDescription className="sr-only">Encontre registros e execute ações rápidas.</DialogDescription>
          </DialogHeader>
          <Input
            autoFocus
            aria-label="Busca global no modal"
            value={term}
            onChange={(event) => setTerm(event.target.value)}
            placeholder="Buscar clientes, OS, estoque ou modelos..."
          />
          {content}
        </DialogContent>
      </Dialog>
    </>
  );
}
