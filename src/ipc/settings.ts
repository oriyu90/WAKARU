import { call } from "./client";
import type { Settings } from "./types.gen";

/** Deep-partial patch — the backend merges it over the stored settings. */
export type SettingsPatch = {
  [K in keyof Settings]?: Partial<Settings[K]>;
};

export const settingsApi = {
  get: () => call<Settings>("app_get_settings"),
  update: (patch: SettingsPatch) => call<Settings>("app_update_settings", { patch }),
};
