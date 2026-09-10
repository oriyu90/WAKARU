import { call } from "./client";
import type {
  AiProfile,
  AiProfileInput,
  RoleBindings,
  TestResult,
  Role,
  ApiProtocol,
} from "./types.gen";

export const aiApi = {
  listProfiles: () => call<AiProfile[]>("ai_list_profiles"),
  upsertProfile: (input: AiProfileInput) =>
    call<AiProfile>("ai_upsert_profile", { input }),
  deleteProfile: (id: string) => call<void>("ai_delete_profile", { id }),
  testProfile: (id: string) => call<TestResult>("ai_test_profile", { id }),
  listModels: (profileId: string) =>
    call<string[]>("ai_list_models", { profileId }),
  getRoleBindings: () => call<RoleBindings>("ai_get_role_bindings"),
  setRoleBinding: (role: Role, profileId: string, model: string) =>
    call<void>("ai_set_role_binding", { role, profileId, model, params: null }),
  clearRoleBinding: (role: Role) => call<void>("ai_clear_role_binding", { role }),
  cancelRequest: (streamId: string) =>
    call<void>("ai_cancel_request", { streamId }),
  debugChat: (prompt: string) => call<string>("ai_debug_chat", { prompt }),
};

export const AI_PRESETS: { label: string; baseUrl: string; protocol: ApiProtocol }[] = [
  { label: "LM Studio", baseUrl: "http://localhost:1234/v1", protocol: "openai" },
  { label: "MLXBar", baseUrl: "http://127.0.0.1:11435/v1", protocol: "openai" },
  { label: "Ollama", baseUrl: "http://localhost:11434/v1", protocol: "openai" },
  { label: "OpenAI", baseUrl: "https://api.openai.com/v1", protocol: "openai" },
  { label: "Anthropic", baseUrl: "https://api.anthropic.com/v1", protocol: "anthropic" },
  { label: "Custom (OpenAI-compatible)", baseUrl: "", protocol: "openai" },
  { label: "Custom (Anthropic-compatible)", baseUrl: "", protocol: "anthropic" },
];
