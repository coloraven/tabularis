import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { BackgroundJobsContext } from "./BackgroundJobsContext";
import { getJobKind } from "./registry";
import { ensureBuiltinJobKinds } from "./registerBuiltinJobs";
import type {
  BackgroundJob,
  BackgroundJobProgressEvent,
  ExportProgressEvent,
  JobKindId,
  JobProgress,
  JobStartContext,
  JobStatus,
} from "./types";

ensureBuiltinJobKinds();

function newJobId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `job-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

export function BackgroundJobsProvider({ children }: { children: ReactNode }) {
  const [jobs, setJobs] = useState<BackgroundJob[]>([]);
  const cancelledRef = useRef(new Set<string>());
  const jobsRef = useRef(jobs);
  jobsRef.current = jobs;

  const patchJob = useCallback(
    (jobId: string, patch: (job: BackgroundJob) => BackgroundJob) => {
      setJobs((prev) =>
        prev.map((job) => (job.id === jobId ? patch(job) : job)),
      );
    },
    [],
  );

  const makeCtx = useCallback(
    (jobId: string): JobStartContext => ({
      jobId,
      updateProgress: (progress: JobProgress) => {
        patchJob(jobId, (job) => ({
          ...job,
          progress: { ...job.progress, ...progress },
          updatedAt: Date.now(),
        }));
      },
      setStatus: (status: JobStatus, error?: string) => {
        patchJob(jobId, (job) => ({
          ...job,
          status,
          error,
          updatedAt: Date.now(),
        }));
      },
      patchMeta: (meta: Record<string, unknown>) => {
        patchJob(jobId, (job) => ({
          ...job,
          meta: { ...job.meta, ...meta },
          updatedAt: Date.now(),
        }));
      },
    }),
    [patchJob],
  );

  useEffect(() => {
    const unlistenExport = listen<ExportProgressEvent>(
      "export_progress",
      (event) => {
        const { job_id, rows_processed } = event.payload;
        if (!job_id) return;
        patchJob(job_id, (job) => ({
          ...job,
          status: job.status === "queued" ? "running" : job.status,
          progress: { ...job.progress, current: rows_processed },
          updatedAt: Date.now(),
        }));
      },
    );

    const unlistenGeneric = listen<BackgroundJobProgressEvent>(
      "background_job_progress",
      (event) => {
        const { job_id, current, total, message } = event.payload;
        if (!job_id) return;
        patchJob(job_id, (job) => ({
          ...job,
          status: job.status === "queued" ? "running" : job.status,
          progress: {
            ...job.progress,
            ...(current != null ? { current } : {}),
            ...(total != null ? { total } : {}),
            ...(message != null ? { message } : {}),
          },
          updatedAt: Date.now(),
        }));
      },
    );

    return () => {
      void unlistenExport.then((f) => f());
      void unlistenGeneric.then((f) => f());
    };
  }, [patchJob]);

  const enqueue = useCallback(
    (
      kind: JobKindId,
      title: string,
      input: unknown,
      meta: Record<string, unknown> = {},
    ): string => {
      const id = newJobId();
      const now = Date.now();
      const job: BackgroundJob = {
        id,
        kind,
        title,
        status: "queued",
        createdAt: now,
        updatedAt: now,
        meta,
      };
      setJobs((prev) => [job, ...prev]);

      const plugin = getJobKind(kind);
      if (!plugin) {
        patchJob(id, (j) => ({
          ...j,
          status: "failed",
          error: `Unknown job kind: ${kind}`,
          updatedAt: Date.now(),
        }));
        return id;
      }

      const ctx = makeCtx(id);
      void (async () => {
        try {
          await plugin.start(ctx, input);
          if (cancelledRef.current.has(id)) {
            cancelledRef.current.delete(id);
            patchJob(id, (j) =>
              j.status === "cancelled"
                ? j
                : { ...j, status: "cancelled", updatedAt: Date.now() },
            );
            return;
          }
          patchJob(id, (j) => {
            if (
              j.status === "failed" ||
              j.status === "cancelled" ||
              j.status === "completed"
            ) {
              return j;
            }
            return { ...j, status: "completed", updatedAt: Date.now() };
          });
        } catch (e) {
          if (cancelledRef.current.has(id)) {
            cancelledRef.current.delete(id);
            patchJob(id, (j) => ({
              ...j,
              status: "cancelled",
              updatedAt: Date.now(),
            }));
            return;
          }
          const message = String(e);
          const cancelled =
            /cancel/i.test(message) || message.includes("Export cancelled");
          patchJob(id, (j) => ({
            ...j,
            status: cancelled ? "cancelled" : "failed",
            error: cancelled ? undefined : message,
            updatedAt: Date.now(),
          }));
        }
      })();

      return id;
    },
    [makeCtx, patchJob],
  );

  const cancel = useCallback(
    async (jobId: string) => {
      const job = jobsRef.current.find((j) => j.id === jobId);
      if (!job) return;
      const plugin = getJobKind(job.kind);
      cancelledRef.current.add(jobId);
      try {
        await plugin?.cancel(jobId);
      } catch (e) {
        console.error("Failed to cancel job", e);
      }
      patchJob(jobId, (j) => ({
        ...j,
        status: "cancelled",
        updatedAt: Date.now(),
      }));
    },
    [patchJob],
  );

  const clearFinished = useCallback(() => {
    setJobs((prev) =>
      prev.filter(
        (j) =>
          j.status === "queued" ||
          j.status === "running",
      ),
    );
  }, []);

  const remove = useCallback((jobId: string) => {
    setJobs((prev) => prev.filter((j) => j.id !== jobId));
  }, []);

  const runningCount = useMemo(
    () =>
      jobs.filter((j) => j.status === "running" || j.status === "queued")
        .length,
    [jobs],
  );

  const value = useMemo(
    () => ({
      jobs,
      runningCount,
      enqueue,
      cancel,
      clearFinished,
      remove,
    }),
    [jobs, runningCount, enqueue, cancel, clearFinished, remove],
  );

  return (
    <BackgroundJobsContext.Provider value={value}>
      {children}
    </BackgroundJobsContext.Provider>
  );
}
