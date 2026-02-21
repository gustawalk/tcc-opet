import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { 
  Plus, 
  Search, 
  MoreVertical, 
  ClipboardList, 
  Edit, 
  Trash2, 
  Save, 
  X,
  GripVertical
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
import { ChecklistTemplate } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";

// Mock para simular busca de templates
const fetchTemplates = async (): Promise<ChecklistTemplate[]> => {
  await new Promise(resolve => setTimeout(resolve, 800));
  return [
    { 
      id: "1", 
      title: "Checklist Smartphone", 
      items: ["Tela/Touch", "Câmera Frontal", "Câmera Traseira", "Microfone", "Alto-falante", "Carga", "Botões", "Wifi/Sinal"],
      created_at: "2023-10-01" 
    },
    { 
      id: "2", 
      title: "Checklist Notebook", 
      items: ["Teclado", "Touchpad", "Tela", "Webcam", "Portas USB", "Carregador", "Bateria", "Som", "Temperatura"],
      created_at: "2023-10-05" 
    },
  ];
};

export function Templates() {
  const [searchTerm, setSearchTerm] = useState("");
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<ChecklistTemplate | null>(null);
  
  // Form State
  const [title, setTitle] = useState("");
  const [items, setItems] = useState<string[]>([]);
  const [newItem, setNewItem] = useState("");

  const { data: templates = [], isLoading } = useQuery({
    queryKey: ["checklist-templates"],
    queryFn: fetchTemplates,
  });

  const filteredTemplates = useMemo(() => {
    return templates.filter(t => 
      t.title.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [templates, searchTerm]);

  const handleAddTemplate = () => {
    setSelectedTemplate(null);
    setTitle("");
    setItems([]);
    setNewItem("");
    setIsSheetOpen(true);
  };

  const handleEditTemplate = (template: ChecklistTemplate) => {
    setSelectedTemplate(template);
    setTitle(template.title);
    setItems([...template.items]);
    setNewItem("");
    setIsSheetOpen(true);
  };

  const handleAddItem = () => {
    if (newItem.trim()) {
      setItems([...items, newItem.trim()]);
      setNewItem("");
    }
  };

  const handleRemoveItem = (index: number) => {
    setItems(items.filter((_, i) => i !== index));
  };

  const handleSave = () => {
    if (!title.trim()) {
      alert("O título é obrigatório");
      return;
    }
    if (items.length === 0) {
      alert("Adicione pelo menos um item ao checklist");
      return;
    }

    const payload = {
      title,
      items
    };

    if (selectedTemplate) {
      console.log("Ação: Atualizar Template", { id: selectedTemplate.id, ...payload });
      alert("Template atualizado!");
    } else {
      console.log("Ação: Criar Novo Template", payload);
      alert("Template criado!");
    }
    setIsSheetOpen(false);
  };

  const handleDeleteTemplate = (id: string) => {
    if (window.confirm("Deseja realmente excluir este template?")) {
      console.log("Ação: Deletar Template", id);
      alert("Template removido!");
    }
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Templates</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie modelos de checklists para suas ordens de serviço.
          </p>
        </div>
        <Button onClick={handleAddTemplate} className="gap-2">
          <Plus className="h-4 w-4" /> Novo Template
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Modelos de Checklist</CardTitle>
              <CardDescription>Checklists padronizados para diferentes tipos de aparelhos.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar template..."
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
                ) : filteredTemplates.length > 0 ? (
                  filteredTemplates.map((template) => (
                    <TableRow key={template.id}>
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
                        {template.created_at ? new Date(template.created_at).toLocaleDateString('pt-BR') : '-'}
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
                      Nenhum template encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      {/* Sheet para Cadastro/Edição de Template */}
      <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent className="sm:max-w-md flex flex-col h-full">
          <SheetHeader>
            <SheetTitle>{selectedTemplate ? "Editar Template" : "Novo Template"}</SheetTitle>
            <SheetDescription>
              Crie uma lista de verificação para ser preenchida na entrada de aparelhos.
            </SheetDescription>
          </SheetHeader>

          <div className="flex-1 overflow-hidden flex flex-col gap-6 py-6">
            <div className="grid gap-2">
              <Label htmlFor="title">Título do Template</Label>
              <Input 
                id="title" 
                value={title}
                placeholder="Ex: Checklist iPhone"
                onChange={(e) => setTitle(e.target.value)}
              />
            </div>
            
            <Separator />

            <div className="flex flex-col gap-4 flex-1 overflow-hidden">
              <Label>Itens do Checklist</Label>
              <div className="flex gap-2">
                <Input 
                  placeholder="Novo item..." 
                  value={newItem}
                  onChange={(e) => setNewItem(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleAddItem()}
                />
                <Button size="icon" onClick={handleAddItem}>
                  <Plus className="h-4 w-4" />
                </Button>
              </div>

              <ScrollArea className="flex-1 border rounded-md p-2">
                <div className="space-y-2">
                  {items.map((item, index) => (
                    <div key={index} className="flex items-center gap-2 group bg-muted/50 p-2 rounded-sm border border-transparent hover:border-primary/20 transition-colors">
                      <GripVertical className="h-3 w-3 text-muted-foreground" />
                      <span className="text-sm flex-1">{item}</span>
                      <Button 
                        variant="ghost" 
                        size="icon" 
                        className="h-6 w-6 text-destructive opacity-0 group-hover:opacity-100 transition-opacity"
                        onClick={() => handleRemoveItem(index)}
                      >
                        <X className="h-3 w-3" />
                      </Button>
                    </div>
                  ))}
                  {items.length === 0 && (
                    <div className="text-center py-8 text-muted-foreground text-xs italic">
                      Nenhum item adicionado ainda.
                    </div>
                  )}
                </div>
              </ScrollArea>
            </div>
          </div>

          <SheetFooter className="pt-4 border-t">
            <Button variant="outline" className="w-full" onClick={() => setIsSheetOpen(false)}>
              Cancelar
            </Button>
            <Button className="w-full gap-2" onClick={handleSave}>
              <Save className="h-4 w-4" /> {selectedTemplate ? "Salvar Alterações" : "Criar Template"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
