import { call } from "./client";
import type {
  AiProfile,
  AiProfileInput,
  RoleBindings,
  TestResult,
  Role,
} from "./types.gen";

export const aiApi = {
  listProfiles: () => call<AiProfile[]>("ai_list_profiles"),
  upsertProfile: (input: AiProfileInput) =>
    call<AiProfile>("ai_upsert_profile", { input }),
  deleteProfile: (id: string) => call<void>("ai_delete_profile", { id }),
  testProfile: (id: string) => call<TestResult>("ai_test_profile", { id }),
  getRoleBindings: () => call<RoleBindings>("ai_get_role_bindings"),
  setRoleBinding: (role: Role, profileId: string, model: string) =>
    call<void>("ai_set_role_binding", { role, profileId, model, params: null }),
  clearRoleBinding: (role: Role) => call<void>("ai_clear_role_binding", { role }),
  cancelRequest: (streamId: string) =>
    call<void>("ai_cancel_request", { streamId }),
  debugChat: (prompt: string) => call<string>("ai_debug_chat", { prompt }),
};

export const AI_PRESETS: { label: string; baseUrl: string; note?: string }[] = [
  { label: "LM Studio", baseUrl: "http://localhost:1234/v1", note: "No API key needed" },
  { label: "Ollama", baseUrl: "http://localhost:11434/v1", note: "No API key needed" },
  { label: "OpenAI", baseUrl: "https://api.openai.com/v1" },
  { label: "Custom", baseUrl: "" },
];
