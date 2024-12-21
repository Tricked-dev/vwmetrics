FROM rust:1-slim AS builder

WORKDIR /usr/src/vwmetrics

COPY ./src /usr/src/vwmetrics/src
COPY Cargo.toml /usr/src/vwmetrics/Cargo.toml
COPY Cargo.lock /usr/src/vwmetrics/Cargo.lock

RUN cargo build --release

# hadolint ignore=DL3007
FROM gcr.io/distroless/cc-debian12:latest AS production

ARG BUILD_VERSION
ARG BUILD_DATE
ARG BUILD_COMMIT_SHA

LABEL org.opencontainers.image.title="VWMetrics" \
      org.opencontainers.image.version="${BUILD_VERSION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.revision="${BUILD_COMMIT_SHA}" \
      org.opencontainers.image.description="Turn your Vaultwarden database into Prometheus metrics." \
      org.opencontainers.image.documentation="https://github.com/Tricked-dev/vwmetrics" \
      org.opencontainers.image.base.name="gcr.io/distroless/cc-debian12:latest" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/Tricked-dev/vwmetrics"

COPY --from=builder --chown=nobody:nogroup /usr/src/vwmetrics/target/release/vwmetrics /usr/local/bin/vwmetrics

USER nobody

ENV HOST=0.0.0.0 PORT=3040

EXPOSE 3040/tcp

ENTRYPOINT ["/usr/local/bin/vwmetrics"]
#CMD ["--help"]