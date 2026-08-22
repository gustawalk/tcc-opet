import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { dataCommand } from "@/lib/data-client";
import { 
  Plus, 
  Search, 
  MoreVertical, 
  ClipboardList, 
  Edit, 
  Trash2, 
  AlertTriangle,
} from "lucide-react";
import { 
  Card, 
  CardContent, 
  CardDescription, 
  CardFooter,
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
import { ChecklistTemplate, Page } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Pagination } from "@/components/shared/Pagination";
import { useDebounce } from "@/hooks/use-debounce";
import { toastSuccess, toastError } from "@/lib/errors";
import { ChecklistTemplateSheet } from "@/components/shared/ChecklistTemplateSheet";
import { ChecklistTemplateDetailSheet } from "@/components/shared/ChecklistTemplateDetailSheet";

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const SEARCH_DEBOUNCE_MS = 300;

const fetchTemplatesPage = (args: {
  limit: number;
  offset: number;
  search: string;
}): Promise<Page<ChecklistTemplate>> => {
  return dataCommand<Page<ChecklistTemplate>>("get_checklist_templates_page", args);
};

export function Templates() {
  const listRef = useRef<HTMLDivElement>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const search = useDebounce(searchTerm, SEARCH_DEBOUNCE_MS);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<ChecklistTemplate | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<ChecklistTemplate | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const [isDeleting, setIsDeleting] = useState(false);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["checklist-templates-page", page, pageSize, search],
    queryFn: () =>
      fetchTemplatesPage({
        limit: pageSize,
        offset: (page - 1) * pageSize,
        search,
      }),
    placeholderData: (previousData) => previousData,
  });

  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  useEffect(() => {
    setPage(1);
  }, [search, pageSize]);

  useEffect(() => {
    if (data && page > totalPages) setPage(totalPages);
  }, [data, totalPages, page]);

  const handlePageSizeChange = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };

  const handleAddTemplate = () => {
    setSelectedTemplate(null);
    setIsSheetOpen(true);
  };

  const handleEditTemplate = (template: ChecklistTemplate) => {
    setSelectedTemplate(template);
    setIsSheetOpen(true);
  };

  const handleDeleteTemplate = async (id: string) => {
    setConfirmDeleteId(id);
  };

  const confirmDeleteTemplate = async () => {
    if (!confirmDeleteId || isDeleting) return;
    try {
      setIsDeleting(true);
      await dataCommand("delete_checklist_template", { id: confirmDeleteId });
      await queryClient.invalidateQueries({ queryKey: ["checklist-templates-page"] });
      toastSuccess("Template removido com sucesso.");
    } catch (error) {
      toastError(error, "Erro ao excluir template.");
    } finally {
      setIsDeleting(false);
      setConfirmDeleteId(null);
    }
  };

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-[50vh] gap-4">
        <AlertTriangle className="h-12 w-12 text-destructive" />
        <h3 className="text-xl font-bold">Erro ao carregar modelos de checklist</h3>
        <p className="text-muted-foreground text-center max-w-sm">Não foi possível carregar os modelos de checklist. Tente novamente.</p>
        <Button onClick={() => refetch()}>Tentar Novamente</Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-200 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Modelos de checklist</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie modelos de checklists para suas ordens de serviço.
          </p>
        </div>
        <Button onClick={handleAddTemplate} className="gap-2">
          <Plus className="h-4 w-4" /> Novo modelo
        </Button>
      </div>

      <Card ref={listRef} className="scroll-mt-20">
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Modelos de Checklist</CardTitle>
              <CardDescription>Checklists padronizados para diferentes tipos de aparelhos.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar modelo..."
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
                    <TableHead>Título</TableHead>
                    <TableHead>Itens</TableHead>
                    <TableHead className="hidden md:table-cell">Criado em</TableHead>
                    <TableHead className="text-right w-[100px]">Ações</TableHead>
                  </TableRow>
                </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 2 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-5 w-12 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                    </TableRow>
                  ))
                ) : data && data.items.length > 0 ? (
                  data.items.map((template) => (
                    <TableRow
                      key={template.id}
                      className="cursor-pointer"
                      onClick={() => setDetailTemplate(template)}
                    >
                      <TableCell className="font-medium">
                          <div className="flex items-center gap-2">
                            <ClipboardList className="h-4 w-4 text-primary" />
                            {template.title}
                          </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary">{template.items.length} itens</Badge>
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-xs text-muted-foreground">
                        {template.createdAt ? new Date(template.createdAt).toLocaleDateString('pt-BR') : '-'}
                      </TableCell>
                      <TableCell
                        className="text-right"
                        onClick={(event) => event.stopPropagation()}
                      >
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon" className="h-8 w-8">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuLabel>Ações</DropdownMenuLabel>
                            <DropdownMenuItem onClick={() => handleEditTemplate(template)}>
                              <Edit className="mr-2 h-4 w-4" /> Editar
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem 
                              className="text-destructive focus:text-destructive"
                              onClick={() => handleDeleteTemplate(template.id)}
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
                      Nenhum modelo encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
              </Table>
            </div>
          </CardContent>
          <CardFooter className="border-t px-6 py-4">
            <Pagination
              className="w-full"
              totalItems={total}
              page={page}
              pageSize={pageSize}
              onPageChange={setPage}
              onPageSizeChange={handlePageSizeChange}
              pageSizeOptions={PAGE_SIZE_OPTIONS}
              scrollTargetRef={listRef}
            />
          </CardFooter>
        </Card>

      <ChecklistTemplateSheet
        open={isSheetOpen}
        onOpenChange={(open) => {
          setIsSheetOpen(open);
          if (!open) setSelectedTemplate(null);
        }}
        template={selectedTemplate}
      />
      <ChecklistTemplateDetailSheet
        template={detailTemplate}
        open={detailTemplate !== null}
        onClose={() => setDetailTemplate(null)}
      />

      {confirmDeleteId && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => !isDeleting && setConfirmDeleteId(null)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Excluir modelo de checklist</h3>
            <p className="text-sm text-muted-foreground">
              Esta ação não pode ser desfeita. Deseja realmente excluir este modelo?
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmDeleteId(null)}>
                Cancelar
              </Button>
              <Button variant="destructive" onClick={confirmDeleteTemplate} disabled={isDeleting}>
                {isDeleting ? "Excluindo..." : "Excluir"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
