import { call } from "./client";
import type {
  ViewerTab,
  DocumentPayload,
  SourceDetail,
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
};
