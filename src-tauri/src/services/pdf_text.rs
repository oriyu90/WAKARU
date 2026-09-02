//! Text-flowing PDF output for `build_document` and the OCR sandwich PDF (P12).
//!
//! Status: the Markdown/DOCX paths of `build_document` are complete; PDF text
//! flow with a CJK-capable face (`printpdf` + a system font via `fontdb`) is the
//! next slice. Until then this returns a recoverable error so `build_document`
//! can hand the reader the `.docx` / `.md` version instead of failing hard.

use crate::error::AppError;
use crate::services::doc_builder::DocRequest;

pub const AVAILABILITY_NOTE: &str =
    "PDF output is not in this build yet — save as .docx or .md and export to PDF from your editor.";

pub fn render_document(_req: &DocRequest) -> Result<Vec<u8>, AppError> {
    Err(AppError::new(
        "DOC_BUILD_PDF_UNAVAILABLE",
        "errors.doc.pdfUnavailable",
        AVAILABILITY_NOTE,
    ))
}
