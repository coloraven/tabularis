import { useMemo } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CheckCircle2,
  Loader2,
  ListTodo,
  Trash2,
  XCircle,
  X,
} from "lucide-react";
import { getJobKind, useBackgroundJobs } from "../jobs";
import type { BackgroundJob, JobStatus } from "../jobs";

function StatusIcon({ status }: { status: JobStatus }) {
  switch (status) {
    case "queued":
    case "running":
      return <Loader2 size={16} className="animate-spin text-blue-400" />;
    case "completed":
      return <CheckCircle2 size={16} className="text-green-400" />;
    case "failed":
      return <XCircle size={16} className="text-red-400" />;
    case "cancelled":
      return <X size={16} className="text-muted" />;
  }
}

function JobRow({
  job,
  onCancel,
  onRemove,
}: {
  job: BackgroundJob;
  onCancel: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  const { t } = useTranslation();
  const plugin = getJobKind(job.kind);
  const Details = plugin?.JobRowDetails;
  const canCancel =
    plugin?.canCancel?.(job) ??
    (job.status === "running" || job.status === "queued");
  const kindLabel = plugin
    ? t(plugin.labelKey, { defaultValue: job.kind })
    : job.kind;

  return (
    <li className="flex gap-3 p-4 border-b border-default last:border-b-0">
      <div className="pt-0.5">
        <StatusIcon status={job.status} />
      </div>
      <div className="flex-1 min-w-0 space-y-1">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium text-primary truncate">
            {job.title}
          </span>
          <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-surface-secondary text-muted">
            {kindLabel}
          </span>
          <span className="text-xs text-muted">
            {t(`jobs.status.${job.status}`)}
          </span>
        </div>
        {Details ? <Details job={job} /> : null}
        {job.error && (
          <p className="text-xs text-red-400 break-words">{job.error}</p>
        )}
        {typeof job.progress?.current === "number" &&
          job.status === "running" && (
            <p className="text-xs text-secondary font-mono">
              {job.progress.current.toLocaleString()}
              {job.progress.total != null
                ? ` / ${job.progress.total.toLocaleString()}`
                : ""}
            </p>
          )}
      </div>
      <div className="flex flex-col gap-1 shrink-0">
        {canCancel && (
          <button
            type="button"
            onClick={() => onCancel(job.id)}
            className="px-2 py-1 text-xs rounded border border-red-900/50 text-red-200 hover:bg-red-900/30"
          >
            {t("jobs.cancel")}
          </button>
        )}
        {(job.status === "completed" ||
          job.status === "failed" ||
          job.status === "cancelled") && (
          <button
            type="button"
            onClick={() => onRemove(job.id)}
            className="px-2 py-1 text-xs rounded border border-strong text-secondary hover:bg-surface-secondary"
            title={t("jobs.remove")}
          >
            <Trash2 size={12} className="inline" />
          </button>
        )}
      </div>
    </li>
  );
}

export function JobsPage() {
  const { t } = useTranslation();
  const { jobs, cancel, clearFinished, remove, runningCount } =
    useBackgroundJobs();

  const hasFinished = useMemo(
    () =>
      jobs.some(
        (j) =>
          j.status === "completed" ||
          j.status === "failed" ||
          j.status === "cancelled",
      ),
    [jobs],
  );

  return (
    <div className="h-full flex flex-col bg-base text-primary">
      <header className="flex items-center justify-between gap-3 px-6 py-4 border-b border-default bg-elevated">
        <div className="flex items-center gap-3 min-w-0">
          <div className="p-2 rounded-lg bg-blue-900/30">
            <ListTodo size={20} className="text-blue-400" />
          </div>
          <div className="min-w-0">
            <h1 className="text-lg font-semibold">{t("jobs.title")}</h1>
            <p className="text-xs text-secondary truncate">
              {t("jobs.subtitle", { count: runningCount })}
            </p>
          </div>
        </div>
        {hasFinished && (
          <button
            type="button"
            onClick={clearFinished}
            className="px-3 py-1.5 text-sm rounded-lg border border-strong text-secondary hover:bg-surface-secondary shrink-0"
          >
            {t("jobs.clearFinished")}
          </button>
        )}
      </header>

      <div className="flex-1 overflow-y-auto">
        {jobs.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-2 text-secondary px-6 text-center">
            <ListTodo size={36} className="text-muted opacity-60" />
            <p className="text-sm">{t("jobs.empty")}</p>
            <p className="text-xs text-muted max-w-sm">{t("jobs.emptyHint")}</p>
          </div>
        ) : (
          <div className="max-w-3xl w-full mx-auto my-4 px-4 sm:px-6">
            <ul className="bg-elevated border border-default rounded-xl overflow-hidden">
              {jobs.map((job) => (
                <JobRow
                  key={job.id}
                  job={job}
                  onCancel={(id) => {
                    void cancel(id);
                  }}
                  onRemove={remove}
                />
              ))}
            </ul>
          </div>
        )}
      </div>

      <footer className="px-6 py-3 border-t border-default text-xs text-muted">
        <Link
          to="/task-manager"
          className="text-blue-400 hover:text-blue-300 underline-offset-2 hover:underline"
        >
          {t("jobs.openProcessManager")}
        </Link>
      </footer>
    </div>
  );
}
