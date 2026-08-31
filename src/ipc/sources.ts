import { call, inTauri } from "./client";
import type { Source } from "./types.gen";

export const sourcesApi = {
  list: (projectId: string) => call<Source[]>("source_list", { projectId }),
  get: (projectId: string, sourceId: string) =>
    call<Source>("source_get", { projectId, sourceId }),
  addFiles: (projectId: string, paths: string[]) =>
    call<Source[]>("source_add_files", { input: { projectId, paths } }),
  addUrl: (projectId: string, url: string) =>
    call<Source>("source_add_url", { projectId, url }),
  reanalyze: (projectId: string, sourceId: string) =>
    call<void>("source_reanalyze", { projectId, sourceId }),
  delete: (projectId: string, sourceId: string) =>
    call<void>("source_delete", { projectId, sourceId }),
};

/** OS file picker for adding sources. Returns absolute paths (never shown to the
 * user directly — the frontend only holds ids after this). */
export async function pickSourceFiles(): Promise<string[]> {
  if (!inTauri) return [];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: true,
    directory: false,
    title: "Add sources",
  });
  if (!picked) return [];
  return Array.isArray(picked) ? picked : [picked];
}
