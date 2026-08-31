import { call } from "./client";
import type { DiskCheck, WhisperModel } from "./types.gen";

export const whisperApi = {
  list: () => call<WhisperModel[]>("whisper_list_models"),
  diskCheck: (name: string) => call<DiskCheck>("whisper_disk_check", { name }),
  download: (name: string) => call<void>("whisper_download_model", { name }),
  cancelDownload: (name: string) => call<void>("whisper_cancel_download", { name }),
  remove: (name: string) => call<void>("whisper_delete_model", { name }),
  select: (name: string) => call<void>("whisper_select_model", { name }),
};
