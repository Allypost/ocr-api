use std::{cmp::Ord, collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use tracing::trace;

use super::{OcrHandler, OcrResult};
use crate::ocr::handlers::{CoordBox, OcrTextItem, Point};

#[derive(Debug, Serialize, Deserialize)]
pub struct Tesseract;

#[typetag::serde]
impl OcrHandler for Tesseract {
    fn ocr(&self, path: &Path, _mime_type: Option<&str>) -> anyhow::Result<OcrResult> {
        trace!(?path, "OCR with Tesseract");

        let img = rusty_tesseract::Image::from_path(path)?;
        let args = rusty_tesseract::Args::default();
        let res = rusty_tesseract::image_to_data(&img, &args)?;

        let items = {
            let mut blocks = HashMap::new();

            for data in res.data.iter().filter(|x| x.conf > 0_f32) {
                let block = blocks
                    .entry((data.block_num, data.line_num))
                    .or_insert_with(Vec::new);
                block.push(data);
            }

            let mut sorted_keys = blocks.keys().collect::<Vec<_>>();
            sorted_keys.sort();

            sorted_keys
                .into_iter()
                .filter_map(|key| blocks.get(key))
                .filter_map(|xs| {
                    let text = xs
                        .iter()
                        .map(|x| x.text.trim())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let text = text.trim();

                    if text.is_empty() {
                        return None;
                    }

                    #[allow(clippy::cast_precision_loss)]
                    let avg_conf =
                        xs.iter().map(|x| x.conf).sum::<f32>() / xs.len() as f32 / 100_f32;

                    let min_top = xs.iter().map(|x| x.top).min_by(Ord::cmp).unwrap_or(0);
                    let max_width = xs.iter().map(|x| x.width).max_by(Ord::cmp).unwrap_or(0);
                    let max_height = xs.iter().map(|x| x.height).max_by(Ord::cmp).unwrap_or(0);
                    let min_left = xs.iter().map(|x| x.left).min_by(Ord::cmp).unwrap_or(0);

                    let text_box = if max_width > 0 && max_height > 0 {
                        Some(CoordBox {
                            tl: Point {
                                x: min_left,
                                y: min_top,
                            },
                            tr: Point {
                                x: min_left + max_width,
                                y: min_top,
                            },
                            bl: Point {
                                x: min_left,
                                y: min_top + max_height,
                            },
                            br: Point {
                                x: min_left + max_width,
                                y: min_top + max_height,
                            },
                        })
                    } else {
                        None
                    };

                    Some(
                        OcrTextItem::from(text)
                            .with_text_box(text_box)
                            .with_confidence(Some(avg_conf)),
                    )
                })
                .collect::<Vec<_>>()
        };

        Ok(items.into())
    }
}
