import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { 
  UserPlus, 
  Search, 
  MoreVertical, 
  Phone, 
  Mail, 
  MapPin, 
  FileText,
  Edit,
  Trash2
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
import { Customer } from "@/lib/types";

// Mock para simular busca de clientes
const fetchCustomers = async (): Promise<Customer[]> => {
  await new Promise(resolve => setTimeout(resolve, 800));
  return [
    { id: "1", name: "Maria Silva", phone: "(41) 99999-1111", email: "maria@email.com", address: "Rua das Flores, 123 - Curitiba", created_at: "2023-01-15" },
    { id: "2", name: "João Pereira", phone: "(41) 98888-2222", email: "joao@email.com", address: "Av. Principal, 500 - Araucária", created_at: "2023-02-20" },
    { id: "3", name: "Empresa ABC", phone: "(41) 3333-4444", email: "contato@abc.com", address: "Rua Industrial, 10 - Curitiba", created_at: "2023-03-10" },
    { id: "4", name: "Carlos Oliveira", phone: "(41) 97777-3333", email: "carlos@email.com", address: "Rua das Palmeiras, 45 - São José dos Pinhais", created_at: "2023-04-05" },
    { id: "5", name: "Ana Santos", phone: "(41) 96666-4444", email: "ana@email.com", address: "Rua XV de Novembro, 1000 - Curitiba", created_at: "2023-05-12" },
  ];
};

export function Customers() {
  const [searchTerm, setSearchTerm] = useState("");

  const { data: customers = [], isLoading } = useQuery({
    queryKey: ["customers"],
    queryFn: fetchCustomers,
  });

  const filteredCustomers = useMemo(() => {
    return customers.filter(customer => 
      customer.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      customer.email.toLowerCase().includes(searchTerm.toLowerCase()) ||
      customer.phone.includes(searchTerm)
    );
  }, [customers, searchTerm]);

  const handleAddCustomer = () => {
    console.log("Ação: Abrir modal de novo cliente");
  };

  const handleEditCustomer = (customer: Customer) => {
    console.log("Ação: Editar cliente", customer);
  };

  const handleDeleteCustomer = (id: string) => {
    console.log("Ação: Deletar cliente", id);
  };

  const handleViewOS = (customer: Customer) => {
    console.log("Ação: Ver Ordens de Serviço de", customer.name);
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Clientes</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie o cadastro de clientes da sua assistência.
          </p>
        </div>
        <Button onClick={handleAddCustomer} className="gap-2">
          <UserPlus className="h-4 w-4" /> Novo Cliente
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Base de Clientes</CardTitle>
              <CardDescription>Consulte e gerencie as informações de contato dos seus clientes.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar cliente..."
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
                  <TableHead>Cliente</TableHead>
                  <TableHead className="hidden md:table-cell">Contato</TableHead>
                  <TableHead className="hidden lg:table-cell">Endereço</TableHead>
                  <TableHead className="hidden md:table-cell">Cadastrado em</TableHead>
                  <TableHead className="text-right w-[100px]">Ações</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 5 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-5 w-32 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-40 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden lg:table-cell"><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                    </TableRow>
                  ))
                ) : filteredCustomers.length > 0 ? (
                  filteredCustomers.map((customer) => (
                    <TableRow key={customer.id}>
                      <TableCell className="font-medium">
                        <div className="flex flex-col">
                          {customer.name}
                          <span className="text-xs text-muted-foreground md:hidden">{customer.phone}</span>
                        </div>
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        <div className="flex flex-col gap-1">
                          <div className="flex items-center gap-1.5 text-xs">
                            <Phone className="h-3 w-3 text-muted-foreground" />
                            {customer.phone}
                          </div>
                          <div className="flex items-center gap-1.5 text-xs">
                            <Mail className="h-3 w-3 text-muted-foreground" />
                            {customer.email}
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="hidden lg:table-cell max-w-[250px] truncate text-xs">
                        <div className="flex items-center gap-1.5">
                          <MapPin className="h-3 w-3 text-muted-foreground shrink-0" />
                          {customer.address}
                        </div>
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-xs">
                        {customer.created_at ? new Date(customer.created_at).toLocaleDateString('pt-BR') : '-'}
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
                            <DropdownMenuItem onClick={() => handleViewOS(customer)}>
                              <FileText className="mr-2 h-4 w-4" /> Ver OS
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleEditCustomer(customer)}>
                              <Edit className="mr-2 h-4 w-4" /> Editar
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem 
                              className="text-destructive focus:text-destructive"
                              onClick={() => handleDeleteCustomer(customer.id)}
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
                    <TableCell colSpan={5} className="h-24 text-center text-muted-foreground">
                      Nenhum cliente encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
