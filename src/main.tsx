import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router";
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/base.css";
import { Providers } from "./app/providers";
import { router } from "./app/router";
import { inTauri } from "./ipc/client";
import { useUiStore, applyUiToDocument } from "./stores/ui";

// Mark the document when running inside the Tauri shell so the toolbar can
// reserve space for the macOS overlay title bar (traffic lights).
if (inTauri) document.documentElement.dataset.tauri = "true";

// Apply saved theme / scale / monochrome, and keep <html> in sync with the store.
applyUiToDocument(useUiStore.getState());
useUiStore.subscribe((s) => applyUiToDocument(s));

// Follow the OS theme live while the preference is "system".
window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", () => applyUiToDocument(useUiStore.getState()));

createRoot(document.getElementById("app")!).render(
  <StrictMode>
    <Providers>
      <RouterProvider router={router} />
    </Providers>
  </StrictMode>,
);
