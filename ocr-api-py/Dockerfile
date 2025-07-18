FROM python:3.12-slim
RUN apt-get update && apt-get install libgl1 libglib2.0-0 libsm6 libxrender1 libxext6 -y
# Install uv - https://github.com/astral-sh/uv
COPY --from=ghcr.io/astral-sh/uv:0.3.3 /uv /bin/uv
ENV UV_SYSTEM_PYTHON=1
ENV UV_COMPILE_BYTECODE=1
RUN uv pip install --system fastapi[standard] uvicorn \
  && rm -rf ~/.cache/pip ~/.cache/uv \
  ;
WORKDIR /app
ENV VIRTUAL_ENV=/app/.venv
ENV PATH="/app/.venv/bin:$PATH"
# Install dependencies
ADD uv.lock pyproject.toml ./
RUN uv sync --frozen --no-install-project \
  && rm -rf ~/.cache/pip ~/.cache/uv \
  ;
# Preload model data
ADD preload_models.py .
RUN uv run --no-project preload_models.py \
  && rm -rf ~/.cache/pip ~/.cache/uv \
  ;
ADD . .
# Sync the project
RUN uv sync --frozen \
  && rm -rf ~/.cache/pip ~/.cache/uv \
  ;
EXPOSE 8000
CMD [ "uv", "run", "fastapi", "run", "./src/ocr_api/__init__.py" ]
