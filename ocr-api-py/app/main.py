#!/usr/bin/env python

import shutil
from tempfile import NamedTemporaryFile

import easyocr
from fastapi import FastAPI, UploadFile

reader = easyocr.Reader(["en", "hr"])
app = FastAPI()


def coords(c):
    return {
        "x": int(c[0]),
        "y": int(c[1]),
    }


@app.post("/ocr")
async def create_upload_file(file: UploadFile):
    if file.content_type not in ["image/jpeg", "image/png"]:
        return {
            "error": "Invalid file type. Only JPEG and PNG are supported.",
        }

    with NamedTemporaryFile() as temp:
        shutil.copyfileobj(file.file, temp)
        ocr_result = reader.readtext(temp.name)
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

    return {"data": ocr_result}
