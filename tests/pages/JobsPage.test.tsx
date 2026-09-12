import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { JobsPage } from "../../src/pages/JobsPage";
import type { BackgroundJob } from "../../src/jobs";
import { ensureBuiltinJobKinds } from "../../src/jobs";

vi.mock("lucide-react", async (importOriginal) => await importOriginal());

const exportJob: BackgroundJob = {
  id: "1",
  kind: "export",
  title: "Export CSV · a.csv",
  status: "completed",
  createdAt: 1,
  updatedAt: 1,
  meta: { format: "csv", fileName: "a.csv" },
};

const mockApi = {
  jobs: [exportJob] as BackgroundJob[],
  runningCount: 0,
  enqueue: vi.fn(),
  cancel: vi.fn(),
  clearFinished: vi.fn(),
  remove: vi.fn(),
};

vi.mock("../../src/jobs/useBackgroundJobs", () => ({
  useBackgroundJobs: () => mockApi,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string; count?: number }) => {
      const map: Record<string, string> = {
        "jobs.allKinds": "All",
        "jobs.kinds.export": "Export",
        "jobs.title": "Tasks",
        "jobs.empty": "No background tasks",
        "jobs.emptyFiltered": "No tasks in this category",
        "jobs.emptyFilteredHint": "filtered hint",
        "jobs.emptyHint": "hint",
        "jobs.clearFinished": "Clear finished",
        "jobs.openProcessManager": "Open process manager",
        "jobs.cancel": "Cancel",
        "jobs.remove": "Remove",
        "jobs.status.completed": "completed",
        "jobs.export.format": "Format",
        "jobs.export.rows": "Rows",
      };
      if (key === "jobs.subtitle") return `${opts?.count ?? 0} running`;
      return map[key] ?? opts?.defaultValue ?? key;
    },
  }),
}));

describe("JobsPage", () => {
  beforeEach(() => {
    ensureBuiltinJobKinds();
    mockApi.jobs = [exportJob];
    mockApi.remove.mockClear();
    mockApi.clearFinished.mockClear();
  });

  it("lists kind categories in the sidebar", () => {
    render(
      <MemoryRouter>
        <JobsPage />
      </MemoryRouter>,
    );

    expect(screen.getByRole("button", { name: /All/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Export/ })).toBeInTheDocument();
    expect(screen.getByText("Export CSV · a.csv")).toBeInTheDocument();
  });

  it("clears finished jobs for the selected kind only", () => {
    render(
      <MemoryRouter>
        <JobsPage />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
    fireEvent.click(screen.getByRole("button", { name: "Clear finished" }));
    expect(mockApi.remove).toHaveBeenCalledWith("1");
    expect(mockApi.clearFinished).not.toHaveBeenCalled();
  });
});
