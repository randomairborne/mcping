FROM rust:alpine AS server-builder

RUN apk add musl-dev

WORKDIR /build

COPY . .

RUN cargo build --release

FROM ghcr.io/randomairborne/asset-squisher:latest AS client-builder

COPY /assets/ /uncompressed-assets/

RUN asset-squisher /uncompressed-assets/ /assets/

FROM scratch

WORKDIR /

COPY --from=server-builder /build/target/release/mcping /usr/bin/mcping
COPY --from=server-builder /build/target/release/mcping-healthcheck /usr/bin/mcping-healthcheck
COPY --from=client-builder /assets/ /var/www/mcping/

ENV ASSET_DIR="/var/www/mcping/"

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD ["/usr/bin/mcping-healthcheck", "http://127.0.0.1:8080"]
ENTRYPOINT ["/usr/bin/mcping"]
