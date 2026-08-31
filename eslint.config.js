import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

const browserGlobals = {
  window: "readonly",
  document: "readonly",
  navigator: "readonly",
  localStorage: "readonly",
  console: "readonly",
  setTimeout: "readonly",
  clearTimeout: "readonly",
  matchMedia: "readonly",
  KeyboardEvent: "readonly",
  MouseEvent: "readonly",
  HTMLElement: "readonly",
  HTMLButtonElement: "readonly",
  HTMLDialogElement: "readonly",
  HTMLInputElement: "readonly",
  HTMLDivElement: "readonly",
  SVGSVGElement: "readonly",
};

export default tseslint.config(
  { ignores: ["dist", "src-tauri", "src/ipc/types.gen.ts", "node_modules", "*.config.js", "*.config.ts"] },
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: browserGlobals,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
  {
    files: ["scripts/**/*.mjs"],
    languageOptions: {
      globals: { process: "readonly", console: "readonly", URL: "readonly" },
    },
  },
);
