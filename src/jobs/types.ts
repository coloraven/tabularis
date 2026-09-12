import type { ComponentType } from "react";

export type JobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export type JobKindId = string;

export interface JobProgress {
  current?: number;
  total?: number;
  message?: string;
}

export interface BackgroundJob {
  id: string;
  kind: JobKindId;
  title: string;
  status: JobStatus;
  progress?: JobProgress;
  createdAt: number;
  updatedAt: number;
  error?: string;
  meta: Record<string, unknown>;
}

export interface JobStartContext {
  jobId: string;
  updateProgress: (progress: JobProgress) => void;
  setStatus: (status: JobStatus, error?: string) => void;
  patchMeta: (meta: Record<string, unknown>) => void;
}

export interface JobKindPlugin<TInput = unknown> {
  id: JobKindId;
  labelKey: string;
  start: (ctx: JobStartContext, input: TInput) => Promise<void>;
  cancel: (jobId: string) => Promise<void>;
  canCancel?: (job: BackgroundJob) => boolean;
  JobRowDetails?: ComponentType<{ job: BackgroundJob }>;
}

export interface BackgroundJobProgressEvent {
  job_id: string;
  current?: number;
  total?: number;
  message?: string;
}

export interface ExportProgressEvent {
  job_id: string;
  rows_processed: number;
}
