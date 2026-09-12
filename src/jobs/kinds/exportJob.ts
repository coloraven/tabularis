import { invoke } from "@tauri-apps/api/core";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import type { ExportFormatId } from "../../export";
import { registerJobKind } from "../registry";
import type { JobKindPlugin, JobStartContext } from "../types";
import { ExportJobRowDetails } from "./ExportJobRowDetails";

export const EXPORT_JOB_KIND = "export";

export interface ExportJobInput {
  connectionId?: string;
  query?: string;
  filePath: string;
  formatId: ExportFormatId;
  csvDelimiter?: string;
  offset?: number;
  maxRows?: number | null;
  database?: string;
  /** When set, write this text locally instead of streaming from the backend. */
  memoryText?: string;
  memoryRowCount?: number;
}

async function startExport(
  ctx: JobStartContext,
  input: ExportJobInput,
): Promise<void> {
  ctx.setStatus("running");
  ctx.patchMeta({
    format: input.formatId,
    fileName: input.filePath.split(/[/\\]/).pop() || input.filePath,
    filePath: input.filePath,
    connectionId: input.connectionId,
  });

  if (input.memoryText != null) {
    await writeTextFile(input.filePath, input.memoryText);
    ctx.updateProgress({
      current: input.memoryRowCount ?? 0,
    });
    ctx.setStatus("completed");
    return;
  }

  if (!input.connectionId || !input.query) {
    throw new Error("Export requires a connection and query");
  }

  await invoke("export_query_to_file", {
    jobId: ctx.jobId,
    connectionId: input.connectionId,
    query: input.query,
    filePath: input.filePath,
    format: input.formatId,
    csvDelimiter:
      input.formatId === "csv" ? input.csvDelimiter : undefined,
    offset: input.offset && input.offset > 0 ? input.offset : undefined,
    maxRows: input.maxRows ?? undefined,
    ...(input.database ? { database: input.database } : {}),
  });
}

async function cancelExport(jobId: string): Promise<void> {
  await invoke("cancel_export", { jobId });
}

export const exportJobKind: JobKindPlugin<ExportJobInput> = {
  id: EXPORT_JOB_KIND,
  labelKey: "jobs.kinds.export",
  start: startExport,
  cancel: cancelExport,
  canCancel: (job) => job.status === "running" || job.status === "queued",
  JobRowDetails: ExportJobRowDetails,
};

export function registerExportJobKind(): void {
  registerJobKind(exportJobKind as JobKindPlugin);
}
