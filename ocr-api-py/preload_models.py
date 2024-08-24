import doctr.models
import easyocr
import surya.model.detection.model
import surya.model.recognition.model
import surya.model.recognition.processor
import surya.ocr

easyocr.Reader(["hr", "en"])
doctr.models.ocr_predictor(assume_straight_pages=False, pretrained=True)
surya_det_processor, surya_det_model = (
    surya.model.detection.model.load_processor(),
    surya.model.detection.model.load_model(),
)
surya_rec_model, surya_rec_processor = (
    surya.model.recognition.model.load_model(),
    surya.model.recognition.processor.load_processor(),
)
