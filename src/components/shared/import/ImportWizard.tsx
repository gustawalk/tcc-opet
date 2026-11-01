import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ImportFileStep } from "./ImportFileStep";
import { ImportProgress } from "./ImportProgress";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Pagination } from "@/components/shared/Pagination";
import { dataCommand } from "@/lib/data-client";
import { getErrorMessage } from "@/lib/errors";
import {
  IMPORT_FIELDS,
  IMPORT_QUERY_PREFIXES,
  importRequest,
  mapImportRows,
  suggestMapping,
} from "@/lib/data-import";
import type {
  CommitOutcome,
  DraftRow,
  FieldMapping,
  ImportEntity,
  ImportInspection,
  ImportMapping,
  ImportRequest,
  ImportResult,
  ImportValidation,
  NumberFormat,
  ParsedImport,
} from "@/lib/data-import";

const titles: Record<ImportEntity, string> = {
  customers: "clientes",
  employees: "funcionários",
  inventory: "itens do estoque",
};
type Step = "file" | "mapping" | "review" | "confirm" | "done";
interface FrozenBatch {
  batchId: string;
  request: ImportRequest;
}

export function ImportWizard({
  entity,
  onClose,
}: {
  entity: ImportEntity;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const fields = IMPORT_FIELDS[entity];
  const [step, setStep] = useState<Step>("file");
  const [path, setPath] = useState("");
  const [inspection, setInspection] = useState<ImportInspection | null>(null);
  const [worksheet, setWorksheet] = useState("");
  const [delimiter, setDelimiter] = useState(";");
  const [source, setSource] = useState<ParsedImport | null>(null);
  const [mapping, setMapping] = useState<ImportMapping>({});
  const [numberFormat, setNumberFormat] = useState<NumberFormat>("brazilian");
  const [rows, setRows] = useState<DraftRow[]>([]);
  const [validation, setValidation] = useState<ImportValidation | null>(null);
  const [validating, setValidating] = useState(false);
  const [validationAttempt, setValidationAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [errorsOnly, setErrorsOnly] = useState(false);
  const [frozen, setFrozen] = useState<FrozenBatch | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [pendingAction, setPendingAction] = useState<(() => void) | null>(null);
  const submitting = useRef(false);
  const discardCancel = useRef<HTMLButtonElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (pendingAction) discardCancel.current?.focus();
    else if (previousFocus.current?.isConnected) {
      previousFocus.current.focus();
      previousFocus.current = null;
    }
  }, [pendingAction]);
  const request = useMemo(
    () => importRequest(entity, numberFormat, rows),
    [entity, numberFormat, rows],
  );
  const excludedCount = rows.length - request.rows.length;
  const errors = useMemo(
    () =>
      new Map(validation?.rows.map((row) => [row.sourceRow, row.errors]) ?? []),
    [validation],
  );
  const visible = rows.filter(
    (row) =>
      !errorsOnly ||
      (!row.excluded &&
        Object.keys(errors.get(row.sourceRow) ?? {}).length > 0),
  );
  const currentPage = Math.min(
    page,
    Math.max(1, Math.ceil(visible.length / pageSize)),
  );
  const pageRows = visible.slice(
    (currentPage - 1) * pageSize,
    currentPage * pageSize,
  );

  useEffect(() => {
    if (step !== "review" || frozen || request.rows.length === 0) return;
    let cancelled = false;
    const timer = setTimeout(() => {
      void dataCommand<ImportValidation>("validate_import_rows", { request })
        .then((next) => {
          if (!cancelled) {
            setValidation(next);
            setMessage("");
          }
        })
        .catch((error: unknown) => {
          if (!cancelled) {
            setValidation(null);
            setMessage(getErrorMessage(error));
          }
        })
        .finally(() => {
          if (!cancelled) setValidating(false);
        });
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [request, step, frozen, validationAttempt]);

  function discardThen(action: () => void) {
    if (busy || frozen) return;
    if (source && step !== "done") {
      previousFocus.current = document.activeElement as HTMLElement | null;
      setPendingAction(() => action);
    } else action();
  }

  async function parseFile(filePath: string, sheet: string, separator: string) {
    setSource(null);
    setRows([]);
    setValidation(null);
    setMessage("");
    setBusy(true);
    try {
      const parsed = await invoke<ParsedImport>("parse_import_file", {
        path: filePath,
        worksheet: sheet || null,
        delimiter: separator,
      });
      setSource(parsed);
      setMapping(suggestMapping(entity, parsed.headers));
      setPage(1);
    } catch (error) {
      setMessage(getErrorMessage(error));
    } finally {
      setBusy(false);
    }
  }

  async function selectFile() {
    setBusy(true);
    setMessage("");
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "CSV / Excel", extensions: ["csv", "xlsx"] }],
      });
      if (typeof selected !== "string") return;
      setSource(null);
      setRows([]);
      setValidation(null);
      setInspection(null);
      setPath(selected);
      const inspected = await invoke<ImportInspection>("inspect_import_file", {
        path: selected,
      });
      setInspection(inspected);
      const sheet = inspected.worksheets[0] ?? "";
      setWorksheet(sheet);
      setDelimiter(inspected.delimiter);
      await parseFile(selected, sheet, inspected.delimiter);
    } catch (error) {
      setMessage(getErrorMessage(error));
    } finally {
      setBusy(false);
    }
  }

  function startReview() {
    if (!source) return;
    setRows(mapImportRows(entity, source, mapping));
    setValidation(null);
    setValidating(true);
    setErrorsOnly(false);
    setPage(1);
    setMessage("");
    setStep("review");
  }

  function updateRow(sourceRow: number, field: string, value: string) {
    setRows((previous) =>
      previous.map((row) =>
        row.sourceRow === sourceRow
          ? {
              ...row,
              values: { ...row.values, [field]: value },
              numericFields: row.numericFields.filter((key) => key !== field),
            }
          : row,
      ),
    );
    setValidating(true);
    setMessage("");
  }

  function toggleRow(sourceRow: number) {
    const next = rows.map((row) =>
      row.sourceRow === sourceRow ? { ...row, excluded: !row.excluded } : row,
    );
    setRows(next);
    setValidating(next.some((row) => !row.excluded));
    setMessage("");
  }

  async function saveBatch() {
    if (submitting.current) return;
    submitting.current = true;
    setBusy(true);
    setMessage("");
    const batch = frozen ?? {
      batchId: crypto.randomUUID(),
      request: structuredClone(request),
    };
    setFrozen(batch);
    try {
      const outcome = await dataCommand<CommitOutcome>(
        "commit_import",
        { batchId: batch.batchId, request: batch.request },
        { mutation: true, idempotencyKey: batch.batchId },
      );
      if (outcome.result) {
        setResult(outcome.result);
        setFrozen(null);
        setStep("done");
        for (const prefix of IMPORT_QUERY_PREFIXES[entity])
          void queryClient.invalidateQueries({ queryKey: [prefix] });
      } else if (outcome.validation) {
        setFrozen(null);
        setValidation(outcome.validation);
        setValidating(false);
        setStep("review");
        setMessage(
          "Os dados mudaram durante a revisão. Corrija os erros indicados e confirme novamente.",
        );
      } else {
        setMessage(
          "Não foi possível confirmar o resultado. Tente novamente com os mesmos dados.",
        );
      }
    } catch (error) {
      setMessage(
        `${getErrorMessage(error)} O resultado ainda não foi confirmado. Tente novamente: a mesma importação não será gravada duas vezes.`,
      );
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }

  const descriptions: Record<Step, string> = {
    file: "Traga os cadastros do outro sistema em poucos passos.",
    mapping: "Conecte as colunas do arquivo aos campos do OpetS.",
    review: "Confira os dados e corrija o que precisar antes de importar.",
    confirm: "Tudo pronto? Confira o resumo e confirme a importação.",
    done: "Seus novos registros já estão disponíveis no aplicativo.",
  };
  const canConfirm =
    request.rows.length > 0 && validation?.invalidCount === 0 && !validating;
  return (
    <>
      <Dialog
        open
        onOpenChange={(next) => {
          if (!next) {
            if (pendingAction) setPendingAction(null);
            else discardThen(onClose);
          }
        }}
      >
        <DialogContent
          className={cn(
            "w-[calc(100vw-2rem)] max-h-[92vh] flex flex-col overflow-hidden gap-0 p-0",
            step === "review" || (step === "file" && path)
              ? "max-w-[1200px]"
              : step === "mapping"
                ? "max-w-4xl"
                : "max-w-3xl",
          )}
          role={pendingAction ? "alertdialog" : "dialog"}
          showClose={!busy && !frozen && !pendingAction}
          onInteractOutside={(event) => {
            if (busy || frozen) event.preventDefault();
          }}
          onEscapeKeyDown={(event) => {
            if (busy || frozen) event.preventDefault();
          }}
        >
          <DialogHeader className="px-6 pb-4 pt-6 text-left">
            <DialogTitle>
              {pendingAction
                ? "Descartar a revisão atual?"
                : `Importar ${titles[entity]}`}
            </DialogTitle>
            <DialogDescription>
              {pendingAction
                ? "As alterações e exclusões desta revisão serão perdidas. Nenhum registro será salvo."
                : descriptions[step]}
            </DialogDescription>
          </DialogHeader>
          {!pendingAction && step !== "done" && <ImportProgress step={step} />}
          {pendingAction && (
            <>
              <DialogFooter className="border-t bg-muted/20 px-6 py-4">
                <Button
                  ref={discardCancel}
                  variant="outline"
                  onClick={() => setPendingAction(null)}
                >
                  Manter revisão
                </Button>
                <Button
                  onClick={() => {
                    const action = pendingAction;
                    setPendingAction(null);
                    action();
                  }}
                >
                  Descartar e continuar
                </Button>
              </DialogFooter>
            </>
          )}
          <div className={pendingAction ? "hidden" : "contents"}>
            <div
              className="min-h-0 flex-1 overflow-auto border-t px-6 py-5 space-y-5"
              aria-busy={busy || validating}
            >
              {message && (
                <p
                  role="alert"
                  className="rounded-md border border-destructive p-3 text-sm text-destructive"
                >
                  {message}
                </p>
              )}
              {step === "review" &&
                message &&
                !validating &&
                request.rows.length > 0 && (
                  <Button
                    variant="outline"
                    onClick={() => {
                      setValidating(true);
                      setValidationAttempt((attempt) => attempt + 1);
                    }}
                  >
                    Tentar validar novamente
                  </Button>
                )}
              {busy && (
                <p role="status" className="flex items-center gap-2 text-sm">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {step === "confirm"
                    ? "Salvando importação…"
                    : "Lendo arquivo…"}
                </p>
              )}
              {step === "file" && (
                <ImportFileStep
                  path={path}
                  source={source}
                  inspection={inspection}
                  worksheet={worksheet}
                  delimiter={delimiter}
                  busy={busy}
                  onSelect={() =>
                    discardThen(() => {
                      void selectFile();
                    })
                  }
                  onWorksheetChange={(value) =>
                    discardThen(() => {
                      setWorksheet(value);
                      void parseFile(path, value, delimiter);
                    })
                  }
                  onDelimiterChange={(value) =>
                    discardThen(() => {
                      setDelimiter(value);
                      void parseFile(path, worksheet, value);
                    })
                  }
                />
              )}
              {source &&
                step !== "done" &&
                source.warnings.map((warning) => (
                  <p
                    key={warning}
                    role="note"
                    className="rounded-md border p-3 text-sm"
                  >
                    {warning}
                  </p>
                ))}
              {step === "mapping" && source && (
                <>
                  <p className="text-sm text-muted-foreground">
                    Escolha uma coluna ou um valor fixo para cada campo. *
                    Obrigatório.
                  </p>
                  <div className="grid gap-4 sm:grid-cols-2">
                    {fields.map((field) => {
                      const choice = mapping[field.key] ?? { kind: "ignore" };
                      const selected =
                        choice.kind === "column"
                          ? `column:${choice.index}`
                          : choice.kind;
                      return (
                        <div
                          key={field.key}
                          className="rounded-md border p-3 space-y-2"
                        >
                          <Label htmlFor={`map-${field.key}`}>
                            {field.label}
                            {field.required ? " *" : ""}
                          </Label>
                          <Select
                            value={selected}
                            onValueChange={(value) => {
                              const next: FieldMapping = value.startsWith(
                                "column:",
                              )
                                ? {
                                    kind: "column",
                                    index: Number(value.slice(7)),
                                  }
                                : value === "fixed"
                                  ? {
                                      kind: "fixed",
                                      value: field.defaultValue ?? "",
                                    }
                                  : { kind: "ignore" };
                              setMapping((previous) => ({
                                ...previous,
                                [field.key]: next,
                              }));
                            }}
                          >
                            <SelectTrigger
                              id={`map-${field.key}`}
                              className="bg-background"
                            >
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="ignore">
                                Não associar (em branco)
                              </SelectItem>
                              <SelectItem value="fixed">Valor fixo</SelectItem>
                              <SelectGroup>
                                <SelectLabel>Colunas do arquivo</SelectLabel>
                                {source.headers.map((header, index) => (
                                  <SelectItem
                                    key={index}
                                    value={`column:${index}`}
                                  >
                                    {index + 1}. {header || "Sem título"}
                                  </SelectItem>
                                ))}
                              </SelectGroup>
                            </SelectContent>
                          </Select>
                          {choice.kind === "fixed" && (
                            <Input
                              aria-label={`Valor fixo: ${field.label}`}
                              value={choice.value}
                              onChange={(event) =>
                                setMapping((previous) => ({
                                  ...previous,
                                  [field.key]: {
                                    kind: "fixed",
                                    value: event.target.value,
                                  },
                                }))
                              }
                            />
                          )}
                          {choice.kind === "column" && (
                            <p className="text-xs text-muted-foreground truncate">
                              Exemplo:{" "}
                              {(
                                source.rows[0]?.cells[choice.index] ?? ""
                              ).replace(/\0/g, "") || "(vazio)"}
                            </p>
                          )}
                          {field.key === "type" && (
                            <p className="text-xs text-muted-foreground">
                              Peça / part; serviço / service; item.
                            </p>
                          )}
                          {field.key === "tracksStock" && (
                            <p className="text-xs text-muted-foreground">
                              Sim/não ou true/false. Peças sempre controlam
                              estoque; serviços não controlam.
                            </p>
                          )}
                        </div>
                      );
                    })}
                  </div>
                  {entity === "inventory" && (
                    <div className="grid gap-1">
                      <Label htmlFor="import-number-format">
                        Formato dos números do arquivo
                      </Label>
                      <Select
                        value={numberFormat}
                        onValueChange={(value) =>
                          setNumberFormat(value as NumberFormat)
                        }
                      >
                        <SelectTrigger
                          id="import-number-format"
                          className="max-w-sm bg-background"
                        >
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="brazilian">
                            Brasileiro: 1.234,56
                          </SelectItem>
                          <SelectItem value="international">
                            Internacional: 1,234.56
                          </SelectItem>
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        Células numéricas do Excel são interpretadas pelo valor
                        numérico. Valores digitados na revisão usam o formato
                        escolhido.
                      </p>
                    </div>
                  )}
                </>
              )}
              {step === "review" && (
                <>
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <p className="text-sm" role="status">
                      {request.rows.length} incluídas · {excludedCount}{" "}
                      excluídas ·{" "}
                      {validating
                        ? "Validando…"
                        : `${validation?.invalidCount ?? "—"} com erros`}
                    </p>
                    <Label className="flex items-center gap-2">
                      <Checkbox
                        checked={errorsOnly}
                        onChange={(event) => {
                          setErrorsOnly(event.target.checked);
                          setPage(1);
                        }}
                      />
                      Somente linhas com erros
                    </Label>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Corrija as células ou exclua as linhas que não deseja
                    importar. Todos os registros incluídos devem ser válidos.
                  </p>
                  <div className="overflow-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Linha</TableHead>
                          <TableHead>Incluir</TableHead>
                          {fields.map((field) => (
                            <TableHead key={field.key}>
                              {field.label}
                              {field.required ? " *" : ""}
                            </TableHead>
                          ))}
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {pageRows.map((row) => (
                          <TableRow
                            key={row.sourceRow}
                            className={row.excluded ? "opacity-60" : undefined}
                          >
                            <TableCell>{row.sourceRow}</TableCell>
                            <TableCell>
                              <Checkbox
                                aria-label={`Incluir linha ${row.sourceRow}`}
                                checked={!row.excluded}
                                onChange={() => toggleRow(row.sourceRow)}
                              />
                            </TableCell>
                            {fields.map((field) => {
                              const fieldError = !row.excluded
                                ? errors.get(row.sourceRow)?.[field.key]
                                : undefined;
                              const errorId = `error-${row.sourceRow}-${field.key}`;
                              const type = row.values.type
                                ?.trim()
                                .toLowerCase();
                              const automaticStock =
                                field.key === "tracksStock" && !fieldError
                                  ? ["part", "peça", "peca"].includes(type)
                                    ? "true"
                                    : [
                                          "service",
                                          "serviço",
                                          "servico",
                                        ].includes(type)
                                      ? "false"
                                      : undefined
                                  : undefined;
                              return (
                                <TableCell
                                  key={field.key}
                                  className="align-top min-w-48"
                                >
                                  <Input
                                    aria-label={`${field.label}, linha ${row.sourceRow}`}
                                    aria-invalid={!!fieldError}
                                    aria-describedby={
                                      fieldError ? errorId : undefined
                                    }
                                    disabled={row.excluded}
                                    readOnly={automaticStock !== undefined}
                                    title={
                                      automaticStock !== undefined
                                        ? "Definido automaticamente pelo tipo do item."
                                        : undefined
                                    }
                                    value={
                                      automaticStock ??
                                      row.values[field.key].replace(/\0/g, "")
                                    }
                                    onChange={(event) =>
                                      updateRow(
                                        row.sourceRow,
                                        field.key,
                                        event.target.value,
                                      )
                                    }
                                  />
                                  {fieldError && (
                                    <p
                                      id={errorId}
                                      className="mt-1 text-xs text-destructive"
                                    >
                                      {fieldError}
                                    </p>
                                  )}
                                </TableCell>
                              );
                            })}
                          </TableRow>
                        ))}
                        {pageRows.length === 0 && (
                          <TableRow>
                            <TableCell colSpan={fields.length + 2}>
                              Nenhuma linha nesta visualização.
                            </TableCell>
                          </TableRow>
                        )}
                      </TableBody>
                    </Table>
                  </div>
                  <Pagination
                    totalItems={visible.length}
                    page={currentPage}
                    pageSize={pageSize}
                    onPageChange={setPage}
                    onPageSizeChange={(size) => {
                      setPageSize(size);
                      setPage(1);
                    }}
                  />
                </>
              )}
              {step === "confirm" && (
                <div className="space-y-3">
                  <p>
                    Serão criados <strong>{request.rows.length}</strong>{" "}
                    registros. <strong>{excludedCount}</strong> linhas foram
                    excluídas.
                  </p>
                  <p className="text-sm text-muted-foreground">
                    Registros existentes serão mantidos. Dados repetidos podem
                    criar novos cadastros; e-mails de funcionários devem ser
                    únicos. A gravação será feita em uma única operação.
                  </p>
                  {frozen && !busy && (
                    <p className="text-sm">
                      Aguarde a confirmação antes de iniciar outra importação. A
                      tentativa será repetida com os mesmos dados.
                    </p>
                  )}
                </div>
              )}
              {step === "done" && result && (
                <div role="status" className="space-y-2">
                  <p className="font-medium">Importação concluída.</p>
                  <p>
                    {result.createdCount} registros criados · {excludedCount}{" "}
                    linhas excluídas.
                  </p>
                </div>
              )}
            </div>
            <DialogFooter className="shrink-0 gap-2 border-t bg-muted/20 px-6 py-4">
              {step === "done" ? (
                <Button onClick={onClose}>Concluir</Button>
              ) : (
                <>
                  <Button
                    variant="outline"
                    disabled={busy || !!frozen}
                    onClick={() => discardThen(onClose)}
                  >
                    Cancelar
                  </Button>
                  {step === "file" && source && (
                    <Button
                      disabled={busy || !source}
                      onClick={() => {
                        setMessage("");
                        setStep("mapping");
                      }}
                    >
                      Mapear campos{" "}
                      <ArrowRight className="ml-2 h-4 w-4" aria-hidden="true" />
                    </Button>
                  )}
                  {step === "mapping" && (
                    <>
                      <Button
                        variant="outline"
                        onClick={() =>
                          discardThen(() => {
                            setRows([]);
                            setStep("file");
                          })
                        }
                      >
                        Voltar ao arquivo
                      </Button>
                      <Button onClick={startReview}>Revisar dados</Button>
                    </>
                  )}
                  {step === "review" && (
                    <>
                      <Button
                        variant="outline"
                        onClick={() =>
                          discardThen(() => {
                            setRows([]);
                            setValidation(null);
                            setStep("mapping");
                          })
                        }
                      >
                        Alterar mapeamento
                      </Button>
                      <Button
                        disabled={!canConfirm}
                        onClick={() => {
                          setMessage("");
                          setStep("confirm");
                        }}
                      >
                        Continuar
                      </Button>
                    </>
                  )}
                  {step === "confirm" && (
                    <>
                      <Button
                        variant="outline"
                        disabled={busy || !!frozen}
                        onClick={() => setStep("review")}
                      >
                        Voltar à revisão
                      </Button>
                      <Button
                        disabled={busy}
                        onClick={() => {
                          void saveBatch();
                        }}
                      >
                        {frozen ? "Tentar novamente" : "Confirmar importação"}
                      </Button>
                    </>
                  )}
                </>
              )}
            </DialogFooter>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
