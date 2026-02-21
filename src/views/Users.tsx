import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { 
  UserPlus, 
  Search, 
  MoreVertical, 
  ShieldCheck, 
  ShieldAlert, 
  Edit, 
  Trash2,
  Mail,
  User,
  Key,
  Save
} from "lucide-react";
import { 
  Card, 
  CardContent, 
  CardDescription, 
  CardHeader, 
  CardTitle 
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { 
  Table, 
  TableBody, 
  TableCell, 
  TableHead, 
  TableHeader, 
  TableRow 
} from "@/components/ui/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { User as UserType } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";

// Mock para simular busca de usuários
const fetchUsers = async (): Promise<UserType[]> => {
  await new Promise(resolve => setTimeout(resolve, 800));
  return [
    { id: "1", name: "Gustavo Admin", email: "admin@opet.com.br", role: 'admin', created_at: "2023-01-01" },
    { id: "2", name: "João Técnico", email: "joao@opet.com.br", role: 'tech', created_at: "2023-02-15" },
    { id: "3", name: "Maria Técnica", email: "maria@opet.com.br", role: 'tech', created_at: "2023-03-20" },
  ];
};

export function Users() {
  const [searchTerm, setSearchTerm] = useState("");
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedUser, setSelectedUser] = useState<UserType | null>(null);
  
  // Form State
  const [formData, setFormData] = useState({
    name: "",
    email: "",
    role: "tech" as "admin" | "tech",
    password: ""
  });

  const { data: users = [], isLoading } = useQuery({
    queryKey: ["users"],
    queryFn: fetchUsers,
  });

  const filteredUsers = useMemo(() => {
    return users.filter(user => 
      user.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      user.email.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [users, searchTerm]);

  const handleAddUser = () => {
    setSelectedUser(null);
    setFormData({ name: "", email: "", role: "tech", password: "" });
    setIsSheetOpen(true);
  };

  const handleEditUser = (user: UserType) => {
    setSelectedUser(user);
    setFormData({ 
      name: user.name, 
      email: user.email, 
      role: user.role, 
      password: "" // Não carregamos a senha
    });
    setIsSheetOpen(true);
  };

  const handleSave = () => {
    if (selectedUser) {
      console.log("Ação: Atualizar usuário", { id: selectedUser.id, ...formData });
      alert("Usuário atualizado com sucesso!");
    } else {
      console.log("Ação: Criar novo usuário", formData);
      alert("Usuário criado com sucesso!");
    }
    setIsSheetOpen(false);
  };

  const handleDeleteUser = (id: string) => {
    if (window.confirm("Deseja realmente excluir este usuário?")) {
      console.log("Ação: Deletar usuário", id);
      alert("Usuário removido!");
    }
  };

  const handleResetPassword = (user: UserType) => {
    console.log("Ação: Resetar senha de", user.name);
    alert(`Link de reset enviado para ${user.email}`);
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Usuários</h2>
          <p className="text-muted-foreground mt-1">
            Controle de acesso à plataforma para técnicos e administradores.
          </p>
        </div>
        <Button onClick={handleAddUser} className="gap-2">
          <UserPlus className="h-4 w-4" /> Novo Usuário
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Equipe</CardTitle>
              <CardDescription>Gerencie quem pode acessar o sistema e suas permissões.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar usuário..."
                className="pl-9"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
              />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nome</TableHead>
                  <TableHead>Função</TableHead>
                  <TableHead className="hidden md:table-cell">Cadastrado em</TableHead>
                  <TableHead className="text-right w-[100px]">Ações</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 3 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-10 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-32 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                    </TableRow>
                  ))
                ) : filteredUsers.length > 0 ? (
                  filteredUsers.map((user) => (
                    <TableRow key={user.id}>
                      <TableCell>
                        <div className="flex flex-col gap-1">
                          <span className="font-medium flex items-center gap-1.5">
                            <User className="h-3 w-3 text-muted-foreground" />
                            {user.name}
                          </span>
                          <span className="text-xs text-muted-foreground flex items-center gap-1.5">
                            <Mail className="h-3 w-3 text-muted-foreground" />
                            {user.email}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        {user.role === 'admin' ? (
                          <Badge variant="default" className="gap-1.5">
                            <ShieldCheck className="h-3 w-3" /> Administrador
                          </Badge>
                        ) : (
                          <Badge variant="secondary" className="gap-1.5">
                            <ShieldAlert className="h-3 w-3" /> Técnico
                          </Badge>
                        )}
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-xs text-muted-foreground">
                        {new Date(user.created_at || '').toLocaleDateString('pt-BR')}
                      </TableCell>
                      <TableCell className="text-right">
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon" className="h-8 w-8">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuLabel>Ações</DropdownMenuLabel>
                            <DropdownMenuItem onClick={() => handleEditUser(user)}>
                              <Edit className="mr-2 h-4 w-4" /> Editar
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleResetPassword(user)}>
                              <Key className="mr-2 h-4 w-4" /> Resetar Senha
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem 
                              className="text-destructive focus:text-destructive"
                              onClick={() => handleDeleteUser(user.id)}
                            >
                              <Trash2 className="mr-2 h-4 w-4" /> Excluir
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={4} className="h-24 text-center text-muted-foreground">
                      Nenhum usuário encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      {/* Sheet para Cadastro/Edição */}
      <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>{selectedUser ? "Editar Usuário" : "Novo Usuário"}</SheetTitle>
            <SheetDescription>
              Preencha os dados de acesso e permissão do colaborador.
            </SheetDescription>
          </SheetHeader>

          <div className="grid gap-4 py-6">
            <div className="grid gap-2">
              <Label htmlFor="name">Nome Completo</Label>
              <Input 
                id="name" 
                value={formData.name}
                onChange={(e) => setFormData({...formData, name: e.target.value})}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="email">E-mail</Label>
              <Input 
                id="email" 
                type="email"
                value={formData.email}
                onChange={(e) => setFormData({...formData, email: e.target.value})}
              />
            </div>
            <div className="grid gap-2">
              <Label>Função / Permissão</Label>
              <div className="flex gap-2">
                <Button 
                  variant={formData.role === 'admin' ? "default" : "outline"}
                  className="flex-1 gap-2"
                  onClick={() => setFormData({...formData, role: 'admin'})}
                >
                  <ShieldCheck className="h-4 w-4" /> Admin
                </Button>
                <Button 
                  variant={formData.role === 'tech' ? "default" : "outline"}
                  className="flex-1 gap-2"
                  onClick={() => setFormData({...formData, role: 'tech'})}
                >
                  <ShieldAlert className="h-4 w-4" /> Técnico
                </Button>
              </div>
            </div>
            {!selectedUser && (
              <div className="grid gap-2">
                <Label htmlFor="password">Senha Inicial</Label>
                <Input 
                  id="password" 
                  type="password"
                  value={formData.password}
                  onChange={(e) => setFormData({...formData, password: e.target.value})}
                />
              </div>
            )}
          </div>

          <SheetFooter>
            <Button variant="outline" className="w-full" onClick={() => setIsSheetOpen(false)}>
              Cancelar
            </Button>
            <Button className="w-full gap-2" onClick={handleSave}>
              <Save className="h-4 w-4" /> {selectedUser ? "Salvar Alterações" : "Criar Usuário"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
