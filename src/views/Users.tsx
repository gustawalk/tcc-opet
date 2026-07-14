import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { 
  UserPlus, 
  Search, 
  MoreVertical, 
  Edit, 
  Trash2,
  Mail,
  User,
  Save,
  Calendar,
  Phone,
  IdCard,
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
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";
import { userSchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { useSort } from "@/hooks/useSort";
import { SortableHeader } from "@/components/shared/SortableHeader";
import { formatBRPhone, formatCPF, formatDate, formatName } from "@/lib/formatters";

const fetchUsers = async (): Promise<UserType[]> => {
  return await invoke<UserType[]>("get_users");
};

export function Users() {
  const [searchTerm, setSearchTerm] = useState("");
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedUser, setSelectedUser] = useState<UserType | null>(null);
  const [errors, setErrors] = useState<ValidationErrors>({});
  const queryClient = useQueryClient();
  const { sortConfig, cycleSort } = useSort();
  
  const [formData, setFormData] = useState({
    name: "",
    email: "",
    phone: "",
    cpf: "",
    joinDate: new Date().toISOString().split("T")[0],
  });

  const { data: users = [], isLoading } = useQuery({
    queryKey: ["users"],
    queryFn: fetchUsers,
  });

  const createMutation = useMutation({
    mutationFn: async (data: { name: string; email: string; phone?: string; cpf?: string; joinDate?: string }) => {
      return await invoke("create_user", data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
      setIsSheetOpen(false);
    },
    onError: (err) => {
      alert(`Erro ao criar usuário: ${err}`);
    },
  });

  const updateMutation = useMutation({
    mutationFn: async (data: { id: string; name: string; email: string; phone?: string; cpf?: string; joinDate?: string }) => {
      return await invoke("update_user", data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
      setIsSheetOpen(false);
    },
    onError: (err) => {
      alert(`Erro ao atualizar usuário: ${err}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      return await invoke("delete_user", { id });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
    },
    onError: (err) => {
      alert(`Erro ao excluir usuário: ${err}`);
    },
  });

  const getUserSortValue = (user: UserType, column: string): string | number => {
    switch (column) {
      case "name": return user.name;
      case "phone": return user.phone || "";
      case "joinDate": return user.joinDate || "";
      default: return "";
    }
  };

  const filteredUsers = useMemo(() => {
    let result = users.filter(user => 
      user.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      user.email.toLowerCase().includes(searchTerm.toLowerCase())
    );

    if (sortConfig.direction && sortConfig.column) {
      const dir = sortConfig.direction;
      const col = sortConfig.column;
      result = [...result].sort((a, b) => {
        const valA = getUserSortValue(a, col);
        const valB = getUserSortValue(b, col);
        if (valA == null && valB == null) return 0;
        if (valA == null) return 1;
        if (valB == null) return -1;
        if (typeof valA === "string" && typeof valB === "string") {
          return dir === "asc" ? valA.localeCompare(valB, "pt-BR") : valB.localeCompare(valA, "pt-BR");
        }
        return dir === "asc" ? (valA as number) - (valB as number) : (valB as number) - (valA as number);
      });
    }

    return result;
  }, [users, searchTerm, sortConfig]);

  const handleAddUser = () => {
    setSelectedUser(null);
    setErrors({});
    setFormData({
      name: "",
      email: "",
      phone: "",
      cpf: "",
      joinDate: new Date().toISOString().split("T")[0],
    });
    setIsSheetOpen(true);
  };

  const handleEditUser = (user: UserType) => {
    setSelectedUser(user);
    setErrors({});
    setFormData({ 
      name: user.name, 
      email: user.email, 
      phone: user.phone || "",
      cpf: user.cpf || "",
      joinDate: user.joinDate || new Date().toISOString().split("T")[0],
    });
    setIsSheetOpen(true);
  };

  const handleSave = () => {
    const result = userSchema.safeParse(formData);
    const fieldErrors = parseErrors(result);
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }
    setErrors({});
    if (selectedUser) {
      updateMutation.mutate({ id: selectedUser.id, ...formData });
    } else {
      createMutation.mutate(formData);
    }
  };

  const handleDeleteUser = (id: string) => {
    if (window.confirm("Deseja realmente excluir este usuário?")) {
      deleteMutation.mutate(id);
    }
  };

  const updateField = (field: string, value: string) => {
    setFormData({ ...formData, [field]: value });
    setErrors(clearFieldError(errors, field));
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Funcionários</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie os colaboradores da sua assistência técnica.
          </p>
        </div>
        <Button onClick={handleAddUser} className="gap-2">
          <UserPlus className="h-4 w-4" /> Novo Funcionário
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Equipe</CardTitle>
              <CardDescription>Lista de todos os colaboradores cadastrados.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar funcionário..."
                className="pl-9"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
              />
            </div>
          </div>
        </CardHeader>
        <CardContent>
            <div className="max-h-[500px] overflow-y-auto rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <SortableHeader column="name" label="Nome" sortConfig={sortConfig} onSort={cycleSort} />
                    <SortableHeader column="phone" label="Telefone" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell" />
                    <SortableHeader column="joinDate" label="Data Entrada" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell" />
                    <TableHead className="text-right w-[100px]">Ações</TableHead>
                  </TableRow>
                </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 3 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-10 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
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
                      <TableCell className="hidden md:table-cell text-xs text-muted-foreground">
                        {user.phone || "—"}
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-xs text-muted-foreground">
                        {user.joinDate ? formatDate(user.joinDate) : "—"}
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
                      Nenhum funcionário encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>

        <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>{selectedUser ? "Editar Funcionário" : "Novo Funcionário"}</SheetTitle>
            <SheetDescription>
              Preencha os dados do colaborador.
            </SheetDescription>
          </SheetHeader>

          <div className="grid gap-4 py-6">
            <div className="grid gap-2">
              <Label htmlFor="name">Nome Completo</Label>
              <Input 
                id="name" 
                value={formData.name}
                onChange={(e) => updateField("name", formatName(e.target.value))}
              />
              {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="email">E-mail</Label>
              <Input 
                id="email" 
                type="email"
                value={formData.email}
                onChange={(e) => updateField("email", e.target.value)}
              />
              {errors.email && <p className="text-xs text-destructive">{errors.email}</p>}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="phone">Telefone</Label>
              <div className="relative">
                <Phone className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  id="phone"
                  className="pl-9"
                  value={formData.phone}
                  onChange={(e) => updateField("phone", formatBRPhone(e.target.value))}
                  placeholder="(41) 99999-8888"
                />
              </div>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="cpf">CPF</Label>
              <div className="relative">
                <IdCard className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  id="cpf"
                  className="pl-9"
                  value={formData.cpf}
                  onChange={(e) => updateField("cpf", formatCPF(e.target.value))}
                  placeholder="000.000.000-00"
                />
              </div>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="joinDate">Data de Entrada</Label>
              <div className="relative">
                <Calendar className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  id="joinDate"
                  type="date"
                  className="pl-9"
                  value={formData.joinDate}
                  onChange={(e) => updateField("joinDate", e.target.value)}
                />
              </div>
            </div>
          </div>

          <SheetFooter>
            <Button variant="outline" className="w-full" onClick={() => setIsSheetOpen(false)}>
              Cancelar
            </Button>
            <Button className="w-full gap-2" onClick={handleSave} disabled={createMutation.isPending || updateMutation.isPending}>
              <Save className="h-4 w-4" /> {createMutation.isPending || updateMutation.isPending ? "Salvando..." : selectedUser ? "Salvar Alterações" : "Criar Funcionário"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
