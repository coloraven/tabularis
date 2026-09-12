import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ExportSettingsModal } from "../../../src/components/modals/ExportSettingsModal";

vi.mock("../../../src/components/ui/Modal", () => ({
  Modal: ({
    isOpen,
    children,
  }: {
    isOpen: boolean;
    children: React.ReactNode;
  }) => (isOpen ? <div data-testid="modal">{children}</div> : null),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, string>) => {
      if (key === "editor.exportSettings.title") {
        return `Export · ${opts?.format ?? ""}`;
      }
      if (key.startsWith("editor.exportSettings.scope_")) {
        return key.replace("editor.exportSettings.scope_", "");
      }
      if (key === "editor.exportSettings.confirm") return "Export";
      if (key === "common.cancel") return "Cancel";
      if (key === "common.close") return "Close";
      if (key === "editor.exportSettings.unlimited") return "all";
      if (key === "editor.exportSettings.windowPreview") {
        return `offset=${opts?.offset};max=${opts?.maxRows}`;
      }
      if (key === "settings.csvDelimiter") return "CSV delimiter";
      return key;
    },
  }),
}));

describe("ExportSettingsModal", () => {
  it("confirms the selected scope and format", () => {
    const onConfirm = vi.fn();
    render(
      <ExportSettingsModal
        isOpen
        formatId="json"
        scopeContext={{ loadedRows: 20, pageSize: 100, currentPage: 1 }}
        onClose={vi.fn()}
        onConfirm={onConfirm}
      />,
    );

    fireEvent.click(screen.getByText("max_rows"));
    fireEvent.click(screen.getByRole("button", { name: "Export" }));

    expect(onConfirm).toHaveBeenCalledWith(
      expect.objectContaining({
        formatId: "json",
        scope: expect.objectContaining({ mode: "max_rows" }),
      }),
    );
  });

  it("renders CSV delimiter options for the csv plugin", () => {
    render(
      <ExportSettingsModal
        isOpen
        formatId="csv"
        scopeContext={{ loadedRows: 0 }}
        initialFormatOptions={{ csvDelimiter: ";" }}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    expect(screen.getByDisplayValue(";")).toBeInTheDocument();
  });
});
