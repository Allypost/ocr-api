# OCR APIs

[![Build Status](https://drone.allypost.net/api/badges/Allypost/ocr-api/status.svg)](https://drone.allypost.net/Allypost/ocr-api)

| Language | OCR implementations | Docker Image |
| --- | --- | --- |
| [Python](./ocr-api-py/) | [EasyOCR](https://github.com/JaidedAI/EasyOCR), [doctr](https://github.com/mindee/doctr), [surya](https://github.com/VikParuchuri/surya) | [![OCR API Py Image Size](https://img.shields.io/docker/image-size/allypost/ocr-api-py)](https://hub.docker.com/r/allypost/ocr-api-py) |
| [Rust](./ocr-api-rs/) | [ocrs](https://github.com/robertknight/ocrs), [tesseract](https://github.com/tesseract-ocr/tesseract) | [![OCR API Rust Image Size](https://img.shields.io/docker/image-size/allypost/ocr-api-rs)](https://hub.docker.com/r/allypost/ocr-api-rs) |

There is also an API Gateway available to unify the different OCR APIs:

| [OCR API](./ocr-api/) | [![OCR API Image Size](https://img.shields.io/docker/image-size/allypost/ocr-api)](https://hub.docker.com/r/allypost/ocr-api) |
| --- | --- |

--------------------

A collection of OCR APIs.
Multiple sub-projects are available in this repository, each hosting a different model.
