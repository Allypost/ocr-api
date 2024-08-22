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


@app.get("/")
async def root():
    return {
        "available_handlers": ["easyocr"],
        "handler_template": "/ocr/{handler_name}",
    }


@app.post("/ocr/easyocr")
async def create_upload_file(file: UploadFile):
    with NamedTemporaryFile() as temp:
        shutil.copyfileobj(file.file, temp)
        try:
            ocr_result = reader.readtext(temp.name)
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
