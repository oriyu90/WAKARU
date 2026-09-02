import { call } from "./client";
import type { OcrPage } from "./types.gen";

export const ocrApi = {
  /** OCR one rasterised PDF page. `pngBase64` has no `data:` prefix. */
  page: (input: {
    projectId: string;
    sourceId: string;
    page: number;
    total: number;
    pngBase64: string;
  }) => call<OcrPage>("ocr_page", { input }),

  /** Mark OCR complete; returns the `searchable.pdf` asset path if one was built. */
  finalize: (input: { projectId: string; sourceId: string; allOk: boolean }) =>
    call<string | null>("ocr_finalize", { input }),
};
