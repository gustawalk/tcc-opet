import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Building2, Database, Eye, EyeOff, History, Info, LoaderCircle, MapPin, Moon, RefreshCw, Save, Sun, Upload } from "lucide-react";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
  type BackupInspection,
} from "@/lib/types";
import { settingsSchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { formatCNPJ } from "@/lib/formatters";
import { toastSuccess, toastError } from "@/lib/errors";
import { releaseNotes } from "@/lib/release-notes";
import {
  getThemePreference,
  setThemePreference,
  Theme,
} from "@/lib/theme";

const ERROR_MESSAGES: Record<string, string> = {
  "error sending request for url (https://github.com/gustawalk/tcc-opet/releases/latest/download/updater.json)": "Não foi possível verificar as atualizações."
}

const fetchSettings = async (): Promise<Settings> => {
  return await invoke<Settings>("get_settings");
};

const fetchSystemInfo = async (): Promise<SystemInfo> => {
  return await invoke<SystemInfo>("get_system_info");
};

type UpdateProgress = {
  downloadedBytes: number;
  totalBytes?: number;
  phase: "downloading" | "installing";
};

type BackupPassphraseDialogMode = "export" | "restore";

const UPDATE_PATCH_NOTES_STORAGE_KEY = "opets.pending-update-patch-notes";

const formatVersion = (version: string) => (version.startsWith("v") ? version : `v${version}`);

export function Settings() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [restoreSource, setRestoreSource] = useState<string | null>(null);
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [pendingRestoreSource, setPendingRestoreSource] = useState<string | null>(null);
  const [backupPassphrase, setBackupPassphrase] = useState("");
  const [isBackupPassphraseVisible, setIsBackupPassphraseVisible] = useState(false);
  const [backupPassphraseDialog, setBackupPassphraseDialog] =
    useState<BackupPassphraseDialogMode | null>(null);
  const [isValidatingBackupPassphrase, setIsValidatingBackupPassphrase] = useState(false);
  const [isResetConfirmOpen, setIsResetConfirmOpen] = useState(false);
  const [isResetStarting, setIsResetStarting] = useState(false);
  const [isLogoUploading, setIsLogoUploading] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<{ available: boolean; version?: string } | null>(null);
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress | null>(null);
  const [isReleaseHistoryOpen, setIsReleaseHistoryOpen] = useState(false);
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
    mutationFn: async ({ destination, passphrase }: { destination: string; passphrase: string }) =>
      invoke<BackupSummary>("export_backup", { destination, passphrase }),
    onSuccess: (backup) => toastSuccess(`Backup exportado com ${backup.attachmentCount} anexo(s).`),
    onError: (err) => toastError(err, "Erro ao exportar backup."),
  });

  const restoreMutation = useMutation({
    mutationFn: async ({ source, passphrase }: { source: string; passphrase: string }) =>
      invoke<BackupSummary>("restore_backup", { source, passphrase }),
    onSuccess: async (backup) => {
      setRestoreSource(null);
      setRestorePassphrase("");
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
      try {
        const update = await check();
        if (update) {
          setUpdateInfo({ available: true, version: update.version });
          setPendingUpdate(update);
        } else {
          setUpdateInfo({ available: false });
          toastSuccess("Você já está usando a versão mais recente.");
        }
      } catch (err) {
        const errorMessage = String(err);
        if (errorMessage in ERROR_MESSAGES) {
          toastError(ERROR_MESSAGES[errorMessage])
        }
      }
    },
    onError: (err) => {
      setUpdateInfo({ available: false });
      toastError(err, "Erro ao verificar atualizações.");
    },
  });

  const installUpdate = async () => {
    if (!pendingUpdate) return;

    try {
      if (pendingUpdate.body?.trim()) {
        localStorage.setItem(
          UPDATE_PATCH_NOTES_STORAGE_KEY,
          JSON.stringify({ version: pendingUpdate.version, body: pendingUpdate.body }),
        );
      }
      setIsUpdating(true);
      setUpdateProgress({ downloadedBytes: 0, phase: "downloading" });

      await pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            setUpdateProgress({
              downloadedBytes: 0,
              totalBytes: event.data.contentLength,
              phase: "downloading",
            });
            break;
          case "Progress":
            setUpdateProgress((current) => ({
              downloadedBytes: (current?.downloadedBytes ?? 0) + event.data.chunkLength,
              totalBytes: current?.totalBytes,
              phase: "downloading",
            }));
            break;
          case "Finished":
            setUpdateProgress((current) => ({
              downloadedBytes: current?.downloadedBytes ?? 0,
              totalBytes: current?.totalBytes,
              phase: "installing",
            }));
            break;
        }
      });

      await relaunch();
    } catch (err) {
      setIsUpdating(false);
      setUpdateProgress(null);
      toastError(err, "Não foi possível concluir a atualização.");
    }
  };

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

  const handleExport = async (passphrase: string) => {
    try {
      const destination = await save({
        defaultPath: "opets-backup.osbkp",
        filters: [{ name: "Backup OPETS", extensions: ["osbkp"] }],
      });
      if (!destination) return;
      exportMutation.mutate({ destination, passphrase });
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
      if (typeof selected !== "string") return;
      const inspection = await invoke<BackupInspection>("inspect_backup", {
        source: selected,
      });
      if (inspection.requiresPassphrase) {
        setPendingRestoreSource(selected);
        setBackupPassphrase("");
        setIsBackupPassphraseVisible(false);
        setBackupPassphraseDialog("restore");
      } else {
        setRestorePassphrase("");
        setRestoreSource(selected);
      }
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
  const closeBackupPassphraseDialog = () => {
    setBackupPassphraseDialog(null);
    setBackupPassphrase("");
    setIsBackupPassphraseVisible(false);
    setPendingRestoreSource(null);
  };
  const confirmBackupPassphrase = async () => {
    const mode = backupPassphraseDialog;
    const passphrase = backupPassphrase;

    if (mode === "export") {
      setBackupPassphraseDialog(null);
      setBackupPassphrase("");
      setIsBackupPassphraseVisible(false);
      void handleExport(passphrase);
      return;
    }
    if (mode === "restore" && pendingRestoreSource) {
      try {
        setIsValidatingBackupPassphrase(true);
        await invoke("validate_backup_passphrase", {
          source: pendingRestoreSource,
          passphrase,
        });
        setRestorePassphrase(passphrase);
        setRestoreSource(pendingRestoreSource);
        setBackupPassphraseDialog(null);
        setBackupPassphrase("");
        setIsBackupPassphraseVisible(false);
        setPendingRestoreSource(null);
      } catch (err) {
        toastError(err, "Não foi possível validar a senha do backup.");
      } finally {
        setIsValidatingBackupPassphrase(false);
      }
    }
  };
  const updateProgressPercentage = updateProgress?.totalBytes
    ? Math.min(100, Math.round((updateProgress.downloadedBytes / updateProgress.totalBytes) * 100))
    : null;

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
                    onChange={(e) => { setLocalSettings({ ...localSettings, companyName: e.target.value }); setErrors(clearFieldError(errors, "companyName")); }}
                  />
                  {errors.companyName && <p className="text-xs text-destructive">{errors.companyName}</p>}
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="cnpj">CNPJ</Label>
                  <Input
                    id="cnpj"
                    value={localSettings.cnpj}
                    onChange={(e) => { setLocalSettings({ ...localSettings, cnpj: formatCNPJ(e.target.value) }); setErrors(clearFieldError(errors, "cnpj")); }}
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
                      onChange={(e) => { setLocalSettings({ ...localSettings, address: e.target.value }); setErrors(clearFieldError(errors, "address")); }}
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
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full justify-start gap-2"
                  onClick={() => {
                    setBackupPassphrase("");
                    setIsBackupPassphraseVisible(false);
                    setBackupPassphraseDialog("export");
                  }}
                  disabled={exportMutation.isPending}
                >
                  <Save className="h-4 w-4" /> {exportMutation.isPending ? "Exportando..." : "Exportar Backup"}
                </Button>
                <Button variant="outline" size="sm" className="w-full justify-start gap-2" onClick={handleImport} disabled={restoreMutation.isPending}>
                  <Upload className="h-4 w-4" /> {restoreMutation.isPending ? "Restaurando..." : "Importar Backup"}
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex-row items-center justify-between gap-3 space-y-0">
              <CardTitle className="flex items-center gap-2 text-lg">
                <Info className="h-5 w-5 text-primary" /> Sobre o Sistema
              </CardTitle>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="shrink-0 gap-2"
                onClick={() => setIsReleaseHistoryOpen(true)}
              >
                <History className="h-4 w-4" /> Histórico de versões
              </Button>
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

      <Dialog open={isReleaseHistoryOpen} onOpenChange={setIsReleaseHistoryOpen}>
        <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>Histórico de versões</DialogTitle>
            <DialogDescription>
              Consulte as novidades de cada atualização, mesmo sem conexão com a internet.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-6">
            {releaseNotes.map((release) => (
              <article key={release.version} className="rounded-lg border p-4">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <h3 className="font-semibold">{release.title}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{release.date}</p>
                  </div>
                  <Badge>{release.version}</Badge>
                </div>
                <div className="mt-4 space-y-4">
                  {release.sections.map((section) => (
                    <section key={section.title}>
                      <h4 className="text-sm font-medium">{section.title}</h4>
                      <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                        {section.items.map((item) => <li key={item}>{item}</li>)}
                      </ul>
                    </section>
                  ))}
                </div>
              </article>
            ))}
          </div>
        </DialogContent>
      </Dialog>

      {isSettingsError && (
        <Card className="border-destructive/20">
          <CardContent className="flex items-center justify-between gap-4 pt-6">
            <p className="text-sm text-destructive">Não foi possível carregar as configurações salvas.</p>
            <Button variant="outline" size="sm" onClick={() => refetchSettings()}>Tentar novamente</Button>
          </CardContent>
        </Card>
      )}

      <AlertDialog
        open={backupPassphraseDialog !== null}
        onOpenChange={(open) => {
          if (!open) closeBackupPassphraseDialog();
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {backupPassphraseDialog === "export"
                ? "Proteger backup"
                : "Senha do backup"}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {backupPassphraseDialog === "export"
                ? "Defina uma senha para proteger este backup. Deixe em branco para usar a chave do aplicativo."
                : "Informe a senha definida para este backup."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="backup-passphrase">
              Senha{" "}
              {backupPassphraseDialog === "export" && (
                <span className="font-normal text-muted-foreground">(opcional)</span>
              )}
            </Label>
            <div className="relative">
              <Input
                id="backup-passphrase"
                type={isBackupPassphraseVisible ? "text" : "password"}
                autoComplete={
                  backupPassphraseDialog === "export" ? "new-password" : "current-password"
                }
                className="pr-10 hide-native-password-reveal"
                value={backupPassphrase}
                onChange={(event) => setBackupPassphrase(event.target.value)}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-0 top-0 h-full w-10 text-muted-foreground"
                aria-label={isBackupPassphraseVisible ? "Ocultar senha" : "Mostrar senha"}
                onClick={() => setIsBackupPassphraseVisible((visible) => !visible)}
              >
                {isBackupPassphraseVisible ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isValidatingBackupPassphrase}>Cancelar</AlertDialogCancel>
            <Button type="button" onClick={() => void confirmBackupPassphrase()} disabled={isValidatingBackupPassphrase}>
              {isValidatingBackupPassphrase ? "Validando..." : "Continuar"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {restoreSource && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => {
            if (!restoreMutation.isPending) {
              setRestoreSource(null);
              setRestorePassphrase("");
            }
          }}
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
              <Button
                variant="outline"
                onClick={() => {
                  setRestoreSource(null);
                  setRestorePassphrase("");
                }}
                disabled={restoreMutation.isPending}
              >
                Cancelar
              </Button>
              <Button
                variant="destructive"
                disabled={restoreMutation.isPending}
                onClick={() => {
                  if (restoreSource) {
                    restoreMutation.mutate({
                      source: restoreSource,
                      passphrase: restorePassphrase,
                    });
                  }
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
      <AlertDialog
        open={pendingUpdate !== null}
        onOpenChange={(open) => {
          if (!open && !isUpdating) {
            setPendingUpdate(null);
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Atualização disponível</AlertDialogTitle>
            <AlertDialogDescription>
              Uma nova versão do OpetS está pronta para instalar.
            </AlertDialogDescription>
          </AlertDialogHeader>
          {pendingUpdate && (
            <div className="space-y-4 text-sm">
              <div className="rounded-md border bg-muted/50 px-3 py-2 font-mono">
                {formatVersion(pendingUpdate.currentVersion)} → {formatVersion(pendingUpdate.version)}
              </div>
              {pendingUpdate.body?.trim() && (
                <div className="space-y-1">
                  <p className="font-medium">Notas da versão</p>
                  <p className="max-h-48 overflow-y-auto whitespace-pre-line rounded-md border bg-muted/50 p-3 text-muted-foreground">
                    {pendingUpdate.body}
                  </p>
                </div>
              )}
            </div>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isUpdating}>Cancelar</AlertDialogCancel>
            <Button type="button" onClick={() => void installUpdate()} disabled={isUpdating}>
              {isUpdating ? "Atualizando..." : "Atualizar agora"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
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
      {isUpdating && updateProgress && (
        <div
          className="fixed inset-0 z-[110] flex items-center justify-center bg-background/80 p-6 backdrop-blur-sm"
          role="status"
          aria-live="assertive"
        >
          <div className="flex w-full max-w-sm flex-col items-center gap-4 rounded-lg border bg-card p-8 text-center shadow-xl">
            <LoaderCircle className="h-8 w-8 animate-spin text-primary" />
            <div className="space-y-1">
              <p className="font-semibold">
                {updateProgress.phase === "downloading" ? "Baixando atualização" : "Instalando atualização"}
              </p>
              <p className="text-sm text-muted-foreground">
                {updateProgress.phase === "downloading"
                  ? updateProgressPercentage === null
                    ? "Aguarde enquanto a nova versão é baixada."
                    : `${updateProgressPercentage}% concluído.`
                  : "A nova versão será iniciada em instantes."}
              </p>
            </div>
            {updateProgressPercentage !== null && updateProgress.phase === "downloading" && (
              <div
                className="h-2 w-full overflow-hidden rounded-full bg-muted"
                aria-label={`Download da atualização: ${updateProgressPercentage}%`}
              >
                <div className="h-full bg-primary transition-all" style={{ width: `${updateProgressPercentage}%` }} />
              </div>
            )}
          </div>
        </div>
      )}
    </form>
  );
}
