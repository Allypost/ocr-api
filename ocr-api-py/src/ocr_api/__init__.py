#!/usr/bin/env python

import shutil
from tempfile import NamedTemporaryFile
from typing import Callable

import easyocr
from fastapi import FastAPI, UploadFile
import doctr.io
import doctr.models

app = FastAPI()


def coords(c):
    return {
        "x": int(c[0]),
        "y": int(c[1]),
    }


@app.get("/")
async def root():
    return {
        "available_handlers": ["easyocr", "doctr"],
        "handler_template": "/ocr/{handler_name}",
    }


easyocr_reader = easyocr.Reader(["en", "hr"])


@app.post("/ocr/easyocr")
async def handle_easyocr_ocr(file: UploadFile):
    try:
        ocr_result = upload_file(
            file=file,
            after=lambda file_name: easyocr_reader.readtext(file_name),
        )
    except Exception as e:
        return {"engine": "easyocr", "error": str(e)}
    ocr_result = [
        {
            "text": text,
            "box": {
                "tl": coords(box[0]),
                "tr": coords(box[1]),
                "br": coords(box[2]),
                "bl": coords(box[3]),
            },
            "confidence": float(conf),
        }
        for (box, text, conf) in ocr_result
    ]

    return {"engine": "easyocr", "data": ocr_result}


doctr_model = doctr.models.ocr_predictor(
    # det_arch="db_mobilenet_v3_large",
    # reco_arch="crnn_mobilenet_v3_large",
    assume_straight_pages=False,
    pretrained=True,
)


@app.post("/ocr/doctr")
async def handle_doctr_ocr(file: UploadFile):
    try:
        result = upload_file(
            file=file,
            after=lambda file_name: doctr_model(
                doctr.io.DocumentFile.from_images(file_name)
            ),
        ).export()["pages"][0]

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
                    "tl": coords(line["geometry"][0] * dimensions),
                    "tr": coords(line["geometry"][1] * dimensions),
                    "br": coords(line["geometry"][2] * dimensions),
                    "bl": coords(line["geometry"][3] * dimensions),
                },
                "confidence": float(line["objectness_score"]),
            }
            for block in result["blocks"]
            for line in block["lines"]
        )
        ocr_result = [
            line for line in ocr_result if line["text"] and line["confidence"] > 0.1
        ]
    except Exception as e:
        return {"engine": "doctr", "error": str(e)}

    return {"engine": "doctr", "data": ocr_result}


def upload_file(file: UploadFile, after: Callable):
    with NamedTemporaryFile() as temp:
        shutil.copyfileobj(file.file, temp)
        return after(temp.name)
