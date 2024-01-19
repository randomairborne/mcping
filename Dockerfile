FROM alpine AS client-builder

RUN apk add zstd brotli pigz

COPY /assets/ /assets/

RUN find /assets/ -type f ! -name "*.png" -exec pigz -k9 '{}' \; -exec pigz -zk9 '{}' \; -exec brotli -k9 '{}' \; -exec zstd -qk19 '{}' \;

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

