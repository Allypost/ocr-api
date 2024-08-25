import easyocr
from fastapi import UploadFile

from . import Handler

easyocr_reader = easyocr.Reader(["en", "hr"])


class EasyOcr(Handler):
    def handle(self, file: UploadFile):
        ocr_result = self.upload(
            file=file, after=lambda file_name: easyocr_reader.readtext(file_name)
        )

        ocr_result = [
            {
                "text": text,
                "box": self.box(box),
                "confidence": float(conf),
            }
            for (box, text, conf) in ocr_result
        ]

        return ocr_result
