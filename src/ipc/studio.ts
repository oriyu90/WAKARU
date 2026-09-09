import { call } from "./client";
import i18n from "../i18n";
import type {
  Artifact,
  Source,
  StudioSendResult,
  StudioTab,
} from "./types.gen";

const lang = () => i18n.language;

export const studioApi = {
  listTabs: (projectId: string) =>
    call<StudioTab[]>("studio_list_tabs", { projectId }),
  getTab: (projectId: string, tabId: string) =>
    call<StudioTab>("studio_get_tab", { projectId, tabId }),
  createTab: (projectId: string, title?: string) =>
    call<StudioTab>("studio_create_tab", { projectId, title: title ?? null }),
  renameTab: (projectId: string, tabId: string, title: string) =>
    call<void>("studio_rename_tab", { projectId, tabId, title }),
  closeTab: (projectId: string, tabId: string) =>
    call<void>("studio_close_tab", { projectId, tabId }),
  reorderTabs: (projectId: string, orderedIds: string[]) =>
    call<void>("studio_reorder_tabs", { projectId, orderedIds }),
  send: (projectId: string, tabId: string, text: string, scope: string) =>
    call<StudioSendResult>("studio_send", {
      input: { projectId, tabId, text, scope },
      uiLang: lang(),
    }),
  cancel: (tabId: string) => call<void>("studio_cancel", { tabId }),
  resolveTool: (projectId: string, tabId: string, approved: boolean) =>
    call<StudioSendResult>("studio_resolve_tool", {
      projectId,
      tabId,
      approved,
      uiLang: lang(),
    }),
  listArtifacts: (projectId: string) =>
    call<Artifact[]>("studio_list_artifacts", { projectId }),
  importArtifact: (projectId: string, artifactId: string) =>
    call<Source>("studio_import_artifact_as_source", { projectId, artifactId }),
  downloadArtifact: (projectId: string, artifactId: string, destDir: string) =>
    call<string>("studio_download_artifact", { projectId, artifactId, destDir }),
};
