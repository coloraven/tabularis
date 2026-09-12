import { useTranslation } from "react-i18next";
import type { BackgroundJob } from "../types";

export function ExportJobRowDetails({ job }: { job: BackgroundJob }) {
  const { t } = useTranslation();
  const format =
    typeof job.meta.format === "string" ? job.meta.format : undefined;
  const fileName =
    typeof job.meta.fileName === "string" ? job.meta.fileName : undefined;
  const rows = job.progress?.current;

  return (
    <div className="text-xs text-muted space-y-0.5">
      {format && (
        <div>
          {t("jobs.export.format")}:{" "}
          <span className="text-secondary">{format}</span>
        </div>
      )}
      {fileName && (
        <div className="truncate" title={fileName}>
          {fileName}
        </div>
      )}
      {typeof rows === "number" && (
        <div>
          {t("jobs.export.rows")}:{" "}
          <span className="font-mono text-secondary">
            {rows.toLocaleString()}
          </span>
        </div>
      )}
    </div>
  );
}
