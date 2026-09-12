import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CheckCircle2,
  Download,
  Loader2,
  ListTodo,
  Trash2,
  XCircle,
  X,
} from "lucide-react";
import clsx from "clsx";
import {
  EXPORT_JOB_KIND,
  getJobKind,
  listJobKinds,
  useBackgroundJobs,
  type BackgroundJob,
  type JobKindId,
  type JobStatus,
} from "../jobs";

const ALL_FILTER = "all" as const;
type KindFilter = typeof ALL_FILTER | JobKindId;

function kindIcon(kind: JobKindId) {
  if (kind === EXPORT_JOB_KIND) return Download;
  return ListTodo;
}

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
  showKindBadge,
}: {
  job: BackgroundJob;
  onCancel: (id: string) => void;
  onRemove: (id: string) => void;
  showKindBadge: boolean;
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
          {showKindBadge && (
            <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-surface-secondary text-muted">
              {kindLabel}
            </span>
          )}
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

function countForKind(jobs: BackgroundJob[], kind: JobKindId): number {
  return jobs.filter((j) => j.kind === kind).length;
}

function runningForKind(jobs: BackgroundJob[], kind?: JobKindId): number {
  return jobs.filter(
    (j) =>
      (kind == null || j.kind === kind) &&
      (j.status === "running" || j.status === "queued"),
  ).length;
}

export function JobsPage() {
  const { t } = useTranslation();
  const { jobs, cancel, clearFinished, remove, runningCount } =
    useBackgroundJobs();
  const [kindFilter, setKindFilter] = useState<KindFilter>(ALL_FILTER);

  const registeredKinds = listJobKinds();

  const kindNavItems = useMemo(() => {
    const known = new Set(registeredKinds.map((k) => k.id));
    const extras = Array.from(
      new Set(jobs.map((j) => j.kind).filter((id) => !known.has(id))),
    );
    return [
      ...registeredKinds.map((plugin) => ({
        id: plugin.id as KindFilter,
        labelKey: plugin.labelKey,
        fallback: plugin.id,
      })),
      ...extras.map((id) => ({
        id: id as KindFilter,
        labelKey: `jobs.kinds.${id}`,
        fallback: id,
      })),
    ];
  }, [jobs, registeredKinds]);

  const filteredJobs = useMemo(() => {
    if (kindFilter === ALL_FILTER) return jobs;
    return jobs.filter((j) => j.kind === kindFilter);
  }, [jobs, kindFilter]);

  const hasFinishedInView = useMemo(
    () =>
      filteredJobs.some(
        (j) =>
          j.status === "completed" ||
          j.status === "failed" ||
          j.status === "cancelled",
      ),
    [filteredJobs],
  );

  const headerRunning =
    kindFilter === ALL_FILTER
      ? runningCount
      : runningForKind(jobs, kindFilter);

  const activePlugin =
    kindFilter === ALL_FILTER ? undefined : getJobKind(kindFilter);
  const resolvedSectionTitle =
    kindFilter === ALL_FILTER
      ? t("jobs.title")
      : activePlugin
        ? t(activePlugin.labelKey, { defaultValue: activePlugin.id })
        : t(`jobs.kinds.${kindFilter}`, { defaultValue: kindFilter });

  const handleClearFinished = () => {
    if (kindFilter === ALL_FILTER) {
      clearFinished();
      return;
    }
    for (const job of filteredJobs) {
      if (
        job.status === "completed" ||
        job.status === "failed" ||
        job.status === "cancelled"
      ) {
        remove(job.id);
      }
    }
  };

  return (
    <div className="h-full flex bg-base text-primary">
      <nav className="w-52 flex flex-col border-r border-default bg-elevated shrink-0">
        <div className="px-4 py-4 border-b border-default">
          <h2 className="text-sm font-semibold text-primary">
            {t("jobs.title")}
          </h2>
          <p className="text-[11px] text-muted mt-0.5">
            {t("jobs.subtitle", { count: runningCount })}
          </p>
        </div>
        <div className="flex-1 py-2 px-2 overflow-y-auto space-y-0.5">
          <button
            type="button"
            onClick={() => setKindFilter(ALL_FILTER)}
            className={clsx(
              "w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors text-left",
              kindFilter === ALL_FILTER
                ? "bg-surface-secondary text-primary"
                : "text-muted hover:text-primary hover:bg-surface-secondary/50",
            )}
          >
            <ListTodo size={16} />
            <span className="truncate flex-1">{t("jobs.allKinds")}</span>
            {jobs.length > 0 && (
              <span className="text-[10px] font-semibold text-muted tabular-nums">
                {jobs.length}
              </span>
            )}
          </button>

          {kindNavItems.map((item) => {
            const Icon = kindIcon(item.id);
            const count = countForKind(jobs, item.id);
            const running = runningForKind(jobs, item.id);
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setKindFilter(item.id)}
                className={clsx(
                  "w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors text-left",
                  kindFilter === item.id
                    ? "bg-surface-secondary text-primary"
                    : "text-muted hover:text-primary hover:bg-surface-secondary/50",
                )}
              >
                <Icon size={16} />
                <span className="truncate flex-1">
                  {t(item.labelKey, { defaultValue: item.fallback })}
                </span>
                {running > 0 ? (
                  <span className="min-w-[1.25rem] text-center rounded-full bg-blue-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-blue-400">
                    {running}
                  </span>
                ) : count > 0 ? (
                  <span className="text-[10px] font-semibold text-muted tabular-nums">
                    {count}
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
        <div className="p-2 border-t border-default">
          <Link
            to="/task-manager"
            className="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs text-muted hover:text-primary hover:bg-surface-secondary/50 transition-colors"
          >
            {t("jobs.openProcessManager")}
          </Link>
        </div>
      </nav>

      <div className="flex-1 flex flex-col min-w-0">
        <header className="flex items-center justify-between gap-3 px-6 py-4 border-b border-default bg-elevated">
          <div className="min-w-0">
            <h1 className="text-lg font-semibold">{resolvedSectionTitle}</h1>
            <p className="text-xs text-secondary truncate">
              {t("jobs.subtitle", { count: headerRunning })}
            </p>
          </div>
          {hasFinishedInView && (
            <button
              type="button"
              onClick={handleClearFinished}
              className="px-3 py-1.5 text-sm rounded-lg border border-strong text-secondary hover:bg-surface-secondary shrink-0"
            >
              {t("jobs.clearFinished")}
            </button>
          )}
        </header>

        <div className="flex-1 overflow-y-auto">
          {filteredJobs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-2 text-secondary px-6 text-center">
              <ListTodo size={36} className="text-muted opacity-60" />
              <p className="text-sm">
                {jobs.length === 0
                  ? t("jobs.empty")
                  : t("jobs.emptyFiltered")}
              </p>
              <p className="text-xs text-muted max-w-sm">
                {jobs.length === 0
                  ? t("jobs.emptyHint")
                  : t("jobs.emptyFilteredHint")}
              </p>
            </div>
          ) : (
            <div className="max-w-3xl w-full mx-auto my-4 px-4 sm:px-6">
              <ul className="bg-elevated border border-default rounded-xl overflow-hidden">
                {filteredJobs.map((job) => (
                  <JobRow
                    key={job.id}
                    job={job}
                    showKindBadge={kindFilter === ALL_FILTER}
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
      </div>
    </div>
  );
}
