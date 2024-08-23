FROM pytorch/pytorch:latest
WORKDIR /app
RUN pip install fastapi[standard] uvicorn
RUN pip install 'python-doctr[torch]'
COPY requirements.txt ./
RUN pip install --no-cache-dir --upgrade -r requirements.txt
RUN python -c 'import easyocr; easyocr.Reader(["hr", "en"])'
RUN python -c 'import doctr.models; doctr.models.ocr_predictor(assume_straight_pages=False, pretrained=True)'
COPY ./src ./
EXPOSE 8000
CMD [ "fastapi", "run", "./ocr_api/__init__.py" ]
