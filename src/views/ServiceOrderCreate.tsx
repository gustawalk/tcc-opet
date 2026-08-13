import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  CheckCircle2,
  ClipboardCheck,
  Mail,
  MapPin,
  Paperclip,
  Phone,
  Save,
  Search,
  ShieldCheck,
  Trash2,
  User,
  Wrench,
  X,
} from "lucide-react";
import { useNavigate } from "react-router-dom";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { SearchableSelect } from "@/components/shared/SearchableSelect";
import { ChecklistTemplateSheet } from "@/components/shared/ChecklistTemplateSheet";
import { EmployeeCreateSheet } from "@/components/shared/EmployeeCreateSheet";
import { InventoryItemSheet } from "@/components/shared/InventoryItemSheet";
import {
  ServiceOrderItemLine,
  ServiceOrderItemsEditor,
} from "@/components/shared/ServiceOrderItemsEditor";
import { formatBRPhone, formatCurrency, formatName } from "@/lib/formatters";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  clearFieldError,
  newCustomerSchema,
  parseErrors,
  serviceOrderCreateSchema,
  ValidationErrors,
} from "@/lib/validation";
import {
  ChecklistItem,
  ChecklistTemplate,
  Customer,
  InventoryItem,
  User as UserType,
} from "@/lib/types";

const EMPTY_CUSTOMERS: Customer[] = [];
const EMPTY_USERS: UserType[] = [];
const EMPTY_TEMPLATES: ChecklistTemplate[] = [];
const EMPTY_INVENTORY: InventoryItem[] = [];

type PendingAttachmentSelection = {
  token: string;
  fileNames: string[];
};

const fetchCustomers = () => invoke<Customer[]>("get_customers");
const fetchUsers = () => invoke<UserType[]>("get_users");
const fetchTemplates = () =>
  invoke<ChecklistTemplate[]>("get_checklist_templates");
const fetchInventory = () => invoke<InventoryItem[]>("get_inventory_items");

