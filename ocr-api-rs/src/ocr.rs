use std::path::Path;

use image::{ImageFormat, ImageReader};
use ocrs::{DecodeMethod, DimOrder, ImageSource, OcrEngine, OcrEngineParams, TextItem};
use once_cell::sync::Lazy;
use rten::Model;
use rten_imageproc::RotatedRect;
use rten_tensor::{prelude::*, NdTensor};
use serde::Serialize;
use serde_json::json;
use tracing::{debug, trace};

static DETECTION_MODEL_DATA: &[u8] = include_bytes!("../models/text-detection.rten");
static RECOGNITION_MODEL_DATA: &[u8] = include_bytes!("../models/text-recognition.rten");

pub static OCR_ENGINE: Lazy<OcrEngine> = Lazy::new(|| {
    debug!("Loading OCR models");
    trace!("Loading detection model");
    let detection_model =
        Model::load(DETECTION_MODEL_DATA.to_vec()).expect("Failed to load detection model!");
    trace!("Loading recognition model");
    let recognition_model =
        Model::load(RECOGNITION_MODEL_DATA.to_vec()).expect("Failed to load recognition model!");
    debug!("Loaded OCR models");

    debug!("Creating OCR engine");
    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        debug: false,
        decode_method: DecodeMethod::Greedy,
    })
    .expect("Failed to create OCR engine!");
    debug!("Created OCR engine");

    engine
});

#[tracing::instrument]
pub fn ocr_image(path: &Path, mime_type: Option<&str>) -> anyhow::Result<Vec<serde_json::Value>> {
    trace!("Reading image from path");
    let img = {
        let mut img = ImageReader::new(std::io::BufReader::new(std::fs::File::open(path)?));

        if let Some(mime_type) = mime_type {
            trace!(?mime_type, "Setting image format from MIME type");
            let format = ImageFormat::from_mime_type(mime_type).ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to determine image format from MIME type: {}",
                    mime_type
                )
            })?;

            img.set_format(format);
        } else {
            trace!("Guessing image format");
            img = img.with_guessed_format()?;
            trace!(format = ?img.format(), "Guessed image format");
        }

        img
    };

    trace!("Decoding image into tensor");
    // Read image into HWC tensor.
    let color_img: NdTensor<u8, 3> = img.decode().map(|image| {
        let image = image.into_rgb8();
        let (width, height) = image.dimensions();
        let in_chans = 3;

        NdTensor::from_data(
            [height as usize, width as usize, in_chans],
            image.into_vec(),
        )
    })?;

    trace!("Creating image source from tensor");
    let color_img_source = ImageSource::from_tensor(color_img.view(), DimOrder::Hwc)?;

    let engine = &OCR_ENGINE;

    debug!("Running OCR engine");
    trace!("Preparing input for OCR engine");
    let ocr_input = engine.prepare_input(color_img_source)?;
    trace!("Prepared input");
    trace!("Detecting words in image");
    let word_rects = engine.detect_words(&ocr_input)?;
    trace!(?word_rects, "Detected words");
    trace!("Finding text lines in image");
    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);
    trace!(?line_rects, "Found text lines");
    trace!("Recognizing text in lines");
    let line_texts = engine.recognize_text(&ocr_input, &line_rects)?;
    trace!("Recognized text");
    debug!("Finished running OCR engine");

    let line_items = line_texts
        .iter()
        .filter_map(|line| line.as_ref())
        .map(|line| {
            json!({
                "text": line.to_string(),
                "box": CoordBox::from(&line.rotated_rect()),
            })
        })
        .collect::<Vec<_>>();

    Ok(line_items)
}

#[derive(Debug, Serialize)]
struct CoordBox {
    tl: Point,
    tr: Point,
    br: Point,
    bl: Point,
}
impl From<&RotatedRect> for CoordBox {
    fn from(rr: &RotatedRect) -> Self {
        let corners = rr.corners();
        Self {
            tl: corners[0].into(),
            tr: corners[1].into(),
            br: corners[2].into(),
            bl: corners[3].into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct Point {
    x: i32,
    y: i32,
}
impl From<rten_imageproc::Point<f32>> for Point {
    fn from(point: rten_imageproc::Point<f32>) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self {
            x: point.x.round() as i32,
            y: point.y.round() as i32,
        }
    }
}
