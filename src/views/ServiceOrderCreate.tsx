import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  Save,
  User,
  Phone,
  Mail,
  MapPin,
  Wrench,
  FileText,
  Search,
  CheckCircle2,
  Plus,
  ShieldCheck,
  ClipboardCheck,
  ChevronRight
} from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Customer, User as UserType, ChecklistTemplate, ChecklistItem } from "@/lib/types";
import { Checkbox } from "@/components/ui/checkbox";
import { useNavigate } from "react-router-dom";

// Mock para simular busca de clientes do Tauri invoke("get_customers")
const fetchCustomers = async (): Promise<Customer[]> => {
  await new Promise(resolve => setTimeout(resolve, 500));
  return [
    { id: "1", name: "Maria Silva", phone: "(41) 99999-1111", email: "maria@email.com", address: "Rua das Flores, 123 - Curitiba" },
    { id: "2", name: "João Pereira", phone: "(41) 98888-2222", email: "joao@email.com", address: "Av. Principal, 500 - Araucária" },
    { id: "3", name: "Empresa ABC", phone: "(41) 3333-4444", email: "contato@abc.com", address: "Rua Industrial, 10 - Curitiba" },
  ];
};

// Mock para usuários/técnicos
const fetchTechs = async (): Promise<UserType[]> => {
  return [
    { id: "1", name: "Gustavo Admin", email: "admin@opet.com.br", role: 'admin' },
    { id: "2", name: "João Técnico", email: "joao@opet.com.br", role: 'tech' },
  ];
};

// Mock para buscar templates
const fetchTemplates = async (): Promise<ChecklistTemplate[]> => {
  return [
    { id: "1", title: "Checklist Smartphone", items: ["Tela/Touch", "Câmeras", "Microfone/Áudio", "Carga", "Botões", "Wi-Fi"] },
    { id: "2", title: "Checklist Notebook", items: ["Teclado", "Tela", "Webcam", "Portas USB", "Carregador", "Bateria"] },
  ];
};

