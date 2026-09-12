import { registerExportJobKind } from "./kinds/exportJob";

let registered = false;

/** Idempotent registration of built-in job kinds (export today). */
export function ensureBuiltinJobKinds(): void {
  if (registered) return;
  registerExportJobKind();
  registered = true;
}
