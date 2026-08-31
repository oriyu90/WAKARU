import { call, inTauri } from "./client";
import type { ExportInput, ExportResult, ExportEstimate, Project } from "./types.gen";

export const exportApi = {
  estimates: (projectIds: string[]) =>
    call<ExportEstimate[]>("project_export_estimates", { projectIds }),
  export: (input: ExportInput) => call<ExportResult>("project_export", { input }),
  import: (zipPath: string) => call<Project>("project_import", { zipPath }),
};

export async function pickZip(): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const f = await open({
    multiple: false,
    filters: [{ name: "WAKARU project", extensions: ["zip"] }],
  });
  return typeof f === "string" ? f : null;
}
