import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { Building2, Database, Info, LoaderCircle, MapPin, Moon, RefreshCw, Save, Sun, Upload } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  type Settings,
  type SystemInfo,
  type BackupSummary,
} from "@/lib/types";
import { settingsSchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { formatCNPJ } from "@/lib/formatters";
import { toastSuccess, toastError } from "@/lib/errors";
import {
  getThemePreference,
  setThemePreference,
  Theme,
} from "@/lib/theme";

const fetchSettings = async (): Promise<Settings> => {
  return await invoke<Settings>("get_settings");
};

const fetchSystemInfo = async (): Promise<SystemInfo> => {
  return await invoke<SystemInfo>("get_system_info");
};

export function Settings() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [restoreSource, setRestoreSource] = useState<string | null>(null);
  const [isResetConfirmOpen, setIsResetConfirmOpen] = useState(false);
  const [isResetStarting, setIsResetStarting] = useState(false);
  const [isLogoUploading, setIsLogoUploading] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<{ available: boolean; version?: string } | null>(null);
  const [theme, setTheme] = useState<Theme>(getThemePreference);

  const { data: settingsData, isError: isSettingsError, refetch: refetchSettings } = useQuery({
    queryKey: ["settings"],
    queryFn: fetchSettings,
  });

  const { data: systemInfo, isLoading: isSystemInfoLoading, isError: isSystemInfoError, refetch: refetchSystemInfo } = useQuery({
    queryKey: ["system-info"],
    queryFn: fetchSystemInfo,
  });

  const [localSettings, setLocalSettings] = useState<Settings>({
    companyName: "",
    cnpj: "",
    address: "",
    logoPath: "",
  });

  useEffect(() => {
    if (settingsData) {
      setLocalSettings(settingsData);
    }
  }, [settingsData]);

  const updateMutation = useMutation({
    mutationFn: async (data: Settings) => {
      return await invoke("update_settings", { settings: data });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      toastSuccess("Configurações salvas com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao salvar configurações."),
  });

  const exportMutation = useMutation({
    mutationFn: async (destination: string) => invoke<BackupSummary>("export_backup", { destination }),
    onSuccess: (backup) => toastSuccess(`Backup exportado com ${backup.attachmentCount} anexo(s).`),
    onError: (err) => toastError(err, "Erro ao exportar backup."),
  });

  const restoreMutation = useMutation({
    mutationFn: async (source: string) => invoke<BackupSummary>("restore_backup", { source }),
    onSuccess: async (backup) => {
      setRestoreSource(null);
      await queryClient.invalidateQueries();
      toastSuccess(`Backup restaurado com ${backup.attachmentCount} anexo(s).`);
    },
    onError: (err) => toastError(err, "Erro ao restaurar backup."),
  });

  const resetMutation = useMutation({
    mutationFn: async () => invoke("reset_database"),
    onSuccess: () => {
      setIsResetConfirmOpen(false);
      queryClient.clear();
      toastSuccess("Todos os dados foram resetados.");
      navigate("/");
    },
    onError: (err) => toastError(err, "Erro ao resetar os dados."),
    onSettled: () => setIsResetStarting(false),
  });
  const isResetting = isResetStarting || resetMutation.isPending;

  const updateCheckMutation = useMutation({
    mutationFn: async () => {
      const update = await check();
      if (update) {
        setUpdateInfo({ available: true, version: update.version });
        toastSuccess(`Nova versão disponível: ${update.version}.`);
      } else {
        setUpdateInfo({ available: false });
        toastSuccess("Você já está usando a versão mais recente.");
      }
    },
    onError: (err) => {
      setUpdateInfo({ available: false });
      toastError(err, "Erro ao verificar atualizações.");
    },
  });

  const handleSave = () => {
    const result = settingsSchema.safeParse(localSettings);
    const fieldErrors = parseErrors(result);
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }
    setErrors({});
    updateMutation.mutate(localSettings);
  };

  const handleLogoUpload = async () => {
    try {
      setIsLogoUploading(true);
      const dataUrl = await invoke<string | null>("select_company_logo");
      if (dataUrl) {
        setLocalSettings((prev) => ({ ...prev, logoPath: dataUrl }));
      }
    } catch (err) {
      toastError(err, "Erro ao carregar a logo.");
    } finally {
      setIsLogoUploading(false);
    }
  };

  const handleExport = async () => {
    try {
      const destination = await save({
        defaultPath: "opets-backup.osbkp",
        filters: [{ name: "Backup OPETS", extensions: ["osbkp"] }],
      });
      if (destination) exportMutation.mutate(destination);
    } catch (err) {
      toastError(err, "Erro ao selecionar o destino do backup.");
    }
  };

  const handleImport = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Backup OPETS", extensions: ["osbkp"] }],
      });
      if (typeof selected === "string") setRestoreSource(selected);
    } catch (err) {
      toastError(err, "Erro ao selecionar o arquivo de backup.");
    }
  };
  const startReset = () => {
    setIsResetStarting(true);
    requestAnimationFrame(() => resetMutation.mutate());
  };
  const toggleTheme = () => {
    const nextTheme = theme === "dark" ? "light" : "dark";
    setTheme(nextTheme);
    setThemePreference(nextTheme);
  };

  return (
    <form className="flex flex-col gap-6 animate-in fade-in duration-200 max-w-4xl mx-auto" autoComplete="off" onSubmit={(e) => e.preventDefault()}>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Configurações</h2>
          <p className="text-muted-foreground mt-1">
            Personalize as informações da sua assistência e gerencie o sistema.
          </p>
        </div>
        <Button onClick={handleSave} disabled={updateMutation.isPending} className="gap-2">
          <Save className="h-4 w-4" /> {updateMutation.isPending ? "Salvando..." : "Salvar Alterações"}
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
                <div className="w-32 h-32 rounded-lg border-2 border-dashed flex items-center justify-center bg-muted/50 overflow-hidden relative">
                  {localSettings.logoPath ? (
                    <img src={localSettings.logoPath} alt="Logo" className="w-full h-full object-contain" />
                  ) : (
                    <Building2 className="h-10 w-10 text-muted-foreground" />
                  )}
                </div>
                <Button variant="outline" size="sm" onClick={handleLogoUpload} disabled={isLogoUploading}>
                  {isLogoUploading ? "Carregando..." : "Alterar Logo"}
                </Button>
              </div>

              <div className="flex-1 space-y-4">
                <div className="grid gap-2">
                  <Label htmlFor="name">Razão Social</Label>
                  <Input 
                    id="name" 
                    value={localSettings.companyName} 
                    onChange={(e) => { setLocalSettings({...localSettings, companyName: e.target.value}); setErrors(clearFieldError(errors, "companyName")); }}
                  />
                  {errors.companyName && <p className="text-xs text-destructive">{errors.companyName}</p>}
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="cnpj">CNPJ</Label>
                  <Input 
                    id="cnpj" 
                    value={localSettings.cnpj} 
                    onChange={(e) => { setLocalSettings({...localSettings, cnpj: formatCNPJ(e.target.value)}); setErrors(clearFieldError(errors, "cnpj")); }}
                  />
                  {errors.cnpj && <p className="text-xs text-destructive">{errors.cnpj}</p>}
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="address">Endereço Completo</Label>
                  <div className="relative">
                    <MapPin className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                    <Input 
                      id="address" 
                      className="pl-9"
                      value={localSettings.address} 
                      onChange={(e) => { setLocalSettings({...localSettings, address: e.target.value}); setErrors(clearFieldError(errors, "address")); }}
                    />
                  </div>
                  {errors.address && <p className="text-xs text-destructive">{errors.address}</p>}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Aparência</CardTitle>
            <CardDescription>
              A preferência é salva somente neste dispositivo.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium">Tema escuro</p>
              <p className="text-sm text-muted-foreground">
                {theme === "dark" ? "Ativado" : "Desativado"}
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              className="gap-2"
              aria-pressed={theme === "dark"}
              onClick={toggleTheme}
            >
              {theme === "dark" ? (
                <Sun className="h-4 w-4" />
              ) : (
                <Moon className="h-4 w-4" />
              )}
              {theme === "dark" ? "Usar tema claro" : "Usar tema escuro"}
            </Button>
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
                  {isSystemInfoLoading ? "Carregando..." : systemInfo?.databasePath}
                </code>
              </div>
              {isSystemInfoError && (
                <div className="flex items-center justify-between gap-2 text-sm text-destructive">
                  <span>Não foi possível carregar as informações do sistema.</span>
                  <Button variant="outline" size="sm" onClick={() => refetchSystemInfo()}>Tentar novamente</Button>
                </div>
              )}
              <div className="flex flex-col gap-2">
                <Button variant="outline" size="sm" className="w-full justify-start gap-2" onClick={handleExport} disabled={exportMutation.isPending}>
                  <Save className="h-4 w-4" /> {exportMutation.isPending ? "Exportando..." : "Exportar Backup"}
                </Button>
                <Button variant="outline" size="sm" className="w-full justify-start gap-2" onClick={handleImport} disabled={restoreMutation.isPending}>
                  <Upload className="h-4 w-4" /> {restoreMutation.isPending ? "Restaurando..." : "Importar Backup"}
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
                  <span className="font-mono font-bold">{isSystemInfoLoading ? "..." : systemInfo?.appVersion}</span>
                </div>
                <Separator />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Tauri Core:</span>
                  <span className="font-mono">{isSystemInfoLoading ? "..." : systemInfo?.tauriVersion}</span>
                </div>
                <Separator />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ambiente:</span>
                  <Badge variant="outline">{isSystemInfoLoading ? "Carregando..." : systemInfo?.environment}</Badge>
                </div>
              </div>
              <div className="pt-2">
                <Button
                  variant="ghost"
                  size="sm"
                  className="w-full gap-2"
                  onClick={() => updateCheckMutation.mutate()}
                  disabled={updateCheckMutation.isPending}
                >
                  <RefreshCw className={`h-4 w-4 ${updateCheckMutation.isPending ? "animate-spin" : ""}`} />
                  {updateCheckMutation.isPending ? "Verificando..." : "Verificar Atualizações"}
                </Button>
                <p className="mt-2 text-center text-xs text-muted-foreground">
                  {updateInfo === null
                    ? "Clique em Verificar Atualizações para buscar novas versões."
                    : updateInfo.available
                      ? `Versão ${updateInfo.version} disponível.`
                      : "Seu aplicativo está atualizado."}
                </p>
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
            <Button variant="destructive" size="sm" onClick={() => setIsResetConfirmOpen(true)} disabled={isResetting}>
              {isResetting ? "Resetando..." : "Resetar Todos os Dados"}
            </Button>
          </CardContent>
        </Card>
      </div>

      {isSettingsError && (
        <Card className="border-destructive/20">
          <CardContent className="flex items-center justify-between gap-4 pt-6">
            <p className="text-sm text-destructive">Não foi possível carregar as configurações salvas.</p>
            <Button variant="outline" size="sm" onClick={() => refetchSettings()}>Tentar novamente</Button>
          </CardContent>
        </Card>
      )}

      {restoreSource && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => !restoreMutation.isPending && setRestoreSource(null)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Restaurar backup</h3>
            <p className="text-sm text-muted-foreground">
              Os dados atuais serão substituídos pelo conteúdo deste backup. Esta ação não pode ser desfeita.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setRestoreSource(null)} disabled={restoreMutation.isPending}>
                Cancelar
              </Button>
              <Button
                variant="destructive"
                disabled={restoreMutation.isPending}
                onClick={() => {
                  if (restoreSource) restoreMutation.mutate(restoreSource);
                }}
              >
                {restoreMutation.isPending ? "Restaurando..." : "Restaurar backup"}
              </Button>
            </div>
          </div>
        </div>
      )}

      {isResetConfirmOpen && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => !isResetting && setIsResetConfirmOpen(false)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Resetar todos os dados</h3>
            <p className="text-sm text-muted-foreground">
              Todos os dados e anexos serão excluídos permanentemente. Esta ação não pode ser desfeita.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setIsResetConfirmOpen(false)} disabled={isResetting}>
                Cancelar
              </Button>
              <Button
                variant="destructive"
                disabled={isResetting}
                onClick={() => startReset()}
              >
                {isResetting ? "Resetando..." : "Resetar dados"}
              </Button>
            </div>
          </div>
        </div>
      )}
      {isResetting && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-background/80 p-6 backdrop-blur-sm"
          role="status"
          aria-live="assertive"
        >
          <div className="flex max-w-sm flex-col items-center gap-4 rounded-lg border bg-card p-8 text-center shadow-xl">
            <LoaderCircle className="h-8 w-8 animate-spin text-primary" />
            <div className="space-y-1">
              <p className="font-semibold">Resetando dados</p>
              <p className="text-sm text-muted-foreground">
                Não feche o aplicativo enquanto removemos os dados e anexos.
              </p>
            </div>
          </div>
        </div>
      )}
    </form>
  );
}
