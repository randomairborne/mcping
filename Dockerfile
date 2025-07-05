FROM ghcr.io/randomairborne/asset-squisher:latest AS client-builder

COPY /assets/ /uncompressed-assets/

RUN asset-squisher /uncompressed-assets/ /assets/

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/mcping /usr/local/bin
COPY --from=server-builder /app/target/release/mcping-healthcheck /usr/local/bin/mcping-healthcheck
COPY --from=client-builder /assets/ /var/www/mcping/

ENV ASSET_DIR="/var/www/mcping/"
HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD ["/usr/bin/mcping-healthcheck", "http://127.0.0.1:8080"]
ENTRYPOINT ["/usr/local/bin/mcping"]