export function ServiceOrderCreate() {
  const navigate = useNavigate()

  // Estados do formulário
  const [customerSearch, setCustomerSearch] = useState("");
  const [selectedCustomer, setSelectedCustomer] = useState<Customer | null>(null);
  const [showCustomerList, setShowCustomerList] = useState(false);

  const [formData, setFormData] = useState({
    phone: "",
    email: "",
    address: "",
    equipment: "",
    description: "",
    status: "Orçamento" as const,
    techId: "1" // Usuário logado fake
  });

  // Estado da Checklist
  const [selectedTemplate, setSelectedTemplate] = useState<ChecklistTemplate | null>(null);
  const [checklistItems, setChecklistItems] = useState<ChecklistItem[]>([]);
  const [showTemplateList, setShowTemplateList] = useState(false);

  // Busca de clientes
  const { data: customers = [] } = useQuery({
    queryKey: ["customers-list"],
    queryFn: fetchCustomers,
  });

  // Busca de técnicos
  const { data: techs = [] } = useQuery({
    queryKey: ["techs-list"],
    queryFn: fetchTechs,
  });

  // Busca de templates
  const { data: templates = [] } = useQuery({
    queryKey: ["checklist-templates"],
    queryFn: fetchTemplates,
  });

  // Filtragem de clientes para o search
  const filteredCustomers = useMemo(() => {
    if (!customerSearch) return [];
    return customers.filter(c =>
      c.name.toLowerCase().includes(customerSearch.toLowerCase())
    );
  }, [customerSearch, customers]);

  const handleSelectCustomer = (customer: Customer) => {
    setSelectedCustomer(customer);
    setCustomerSearch(customer.name);
    setFormData({
      ...formData,
      phone: customer.phone,
      email: customer.email,
      address: customer.address
    });
    setShowCustomerList(false);
  };

  const handleSelectTemplate = (template: ChecklistTemplate) => {
    setSelectedTemplate(template);
    const initialItems = template.items.map((item, idx) => ({
      id: `${idx}`,
      label: item,
      checked: false
    }));
    setChecklistItems(initialItems);
    setShowTemplateList(false);
  };

  const toggleChecklistItem = (id: string) => {
    setChecklistItems(prev =>
      prev.map(item => item.id === id ? { ...item, checked: !item.checked } : item)
    );
  };

  const handleNewCustomer = () => {
    setSelectedCustomer(null);
    setShowCustomerList(false);
  };

  const handleSave = () => {
    const selectedTech = techs.find(t => t.id === formData.techId);
    const payload = {
      ...formData,
      customerName: customerSearch,
      customerId: selectedCustomer?.id,
      openedBy: selectedTech?.name,
      checklist: selectedTemplate ? {
        title: selectedTemplate.title,
        items: checklistItems
      } : null
    };
    console.log("Salvando OS com Checklist:", payload);
    // Aqui viria o invoke("create_os", { payload })
    alert(`Ordem de serviço criada com sucesso por ${selectedTech?.name}!`);
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" className="rounded-full" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-5 w-5" />
          </Button>
          <div>
            <h2 className="text-3xl font-bold tracking-tight">Nova Ordem de Serviço</h2>
            <p className="text-muted-foreground mt-1">
              Preencha os dados do cliente e do equipamento para iniciar o serviço.
            </p>
          </div>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        {/* Coluna da Esquerda: Dados do Cliente */}
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <User className="h-5 w-5 text-primary" /> Dados do Cliente
            </CardTitle>
            <CardDescription>
              Procure um cliente existente ou cadastre um novo.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="relative">
              <Label htmlFor="customer">Nome do Cliente</Label>
              <div className="relative mt-1">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  id="customer"
                  placeholder="Digite o nome para buscar..."
                  className="pl-9"
                  value={customerSearch}
                  onChange={(e) => {
                    setCustomerSearch(e.target.value);
                    setShowCustomerList(true);
                  }}
                  onFocus={() => setShowCustomerList(true)}
                />
              </div>

              {/* Lista de busca de cliente */}
              {showCustomerList && (customerSearch.length > 0 || filteredCustomers.length > 0) && (
                <Card className="absolute z-10 w-full mt-1 shadow-lg border-primary/20 overflow-hidden">
                  <ScrollArea className="max-h-[200px]">
                    <div className="p-1">
                      {filteredCustomers.length > 0 ? (
                        filteredCustomers.map((c) => (
                          <div
                            key={c.id}
                            className="flex items-center justify-between p-2 hover:bg-accent rounded-sm cursor-pointer transition-colors"
                            onClick={() => handleSelectCustomer(c)}
                          >
                            <div className="flex flex-col">
                              <span className="text-sm font-medium">{c.name}</span>
                              <span className="text-xs text-muted-foreground">{c.phone}</span>
                            </div>
                            <Badge variant="secondary" className="text-[10px]">Cadastrado</Badge>
                          </div>
                        ))
                      ) : (
                        <div
                          className="flex items-center gap-2 p-2 hover:bg-accent rounded-sm cursor-pointer text-primary"
                          onClick={handleNewCustomer}
                        >
                          <Plus className="h-4 w-4" />
                          <span className="text-sm font-medium">Cadastrar "{customerSearch}"</span>
                        </div>
                      )}
                    </div>
                  </ScrollArea>
                </Card>
              )}
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="phone">Telefone</Label>
                <div className="relative">
                  <Phone className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    id="phone"
                    placeholder="(00) 00000-0000"
                    className="pl-9"
                    value={formData.phone}
                    onChange={(e) => setFormData({ ...formData, phone: e.target.value })}
                  />
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">E-mail</Label>
                <div className="relative">
                  <Mail className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    id="email"
                    type="email"
                    placeholder="cliente@email.com"
                    className="pl-9"
                    value={formData.email}
                    onChange={(e) => setFormData({ ...formData, email: e.target.value })}
                  />
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="address">Endereço Completo</Label>
              <div className="relative">
                <MapPin className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  id="address"
                  placeholder="Rua, Número, Bairro, Cidade"
                  className="pl-9"
                  value={formData.address}
                  onChange={(e) => setFormData({ ...formData, address: e.target.value })}
                />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Coluna da Direita: Status e Resumo */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Status Inicial</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                <Badge variant="default" className="px-3 py-1">Orçamento</Badge>
                <p className="text-xs text-muted-foreground mt-2">
                  Novas ordens de serviço são iniciadas como "Orçamento" por padrão.
                </p>
              </div>
            </CardContent>
          </Card>

          <Card className="bg-primary/5 border-primary/20">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-semibold flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-primary" /> Resumo do Registro
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Tipo de Cliente:</span>
                <span className="font-medium">{selectedCustomer ? "Existente" : "Novo Cadastro"}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Técnico Resp.:</span>
                <span className="font-medium flex items-center gap-1">
                  <ShieldCheck className="h-3 w-3 text-primary" />
                  {techs.find(t => t.id === formData.techId)?.name || "Nenhum"}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Data de Abertura:</span>
                <span className="font-medium">{new Date().toLocaleDateString('pt-BR')}</span>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* CHECKLIST SELECTION */}
        <Card className="md:col-span-1">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-lg">
              <ClipboardCheck className="h-5 w-5 text-primary" /> Checklist de Entrada
            </CardTitle>
            <CardDescription>
              Selecione um template para realizar a conferência inicial.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="relative">
              <Label>Template</Label>
              <Button
                variant="outline"
                className="w-full justify-between mt-1 text-left font-normal"
                onClick={() => setShowTemplateList(!showTemplateList)}
              >
                {selectedTemplate ? selectedTemplate.title : "Selecione um modelo..."}
                <Plus className="h-4 w-4 ml-2 opacity-50" />
              </Button>

              {showTemplateList && (
                <Card className="absolute z-10 w-full mt-1 shadow-lg border-primary/20 overflow-hidden">
                  <ScrollArea className="max-h-48">
                    <div className="p-1">
                      {templates.map((t) => (
                        <div
                          key={t.id}
                          className="flex items-center justify-between p-2 hover:bg-accent rounded-sm cursor-pointer transition-colors"
                          onClick={() => handleSelectTemplate(t)}
                        >
                          <span className="text-sm">{t.title}</span>
                          <ChevronRight className="h-3 w-3 text-muted-foreground" />
                        </div>
                      ))}
                    </div>
                  </ScrollArea>
                </Card>
              )}
            </div>

            {selectedTemplate && (
              <div className="space-y-3 pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold uppercase text-muted-foreground">Itens para Conferência</span>
                  <Badge variant="outline" className="text-[10px]">{checklistItems.filter(i => i.checked).length}/{checklistItems.length}</Badge>
                </div>
                <div className="grid gap-2 border rounded-md p-3 bg-muted/20">
                  {checklistItems.map((item) => (
                    <div key={item.id} className="flex items-center space-x-2">
                      <Checkbox
                        id={`item-${item.id}`}
                        checked={item.checked}
                        onChange={() => toggleChecklistItem(item.id)}
                      />
                      <Label
                        htmlFor={`item-${item.id}`}
                        className="text-sm font-medium leading-none cursor-pointer peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                      >
                        {item.label}
                      </Label>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Dados do Equipamento */}
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-lg">
              <Wrench className="h-5 w-5 text-primary" /> Dados do Equipamento & Problema
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="equipment">Equipamento (Marca/Modelo)</Label>
                <Input
                  id="equipment"
                  placeholder="Ex: Notebook Dell Inspiron 15"
                  value={formData.equipment}
                  onChange={(e) => setFormData({ ...formData, equipment: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="serial">Nº de Série / IMEI (Opcional)</Label>
                <Input id="serial" placeholder="Ex: ABC-12345" />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="description">Descrição Detalhada do Problema</Label>
              <div className="relative">
                <FileText className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                <Textarea
                  id="description"
                  placeholder="Descreva o que o cliente relatou e o estado inicial do aparelho..."
                  className="pl-9 min-h-[120px]"
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                />
              </div>
            </div>
          </CardContent>
          <CardFooter className="bg-muted/30 border-t flex justify-between items-center py-4">
            <p className="text-xs text-muted-foreground">
              Certifique-se de que os dados estão corretos antes de salvar.
            </p>
            <Button onClick={handleSave} className="gap-2">
              <Save className="h-4 w-4" /> Criar Ordem de Serviço
            </Button>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}
