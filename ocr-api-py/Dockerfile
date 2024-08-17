FROM pytorch/pytorch:latest
WORKDIR /app
RUN pip install fastapi[standard] uvicorn
COPY requirements.txt ./
RUN pip install --no-cache-dir --upgrade -r requirements.txt
RUN python -c 'import easyocr; easyocr.Reader(["hr", "en"])'
COPY ./app ./
CMD [ "fastapi", "run", "./main.py" ]