import { useContext } from "react";
import { BackgroundJobsContext } from "./BackgroundJobsContext";

export function useBackgroundJobs() {
  const ctx = useContext(BackgroundJobsContext);
  if (!ctx) {
    throw new Error("useBackgroundJobs must be used within BackgroundJobsProvider");
  }
  return ctx;
}
