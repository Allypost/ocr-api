import doctr.models
import easyocr

easyocr.Reader(["hr", "en"])
doctr.models.ocr_predictor(assume_straight_pages=False, pretrained=True)
