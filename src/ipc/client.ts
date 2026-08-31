import { invoke } from "@tauri-apps/api/core";
import type { AppError } from "./types.gen";

/** Thrown for any `Result::Err(AppError)` returned by a Rust command. */
export class IpcError extends Error {
  readonly code: string;
  readonly i18nKey: string;
  readonly details: unknown;
  readonly retriable: boolean;

  constructor(e: AppError) {
    super(e.message);
    this.name = "IpcError";
    this.code = e.code;
    this.i18nKey = e.i18nKey;
    this.details = e.details;
    this.retriable = e.retriable;
  }
}

function isAppError(x: unknown): x is AppError {
  return (
    typeof x === "object" &&
    x !== null &&
    "code" in x &&
    "i18nKey" in x &&
    "message" in x
  );
}

/** Call a Rust `#[tauri::command]`. Rejects with `IpcError` on `AppError`. */
export async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    if (isAppError(raw)) throw new IpcError(raw);
    throw raw;
  }
}

/** True inside the Tauri shell; false in a plain browser (Vitest, `vite` alone). */
export const inTauri = "__TAURI_INTERNALS__" in globalThis;
