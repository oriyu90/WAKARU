import { call } from "./client";
import i18n from "../i18n";
import type {
  Thread,
  GenerateStarted,
  GenerateVisualInput,
  DetailLevel,
  Scope,
  ImportToStudioInput,
  VisualPreview,
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
  ask: (input: { projectId: string; threadId: string; text: string; scope: Scope; locator?: unknown }) =>
    call<string>("illustrator_ask", { input, uiLang: lang() }),
  cancel: (streamId: string) => call<void>("illustrator_cancel", { streamId }),
  importToStudio: (input: ImportToStudioInput) =>
    call<string>("illustrator_import_to_studio", { input }),
  /** Explicit figure creation (plan §4.2). The explanation stream stays
   *  Markdown-only; this stores one typed visual for the current range. */
  generateVisual: (input: GenerateVisualInput) =>
    call<VisualPreview>("illustrator_generate_visual", { input }),
};
