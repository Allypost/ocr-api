use std::{path::Path, sync::Arc};

use convert_case::{Case, Casing};
use once_cell::sync::Lazy;
use serde::Serialize;

pub mod ocrs;
pub mod tesseract;

pub static HANDLERS: Lazy<Vec<Arc<dyn OcrHandler>>> = Lazy::new(handlers);

#[typetag::serde(tag = "$handler")]
pub trait OcrHandler: std::fmt::Debug + Send + Sync {
    fn name(&self) -> String {
        self.typetag_name().to_case(Case::Kebab)
    }

    fn ocr(&self, path: &Path, mime_type: Option<&str>) -> anyhow::Result<OcrResult>;
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OcrResult {
    pub matches: Vec<OcrTextItem>,
}
impl<T> From<Vec<T>> for OcrResult
where
    T: Into<OcrTextItem>,
{
    fn from(matches: Vec<T>) -> Self {
        Self {
            matches: matches.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OcrTextItem {
    text: String,
    #[serde(rename = "box", skip_serializing_if = "Option::is_none")]
    text_box: Option<CoordBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f32>,
}

impl OcrTextItem {
    pub fn with_text_box<T>(mut self, text_box: Option<T>) -> Self
    where
        T: Into<CoordBox>,
    {
        self.text_box = text_box.map(Into::into);
        self
    }

    pub const fn with_confidence(mut self, confidence: Option<f32>) -> Self {
        self.confidence = confidence;
        self
    }
}

impl<T> From<T> for OcrTextItem
where
    T: ToString,
{
    fn from(text: T) -> Self {
        Self {
            text: text.to_string(),
            text_box: None,
            confidence: None,
        }
    }
}

impl From<OcrTextItem> for serde_json::Value {
    fn from(item: OcrTextItem) -> Self {
        serde_json::to_value(item).expect("Failed to convert OcrTextItem to JSON value")
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CoordBox {
    tl: Point,
    tr: Point,
    br: Point,
    bl: Point,
}

impl PartialOrd for CoordBox {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CoordBox {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let top_max_self = self.tl.y.max(self.tr.y);
        let top_max_other = other.tl.y.max(other.tr.y);
        let ord = top_max_self.cmp(&top_max_other);

        if ord != std::cmp::Ordering::Equal {
            return ord;
        }

        let left_min_self = self.tl.x.min(self.bl.x);
        let left_min_other = other.tl.x.min(other.bl.x);

        left_min_self.cmp(&left_min_other)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Point {
    x: i32,
    y: i32,
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ord = self.y.cmp(&other.y);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
        self.x.cmp(&other.x)
    }
}

fn handlers() -> Vec<Arc<dyn OcrHandler>> {
    vec![Arc::new(ocrs::Ocrs), Arc::new(tesseract::Tesseract)]
}
