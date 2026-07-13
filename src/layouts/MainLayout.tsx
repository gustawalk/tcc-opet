import {
  LayoutDashboard,
  Users,
  Package,
  Wrench,
  Settings,
  ClipboardList,
  Plus,
  User
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

const menuItems = [
  { icon: LayoutDashboard, label: "Dashboard", path: "/" },
  { icon: Wrench, label: "Ordens de Serviço", path: "/os" },
  { icon: Users, label: "Clientes", path: "/customers" },
  { icon: Package, label: "Estoque", path: "/inventory" },
  { icon: ClipboardList, label: "Templates", path: "/templates" },
  { icon: Settings, label: "Configurações", path: "/settings" },
  { icon: User, label: "Usuarios", path: "/users" }
];

export function MainLayout({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <SidebarProvider>
      <div className="flex min-h-screen bg-background w-full">
        <Sidebar>
          <SidebarHeader className="h-16 flex items-center px-6 border-b">
            <div className="flex items-center gap-3">
              <div className="rounded-lg flex items-center justify-center">
                <LogoIcon width={32} height={32} />
              </div>
              <span className="font-bold text-lg tracking-tight">OpetS Manager</span>
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
                      <item.icon className="h-4 w-4" />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarContent>
        </Sidebar>
        <SidebarInset className="flex-1 flex flex-col min-w-0 bg-muted/20">
          <header className="h-16 flex items-center justify-between px-6 border-b bg-background sticky top-0 z-10">
            <div className="flex items-center gap-4">
              <SidebarTrigger />
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm" className="hidden sm:flex" onClick={() => navigate("/os/new")}>
                <Plus />
                Nova OS</Button>
            </div>
          </header>
          <main className="flex-1 p-6 lg:p-10 max-w-[1600px] mx-auto w-full">
            {children}
          </main>
        </SidebarInset>
      </div>
    </SidebarProvider>
  );
}
