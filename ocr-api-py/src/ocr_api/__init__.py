#!/usr/bin/env python

from fastapi import FastAPI, UploadFile

from .handlers import Handler

app = FastAPI()


@app.get("/")
async def root():
    return {
        "available_handlers": list(Handler.available_handlers().keys()),
        "handler_template": "/ocr/{handler_name}",
    }


@app.post("/ocr/{handler_name}")
async def handle_ocr(handler_name: str, file: UploadFile):
    handler = Handler.available_handlers().get(handler_name)

    if not handler:
        return {
            "engine": handler_name,
            "error": f"No handler named {handler_name}",
        }

    try:
        result = handler.handle(file=file)

        return {
            "engine": handler_name,
            "data": result,
        }
    except Exception as e:
        return {
            "engine": handler_name,
            "error": str(e),
        }
