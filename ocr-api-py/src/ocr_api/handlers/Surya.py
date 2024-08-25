import surya.model.detection.model
import surya.model.recognition.model
import surya.model.recognition.processor
import surya.ocr
from fastapi import UploadFile
from PIL import Image

from . import Handler

surya_det_processor, surya_det_model = (
    surya.model.detection.model.load_processor(),
    surya.model.detection.model.load_model(),
)
surya_rec_model, surya_rec_processor = (
    surya.model.recognition.model.load_model(),
    surya.model.recognition.processor.load_processor(),
)
# surya_rec_model.decoder.model = torch.compile(surya_rec_model.decoder.model)


class Surya(Handler):
    def handle(self, file: UploadFile):
        image = Image.open(file.file)
        langs = ["en", "hr"]
        predictions = surya.ocr.run_ocr(
            [image],
            [langs],
            surya_det_model,
            surya_det_processor,
            surya_rec_model,
            surya_rec_processor,
        )

        ocr_result = [
            {
                "text": line.text,
                "confidence": line.confidence,
                "box": self.box(line.polygon),
            }
            for pred in predictions
            for line in pred.text_lines
        ]

        return ocr_result
