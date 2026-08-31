import { createContext } from "react";

export type ToastTone = "error" | "info" | "success";

export type Toast = {
  id: string;
  tone: ToastTone;
  message: string;
  /** Optional single action (e.g. Undo). */
  action?: { label: string; onClick: () => void };
};

export type ToastApi = {
  push: (t: Omit<Toast, "id">) => string;
  dismiss: (id: string) => void;
};

export const ToastContext = createContext<ToastApi | null>(null);
