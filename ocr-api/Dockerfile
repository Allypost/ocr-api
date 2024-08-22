ARG RUST_VERSION='1.80'
ARG RUST_TARGET='x86_64-unknown-linux-musl'
ARG BINARY_NAME='ocr-api'

ARG APP_FEATURES=''

ARG RUN_USERNAME='app'
ARG RUN_USER_ID='1000'
ARG RUN_GROUP_ID='1000'


##########
# Step 0 #
##########
##
## Setup base image with cargo-chef
##
FROM rust:${RUST_VERSION} AS chef
# `curl` and `bash` are needed for cargo-binstall
# `musl-tools` and `musl-dev` are needed to build app with musl target
RUN apt-get update && apt-get install -y \
    curl \
    bash \
    musl-tools \
    musl-dev \
    jq \
    && rm -rf /var/lib/apt/lists/*
# Install cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf 'https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh' | bash
# Install cargo-chef
RUN cargo binstall cargo-chef --locked --no-confirm
# Add proper target and compile flags
ARG RUST_TARGET
RUN rustup target add "${RUST_TARGET}"
ENV RUSTFLAGS='-C target-feature=+crt-static'
WORKDIR /app


##########
# Step 1 #
##########
##
## Generate a build plan for rust dependencies
##
FROM chef AS planner
COPY . .
# Generate "lockfile" aka dependency dump
RUN cargo chef prepare \
    --recipe-path recipe.json


##########
# Step 2 #
##########
##
## Build app with the cached dependencies
##
FROM chef AS builder
# Install upx - https://upx.github.io/
RUN cd "$(mktemp --directory)" && \
    curl -sL "$(\
    curl -sL https://api.github.com/repos/upx/upx/releases \
    | jq -r '.[0].assets | .[] | select(.name | test("amd64_linux")) | .browser_download_url' \
    | head -n1\
    )" | tar xvJ  && \
    cd * && \
    mv upx /usr/bin && \
    cd .. && \
    rm -rf "$(pwd)" && \
    echo "Installed upx"
COPY --from=planner /app/recipe.json .
# Build dependencies
ARG RUST_TARGET
ARG APP_FEATURES
ARG BINARY_NAME
RUN cargo chef cook \
    --release \
    --target "${RUST_TARGET}" \
    --features "${APP_FEATURES}" \
    --package "${BINARY_NAME}" \
    --recipe-path recipe.json
ARG RUST_TARGET
RUN rustup target add "${RUST_TARGET}"
# Copy rest of files and compile
# only the remaining app code
COPY . .
ARG RUST_TARGET
ARG APP_FEATURES
ARG BINARY_NAME
RUN cargo build \
    --release \
    --target "${RUST_TARGET}" \
    --features "${APP_FEATURES}" \
    --package "${BINARY_NAME}"
RUN upx --best --lzma "/app/target/${RUST_TARGET}/release/${BINARY_NAME}"


##########
# Step 3 #
##########
##
## Run the app in a configured environment
##
FROM scratch AS runner
ENV PORT=8000
ENV HOST=0.0.0.0
LABEL maintainer="Josip Igrec <me@allypost.net>"
LABEL org.opencontainers.image.title="OCR API"
LABEL org.opencontainers.image.description="A meta-api frontend for OCR APIs"
LABEL org.opencontainers.image.source="https://github.com/Allypost/ocr-api"
LABEL org.opencontainers.image.licenses="MPL-2.0"
LABEL org.opencontainers.image.authors="Josip Igrec <me@allypost.net>"
ARG RUST_TARGET
ARG BINARY_NAME
COPY --from=builder "/app/target/${RUST_TARGET}/release/${BINARY_NAME}" /exe
EXPOSE 8000
ENTRYPOINT [ "/exe" ]