export function ServiceOrderCreate() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const customerRef = useRef<HTMLDivElement>(null);
  const [customerSearch, setCustomerSearch] = useState("");
  const [selectedCustomer, setSelectedCustomer] = useState<Customer | null>(
    null,
  );
  const [originalCustomer, setOriginalCustomer] = useState<Customer | null>(
    null,
  );
  const [showCustomers, setShowCustomers] = useState(false);
  const [lines, setLines] = useState<ServiceOrderItemLine[]>([]);
  const [selectedTemplate, setSelectedTemplate] =
    useState<ChecklistTemplate | null>(null);
  const [checklistItems, setChecklistItems] = useState<ChecklistItem[]>([]);
  const [isTemplateSheetOpen, setIsTemplateSheetOpen] = useState(false);
  const [isEmployeeSheetOpen, setIsEmployeeSheetOpen] = useState(false);
  const [createInventoryType, setCreateInventoryType] = useState<
    InventoryItem["type"] | null
  >(null);
  const [pendingAttachments, setPendingAttachments] =
    useState<PendingAttachmentSelection | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [formData, setFormData] = useState({
    phone: "",
    email: "",
    address: "",
    equipment: "",
    imei: "",
    description: "",
    techId: "",
  });
  const customersQuery = useQuery({
    queryKey: ["customers-list"],
    queryFn: fetchCustomers,
  });
  const usersQuery = useQuery({ queryKey: ["users"], queryFn: fetchUsers });
  const templatesQuery = useQuery({
    queryKey: ["checklist-templates"],
    queryFn: fetchTemplates,
  });
  const inventoryQuery = useQuery({
    queryKey: ["inventory-lookup"],
    queryFn: fetchInventory,
  });
  const customers = customersQuery.data ?? EMPTY_CUSTOMERS;
  const users = usersQuery.data ?? EMPTY_USERS;
  const templates = templatesQuery.data ?? EMPTY_TEMPLATES;
  const inventory = inventoryQuery.data ?? EMPTY_INVENTORY;
  const lookupLoading =
    customersQuery.isLoading ||
    usersQuery.isLoading ||
    templatesQuery.isLoading ||
    inventoryQuery.isLoading;
  const lookupError =
    customersQuery.isError ||
    usersQuery.isError ||
    templatesQuery.isError ||
    inventoryQuery.isError;

  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (
        customerRef.current &&
        !customerRef.current.contains(event.target as Node)
      )
        setShowCustomers(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, []);

  useEffect(() => {
    const token = pendingAttachments?.token;
    return () => {
      if (token) {
        void invoke("discard_pending_service_order_attachments", { token }).catch(
          () => undefined,
        );
      }
    };
  }, [pendingAttachments?.token]);

  const filteredCustomers = useMemo(
    () =>
      customerSearch
        ? customers.filter((customer) =>
            customer.name.toLowerCase().includes(customerSearch.toLowerCase()),
          )
        : [],
    [customers, customerSearch],
  );
  const total = useMemo(
    () =>
      lines.reduce((sum, line) => sum + line.unitPrice * line.quantity, 0),
    [lines],
  );

  const selectCustomer = (customer: Customer) => {
    setSelectedCustomer(customer);
    setOriginalCustomer({ ...customer, phone: formatBRPhone(customer.phone) });
    setCustomerSearch(customer.name);
    setShowCustomers(false);
    setFormData((data) => ({
      ...data,
      phone: formatBRPhone(customer.phone),
      email: customer.email,
      address: customer.address,
    }));
  };
  const clearSelectedCustomer = () => {
    setSelectedCustomer(null);
    setOriginalCustomer(null);
    setCustomerSearch("");
    setShowCustomers(false);
    setFormData((data) => ({
      ...data,
      phone: "",
      email: "",
      address: "",
    }));
    setErrors((current) => {
      const next = { ...current };
      delete next.name;
      delete next.phone;
      delete next.email;
      delete next.address;
      return next;
    });
  };
  const addItem = (item: InventoryItem) => {
    setLines((current) =>
      current.some((line) => line.inventoryItemId === item.id)
        ? current.filter((line) => line.inventoryItemId !== item.id)
        : item.type === "part" && item.currentQuantity <= 0
          ? current
          : [
              ...current,
              {
                id: item.id,
                inventoryItemId: item.id,
                inventoryItemName: item.name,
                itemType: item.type,
                quantity: 1,
                unitPrice: item.salePrice,
                maxQuantity:
                  item.type === "part" ? item.currentQuantity : undefined,
              },
            ],
    );
  };
  const applyTemplate = (template: ChecklistTemplate) => {
    setSelectedTemplate(template);
    setChecklistItems(
      template.items.map((label, index) => ({
        id: String(index),
        label,
        checked: false,
      })),
    );
  };
  const changeQuantity = (line: ServiceOrderItemLine, next: number) => {
    if (!Number.isSafeInteger(next) || next < 1) return;
    setLines((current) =>
      current.map((entry) =>
        entry.id === line.id ? { ...entry, quantity: next } : entry,
      ),
    );
  };
  const selectAttachments = async () => {
    try {
      const selection = await invoke<PendingAttachmentSelection | null>(
        "select_pending_service_order_attachments",
      );
      if (selection) setPendingAttachments(selection);
    } catch (error) {
      toastError(error, "Erro ao selecionar anexos.");
    }
  };
  const handleSave = async () => {
    if (isSubmitting) return;
    const orderErrors = parseErrors(
      serviceOrderCreateSchema.safeParse(formData),
    );
    const customerErrors = selectedCustomer
      ? null
      : parseErrors(
          newCustomerSchema.safeParse({
            name: customerSearch,
            phone: formData.phone,
            email: formData.email,
            address: formData.address,
          }),
        );
    const allErrors = { ...orderErrors, ...customerErrors };
    if (Object.keys(allErrors).length) {
      setErrors(allErrors);
      return;
    }
    setErrors({});
    setIsSubmitting(true);
    try {
      const fieldsChanged =
        originalCustomer &&
        (formData.phone !== originalCustomer.phone ||
          formData.email !== originalCustomer.email ||
          formData.address !== originalCustomer.address);
      await invoke("create_full_service_order", {
        request: {
          customerAction: selectedCustomer
            ? {
                type: "existing",
                id: selectedCustomer.id,
                update: fieldsChanged
                  ? {
                      phone: formData.phone.trim(),
                      email: formData.email.trim(),
                      address: formData.address.trim(),
                    }
                  : null,
              }
            : {
                type: "new",
                name: customerSearch.trim(),
                phone: formData.phone.trim(),
                email: formData.email.trim(),
                address: formData.address.trim(),
              },
          userId: formData.techId || null,
          equipment: formData.equipment,
          imei: formData.imei || null,
          description: formData.description,
          discountBasisPoints: 0,
          parts: lines.map((l) => ({
            inventoryItemId: l.inventoryItemId,
            quantity: l.quantity,
          })),
          checklistItems: checklistItems.map((item) => ({ label: item.label, checked: item.checked })),
          attachmentToken: pendingAttachments?.token ?? null,
        },
      });
      if (pendingAttachments) setPendingAttachments(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["customers-list"] }),
        queryClient.invalidateQueries({ queryKey: ["customersPage"] }),
        queryClient.invalidateQueries({ queryKey: ["service-orders"] }),
        queryClient.invalidateQueries({ queryKey: ["serviceOrdersPage"] }),
        queryClient.invalidateQueries({ queryKey: ["service-order"] }),
        queryClient.invalidateQueries({
          queryKey: ["service-order-attachments"],
        }),
        queryClient.invalidateQueries({ queryKey: ["dashboard-data"] }),
        queryClient.invalidateQueries({ queryKey: ["financial-report"] }),
        queryClient.invalidateQueries({ queryKey: ["inventory-lookup"] }),
      ]);
      toastSuccess("Ordem de serviço criada com sucesso.");
      navigate("/os");
    } catch (error) {
      toastError(error, "Erro ao criar ordem de serviço.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form className="mx-auto flex max-w-5xl flex-col gap-4 animate-in fade-in duration-200 sm:gap-6" autoComplete="off" onSubmit={(e) => e.preventDefault()}>
      <div className="flex items-start gap-3 sm:items-center sm:gap-4">
        <Button
          variant="ghost"
          size="icon"
          className="rounded-full"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="h-5 w-5" />
        </Button>
        <div>
          <h2 className="text-2xl font-bold tracking-tight sm:text-3xl">
            Nova Ordem
          </h2>
          <p className="text-muted-foreground mt-1">
            Preencha os dados do cliente e do equipamento para iniciar o
            serviço.
          </p>
        </div>
      </div>
      {lookupLoading && (
        <Card>
          <CardContent className="p-4 text-sm text-muted-foreground">
            Carregando dados do formulário...
          </CardContent>
        </Card>
      )}
      {lookupError && (
        <Card className="border-destructive">
          <CardContent className="p-4 text-sm text-destructive">
            Não foi possível carregar os dados necessários. Tente novamente.
          </CardContent>
        </Card>
      )}
      <div className="grid gap-6 md:grid-cols-3">
        <Card className="md:col-span-2">
          <CardHeader className="relative pr-14">
            <CardTitle className="flex gap-2">
              <User className="h-5 w-5 text-primary" />
              Dados do Cliente
            </CardTitle>
            <CardDescription>
              Procure um cliente existente ou cadastre um novo.
            </CardDescription>
            {selectedCustomer && (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-4 top-4 h-8 w-8"
                aria-label="Limpar cliente selecionado"
                title="Limpar cliente selecionado"
                onClick={clearSelectedCustomer}
              >
                <X className="h-4 w-4" />
              </Button>
            )}
          </CardHeader>
          <CardContent className="space-y-4">
            <div ref={customerRef} className="relative">
              <Label>Nome do Cliente</Label>
              <div className="relative mt-1">
                <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                <Input
                  className="pl-9"
                  value={customerSearch}
                  onFocus={() => setShowCustomers(true)}
                  onChange={(e) => {
                    setCustomerSearch(e.target.value);
                    setShowCustomers(true);
                    setErrors(clearFieldError(errors, "name"));
                  }}
                  onBlur={() => setCustomerSearch(formatName(customerSearch))}
                />
              </div>
              {errors.name && !selectedCustomer && (
                <p className="text-xs text-destructive mt-1">{errors.name}</p>
              )}
              {showCustomers && customerSearch && (
                <Card className="absolute z-10 w-full mt-1 shadow-lg">
                  <CardContent className="p-1">
                    {filteredCustomers.length ? (
                      filteredCustomers.map((customer) => (
                        <button
                          type="button"
                          className="block w-full text-left p-2 hover:bg-accent rounded"
                          key={customer.id}
                          onClick={() => selectCustomer(customer)}
                        >
                          <span className="text-sm font-medium">
                            {customer.name}
                          </span>
                          <span className="block text-xs text-muted-foreground">
                            {customer.phone}
                          </span>
                        </button>
                      ))
                    ) : (
                      <p className="p-2 text-sm text-primary">
                        Novo cliente: {customerSearch}
                      </p>
                    )}
                  </CardContent>
                </Card>
              )}
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="phone">Telefone</Label>
                <div className="relative">
                  <Phone className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    id="phone"
                    placeholder="(00) 00000-0000"
                    className="pl-9"
                    value={formData.phone}
                    onChange={(e) => {
                      setFormData((data) => ({
                        ...data,
                        phone: formatBRPhone(e.target.value),
                      }));
                      setErrors(clearFieldError(errors, "phone"));
                    }}
                  />
                </div>
                {!selectedCustomer && errors.phone && (
                  <p className="text-xs text-destructive">{errors.phone}</p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">E-mail</Label>
                <div className="relative">
                  <Mail className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    id="email"
                    type="email"
                    placeholder="cliente@email.com"
                    className="pl-9"
                    value={formData.email}
                    onChange={(e) => {
                      setFormData((data) => ({
                        ...data,
                        email: e.target.value,
                      }));
                      setErrors(clearFieldError(errors, "email"));
                    }}
                  />
                </div>
                {!selectedCustomer && errors.email && (
                  <p className="text-xs text-destructive">{errors.email}</p>
                )}
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="address">Endereço Completo</Label>
              <div className="relative">
                <MapPin className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="address"
                  placeholder="Rua, Número, Bairro, Cidade"
                  className="pl-9"
                  value={formData.address}
                  onChange={(e) => {
                    setFormData((data) => ({
                      ...data,
                      address: e.target.value,
                    }));
                    setErrors(clearFieldError(errors, "address"));
                  }}
                />
              </div>
              {!selectedCustomer && errors.address && (
                <p className="text-xs text-destructive">{errors.address}</p>
              )}
            </div>
            <Label>Responsável Técnico</Label>
            <SearchableSelect
              options={users}
              value={formData.techId}
              onSelect={(user) =>
                setFormData((data) => ({ ...data, techId: user.id }))
              }
              placeholder="Selecione um responsável..."
              searchPlaceholder="Buscar por nome..."
              maxOptions={5}
              getKey={(user) => user.id}
              getLabel={(user) => user.name}
              createLabel="Criar funcionário"
              onCreate={() => setIsEmployeeSheetOpen(true)}
            />
          </CardContent>
        </Card>
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Status Inicial</CardTitle>
            </CardHeader>
            <CardContent>
              <Badge>Orçamento</Badge>
              <p className="text-xs text-muted-foreground mt-2">
                Novas ordens iniciam como orçamento.
              </p>
            </CardContent>
          </Card>
          <Card className="bg-primary/5 border-primary/20">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-semibold flex gap-2">
                <CheckCircle2 className="h-4 w-4 text-primary" />
                Resumo do Registro
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              <div className="flex justify-between gap-3">
                <span className="text-muted-foreground">Tipo de cliente:</span>
                <span className="font-medium text-right">
                  {selectedCustomer ? "Cliente reutilizado" : "Novo cadastro"}
                </span>
              </div>
              <div className="flex justify-between gap-3">
                <span className="text-muted-foreground">Técnico resp.:</span>
                <span className="font-medium flex items-center gap-1 text-right">
                  <ShieldCheck className="h-3 w-3 shrink-0 text-primary" />
                  {users.find((user) => user.id === formData.techId)?.name ||
                    "Nenhum"}
                </span>
              </div>
              <div className="flex justify-between gap-3">
                <span className="text-muted-foreground">Total previsto:</span>
                <span className="font-bold text-primary">
                  {formatCurrency(total)}
                </span>
              </div>
              <div className="flex justify-between gap-3">
                <span className="text-muted-foreground">Data de abertura:</span>
                <span className="font-medium">
                  {new Date().toLocaleDateString("pt-BR")}
                </span>
              </div>
            </CardContent>
          </Card>
        </div>
        <div className="grid gap-4 md:col-span-3 md:grid-cols-[1fr_1.25fr_1fr] md:gap-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex gap-2 text-lg">
              <ClipboardCheck className="h-5 w-5 text-primary" />
              Checklist de Entrada
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <SearchableSelect
              options={templates}
              value={selectedTemplate?.id ?? null}
              onSelect={applyTemplate}
              placeholder="Selecione um modelo..."
              searchPlaceholder="Buscar template..."
              maxOptions={5}
              getKey={(template) => template.id}
              getLabel={(template) => template.title}
              createLabel="Novo checklist"
              onCreate={() => setIsTemplateSheetOpen(true)}
            />
            {checklistItems.map((item) => (
              <div className="flex gap-2 items-center" key={item.id}>
                <Checkbox
                  checked={item.checked}
                  onChange={() =>
                    setChecklistItems((items) =>
                      items.map((current) =>
                        current.id === item.id
                          ? { ...current, checked: !current.checked }
                          : current,
                      ),
                    )
                  }
                />
                <Label>{item.label}</Label>
              </div>
            ))}
          </CardContent>
        </Card>
        <ServiceOrderItemsEditor
          inventory={inventory}
          lines={lines}
          onSelectItem={addItem}
          onQuantityChange={changeQuantity}
          onRemove={(line) =>
            setLines((current) => current.filter((entry) => entry.id !== line.id))
          }
          onCreatePart={() => setCreateInventoryType("part")}
          onCreateService={() => setCreateInventoryType("service")}
        />
        <Card>
          <CardHeader>
            <CardTitle className="flex gap-2 text-lg">
              <Wrench className="h-5 w-5 text-primary" />
              Dados do Aparelho
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <Label>Equipamento</Label>
            <Input
              value={formData.equipment}
              onChange={(e) => {
                setFormData((data) => ({ ...data, equipment: e.target.value }));
                setErrors(clearFieldError(errors, "equipment"));
              }}
            />
            {errors.equipment && (
              <p className="text-xs text-destructive">{errors.equipment}</p>
            )}
            <Label>IMEI / Serial</Label>
            <Input
              value={formData.imei}
              onChange={(e) =>
                setFormData((data) => ({ ...data, imei: e.target.value }))
              }
            />
            <Label>Descrição do Problema</Label>
            <Textarea
              value={formData.description}
              onChange={(e) => {
                setFormData((data) => ({
                  ...data,
                  description: e.target.value,
                }));
                setErrors(clearFieldError(errors, "description"));
              }}
            />
            {errors.description && (
              <p className="text-xs text-destructive">{errors.description}</p>
            )}
          </CardContent>
        </Card>
        </div>
        <Card className="md:col-span-3">
          <CardHeader>
            <CardTitle className="flex gap-2 text-lg">
              <Paperclip className="h-5 w-5 text-primary" />
              Anexos
            </CardTitle>
            <CardDescription>
              Adicione fotos do aparelho ou documentos relacionados ao atendimento.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <Button
              variant="outline"
              className="gap-2"
              onClick={selectAttachments}
              disabled={isSubmitting}
            >
              <Paperclip className="h-4 w-4" />
              Selecionar arquivos
            </Button>
            {pendingAttachments?.fileNames.length ? (
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {pendingAttachments.fileNames.map((fileName, index) => (
                  <div
                    className="flex items-center justify-between gap-2 rounded-md border px-3 py-2 text-sm"
                    key={`${fileName}-${index}`}
                  >
                    <span className="min-w-0 truncate" title={fileName}>
                      {fileName}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 shrink-0 text-destructive"
                      onClick={() => setPendingAttachments(null)}
                      disabled={isSubmitting}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">
                Nenhum anexo selecionado.
              </p>
            )}
          </CardContent>
        </Card>
        <div className="flex justify-stretch md:col-span-3 md:justify-end">
          <Button
            onClick={handleSave}
            disabled={isSubmitting || lookupLoading || lookupError}
            className="w-full gap-2 sm:w-auto"
          >
            <Save className="h-4 w-4" />
            {isSubmitting ? "Criando..." : "Criar ordem"}
          </Button>
        </div>
        <ChecklistTemplateSheet
          open={isTemplateSheetOpen}
          onOpenChange={setIsTemplateSheetOpen}
          onCreated={applyTemplate}
        />
        <EmployeeCreateSheet
          open={isEmployeeSheetOpen}
          onOpenChange={setIsEmployeeSheetOpen}
          onCreated={(employee) =>
            setFormData((data) => ({ ...data, techId: employee.id }))
          }
        />
        <InventoryItemSheet
          open={createInventoryType !== null}
          onOpenChange={(open) => {
            if (!open) setCreateInventoryType(null);
          }}
          initialType={createInventoryType ?? "part"}
          initialPartQuantity={1}
          onCreated={addItem}
        />
      </div>
    </form>
  );
}
