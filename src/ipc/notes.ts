import { call } from "./client";
import type {
  Note,
  NoteCreate,
  NoteUpdate,
  VisualCreate,
  VisualPreview,
} from "./types.gen";

export const notesApi = {
  list: (projectId: string, sourceId: string) =>
    call<Note[]>("notes_list", { projectId, sourceId }),
  create: (input: NoteCreate) => call<Note>("notes_create", { input }),
  update: (input: NoteUpdate) => call<Note>("notes_update", { input }),
  remove: (projectId: string, noteId: string) =>
    call<Note>("notes_delete", { projectId, noteId }),
  restore: (projectId: string, noteId: string) =>
    call<Note>("notes_restore", { projectId, noteId }),
};

export const visualsApi = {
  create: (input: VisualCreate) => call<VisualPreview>("visual_create", { input }),
  get: (projectId: string, visualId: string) =>
    call<VisualPreview>("visual_get", { projectId, visualId }),
  listForMessage: (projectId: string, messageId: string) =>
    call<VisualPreview[]>("visual_list_for_message", { projectId, messageId }),
};
