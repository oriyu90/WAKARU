import { call, inTauri } from "./client";
import i18n from "../i18n";
import type {
  ImagesToPdfInput,
  SaveTextInput,
  WrittenFile,
} from "./types.gen";

export const fmApi = {
  pathExists: (path: string) => call<boolean>("fm_path_exists", { path }),
  imagePreview: (path: string) => call<string>("fm_image_preview", { path }),
  imagesToPdf: (input: ImagesToPdfInput) =>
    call<WrittenFile>("fm_images_to_pdf", { input }),
  saveText: (input: SaveTextInput) => call<WrittenFile>("fm_save_text", { input }),
  textToMarkdown: (text: string) =>
    call<string>("fm_text_to_markdown", { input: { text }, uiLang: i18n.language }),
};

export async function pickImages(): Promise<string[]> {
  if (!inTauri) return [];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: true,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff"] }],
  });
  if (!picked) return [];
  return Array.isArray(picked) ? picked : [picked];
}

export async function pickSaveDir(): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const dir = await open({ directory: true, multiple: false });
  return typeof dir === "string" ? dir : null;
}
