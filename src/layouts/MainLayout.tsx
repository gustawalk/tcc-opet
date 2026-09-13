import {
  LayoutDashboard,
  Users,
  Package,
  Wrench,
  Settings,
  ClipboardList,
  ChartNoAxesCombined,
  Plus,
  User,
} from "lucide-react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarProvider,
  SidebarInset,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Button } from "@/components/ui/button";
import { LogoIcon } from "@/components/LogoIcon";
import { GlobalSearch } from "@/components/shared/GlobalSearch";

const menuItems = [
  { icon: LayoutDashboard, label: "Dashboard", path: "/" },
  { icon: Wrench, label: "Ordens de Serviço", path: "/os" },
  { icon: Users, label: "Clientes", path: "/customers" },
  { icon: Package, label: "Estoque", path: "/inventory" },
  { icon: ClipboardList, label: "Modelos de checklist", path: "/templates" },
  { icon: ChartNoAxesCombined, label: "Relatórios", path: "/reports" },
  { icon: Settings, label: "Configurações", path: "/settings" },
  { icon: User, label: "Funcionários", path: "/users" },
];

export function MainLayout({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <SidebarProvider>
      <div className="flex min-h-screen bg-background w-full">
        <Sidebar collapsible="icon">
          <SidebarHeader className="h-16 flex items-center px-6 border-b group-data-[collapsible=icon]:px-0 group-data-[collapsible=icon]:justify-center">
            <div className="flex items-center gap-6 group-data-[collapsible=icon]:gap-0">
              <div className="rounded-lg flex h-7 w-7 shrink-0 items-center justify-center group-data-[collapsible=icon]:h-6 group-data-[collapsible=icon]:w-6">
                <LogoIcon width="100%" height="100%" />
              </div>
              <span className="font-bold text-lg leading-tight tracking-tight group-data-[collapsible=icon]:hidden">OpetS Manager</span>
            </div>
          </SidebarHeader>
          <SidebarContent className="py-4">
            <SidebarMenu>
              {menuItems.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <SidebarMenuButton
                    asChild
                    isActive={item.path === "/" ? location.pathname === "/" : location.pathname.startsWith(item.path)}
                    tooltip={item.label}
                  >
                    <Link to={item.path} className="flex items-center gap-3 px-3">
                      <item.icon className="h-7 w-7" />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarContent>
        </Sidebar>
        <SidebarInset className="flex-1 flex flex-col min-w-0 bg-muted/20">
          <header className="relative h-16 flex items-center justify-between px-6 border-b bg-background sticky top-0 z-10">
            <div className="flex items-center gap-4">
              <SidebarTrigger />
            </div>
            <div className="absolute left-1/2 -translate-x-1/2">
              <GlobalSearch />
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm" className="hidden sm:flex" onClick={() => navigate("/os/new")}>
                <Plus />
                Nova Ordem</Button>
            </div>
          </header>
          <main className="mx-auto w-full max-w-[1600px] flex-1 p-4 sm:p-6 lg:p-10">
            {children}
          </main>
        </SidebarInset>
      </div>
    </SidebarProvider>
  );
}
