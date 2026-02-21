import { useState } from "react";
import { 
  Building2, 
  Save, 
  Upload, 
  MapPin, 
  Database, 
  Info
} from "lucide-react";
import { 
  Card, 
  CardContent, 
  CardDescription, 
  CardHeader, 
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { Settings as SettingsType } from "@/lib/types";

// Mock para configurações iniciais
const initialSettings: SettingsType = {
  company_name: "OPET Tech Assistência",
  cnpj: "12.345.678/0001-99",
  address: "Rua Brigadeiro Franco, 1234 - Rebouças, Curitiba - PR",
  logo_path: ""
};

export function Settings() {
  const [settings, setSettings] = useState<SettingsType>(initialSettings);

  const handleSave = () => {
    console.log("Ação: Salvar configurações do sistema", settings);
    alert("Configurações salvas com sucesso!");
  };

  const handleLogoUpload = () => {
    console.log("Ação: Abrir seletor de arquivo para logo");
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500 max-w-4xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Configurações</h2>
          <p className="text-muted-foreground mt-1">
            Personalize as informações da sua assistência e gerencie o sistema.
          </p>
        </div>
        <Button onClick={handleSave} className="gap-2">
          <Save className="h-4 w-4" /> Salvar Alterações
        </Button>
      </div>

      <div className="grid gap-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Building2 className="h-5 w-5 text-primary" /> Dados da Empresa
            </CardTitle>
            <CardDescription>
              Essas informações aparecerão no cabeçalho das Ordens de Serviço e PDFs gerados.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex flex-col md:flex-row gap-6">
              <div className="flex flex-col items-center gap-4">
                <div className="w-32 h-32 rounded-lg border-2 border-dashed flex items-center justify-center bg-muted/50 overflow-hidden group relative">
                  {settings.logo_path ? (
                    <img src={settings.logo_path} alt="Logo" className="w-full h-full object-contain" />
                  ) : (
                    <Building2 className="h-10 w-10 text-muted-foreground" />
                  )}
                  <div 
                    className="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
                    onClick={handleLogoUpload}
                  >
                    <Upload className="h-6 w-6 text-white" />
                  </div>
                </div>
                <Button variant="outline" size="sm" onClick={handleLogoUpload}>Alterar Logo</Button>
              </div>

              <div className="flex-1 space-y-4">
                <div className="grid gap-2">
                  <Label htmlFor="name">Nome da Assistência / Razão Social</Label>
                  <Input 
                    id="name" 
                    value={settings.company_name} 
                    onChange={(e) => setSettings({...settings, company_name: e.target.value})}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="cnpj">CNPJ</Label>
                  <Input 
                    id="cnpj" 
                    value={settings.cnpj} 
                    onChange={(e) => setSettings({...settings, cnpj: e.target.value})}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="address">Endereço Completo</Label>
                  <div className="relative">
                    <MapPin className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                    <Input 
                      id="address" 
                      className="pl-9"
                      value={settings.address} 
                      onChange={(e) => setSettings({...settings, address: e.target.value})}
                    />
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <div className="grid gap-6 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                <Database className="h-5 w-5 text-primary" /> Banco de Dados & Backup
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex flex-col gap-1">
                <span className="text-sm font-medium">Localização do Banco</span>
                <code className="text-[10px] bg-muted p-2 rounded block truncate">
                  /home/user/.local/share/com.tcc.opet/main.db
                </code>
              </div>
              <div className="flex flex-col gap-2">
                <Button variant="outline" size="sm" className="w-full justify-start gap-2" onClick={() => console.log("Ação: Gerar Backup")}>
                  <Save className="h-4 w-4" /> Exportar Backup (SQL)
                </Button>
                <Button variant="outline" size="sm" className="w-full justify-start gap-2" onClick={() => console.log("Ação: Restaurar Backup")}>
                  <Upload className="h-4 w-4" /> Importar Backup
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                <Info className="h-5 w-5 text-primary" /> Sobre o Sistema
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Versão do App:</span>
                  <span className="font-mono font-bold">1.0.0-stable</span>
                </div>
                <Separator />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Tauri Core:</span>
                  <span className="font-mono">2.0.0</span>
                </div>
                <Separator />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ambiente:</span>
                  <Badge variant="outline">Produção Local</Badge>
                </div>
              </div>
              <div className="pt-2">
                <Button variant="ghost" size="sm" className="w-full gap-2" onClick={() => console.log("Ação: Verificar Atualizações")}>
                  Verificar Atualizações
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>

        <Card className="border-destructive/20">
          <CardHeader>
            <CardTitle className="text-lg text-destructive">Zona de Perigo</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground mb-4">
              As ações abaixo são irreversíveis. Tenha certeza antes de prosseguir.
            </p>
            <Button variant="destructive" size="sm" onClick={() => console.log("Ação: Resetar todo o sistema")}>
              Resetar Todos os Dados
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
