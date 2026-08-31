import { call } from "./client";
import i18n from "../i18n";
import type {
  Thread,
  GenerateStarted,
  DetailLevel,
  Scope,
  ImportToStudioInput,
} from "./types.gen";

const lang = () => i18n.language;

export const illustratorApi = {
  getOrCreateThread: (projectId: string, sourceId: string, locator: unknown) =>
    call<Thread>("illustrator_get_or_create_thread", { projectId, sourceId, locator }),
  generate: (input: {
    projectId: string;
    sourceId: string;
    locator: unknown;
    level: DetailLevel;
    force?: boolean;
  }) =>
    call<GenerateStarted>("illustrator_generate", {
      input: { ...input, force: input.force ?? false },
      uiLang: lang(),
    }),
  ask: (input: { projectId: string; threadId: string; text: string; scope: Scope }) =>
    call<string>("illustrator_ask", { input, uiLang: lang() }),
  cancel: (streamId: string) => call<void>("illustrator_cancel", { streamId }),
  importToStudio: (input: ImportToStudioInput) =>
    call<string>("illustrator_import_to_studio", { input }),
};
