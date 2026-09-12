import type { JobKindId, JobKindPlugin } from "./types";

const plugins = new Map<JobKindId, JobKindPlugin>();

export function registerJobKind(plugin: JobKindPlugin): void {
  plugins.set(plugin.id, plugin);
}

export function getJobKind(id: JobKindId): JobKindPlugin | undefined {
  return plugins.get(id);
}

export function listJobKinds(): JobKindPlugin[] {
  return Array.from(plugins.values());
}
