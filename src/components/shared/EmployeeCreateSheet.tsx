import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { dataCommand } from "@/lib/data-client";
import { IdCard, Phone, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DatePicker } from "@/components/shared/DatePicker";
import { toastError, toastSuccess } from "@/lib/errors";
import { formatBRPhone, formatCPF, formatName } from "@/lib/formatters";
import {
  clearFieldError,
  parseErrors,
  userSchema,
  ValidationErrors,
} from "@/lib/validation";
import { User } from "@/lib/types";

type EmployeeFormData = {
  name: string;
  email: string;
  phone: string;
  cpf: string;
  joinDate: string;
};

interface EmployeeCreateSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (employee: User) => void;
}

const createInitialFormData = (): EmployeeFormData => ({
  name: "",
  email: "",
  phone: "",
  cpf: "",
  joinDate: new Date().toISOString().split("T")[0],
});

export function EmployeeCreateSheet({
  open,
  onOpenChange,
  onCreated,
}: EmployeeCreateSheetProps) {
  const queryClient = useQueryClient();
  const [formData, setFormData] = useState<EmployeeFormData>(
    createInitialFormData,
  );
  const [errors, setErrors] = useState<ValidationErrors>({});

  useEffect(() => {
    if (!open) return;
    setFormData(createInitialFormData());
    setErrors({});
  }, [open]);

  const createMutation = useMutation({
    mutationFn: async (data: EmployeeFormData) => {
      const id = await dataCommand<string>("create_user", data);
      return { id, ...data };
    },
  });

  const updateField = <K extends keyof EmployeeFormData>(
    field: K,
    value: EmployeeFormData[K],
  ) => {
    setFormData((current) => ({ ...current, [field]: value }));
    setErrors((current) => clearFieldError(current, field));
  };

  const handleSave = async () => {
    if (createMutation.isPending) return;

    const fieldErrors = parseErrors(userSchema.safeParse(formData));
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }

    setErrors({});
    try {
      const employee = await createMutation.mutateAsync(formData);
      queryClient.setQueryData<User[]>(["users"], (employees = []) => [
        employee,
        ...employees.filter((entry) => entry.id !== employee.id),
      ]);
      onCreated?.(employee);
      toastSuccess("Funcionário criado com sucesso.");
      onOpenChange(false);
    } catch (error) {
      toastError(error, "Erro ao criar funcionário.");
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>Novo Funcionário</SheetTitle>
          <SheetDescription>
            Cadastre o responsável técnico para esta ordem.
          </SheetDescription>
        </SheetHeader>

        <div className="grid gap-4 py-6">
          <div className="grid gap-2">
            <Label htmlFor="employee-create-name">Nome completo</Label>
            <Input
              id="employee-create-name"
              value={formData.name}
              onChange={(event) => updateField("name", event.target.value)}
              onBlur={(event) =>
                updateField("name", formatName(event.target.value))
              }
            />
            {errors.name && (
              <p className="text-xs text-destructive">{errors.name}</p>
            )}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="employee-create-email">E-mail</Label>
            <Input
              id="employee-create-email"
              type="email"
              value={formData.email}
              onChange={(event) => updateField("email", event.target.value)}
            />
            {errors.email && (
              <p className="text-xs text-destructive">{errors.email}</p>
            )}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="employee-create-phone">Telefone</Label>
            <div className="relative">
              <Phone className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                id="employee-create-phone"
                className="pl-9"
                value={formData.phone}
                onChange={(event) =>
                  updateField("phone", formatBRPhone(event.target.value))
                }
                placeholder="(41) 99999-8888"
              />
            </div>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="employee-create-cpf">CPF</Label>
            <div className="relative">
              <IdCard className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                id="employee-create-cpf"
                className="pl-9"
                value={formData.cpf}
                onChange={(event) =>
                  updateField("cpf", formatCPF(event.target.value))
                }
                placeholder="000.000.000-00"
              />
            </div>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="employee-create-join-date">Data de entrada</Label>
            <DatePicker
              id="employee-create-join-date"
              value={formData.joinDate}
              onChange={(value) => updateField("joinDate", value)}
            />
            {errors.joinDate && (
              <p className="text-xs text-destructive">{errors.joinDate}</p>
            )}
          </div>
        </div>

        <SheetFooter>
          <Button
            type="button"
            variant="outline"
            className="w-full"
            onClick={() => onOpenChange(false)}
          >
            Cancelar
          </Button>
          <Button
            type="button"
            className="w-full gap-2"
            onClick={handleSave}
            disabled={createMutation.isPending}
          >
            <Save className="h-4 w-4" />
            {createMutation.isPending ? "Criando..." : "Criar funcionário"}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
