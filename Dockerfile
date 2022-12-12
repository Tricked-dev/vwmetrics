FROM rust:1-slim AS builder

WORKDIR /usr/src/vwmetrics

COPY ./src /usr/src/vwmetrics/src
COPY Cargo.toml /usr/src/vwmetrics/Cargo.toml
COPY Cargo.lock /usr/src/vwmetrics/Cargo.lock

RUN cargo build --release

FROM debian:stable-slim

COPY --from=builder /usr/src/vwmetrics/target/release/vwmetrics /usr/local/bin/vwmetrics

USER www-data

ENV HOST=0.0.0.0 PORT=3040

EXPOSE 3040

CMD ["/usr/local/bin/vwmetrics"]
