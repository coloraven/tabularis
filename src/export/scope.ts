import type { ExportScope, ExportScopeContext, ExportWindow } from "./types";

/**
 * Maps a user-facing export scope to a stream window (offset + max rows).
 * Dialect-agnostic: the backend applies this via skip/take on the row stream.
 */
export function resolveExportWindow(
  scope: ExportScope,
  ctx: ExportScopeContext,
): ExportWindow {
  switch (scope.mode) {
    case "all":
      return { offset: 0, maxRows: null };
    case "max_rows": {
      const n = Math.max(0, Math.floor(scope.maxRows ?? 0));
      return { offset: 0, maxRows: n === 0 ? null : n };
    }
    case "pages": {
      const pageSize = Math.max(
        1,
        Math.floor(scope.pageSize ?? ctx.pageSize ?? 100),
      );
      const startPage = Math.max(1, Math.floor(scope.startPage ?? 1));
      const pageCount = Math.max(1, Math.floor(scope.pageCount ?? 1));
      return {
        offset: (startPage - 1) * pageSize,
        maxRows: pageCount * pageSize,
      };
    }
    case "loaded": {
      const loaded = Math.max(0, Math.floor(ctx.loadedRows));
      const pageSize = Math.max(1, Math.floor(ctx.pageSize ?? pageSizeFallback(ctx)));
      const currentPage = Math.max(1, Math.floor(ctx.currentPage ?? 1));
      // Prefer aligning with the current page window when pagination is known.
      if (ctx.currentPage != null && ctx.pageSize != null) {
        return {
          offset: (currentPage - 1) * pageSize,
          maxRows: loaded,
        };
      }
      return { offset: 0, maxRows: loaded === 0 ? null : loaded };
    }
    default:
      return { offset: 0, maxRows: null };
  }
}

function pageSizeFallback(ctx: ExportScopeContext): number {
  return ctx.pageSize ?? 100;
}

export function defaultExportScope(ctx: ExportScopeContext): ExportScope {
  return {
    mode: "all",
    maxRows: Math.min(1000, Math.max(ctx.loadedRows || 100, 1)),
    pageSize: ctx.pageSize ?? 100,
    startPage: ctx.currentPage ?? 1,
    pageCount: 1,
  };
}
