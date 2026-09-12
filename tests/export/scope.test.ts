import { describe, expect, it } from "vitest";
import {
  currentPageExportScope,
  defaultExportScope,
  resolveExportWindow,
} from "../../src/export/scope";

describe("resolveExportWindow", () => {
  const ctx = {
    loadedRows: 100,
    currentPage: 2,
    pageSize: 50,
    totalRows: 500,
  };

  it("exports all rows with no window", () => {
    expect(resolveExportWindow({ mode: "all" }, ctx)).toEqual({
      offset: 0,
      maxRows: null,
    });
  });

  it("exports the first N rows", () => {
    expect(
      resolveExportWindow({ mode: "max_rows", maxRows: 25 }, ctx),
    ).toEqual({ offset: 0, maxRows: 25 });
  });

  it("maps pages to offset and maxRows", () => {
    expect(
      resolveExportWindow(
        { mode: "pages", pageSize: 50, startPage: 2, pageCount: 3 },
        ctx,
      ),
    ).toEqual({ offset: 50, maxRows: 150 });
  });

  it("aligns loaded mode with the current page when pagination is known", () => {
    expect(resolveExportWindow({ mode: "loaded" }, ctx)).toEqual({
      offset: 50,
      maxRows: 100,
    });
  });

  it("falls back to a prefix of loaded rows without pagination", () => {
    expect(
      resolveExportWindow({ mode: "loaded" }, { loadedRows: 40 }),
    ).toEqual({ offset: 0, maxRows: 40 });
  });
});

describe("defaultExportScope", () => {
  it("defaults to exporting all rows", () => {
    expect(defaultExportScope({ loadedRows: 10, pageSize: 100 }).mode).toBe(
      "all",
    );
  });
});

describe("currentPageExportScope", () => {
  it("uses a single page when pagination is known", () => {
    expect(
      currentPageExportScope({
        loadedRows: 50,
        currentPage: 3,
        pageSize: 50,
      }),
    ).toEqual({
      mode: "pages",
      startPage: 3,
      pageCount: 1,
      pageSize: 50,
    });
  });

  it("falls back to loaded rows without pagination", () => {
    expect(currentPageExportScope({ loadedRows: 12 })).toEqual({
      mode: "loaded",
    });
  });
});
