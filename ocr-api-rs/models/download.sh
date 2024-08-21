#!/bin/sh

SCRIPT_DIR="$(
    cd "$(dirname "$0")" >/dev/null 2>/dev/null &&
        pwd
)"

curl "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten" -o "$SCRIPT_DIR/ocrs-text-detection.rten"
curl "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten" -o "$SCRIPT_DIR/ocrs-text-recognition.rten"
