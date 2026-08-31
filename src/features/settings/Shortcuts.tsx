import { useTranslation } from "react-i18next";
import styles from "./About.module.css";

const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent);
const M = isMac ? "⌘" : "Ctrl+";

// docs/06 §3.2 — read-only in v1.0.
const KEYS: [string, string][] = [
  [`${M}B`, "shortcut.sidebar"],
  [`${M}K`, "shortcut.palette"],
  [`${M}F`, "shortcut.findInPreview"],
  [`${M}⇧F`, "shortcut.globalSearch"],
  [`${M}W`, "shortcut.closeTab"],
  [`${M}T`, "shortcut.newStudioTab"],
  [`${M}\\`, "shortcut.illustrator"],
  [`${M},`, "shortcut.settings"],
  ["← / →", "shortcut.pageNav"],
  ["Esc", "shortcut.dismiss"],
];

export function Shortcuts() {
  const { t } = useTranslation();
  return (
    <table className={styles.list} style={{ display: "table", width: "100%" }}>
      <tbody>
        {KEYS.map(([k, label]) => (
          <tr key={k}>
            <td style={{ padding: "0.25rem 1rem 0.25rem 0", whiteSpace: "nowrap" }}>
              <kbd>{k}</kbd>
            </td>
            <td style={{ color: "var(--color-muted)" }}>{t(label)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
