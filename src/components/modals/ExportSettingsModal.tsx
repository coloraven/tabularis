import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, X } from "lucide-react";
import { Modal } from "../ui/Modal";
import {
  defaultExportScope,
  EXPORT_FORMAT_PLUGINS,
  getExportFormatPlugin,
  type ExportConfirmPayload,
  type ExportFormatId,
  type ExportFormatOptions,
  type ExportScope,
  type ExportScopeContext,
  type ExportScopeMode,
  resolveExportWindow,
} from "../../export";

export interface ExportSettingsModalProps {
  isOpen: boolean;
  formatId: ExportFormatId | null;
  scopeContext: ExportScopeContext;
  initialFormatOptions?: ExportFormatOptions;
  /** When true, show a format picker (Advanced export entry). */
  allowFormatChange?: boolean;
  onFormatChange?: (formatId: ExportFormatId) => void;
  onClose: () => void;
  onConfirm: (payload: ExportConfirmPayload) => void;
}

const SCOPE_MODES: ExportScopeMode[] = ["all", "max_rows", "pages", "loaded"];

export function ExportSettingsModal({
  isOpen,
  formatId,
  scopeContext,
  initialFormatOptions,
  allowFormatChange = false,
  onFormatChange,
  onClose,
  onConfirm,
}: ExportSettingsModalProps) {
  const { t } = useTranslation();
  const plugin = formatId ? getExportFormatPlugin(formatId) : undefined;

  const [scope, setScope] = useState<ExportScope>(() =>
    defaultExportScope(scopeContext),
  );
  const [formatOptions, setFormatOptions] = useState<ExportFormatOptions>(
    () => initialFormatOptions ?? {},
  );

  useEffect(() => {
    if (!isOpen) return;
    setScope(defaultExportScope(scopeContext));
    setFormatOptions(initialFormatOptions ?? {});
  }, [isOpen, formatId, scopeContext, initialFormatOptions]);

  const preview = useMemo(
    () => resolveExportWindow(scope, scopeContext),
    [scope, scopeContext],
  );

  if (!isOpen || !formatId || !plugin) return null;

  const formatLabel = plugin.labelKey
    ? t(plugin.labelKey, { defaultValue: plugin.label })
    : plugin.label;

  const Options = plugin.OptionsComponent;

  const handleConfirm = () => {
    onConfirm({ formatId, scope, formatOptions });
  };

  const setMode = (mode: ExportScopeMode) => {
    setScope((prev) => ({ ...prev, mode }));
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} closeOnBackdrop>
      <div className="bg-elevated border border-strong rounded-xl shadow-2xl w-[520px] max-h-[90vh] overflow-hidden flex flex-col">
        <div className="flex items-center justify-between p-4 border-b border-default bg-base">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-blue-900/30 rounded-lg">
              <Download size={20} className="text-blue-400" />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-primary">
                {t("editor.exportSettings.title", { format: formatLabel })}
              </h2>
              <p className="text-xs text-secondary">
                {t("editor.exportSettings.subtitle")}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="text-secondary hover:text-primary transition-colors"
            aria-label={t("common.close")}
          >
            <X size={20} />
          </button>
        </div>

        <div className="p-6 space-y-5 overflow-y-auto">
          {allowFormatChange && (
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="text-secondary">
                {t("editor.exportSettings.format")}
              </span>
              <select
                value={formatId}
                onChange={(e) =>
                  onFormatChange?.(e.target.value as ExportFormatId)
                }
                className="w-full px-2 py-1.5 bg-surface-secondary border border-strong rounded text-primary"
              >
                {EXPORT_FORMAT_PLUGINS.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.labelKey
                      ? t(p.labelKey, { defaultValue: p.label })
                      : p.label}
                  </option>
                ))}
              </select>
            </label>
          )}

          <fieldset className="space-y-3">
            <legend className="text-sm font-medium text-primary mb-1">
              {t("editor.exportSettings.scope")}
            </legend>
            {SCOPE_MODES.map((mode) => (
              <label
                key={mode}
                className="flex items-start gap-2.5 text-sm text-secondary cursor-pointer"
              >
                <input
                  type="radio"
                  name="export-scope"
                  className="mt-1"
                  checked={scope.mode === mode}
                  onChange={() => setMode(mode)}
                />
                <span className="flex-1">
                  <span className="text-primary block">
                    {t(`editor.exportSettings.scope_${mode}`)}
                  </span>
                  {mode === "loaded" && (
                    <span className="text-xs text-muted">
                      {t("editor.exportSettings.loadedHint", {
                        loaded: scopeContext.loadedRows.toLocaleString(),
                        total:
                          scopeContext.totalRows != null
                            ? scopeContext.totalRows.toLocaleString()
                            : "—",
                      })}
                    </span>
                  )}
                </span>
              </label>
            ))}
          </fieldset>

          {scope.mode === "max_rows" && (
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="text-secondary">
                {t("editor.exportSettings.rows")}
              </span>
              <input
                type="number"
                min={1}
                value={scope.maxRows ?? 1000}
                onChange={(e) =>
                  setScope((prev) => ({
                    ...prev,
                    maxRows: Math.max(1, Number(e.target.value) || 1),
                  }))
                }
                className="w-40 px-2 py-1.5 bg-surface-secondary border border-strong rounded text-primary"
              />
            </label>
          )}

          {scope.mode === "pages" && (
            <div className="grid grid-cols-3 gap-3">
              <label className="flex flex-col gap-1.5 text-sm">
                <span className="text-secondary">
                  {t("editor.exportSettings.pageSize")}
                </span>
                <input
                  type="number"
                  min={1}
                  value={scope.pageSize ?? 100}
                  onChange={(e) =>
                    setScope((prev) => ({
                      ...prev,
                      pageSize: Math.max(1, Number(e.target.value) || 1),
                    }))
                  }
                  className="px-2 py-1.5 bg-surface-secondary border border-strong rounded text-primary"
                />
              </label>
              <label className="flex flex-col gap-1.5 text-sm">
                <span className="text-secondary">
                  {t("editor.exportSettings.startPage")}
                </span>
                <input
                  type="number"
                  min={1}
                  value={scope.startPage ?? 1}
                  onChange={(e) =>
                    setScope((prev) => ({
                      ...prev,
                      startPage: Math.max(1, Number(e.target.value) || 1),
                    }))
                  }
                  className="px-2 py-1.5 bg-surface-secondary border border-strong rounded text-primary"
                />
              </label>
              <label className="flex flex-col gap-1.5 text-sm">
                <span className="text-secondary">
                  {t("editor.exportSettings.pageCount")}
                </span>
                <input
                  type="number"
                  min={1}
                  value={scope.pageCount ?? 1}
                  onChange={(e) =>
                    setScope((prev) => ({
                      ...prev,
                      pageCount: Math.max(1, Number(e.target.value) || 1),
                    }))
                  }
                  className="px-2 py-1.5 bg-surface-secondary border border-strong rounded text-primary"
                />
              </label>
            </div>
          )}

          <p className="text-xs text-muted">
            {t("editor.exportSettings.windowPreview", {
              offset: preview.offset.toLocaleString(),
              maxRows:
                preview.maxRows == null
                  ? t("editor.exportSettings.unlimited")
                  : preview.maxRows.toLocaleString(),
            })}
          </p>

          {Options && (
            <div className="pt-2 border-t border-default space-y-2">
              <p className="text-sm font-medium text-primary">
                {t("editor.exportSettings.formatOptions")}
              </p>
              <Options value={formatOptions} onChange={setFormatOptions} />
            </div>
          )}
        </div>

        <div className="p-4 border-t border-default bg-base/50 flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-secondary hover:text-primary transition-colors text-sm"
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            onClick={handleConfirm}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-sm font-medium"
          >
            {t("editor.exportSettings.confirm")}
          </button>
        </div>
      </div>
    </Modal>
  );
}
