
FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev pkgconf openssl-dev
RUN cargo build --release

FROM alpine

WORKDIR /
RUN apk add openssl
COPY --from=builder /build/target/release/mcping /usr/bin/mcping

EXPOSE 8080

CMD ["/usr/bin/mcping"]
