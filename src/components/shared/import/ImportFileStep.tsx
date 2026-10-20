import {
  CheckCircle2,
  FileSpreadsheet,
  FileUp,
  Replace,
  Rows3,
  Columns3,
  Info,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { ImportInspection, ParsedImport } from "@/lib/data-import";

interface ImportFileStepProps {
  path: string;
  source: ParsedImport | null;
  inspection: ImportInspection | null;
  worksheet: string;
  delimiter: string;
  busy: boolean;
  onSelect: () => void;
  onWorksheetChange: (value: string) => void;
  onDelimiterChange: (value: string) => void;
}

export function ImportFileStep({
  path,
  source,
  inspection,
  worksheet,
  delimiter,
  busy,
  onSelect,
  onWorksheetChange,
  onDelimiterChange,
}: ImportFileStepProps) {
  return (
    <div className="space-y-5">
      {!path ? (
        <div className="rounded-xl border border-dashed border-primary/30 bg-primary/[0.03] px-6 py-10 text-center sm:py-12">
          <div className="mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl border border-primary/10 bg-background text-primary shadow-sm">
            <FileUp className="h-7 w-7" aria-hidden="true" />
          </div>
          <h3 className="text-lg font-semibold tracking-tight">
            Comece pelo seu arquivo
          </h3>
          <p className="mx-auto mt-2 max-w-sm text-sm text-muted-foreground">
            Selecione a planilha exportada do outro sistema. Você escolherá
            quais colunas importar no próximo passo.
          </p>
          <Button className="mt-6 gap-2" disabled={busy} onClick={onSelect}>
            <FileSpreadsheet className="h-4 w-4" aria-hidden="true" />
            Selecionar arquivo
          </Button>
          <div className="mt-5 flex flex-wrap justify-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <span>CSV ou Excel (.xlsx)</span>
            <span>Até 5.000 linhas</span>
            <span>Até 10 MiB</span>
          </div>
        </div>
      ) : (
        <section
          aria-label="Arquivo selecionado"
          className="rounded-xl border bg-muted/20 p-4 sm:p-5"
        >
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex min-w-0 flex-1 items-center gap-3">
              <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border bg-background text-primary">
                <FileSpreadsheet className="h-5 w-5" aria-hidden="true" />
              </div>
              <div className="min-w-0">
                <p
                  className="truncate text-sm font-semibold"
                  title={path.split(/[\\/]/).pop()}
                >
                  {path.split(/[\\/]/).pop()}
                </p>
                {source && (
                  <p className="mt-1 flex items-center gap-1.5 text-xs text-emerald-700 dark:text-emerald-400">
                    <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" />
                    Arquivo lido com sucesso
                  </p>
                )}
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              disabled={busy}
              onClick={onSelect}
            >
              <Replace className="h-3.5 w-3.5" aria-hidden="true" />
              Trocar arquivo
            </Button>
          </div>
          {(source || inspection) && (
            <div className="mt-4 flex flex-wrap items-end justify-between gap-4 border-t pt-4">
              {source && (
                <div className="flex flex-wrap gap-2">
                  <Badge variant="secondary" className="gap-1.5 font-normal">
                    <Rows3 className="h-3.5 w-3.5" aria-hidden="true" />
                    {source.rows.length.toLocaleString("pt-BR")}{" "}
                    {source.rows.length === 1 ? "linha" : "linhas"}
                  </Badge>
                  <Badge variant="secondary" className="gap-1.5 font-normal">
                    <Columns3 className="h-3.5 w-3.5" aria-hidden="true" />
                    {source.headers.length}{" "}
                    {source.headers.length === 1 ? "coluna" : "colunas"}
                  </Badge>
                </div>
              )}
              {inspection &&
                (inspection.worksheets.length > 0 ? (
                  <div className="grid w-full gap-1.5 sm:w-60">
                    <Label
                      htmlFor="import-sheet"
                      className="text-xs text-muted-foreground"
                    >
                      Planilha
                    </Label>
                    <Select
                      value={worksheet}
                      onValueChange={onWorksheetChange}
                      disabled={busy}
                    >
                      <SelectTrigger
                        id="import-sheet"
                        className="bg-background"
                      >
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {inspection.worksheets.map((sheet) => (
                          <SelectItem key={sheet} value={sheet}>
                            {sheet}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                ) : (
                  <div className="grid w-full gap-1.5 sm:w-60">
                    <Label
                      htmlFor="import-delimiter"
                      className="text-xs text-muted-foreground"
                    >
                      Separador CSV
                    </Label>
                    <Select
                      value={delimiter === "\t" ? "tab" : delimiter}
                      onValueChange={(value) =>
                        onDelimiterChange(value === "tab" ? "\t" : value)
                      }
                      disabled={busy}
                    >
                      <SelectTrigger
                        id="import-delimiter"
                        className="bg-background"
                      >
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value=";">Ponto e vírgula (;)</SelectItem>
                        <SelectItem value=",">Vírgula (,)</SelectItem>
                        <SelectItem value="tab">Tabulação</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                ))}
            </div>
          )}
        </section>
      )}
      {source && (
        <section aria-labelledby="import-preview-title" className="space-y-3">
          <div>
            <h3 id="import-preview-title" className="text-sm font-semibold">
              Prévia do arquivo
            </h3>
            <p className="mt-1 text-xs text-muted-foreground">
              Confira as primeiras {Math.min(3, source.rows.length)} linhas.
              Deslize a tabela para ver todas as colunas.
            </p>
          </div>
          <div className="overflow-hidden rounded-lg border">
            <Table className="w-max min-w-full text-sm">
              <TableHeader className="bg-muted/50">
                <TableRow className="hover:bg-transparent">
                  <TableHead className="sticky left-0 z-10 w-16 bg-muted px-4 text-xs">
                    Linha
                  </TableHead>
                  {source.headers.map((header, index) => (
                    <TableHead key={index} className="h-12 min-w-44 px-4">
                      <div className="flex items-center gap-2">
                        <span className="flex h-5 min-w-5 items-center justify-center rounded bg-background px-1 text-[10px] font-medium text-muted-foreground">
                          {index + 1}
                        </span>
                        <span
                          className="max-w-60 truncate font-medium text-foreground"
                          title={header}
                        >
                          {header || "Sem título"}
                        </span>
                      </div>
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {source.rows.slice(0, 3).map((row) => (
                  <TableRow key={row.sourceRow} className="even:bg-muted/20">
                    <TableCell className="sticky left-0 z-10 bg-background px-4 font-mono text-xs text-muted-foreground">
                      {row.sourceRow}
                    </TableCell>
                    {row.cells.map((value, index) => (
                      <TableCell key={index} className="max-w-72 px-4 py-3">
                        <div
                          className="max-w-64 truncate"
                          title={value.replace(/\0/g, "")}
                        >
                          {value.replace(/\0/g, "") || (
                            <span className="text-muted-foreground/60">—</span>
                          )}
                        </div>
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </section>
      )}
      <details className="group text-xs text-muted-foreground">
        <summary className="flex w-fit cursor-pointer list-none items-center gap-1.5 rounded-sm py-1 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
          <Info className="h-3.5 w-3.5" aria-hidden="true" />
          Como preparar o arquivo
          <span
            aria-hidden="true"
            className="ml-1 transition-transform group-open:rotate-180"
          >
            ⌄
          </span>
        </summary>
        <p className="mt-2 max-w-prose leading-relaxed">
          Use CSV em UTF-8 ou uma planilha XLSX sem senha, com os títulos das
          colunas na primeira linha não vazia. Limite de 5.000 linhas, 100
          colunas e 10 MiB por arquivo.
        </p>
      </details>
    </div>
  );
}
