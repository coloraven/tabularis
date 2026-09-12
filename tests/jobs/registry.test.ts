import { describe, expect, it, beforeEach } from "vitest";
import {
  getJobKind,
  listJobKinds,
  registerJobKind,
  type JobKindPlugin,
} from "../../src/jobs";

describe("JobKind registry", () => {
  beforeEach(() => {
    // Re-register export if cleared; registry is process-global.
    // Tests only add temporary kinds with unique ids.
  });

  it("registers and retrieves a job kind", () => {
    const id = `test-kind-${Date.now()}`;
    const plugin: JobKindPlugin = {
      id,
      labelKey: "jobs.kinds.export",
      start: async () => {},
      cancel: async () => {},
    };
    registerJobKind(plugin);
    expect(getJobKind(id)?.id).toBe(id);
    expect(listJobKinds().some((p) => p.id === id)).toBe(true);
  });

  it("returns undefined for unknown kinds", () => {
    expect(getJobKind("definitely-missing-kind")).toBeUndefined();
  });
});
