#!/bin/sh

SCRIPT_DIR="$(
    cd "$(dirname "$0")" >/dev/null 2>/dev/null &&
        pwd
)"

DETECTION_MODEL="https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten"
RECOGNITION_MODEL="https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten"

curl "$DETECTION_MODEL" -o "$SCRIPT_DIR/text-detection.rten"
curl "$RECOGNITION_MODEL" -o "$SCRIPT_DIR/text-recognition.rten"
