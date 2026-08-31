//! Locator = a citation position, tagged JSON (docs/03 §5). `to_key()` is the
//! one place a stable string key is derived — the front-end never builds keys.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Locator {
    Page {
        page: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bbox: Option<[f64; 4]>,
    },
    Time {
        #[serde(rename = "tStart")]
        t_start: f64,
        #[serde(rename = "tEnd")]
        t_end: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
    },
    Region {
        bbox: [f64; 4],
    },
    Cell {
        sheet: String,
        range: String,
    },
    Line {
        start: u32,
        end: u32,
    },
    Path {
        pointer: String,
    },
    Anchor {
        selector: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        char_start: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        char_end: Option<u32>,
    },
    Whole,
}

impl Locator {
    /// Stable string key (docs/03 §5.2). Same *meaning* -> same key.
    /// Time is expected to already be snapped to a segment boundary by the
    /// caller; it is formatted to 3 fixed decimals here.
    pub fn to_key(&self) -> String {
        match self {
            Locator::Page { page, .. } => format!("page:{page}"),
            Locator::Time { t_start, t_end, .. } => {
                format!("time:{:08.3}-{:08.3}", t_start, t_end)
            }
            Locator::Region { bbox } => format!(
                "region:{:.3}-{:.3}-{:.3}-{:.3}",
                bbox[0], bbox[1], bbox[2], bbox[3]
            ),
            Locator::Cell { sheet, range } => format!("cell:{sheet}!{range}"),
            Locator::Line { start, end } => format!("line:{start}-{end}"),
            Locator::Path { pointer } => format!("path:{pointer}"),
            Locator::Anchor { selector, .. } => format!("anchor:{selector}"),
            Locator::Whole => "whole".to_string(),
        }
    }

    /// Parse from the loose JSON the front-end sends; unknown -> `Whole`.
    pub fn from_value(v: &serde_json::Value) -> Self {
        serde_json::from_value(v.clone()).unwrap_or(Locator::Whole)
    }

    pub fn key_from_value(v: &serde_json::Value) -> String {
        Self::from_value(v).to_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn page_key() {
        assert_eq!(
            Locator::Page {
                page: 12,
                bbox: None
            }
            .to_key(),
            "page:12"
        );
        // bbox does not change the key (it points at the same page).
        assert_eq!(
            Locator::Page {
                page: 12,
                bbox: Some([0.1, 0.2, 0.3, 0.4])
            }
            .to_key(),
            "page:12"
        );
    }

    #[test]
    fn time_key_is_zero_padded_to_3_decimals() {
        assert_eq!(
            Locator::Time {
                t_start: 90.0,
                t_end: 104.8,
                speaker: None
            }
            .to_key(),
            "time:0090.000-0104.800"
        );
    }

    #[test]
    fn same_meaning_same_key() {
        let a = Locator::from_value(&json!({ "t": "page", "page": 3 }));
        let b: Locator = serde_json::from_str(r#"{"t":"page","page":3,"bbox":[0,0,1,1]}"#).unwrap();
        assert_eq!(a.to_key(), b.to_key());
    }

    #[test]
    fn region_rounds_to_3dp() {
        assert_eq!(
            Locator::Region {
                bbox: [0.1000001, 0.2, 0.5, 0.639999]
            }
            .to_key(),
            "region:0.100-0.200-0.500-0.640"
        );
    }

    #[test]
    fn cell_and_line_and_path_and_anchor() {
        assert_eq!(
            Locator::Cell {
                sheet: "Q1".into(),
                range: "A1:D20".into()
            }
            .to_key(),
            "cell:Q1!A1:D20"
        );
        assert_eq!(
            Locator::Line {
                start: 120,
                end: 160
            }
            .to_key(),
            "line:120-160"
        );
        assert_eq!(
            Locator::Path {
                pointer: "/items/3/title".into()
            }
            .to_key(),
            "path:/items/3/title"
        );
        assert_eq!(
            Locator::Anchor {
                selector: "#sec-2".into(),
                char_start: None,
                char_end: None
            }
            .to_key(),
            "anchor:#sec-2"
        );
    }

    #[test]
    fn unknown_json_is_whole() {
        assert_eq!(
            Locator::from_value(&json!({ "weird": 1 })).to_key(),
            "whole"
        );
        assert_eq!(Locator::from_value(&json!({})).to_key(), "whole");
    }
}
