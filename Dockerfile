
FROM rust AS builder

WORKDIR /build
COPY . .

RUN apt install pkg-config libssl-dev
RUN cargo build --release

FROM debian

WORKDIR /

COPY --from=builder /build/target/release/mcping /usr/bin/mcping

EXPOSE 8080

CMD ["/usr/bin/mcping"]
