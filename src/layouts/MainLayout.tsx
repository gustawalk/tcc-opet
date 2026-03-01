import {
  LayoutDashboard,
  Users,
  Package,
  Wrench,
  Settings,
  Search,
  LogOut,
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
  SidebarFooter
} from "@/components/ui/sidebar";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
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
              <div className="h-8 w-8 rounded-lg flex items-center justify-center">
                <LogoIcon />
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
                    isActive={location.pathname === item.path}
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
          <SidebarFooter className="border-t p-4">
            <div className="flex items-center gap-3 px-2 py-1">
              <Avatar className="h-9 w-9 border">
                <AvatarImage src="" />
                <AvatarFallback className="bg-primary/5">AD</AvatarFallback>
              </Avatar>
              <div className="flex flex-col flex-1 min-w-0">
                <span className="text-sm font-semibold truncate">Admin User</span>
                <span className="text-[10px] text-muted-foreground uppercase tracking-wider">Administrador</span>
              </div>
              <Button variant="ghost" size="icon" className="h-8 w-8 text-muted-foreground hover:text-destructive">
                <LogOut className="h-4 w-4" />
              </Button>
            </div>
          </SidebarFooter>
        </Sidebar>
        <SidebarInset className="flex-1 flex flex-col min-w-0 bg-muted/20">
          <header className="h-16 flex items-center justify-between px-6 border-b bg-background sticky top-0 z-10">
            <div className="flex items-center gap-4">
              <SidebarTrigger />
              <Separator orientation="vertical" className="h-4" />
              <div className="hidden md:flex relative max-w-sm">
                <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                <Input
                  type="search"
                  placeholder="Buscar OS ou Cliente..."
                  className="pl-9 h-9 w-[300px] lg:w-[400px]"
                />
              </div>
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
