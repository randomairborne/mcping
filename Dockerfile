ARG LLVMTARGETARCH
FROM --platform=${BUILDPLATFORM} ghcr.io/randomairborne/cross-cargo:${LLVMTARGETARCH} AS server-builder

ARG LLVMTARGETARCH

WORKDIR /build

COPY . .

RUN cargo build --release --target ${LLVMTARGETARCH}-unknown-linux-musl

FROM ghcr.io/randomairborne/asset-squisher:latest AS client-builder

COPY /assets/ /uncompressed-assets/

RUN asset-squisher /uncompressed-assets/ /assets/

FROM scratch
ARG LLVMTARGETARCH

WORKDIR /

COPY --from=server-builder /build/target/${LLVMTARGETARCH}-unknown-linux-musl/release/mcping /usr/bin/mcping
COPY --from=client-builder /assets/ /var/www/mcping/

ENV ASSET_DIR="/var/www/mcping/"

ENTRYPOINT ["/usr/bin/mcping"]
