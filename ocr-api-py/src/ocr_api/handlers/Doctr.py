import doctr.io
import doctr.models
from fastapi import UploadFile

from . import Handler

doctr_model = doctr.models.ocr_predictor(
    # det_arch="db_mobilenet_v3_large",
    # reco_arch="crnn_mobilenet_v3_large",
    assume_straight_pages=False,
    pretrained=True,
)


class Doctr(Handler):
    def handle(self, file: UploadFile):
        result = self.upload(
            file=file,
            after=lambda file_name: doctr_model(
                doctr.io.DocumentFile.from_images(file_name)
            ),
        )
        result = result.export()["pages"][0]

        dimensions = result["dimensions"]

        ocr_result = (
            {
                "text": " ".join(
                    (
                        word
                        for word in (
                            word["value"].strip()
                            for word in line["words"]
                            if word["objectness_score"] > 0.1
                        )
                        if word
                    )
                ).strip(),
                "box": {
                    "tl": self.coords(line["geometry"][0] * dimensions),
                    "tr": self.coords(line["geometry"][1] * dimensions),
                    "br": self.coords(line["geometry"][2] * dimensions),
                    "bl": self.coords(line["geometry"][3] * dimensions),
                },
                "confidence": float(line["objectness_score"]),
            }
            for block in result["blocks"]
            for line in block["lines"]
        )

        ocr_result = [
            line for line in ocr_result if line["text"] and line["confidence"] > 0.1
        ]

        return ocr_result
