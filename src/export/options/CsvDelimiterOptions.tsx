import { useTranslation } from "react-i18next";
import type { ExportFormatOptionsProps } from "../formats";

/** CSV-only option slot for the export settings wizard. */
export function CsvDelimiterOptions({
  value,
  onChange,
}: ExportFormatOptionsProps) {
  const { t } = useTranslation();
  const delimiter =
    typeof value.csvDelimiter === "string" && value.csvDelimiter.length > 0
      ? value.csvDelimiter
      : ",";

  return (
    <label className="flex flex-col gap-1.5 text-sm">
      <span className="text-secondary">{t("settings.csvDelimiter")}</span>
      <input
        type="text"
        maxLength={1}
        value={delimiter}
        onChange={(e) =>
          onChange({
            ...value,
            csvDelimiter: e.target.value.slice(0, 1) || ",",
          })
        }
        className="w-16 px-2 py-1.5 bg-surface-secondary border border-strong rounded text-primary font-mono"
        aria-label={t("settings.csvDelimiter")}
      />
    </label>
  );
}
