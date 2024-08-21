use std::{path::Path, sync::Arc};

use convert_case::{Case, Casing};
use once_cell::sync::Lazy;
use serde::Serialize;

pub mod ocrs;

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
}

impl<T> From<T> for OcrTextItem
where
    T: ToString,
{
    fn from(text: T) -> Self {
        Self {
            text: text.to_string(),
            text_box: None,
        }
    }
}

impl From<OcrTextItem> for serde_json::Value {
    fn from(item: OcrTextItem) -> Self {
        serde_json::to_value(item).expect("Failed to convert OcrTextItem to JSON value")
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CoordBox {
    tl: Point,
    tr: Point,
    br: Point,
    bl: Point,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Point {
    x: i32,
    y: i32,
}

fn handlers() -> Vec<Arc<dyn OcrHandler>> {
    vec![Arc::new(ocrs::Ocrs)]
}
