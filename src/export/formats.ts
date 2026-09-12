import type { ComponentType } from "react";
import { CsvDelimiterOptions } from "./options/CsvDelimiterOptions";
import type { ExportFormatId, ExportFormatOptions } from "./types";

export interface ExportFormatOptionsProps {
  value: ExportFormatOptions;
  onChange: (next: ExportFormatOptions) => void;
}

export interface ExportFormatPlugin {
  id: ExportFormatId;
  /** Short display label shown in the menu (e.g. "CSV", "Parquet"). */
  label: string;
  /** i18n key under editor.*; optional — label is used when absent. */
  labelKey?: string;
  extension: string;
  filterName: string;
  /** Whether the format can be written from in-memory loaded rows without backend. */
  supportsLoadedMemoryExport: boolean;
  OptionsComponent?: ComponentType<ExportFormatOptionsProps>;
  buildInvokeExtras?: (
    options: ExportFormatOptions,
  ) => Record<string, unknown>;
}

export const EXPORT_FORMAT_PLUGINS: ExportFormatPlugin[] = [
  {
    id: "csv",
    label: "CSV",
    extension: "csv",
    filterName: "CSV",
    supportsLoadedMemoryExport: true,
    OptionsComponent: CsvDelimiterOptions,
    buildInvokeExtras: (options) => ({
      csvDelimiter:
        typeof options.csvDelimiter === "string"
          ? options.csvDelimiter
          : undefined,
    }),
  },
  {
    id: "json",
    label: "JSON",
    extension: "json",
    filterName: "JSON",
    supportsLoadedMemoryExport: true,
  },
  {
    id: "markdown",
    label: "Markdown",
    extension: "md",
    filterName: "Markdown",
    supportsLoadedMemoryExport: true,
  },
  {
    id: "parquet",
    label: "Parquet",
    labelKey: "editor.exportParquet",
    extension: "parquet",
    filterName: "Parquet",
    supportsLoadedMemoryExport: false,
  },
];

export function getExportFormatPlugin(
  id: ExportFormatId,
): ExportFormatPlugin | undefined {
  return EXPORT_FORMAT_PLUGINS.find((p) => p.id === id);
}
