
FROM golang:alpine AS builder

WORKDIR /build
COPY . .

RUN go build -o mcping

FROM alpine

WORKDIR /mcping

COPY --from=builder /build/mcping /usr/bin/mcping
COPY ./ping.html /mcping/

EXPOSE 8080

CMD ["/usr/bin/mcping"]