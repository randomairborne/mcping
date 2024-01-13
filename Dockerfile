FROM alpine AS compressor

RUN apk add zstd brotli gzip

COPY /assets/ /assets/

RUN find /assets/ -type f -exec gzip -k9 '{}' \; -exec brotli -k9 '{}' \; -exec zstd -qk19 '{}' \;

FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev

RUN cargo build --release

FROM alpine

WORKDIR /

COPY --from=builder /build/target/release/mcping /usr/bin/mcping
COPY --from=compressor /assets/ /var/www/mcping/

EXPOSE 8080
ENV ASSET_DIR="/var/www/mcping/"

ENTRYPOINT "/usr/bin/mcping"
