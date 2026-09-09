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
if (inTauri) {
  document.documentElement.dataset.tauri = "true";
  // In fullscreen macOS hides the traffic lights, so the reserved left inset
  // is just dead space. Track it and let CSS drop the inset there (P5).
  void (async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const sync = async () => {
        try {
          document.documentElement.dataset.fullscreen = String(
            await win.isFullscreen(),
          );
        } catch {
          /* window gone */
        }
      };
      await sync();
      await win.onResized(sync);
    } catch {
      /* not in a Tauri window (tests, browser preview) */
    }
  })();
}

// Apply saved theme / scale / monochrome, and keep <html> in sync with the
// store — but only re-touch the DOM when a *display* field actually changed
// (the subscription also fires on sidebar toggles etc.).
applyUiToDocument(useUiStore.getState());
let lastUiSig = "";
useUiStore.subscribe((s) => {
  const sig = `${s.theme}|${s.scale}|${s.monochrome}|${s.readingFont}`;
  if (sig === lastUiSig) return;
  lastUiSig = sig;
  applyUiToDocument(s);
});

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
