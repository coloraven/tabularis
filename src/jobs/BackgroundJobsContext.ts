import { createContext } from "react";
import type { BackgroundJob, JobKindId } from "./types";

export interface BackgroundJobsContextType {
  jobs: BackgroundJob[];
  runningCount: number;
  enqueue: (
    kind: JobKindId,
    title: string,
    input: unknown,
    meta?: Record<string, unknown>,
  ) => string;
  cancel: (jobId: string) => Promise<void>;
  clearFinished: () => void;
  remove: (jobId: string) => void;
}

export const BackgroundJobsContext = createContext<
  BackgroundJobsContextType | undefined
>(undefined);
