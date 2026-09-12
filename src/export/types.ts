export type ExportFormatId = "csv" | "json" | "markdown" | "parquet";

export type ExportScopeMode = "all" | "max_rows" | "pages" | "loaded";

export interface ExportScope {
  mode: ExportScopeMode;
  /** Used when mode === "max_rows" */
  maxRows?: number;
  /** Used when mode === "pages" */
  pageSize?: number;
  startPage?: number;
  pageCount?: number;
}

export interface ExportWindow {
  offset: number;
  maxRows: number | null;
}

export interface ExportScopeContext {
  loadedRows: number;
  /** Current result page (1-based), when known */
  currentPage?: number;
  /** Current UI page size, when known */
  pageSize?: number;
  totalRows?: number | null;
}

/** Format-specific options bag; plugins may extend via index signature. */
export type ExportFormatOptions = {
  csvDelimiter?: string;
  [key: string]: unknown;
};

export interface ExportConfirmPayload {
  formatId: ExportFormatId;
  scope: ExportScope;
  formatOptions: ExportFormatOptions;
}
