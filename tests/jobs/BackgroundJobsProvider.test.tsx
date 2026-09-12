import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import {
  BackgroundJobsProvider,
  registerJobKind,
  useBackgroundJobs,
  type JobKindPlugin,
} from "../../src/jobs";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

function wrapper({ children }: { children: ReactNode }) {
  return <BackgroundJobsProvider>{children}</BackgroundJobsProvider>;
}

describe("BackgroundJobs store", () => {
  const kindId = `hook-test-${Math.random().toString(36).slice(2)}`;

  beforeEach(() => {
    const plugin: JobKindPlugin<{ ok: boolean }> = {
      id: kindId,
      labelKey: "jobs.kinds.export",
      start: async (ctx, input) => {
        ctx.setStatus("running");
        if (!input.ok) throw new Error("boom");
        ctx.updateProgress({ current: 3 });
        ctx.setStatus("completed");
      },
      cancel: async () => {},
      canCancel: (job) => job.status === "running" || job.status === "queued",
    };
    registerJobKind(plugin);
  });

  it("enqueues, completes, and clears finished jobs", async () => {
    const { result } = renderHook(() => useBackgroundJobs(), { wrapper });

    let jobId = "";
    act(() => {
      jobId = result.current.enqueue(kindId, "Test job", { ok: true });
    });

    await waitFor(() => {
      const job = result.current.jobs.find((j) => j.id === jobId);
      expect(job?.status).toBe("completed");
    });

    expect(result.current.runningCount).toBe(0);

    act(() => {
      result.current.clearFinished();
    });
    expect(result.current.jobs.find((j) => j.id === jobId)).toBeUndefined();
  });

  it("marks failed jobs", async () => {
    const { result } = renderHook(() => useBackgroundJobs(), { wrapper });

    let jobId = "";
    act(() => {
      jobId = result.current.enqueue(kindId, "Fail job", { ok: false });
    });

    await waitFor(() => {
      const job = result.current.jobs.find((j) => j.id === jobId);
      expect(job?.status).toBe("failed");
      expect(job?.error).toContain("boom");
    });
  });

  it("fails unknown kinds", async () => {
    const { result } = renderHook(() => useBackgroundJobs(), { wrapper });

    let jobId = "";
    act(() => {
      jobId = result.current.enqueue("missing-kind", "Nope", {});
    });

    await waitFor(() => {
      const job = result.current.jobs.find((j) => j.id === jobId);
      expect(job?.status).toBe("failed");
    });
  });
});
