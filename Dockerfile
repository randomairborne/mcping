FROM alpine AS compressor

RUN apk add zstd brotli gzip

COPY /assets/ /assets/

RUN --mount=type=cache,target=/assets/ find /assets/ -type f -exec gzip -k9 '{}' \; -exec brotli -k9 '{}' \; -exec zstd -qk19 '{}' \;

FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev

RUN echo $PATH

RUN cargo version

FROM alpine AS compressor

RUN apk add zstd brotli gzip

COPY /assets/ /assets/

RUN --mount=type=cache,target=/assets/ find /assets/ -type f -exec gzip -k9 '{}' \; -exec brotli -k9 '{}' \; -exec zstd -qk19 '{}' \;

FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev

RUN echo $PATH

RUN cargo version

RUN \
    --mount=type=cache,target=/build/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --release && cp /build/target/release/mcping /build/mcping

FROM alpine

WORKDIR /

COPY --from=builder /build/mcping /usr/bin/mcping
COPY --from=compressor /assets/ /var/www/mcping/

EXPOSE 8080
ENV ASSET_DIR="/var/www/mcping/"

ENTRYPOINT "/usr/bin/mcping"


FROM alpine

WORKDIR /

COPY --from=builder /build/mcping /usr/bin/mcping
COPY --from=compressor /assets/ /var/www/mcping/

EXPOSE 8080
ENV ASSET_DIR="/var/www/mcping/"

ENTRYPOINT "/usr/bin/mcping"
