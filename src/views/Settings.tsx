import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Building2, ChevronDown, Database, Eye, EyeOff, FolderOpen, HardDriveDownload, History, Info, LoaderCircle, LockKeyhole, MapPin, Monitor, Moon, Network, RefreshCw, Save, Server, Sun, Upload, WifiOff } from "lucide-react";
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
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  type Settings,
  type SystemInfo,
  type BackupSummary,
  type BackupInspection,
  type AutomaticBackupRunResult,
  type AutomaticBackupSettings,
  type AutomaticBackupStatus,
  type LanHostStatus,
  type LanMode,
  type LanModeConfig,
  type LanModeStatus,
} from "@/lib/types";
import { dataCommand } from "@/lib/data-client";
import {
  loadLanRemoteBackupSettings,
  saveLanRemoteBackupSettings,
} from "@/lib/lan-backup";
import { settingsSchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { formatCNPJ } from "@/lib/formatters";
import { RelativeDate } from "@/components/shared/RelativeDate";
import { toastSuccess, toastError } from "@/lib/errors";
import { releaseNotes } from "@/lib/release-notes";
import {
  THEME_OPTIONS,
  getThemePreference,
  setThemePreference,
  type Theme,
} from "@/lib/theme";
import {
  FONT_SCALE_OPTIONS,
  getFontScalePreference,
  setFontScalePreference,
  type FontScale,
} from "@/lib/font-scale";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const ERROR_MESSAGES: Record<string, string> = {
  "error sending request for url (https://github.com/gustawalk/tcc-opet/releases/latest/download/updater.json)": "Não foi possível verificar as atualizações."
}

const fetchSettings = async (): Promise<Settings> => {
  return await dataCommand<Settings>("get_settings");
};

const fetchSystemInfo = async (): Promise<SystemInfo> => {
  return await invoke<SystemInfo>("get_system_info");
};

const fetchAutomaticBackupStatus = async (): Promise<AutomaticBackupStatus> => {
  return await invoke<AutomaticBackupStatus>("get_automatic_backup_status");
};

const formatBackupSize = (value?: number | null) => {
  if (value == null) return "";
  if (value < 1024 * 1024) return `${Math.max(1, Math.round(value / 1024))} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
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
  const [fontScale, setFontScale] = useState<FontScale>(getFontScalePreference);
  const [selectedLanMode, setSelectedLanMode] = useState<LanMode>("local");
  const [hostPort, setHostPort] = useState(8743);
  const [clientUrl, setClientUrl] = useState("");
  const [clientDeviceName, setClientDeviceName] = useState("");
  const [verificationCode, setVerificationCode] = useState("");
  const [remoteBackupSettings, setRemoteBackupSettings] = useState(
    loadLanRemoteBackupSettings,
  );

  const { data: lanMode } = useQuery({
    queryKey: ["lan-mode"],
    queryFn: () => invoke<LanModeStatus>("get_lan_mode_config"),
  });
  const { data: hostStatus } = useQuery({
    queryKey: ["lan-host-status"],
    queryFn: () => invoke<LanHostStatus>("get_lan_host_status"),
    enabled: lanMode?.activeMode === "host",
    refetchInterval: 3000,
  });
  const clientConnection = useQuery({
    queryKey: ["lan-client-connection"],
    queryFn: () => invoke("check_lan_client_connection"),
    enabled: lanMode?.activeMode === "client",
    retry: false,
    refetchInterval: 5000,
  });

  const { data: settingsData, isError: isSettingsError, refetch: refetchSettings } = useQuery({
    queryKey: ["settings"],
    queryFn: fetchSettings,
  });

  const { data: systemInfo, isLoading: isSystemInfoLoading, isError: isSystemInfoError, refetch: refetchSystemInfo } = useQuery({
    queryKey: ["system-info"],
    queryFn: fetchSystemInfo,
  });

  const { data: automaticBackupStatus, isLoading: isAutomaticBackupLoading, isError: isAutomaticBackupError, refetch: refetchAutomaticBackup } = useQuery({
    queryKey: ["automatic-backup-status"],
    queryFn: fetchAutomaticBackupStatus,
    refetchInterval: (query) => query.state.data?.running ? 1000 : false,
  });

  const [localSettings, setLocalSettings] = useState<Settings>({
    companyName: "",
    cnpj: "",
    address: "",
    logoPath: "",
  });
  const [automaticBackupSettings, setAutomaticBackupSettings] = useState<AutomaticBackupSettings>({
    enabled: false,
    destination: null,
    intervalHours: 24,
  });
  const [isAutomaticBackupDirty, setIsAutomaticBackupDirty] = useState(false);
  const automaticBackupInitialized = useRef(false);
  const [settingsSaveIsToggle, setSettingsSaveIsToggle] = useState(false);
  const settingsMutationIsToggle = useRef(false);

  useEffect(() => {
    if (settingsData) {
      setLocalSettings(settingsData);
    }
  }, [settingsData]);

  useEffect(() => {
    if (!lanMode) return;
    setSelectedLanMode(lanMode.config.mode);
    setHostPort(lanMode.config.hostPort);
    setClientUrl(lanMode.config.clientUrl ?? "");
    setClientDeviceName(lanMode.config.clientDeviceName ?? "");
  }, [lanMode]);

  useEffect(() => {
    if (automaticBackupStatus && (!automaticBackupInitialized.current || !isAutomaticBackupDirty)) {
      setAutomaticBackupSettings({
        enabled: automaticBackupStatus.enabled,
        destination: automaticBackupStatus.destination,
        intervalHours: automaticBackupStatus.intervalHours,
      });
      automaticBackupInitialized.current = true;
    }
  }, [automaticBackupStatus, isAutomaticBackupDirty]);

  const updateMutation = useMutation({
    mutationFn: async (data: Settings) => {
      return await dataCommand("update_settings", { settings: data });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      toastSuccess("Configurações salvas com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao salvar configurações."),
  });

  const exportMutation = useMutation({
    mutationFn: async ({ destination, passphrase }: { destination: string; passphrase: string }) =>
      lanMode?.activeMode === "client"
        ? invoke<BackupSummary>("download_lan_remote_backup", { destination, passphrase })
        : invoke<BackupSummary>("export_backup", { destination, passphrase }),
    onSuccess: (backup) => toastSuccess(`Backup exportado com ${backup.attachmentCount} anexo(s).`),
    onError: (err) => toastError(err, "Erro ao exportar backup."),
  });

  const automaticBackupSettingsMutation = useMutation({
    mutationFn: async (settings: AutomaticBackupSettings) =>
      invoke<AutomaticBackupStatus>("update_automatic_backup_settings", { settings }),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["automatic-backup-status"] });
    },
    onSuccess: (status) => {
      queryClient.setQueryData(["automatic-backup-status"], status);
      setAutomaticBackupSettings({
        enabled: status.enabled,
        destination: status.destination,
        intervalHours: status.intervalHours,
      });
      setIsAutomaticBackupDirty(false);
      if (!settingsMutationIsToggle.current) {
        toastSuccess("Configuração do backup automático salva.");
      }
    },
    onError: (err) => toastError(err, "Erro ao salvar o backup automático."),
    onSettled: () => {
      settingsMutationIsToggle.current = false;
      setSettingsSaveIsToggle(false);
    },
  });

  const automaticBackupNowMutation = useMutation({
    mutationFn: async () => invoke<AutomaticBackupRunResult>("run_automatic_backup_now"),
    onSuccess: (result) => {
      if (result.created) {
        toastSuccess("Backup automático criado e validado com sucesso.");
      } else if (result.skippedUnchanged) {
        toastSuccess("Os dados não mudaram; nenhum novo arquivo foi necessário.");
      }
    },
    onError: (err) => toastError(err, "Erro ao executar o backup automático."),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["automatic-backup-status"] }),
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

  const lanModeMutation = useMutation({
    mutationFn: async (mode: LanMode) => {
      const config: LanModeConfig = {
        ...(lanMode?.config ?? { mode: "local", hostPort: 8743 }),
        mode,
        hostPort,
      };
      return invoke<LanModeStatus>("update_lan_mode_config", { config });
    },
    onSuccess: async () => relaunch(),
    onError: (err) => toastError(err, "Não foi possível alterar o modo LAN."),
  });

  const pairMutation = useMutation({
    mutationFn: () =>
      invoke<LanModeStatus>("pair_lan_client", {
        url: clientUrl,
        deviceName: clientDeviceName,
        verificationCode,
      }),
    onSuccess: async () => relaunch(),
    onError: (err) => toastError(err, "Não foi possível parear com o host."),
  });

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

  const handleSelectAutomaticBackupDirectory = async () => {
    try {
      const destination = await invoke<string | null>("select_automatic_backup_directory");
      if (destination) {
        setAutomaticBackupSettings((current) => ({ ...current, destination }));
        setIsAutomaticBackupDirty(true);
      }
    } catch (err) {
      toastError(err, "Erro ao selecionar a pasta do backup automático.");
    }
  };

  const handleSelectRemoteBackupDirectory = async () => {
    try {
      const destination = await invoke<string | null>("select_automatic_backup_directory");
      if (destination) {
        setRemoteBackupSettings((current) => ({ ...current, destination }));
      }
    } catch (err) {
      toastError(err, "Erro ao selecionar a pasta do backup remoto.");
    }
  };

  const persistRemoteBackupSettings = () => {
    if (
      remoteBackupSettings.enabled &&
      (!remoteBackupSettings.destination || remoteBackupSettings.intervalHours < 1)
    ) {
      toastError("Selecione uma pasta e informe um intervalo válido.");
      return;
    }
    saveLanRemoteBackupSettings(remoteBackupSettings);
    toastSuccess("Backup remoto automático configurado neste computador.");
  };

  const validatedAutomaticBackupSettings = (enabled = automaticBackupSettings.enabled) => {
    if (!Number.isInteger(automaticBackupSettings.intervalHours) || automaticBackupSettings.intervalHours < 1 || automaticBackupSettings.intervalHours > 168) {
      toastError("O intervalo deve ser um número inteiro entre 1 e 168 horas.");
      return null;
    }
    if (enabled && !automaticBackupSettings.destination) {
      toastError("Selecione uma pasta antes de ativar o backup automático.");
      return null;
    }
    return { ...automaticBackupSettings, enabled };
  };

  const handleSaveAutomaticBackup = () => {
    const settings = validatedAutomaticBackupSettings();
    if (settings) automaticBackupSettingsMutation.mutate(settings);
  };

  const handleAutomaticBackupToggle = (enabled: boolean) => {
    const settings = validatedAutomaticBackupSettings(enabled);
    if (settings) {
      settingsMutationIsToggle.current = true;
      setSettingsSaveIsToggle(true);
      automaticBackupSettingsMutation.mutate(settings);
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
  const handleThemeChange = (nextTheme: Theme) => {
    setTheme(nextTheme);
    setThemePreference(nextTheme);
  };
  const handleFontScaleChange = (nextScale: FontScale) => {
    setFontScale(nextScale);
    setFontScalePreference(nextScale);
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
  const automaticBackupControlsDisabled =
    isAutomaticBackupLoading ||
    isAutomaticBackupError ||
    automaticBackupStatus?.running ||
    automaticBackupSettingsMutation.isPending ||
    automaticBackupNowMutation.isPending;
  const automaticBackupActionsDisabled =
    automaticBackupControlsDisabled || !automaticBackupSettings.enabled;

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

        <div className="grid gap-6 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Aparência</CardTitle>
              <CardDescription>
                As preferências são salvas somente neste dispositivo.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between gap-4">
                <div className="space-y-1">
                  <p className="text-sm font-medium whitespace-nowrap">Tema</p>
                  <p className="text-sm text-muted-foreground whitespace-nowrap">
                    {theme === "system"
                      ? "Segue o sistema"
                      : theme === "dark"
                        ? "Tema escuro"
                        : "Tema claro"}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  {theme === "system" ? (
                    <Monitor className="h-4 w-4 shrink-0 text-muted-foreground" />
                  ) : theme === "dark" ? (
                    <Moon className="h-4 w-4 shrink-0 text-muted-foreground" />
                  ) : (
                    <Sun className="h-4 w-4 shrink-0 text-muted-foreground" />
                  )}
                  <Select
                    value={theme}
                    onValueChange={(value) => handleThemeChange(value as Theme)}
                  >
                    <SelectTrigger aria-label="Tema" className="w-[160px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {THEME_OPTIONS.map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <Separator />
              <div className="flex items-center justify-between gap-4">
                <div className="space-y-1">
                  <p className="text-sm font-medium whitespace-nowrap">Tamanho da fonte</p>
                  <p className="text-sm text-muted-foreground whitespace-nowrap">
                    Escala textos e interface.
                  </p>
                </div>
                <Select
                  value={fontScale}
                  onValueChange={(value) => handleFontScaleChange(value as FontScale)}
                >
                  <SelectTrigger aria-label="Tamanho da fonte" className="w-[160px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {FONT_SCALE_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
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

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-lg">
              <Network className="h-5 w-5 text-primary" /> Rede local
            </CardTitle>
            <CardDescription>
              Compartilhe os dados somente nesta rede, sem depender da internet.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-2">
              <Label htmlFor="lan-mode">Modo deste computador</Label>
              <Select
                value={selectedLanMode}
                onValueChange={(value) => setSelectedLanMode(value as LanMode)}
              >
                <SelectTrigger id="lan-mode" aria-label="Modo LAN">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="local">Local</SelectItem>
                  <SelectItem value="host">Host</SelectItem>
                  <SelectItem value="client">Cliente</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {selectedLanMode === "host" && (
              <div className="space-y-3 rounded-md border p-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <Server className="h-4 w-4" /> Servidor LAN
                  </div>
                  <Badge variant={hostStatus?.running ? "default" : "secondary"}>
                    {hostStatus?.running ? "Ativo" : "Reinício necessário"}
                  </Badge>
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="lan-port">Porta local</Label>
                  <Input
                    id="lan-port"
                    type="number"
                    min={1}
                    max={65535}
                    value={hostPort}
                    onChange={(event) => setHostPort(Number(event.target.value))}
                  />
                </div>
                {hostStatus?.address && (
                  <p className="text-xs text-muted-foreground">
                    Endereço: <code>{`https://${hostStatus.address}`}</code>
                  </p>
                )}
                {hostStatus?.verificationCode && (
                  <div className="space-y-1">
                    <Label>Código de verificação</Label>
                    <code className="block break-all rounded bg-muted p-2 text-xs">
                      {hostStatus.verificationCode}
                    </code>
                    <p className="text-xs text-muted-foreground">
                      Compartilhe este código somente com o funcionário que está pareando agora.
                    </p>
                  </div>
                )}
                {hostStatus?.startupError && (
                  <p className="text-xs text-destructive">{hostStatus.startupError}</p>
                )}
              </div>
            )}

            {selectedLanMode === "client" && (
              <div className="space-y-3 rounded-md border p-3">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-sm font-medium">Conexão com o host</span>
                  {lanMode?.activeMode === "client" && (
                    <Badge variant={clientConnection.isSuccess ? "default" : "secondary"}>
                      {clientConnection.isSuccess ? "Conectado" : "Desconectado"}
                    </Badge>
                  )}
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="lan-client-url">Endereço HTTPS do host</Label>
                  <Input
                    id="lan-client-url"
                    placeholder="https://192.168.1.10:8743"
                    value={clientUrl}
                    onChange={(event) => setClientUrl(event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="lan-device-name">Nome deste computador</Label>
                  <Input
                    id="lan-device-name"
                    placeholder="Balcão 2"
                    value={clientDeviceName}
                    onChange={(event) => setClientDeviceName(event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="lan-verification-code">Código de verificação</Label>
                  <Input
                    id="lan-verification-code"
                    value={verificationCode}
                    onChange={(event) => setVerificationCode(event.target.value)}
                  />
                </div>
                {lanMode?.config.clientCertificateFingerprint && (
                  <div className="flex gap-2 text-xs text-muted-foreground">
                    <LockKeyhole className="mt-0.5 h-4 w-4 shrink-0" />
                    <span className="break-all">
                      Tráfego criptografado. Certificado: {lanMode.config.clientCertificateFingerprint}
                    </span>
                  </div>
                )}
                {clientConnection.isError && (
                  <div className="flex gap-2 text-xs text-destructive">
                    <WifiOff className="h-4 w-4 shrink-0" />
                    O host está indisponível. Leituras e alterações permanecem bloqueadas.
                  </div>
                )}
                <Button
                  type="button"
                  className="w-full"
                  onClick={() => pairMutation.mutate()}
                  disabled={pairMutation.isPending || !clientUrl || !clientDeviceName || !verificationCode}
                >
                  {pairMutation.isPending ? "Pareando..." : "Parear e reiniciar"}
                </Button>
              </div>
            )}

            {selectedLanMode !== "client" && (
              <Button
                type="button"
                className="w-full"
                disabled={lanModeMutation.isPending || selectedLanMode === lanMode?.config.mode}
                onClick={() => lanModeMutation.mutate(selectedLanMode)}
              >
                {lanModeMutation.isPending ? "Salvando..." : "Salvar modo e reiniciar"}
              </Button>
            )}
          </CardContent>
        </Card>

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
              {lanMode?.activeMode !== "client" && (
                <Button variant="outline" size="sm" className="w-full justify-start gap-2" onClick={handleImport} disabled={restoreMutation.isPending}>
                  <Upload className="h-4 w-4" /> {restoreMutation.isPending ? "Restaurando..." : "Importar Backup"}
                </Button>
              )}
            </div>
            <Separator />
            {lanMode?.activeMode === "client" ? (
              <div className="space-y-4 rounded-md border p-3">
                <p className="text-sm text-muted-foreground">
                  O backup exportado é criado pelo host e salvo neste computador. Importação,
                  restauração, reset e configuração do backup principal estão disponíveis somente no
                  computador host.
                </p>
                <div className="flex items-start gap-3 border-t pt-3">
                  <Checkbox
                    id="lan-remote-backup-enabled"
                    checked={remoteBackupSettings.enabled}
                    onChange={(event) =>
                      setRemoteBackupSettings((current) => ({
                        ...current,
                        enabled: event.target.checked,
                      }))
                    }
                  />
                  <Label htmlFor="lan-remote-backup-enabled">
                    Baixar backup remoto automaticamente
                  </Label>
                </div>
                <div className="grid gap-2">
                  <Label>Pasta neste computador</Label>
                  <code className="truncate rounded bg-muted p-2 text-[10px]">
                    {remoteBackupSettings.destination || "Nenhuma pasta selecionada"}
                  </code>
                  <Button type="button" variant="outline" size="sm" onClick={handleSelectRemoteBackupDirectory}>
                    <FolderOpen className="h-4 w-4" /> Selecionar pasta
                  </Button>
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="lan-remote-backup-interval">Intervalo em horas</Label>
                  <Input
                    id="lan-remote-backup-interval"
                    type="number"
                    min={1}
                    max={168}
                    value={remoteBackupSettings.intervalHours}
                    onChange={(event) =>
                      setRemoteBackupSettings((current) => ({
                        ...current,
                        intervalHours: Number(event.target.value),
                      }))
                    }
                  />
                </div>
                <Button type="button" size="sm" className="w-full" onClick={persistRemoteBackupSettings}>
                  Salvar backup remoto
                </Button>
              </div>
            ) : <details className="group rounded-lg border bg-muted/20">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-3 p-3 marker:content-none">
                <div className="flex items-center gap-2">
                  <HardDriveDownload className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">Backup automático</span>
                  <Badge variant={automaticBackupStatus?.enabled ? "default" : "secondary"}>
                    {isAutomaticBackupLoading
                      ? "Carregando"
                      : automaticBackupStatus?.enabled
                        ? "Ativado"
                        : "Desativado"}
                  </Badge>
                </div>
                <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform group-open:rotate-180" />
              </summary>
              <div className="space-y-3 border-t p-3">
                <div className="flex items-start gap-3">
                  <Checkbox
                    id="automatic-backup-enabled"
                    checked={automaticBackupSettings.enabled}
                    disabled={automaticBackupControlsDisabled}
                    onChange={(event) => handleAutomaticBackupToggle(event.target.checked)}
                  />
                  <div className="space-y-1">
                    <Label htmlFor="automatic-backup-enabled">Ativar backup automático</Label>
                    <p className="text-xs text-muted-foreground">
                      O primeiro backup será executado após o intervalo configurado.
                    </p>
                  </div>
                </div>
                <div className="space-y-1.5">
                  <Label>Pasta de destino</Label>
                  <code
                    className="block truncate rounded bg-muted p-2 text-[10px]"
                    title={automaticBackupSettings.destination ?? undefined}
                  >
                    {automaticBackupSettings.destination ?? "Nenhuma pasta selecionada"}
                  </code>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="w-full gap-2"
                    disabled={automaticBackupControlsDisabled}
                    onClick={() => void handleSelectAutomaticBackupDirectory()}
                  >
                    <FolderOpen className="h-4 w-4" /> Selecionar pasta
                  </Button>
                  <p className="text-xs text-muted-foreground">
                    Escolha uma pasta para salvar os backups. Diretorios em nuvem como OneDrive e DropBox são recomendados.
                  </p>
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="automatic-backup-interval">Intervalo em horas</Label>
                  <Input
                    id="automatic-backup-interval"
                    type="number"
                    min={1}
                    max={168}
                    step={1}
                    disabled={automaticBackupActionsDisabled}
                    value={automaticBackupSettings.intervalHours}
                    onChange={(event) => {
                      setAutomaticBackupSettings((current) => ({
                        ...current,
                        intervalHours: Number(event.target.value),
                      }));
                      setIsAutomaticBackupDirty(true);
                    }}
                  />
                  {automaticBackupSettings.intervalHours > 48 && (
                    <p className="text-xs text-amber-700 dark:text-amber-400">
                      Intervalos acima de 48 horas aumentam o risco de perda de dados recentes.
                    </p>
                  )}
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <Button
                    type="button"
                    size="sm"
                    onClick={handleSaveAutomaticBackup}
                    disabled={automaticBackupActionsDisabled || !isAutomaticBackupDirty}
                  >
                    {automaticBackupSettingsMutation.isPending && !settingsSaveIsToggle ? "Salvando..." : "Salvar configurações"}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="gap-2"
                    onClick={() => automaticBackupNowMutation.mutate()}
                    disabled={
                      automaticBackupActionsDisabled ||
                      automaticBackupNowMutation.isPending ||
                      automaticBackupStatus?.running ||
                      !automaticBackupSettings.destination ||
                      automaticBackupSettings.destination !== automaticBackupStatus?.destination
                    }
                  >
                    <HardDriveDownload className="h-4 w-4" />
                    {automaticBackupNowMutation.isPending ? "Executando..." : "Executar agora"}
                  </Button>
                </div>
                {automaticBackupStatus && (
                  <div className="space-y-1 border-t pt-3 text-xs text-muted-foreground">
                    <p>
                      Último backup:{" "}
                      <RelativeDate value={automaticBackupStatus.lastSuccessAt} fallback="Ainda não executado" />
                      {automaticBackupStatus.lastBackupSizeBytes != null
                        ? ` (${formatBackupSize(automaticBackupStatus.lastBackupSizeBytes)})`
                        : ""}
                    </p>
                    <p>
                      Última verificação:{" "}
                      <RelativeDate value={automaticBackupStatus.lastVerifiedAt} fallback="Ainda não executado" />
                    </p>
                    {automaticBackupStatus.enabled && (
                      <p>
                        Próxima verificação elegível:{" "}
                        <RelativeDate value={automaticBackupStatus.nextBackupAt} fallback="Ainda não executado" />
                      </p>
                    )}
                    {automaticBackupStatus.lastError && (
                      <p className="text-destructive">Último erro: {automaticBackupStatus.lastError}</p>
                    )}
                  </div>
                )}
                {isAutomaticBackupError && (
                  <div className="flex items-center justify-between gap-2 text-xs text-destructive">
                    <span>Não foi possível carregar o status.</span>
                    <Button type="button" variant="ghost" size="sm" onClick={() => refetchAutomaticBackup()}>
                      Tentar novamente
                    </Button>
                  </div>
                )}
                <p className="text-[11px] leading-relaxed text-muted-foreground">
                  Retenção leve: 7 pontos diários e 4 semanais.
                </p>
              </div>
            </details>}
          </CardContent>
        </Card>

        {lanMode?.activeMode !== "client" && <Card className="border-destructive/20">
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
        </Card>}
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
            {releaseNotes.map((release, index) => (
              <details key={release.version} className="group rounded-lg border" open={index === 0}>
                <summary className="flex cursor-pointer list-none items-center justify-between gap-2 p-4 marker:content-none">
                  <div className="flex flex-1 flex-wrap items-center justify-between gap-2">
                    <div>
                      <h3 className="font-semibold">{release.title}</h3>
                      <p className="mt-1 text-sm text-muted-foreground">{release.date}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge>{release.version}</Badge>
                      <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform group-open:rotate-180" />
                    </div>
                  </div>
                </summary>
                <div className="space-y-4 border-t p-4">
                  {release.sections.map((section) => (
                    <section key={section.title}>
                      <h4 className="text-sm font-medium">{section.title}</h4>
                      <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                        {section.items.map((item) => <li key={item}>{item}</li>)}
                      </ul>
                    </section>
                  ))}
                </div>
              </details>
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
