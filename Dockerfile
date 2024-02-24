FROM ghcr.io/randomairborne/asset-squisher AS client-builder

COPY /assets/ /uncompressed-assets/

RUN asset-squisher /uncompressed-assets/ /assets/

FROM rust:alpine AS server-builder

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

COPY --from=server-builder /build/mcping /usr/bin/mcping
COPY --from=client-builder /assets/ /var/www/mcping/

EXPOSE 8080
ENV ASSET_DIR="/var/www/mcping/"

ENTRYPOINT "/usr/bin/mcping"

