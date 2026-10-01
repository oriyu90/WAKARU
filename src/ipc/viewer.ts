import { call } from "./client";
import type {
  ViewerTab,
  DocumentPayload,
  SourceDetail,
  TextWindow,
  WebsiteManifest,
} from "./types.gen";

export const viewerApi = {
  getTabs: (projectId: string) =>
    call<ViewerTab[]>("viewer_get_tabs", { projectId }),
  openTab: (projectId: string, sourceId: string, locator?: unknown) =>
    call<ViewerTab>("viewer_open_tab", {
      input: { projectId, sourceId, locator: locator ?? null },
    }),
  closeTab: (projectId: string, tabId: string) =>
    call<void>("viewer_close_tab", { projectId, tabId }),
  updateLocator: (projectId: string, tabId: string, locator: unknown) =>
    call<void>("viewer_update_locator", { projectId, tabId, locator }),
  pinTab: (projectId: string, tabId: string, pinned: boolean) =>
    call<void>("viewer_pin_tab", { projectId, tabId, pinned }),
  reorderTabs: (projectId: string, tabIds: string[]) =>
    call<void>("viewer_reorder_tabs", { projectId, tabIds }),
};

export const documentApi = {
  get: (projectId: string, sourceId: string, ordinal: number) =>
    call<DocumentPayload>("source_get_document", { projectId, sourceId, ordinal }),
  detail: (projectId: string, sourceId: string) =>
    call<SourceDetail>("source_detail", { projectId, sourceId }),
  assetUrl: (projectId: string, sourceId: string, relPath: string) =>
    call<string>("source_asset_url", { projectId, sourceId, relPath }),
  websiteManifest: (projectId: string, sourceId: string) =>
    call<WebsiteManifest>("website_manifest", { projectId, sourceId }),
  /** Bounded UTF-8 window for huge text (plan §3.2). Offsets may exceed
   * 2^53 only for absurd files; the backend u64 arrives as bigint. */
  readWindow: async (
    projectId: string,
    sourceId: string,
    offset: number,
    limit: number,
  ): Promise<{ sourceId: string; offset: number; totalBytes: number; text: string; nextOffset: number | null; isTruncated: boolean; startLine: number }> => {
    const w = await call<TextWindow>("source_read_window", {
      projectId,
      sourceId,
      offset,
      limit,
    });
    return {
      sourceId: w.sourceId,
      offset: Number(w.offset),
      totalBytes: Number(w.totalBytes),
      text: w.text,
      nextOffset: w.nextOffset == null ? null : Number(w.nextOffset),
      isTruncated: w.isTruncated,
      startLine: Number(w.startLine),
    };
  },
};
